//! Session state for the interactive REPL.
//!
//! Tracks the user's current context: position, filters, and preferences.
//! Commands like `find` and `route` use this state as defaults when
//! explicit flags are not provided.

use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use nms_core::address::GalacticAddress;
use nms_core::biome::Biome;
use nms_core::galaxy::Galaxy;
use nms_graph::GalaxyModel;
use nms_query::base::{Alert, AlertSummary, current_alerts};
use nms_query::display::{format_alert_indicator, format_alert_line};
use nms_save::backup::BackupPolicy;
use nms_save::locate::SaveFile;

/// Current Unix time in seconds.
pub fn unix_now() -> i64 {
    unix_secs(SystemTime::now())
}

/// A system time as Unix seconds, zero for anything before the epoch.
pub fn unix_secs(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Lines the log panel keeps by default.
pub const DEFAULT_LOG_LINES: usize = 20;

/// One line of the session log: a notice with the time it was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Unix seconds.
    pub at: i64,
    pub text: String,
}

/// Mutable session state maintained across REPL commands.
#[derive(Debug)]
pub struct SessionState {
    /// Current reference position (for distance calculations).
    pub position: Option<PositionContext>,

    /// Active biome filter (applied to find commands when --biome is not specified).
    pub biome_filter: Option<Biome>,

    /// Default warp range in light-years (for route planning).
    pub warp_range: Option<f64>,

    /// Current galaxy context.
    pub galaxy: Galaxy,

    /// Number of systems in the model.
    pub system_count: usize,

    /// Number of planets in the model.
    pub planet_count: usize,

    /// Base alerts (crops ready, depots full) as of the last refresh.
    pub alerts: Vec<Alert>,

    /// Alert keys already announced, so a notice prints once per event.
    announced_alerts: HashSet<String>,

    /// Snapshot every save the game writes, when the watcher sees one.
    pub backup_enabled: bool,

    /// Where snapshots go and how many to keep; `None` until configured.
    pub backup_policy: Option<BackupPolicy>,

    /// The save the session follows, when it is a game save; `None` for a JSON export.
    pub save_file: Option<SaveFile>,

    /// When the followed save was last written, Unix seconds: its modification time at startup, then each write the watcher reports.
    pub save_written: Option<i64>,

    /// Whether a watcher is reporting save writes to this session.
    pub watching: bool,

    /// The most recent notices, oldest first, capped at `log_capacity`.
    log: VecDeque<LogLine>,
    log_capacity: usize,
}

/// Where the user's reference position is anchored.
#[derive(Debug, Clone)]
pub enum PositionContext {
    /// At a named base.
    Base {
        name: String,
        address: GalacticAddress,
    },
    /// At the player's save file position.
    PlayerPosition(GalacticAddress),
    /// At a manually specified address.
    Address(GalacticAddress),
}

impl PositionContext {
    pub fn address(&self) -> &GalacticAddress {
        match self {
            Self::Base { address, .. } => address,
            Self::PlayerPosition(a) | Self::Address(a) => a,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Base { name, .. } => name.clone(),
            Self::PlayerPosition(_) => "player position".into(),
            Self::Address(a) => format!("0x{:012X}", a.packed()),
        }
    }
}

impl SessionState {
    /// Initialize session state from the loaded model.
    pub fn from_model(model: &GalaxyModel) -> Self {
        let position = model
            .player_state
            .as_ref()
            .map(|ps| PositionContext::PlayerPosition(ps.current_address));

        let galaxy = model
            .player_state
            .as_ref()
            .map(|ps| Galaxy::by_index(ps.current_address.reality_index))
            .unwrap_or_else(|| Galaxy::by_index(0));

        Self {
            position,
            biome_filter: None,
            warp_range: None,
            galaxy,
            system_count: model.systems.len(),
            planet_count: model.planets.len(),
            alerts: Vec::new(),
            announced_alerts: HashSet::new(),
            backup_enabled: false,
            backup_policy: None,
            save_file: None,
            save_written: None,
            watching: false,
            log: VecDeque::new(),
            log_capacity: DEFAULT_LOG_LINES,
        }
    }

    /// Set up backups for this session: where they go, whether automatic snapshots start on, and which save is followed.
    ///
    /// When the path is a game save its modification time seeds `save_written`.
    pub fn configure_backups(
        &mut self,
        policy: BackupPolicy,
        enabled: bool,
        save_path: Option<&Path>,
    ) {
        self.backup_policy = Some(policy);
        self.backup_enabled = enabled;
        self.save_file = save_path.and_then(|p| SaveFile::from_path(p).ok());
        self.save_written = self.save_file.as_ref().map(|f| unix_secs(f.modified()));
    }

    /// Keep this many log lines; older lines are dropped as new ones arrive.
    pub fn set_log_capacity(&mut self, lines: usize) {
        self.log_capacity = lines.max(1);
        while self.log.len() > self.log_capacity {
            self.log.pop_front();
        }
    }

    /// Add a notice to the log at `now`.
    pub fn record(&mut self, now: i64, text: impl Into<String>) {
        if self.log.len() >= self.log_capacity {
            self.log.pop_front();
        }
        self.log.push_back(LogLine {
            at: now,
            text: text.into(),
        });
    }

    /// The kept notices, oldest first.
    pub fn log(&self) -> impl Iterator<Item = &LogLine> {
        self.log.iter()
    }

    /// Recompute base alerts at `now` and return notices for alerts not announced before.
    ///
    /// An alert that disappears (crops harvested, depots emptied) is forgotten, so it is announced again when it next occurs.
    pub fn refresh_alerts(&mut self, model: &GalaxyModel, now: i64) -> Vec<String> {
        self.alerts = current_alerts(model, now);
        let current: HashSet<String> = self.alerts.iter().map(Alert::key).collect();
        self.announced_alerts.retain(|key| current.contains(key));
        let mut notices = Vec::new();
        for alert in &self.alerts {
            if self.announced_alerts.insert(alert.key()) {
                notices.push(alert.text());
            }
        }
        notices
    }

    /// Compact indicator for the prompt, empty when nothing is pending.
    pub fn alert_indicator(&self) -> String {
        format_alert_indicator(&AlertSummary::from_alerts(&self.alerts))
    }

    /// One-line summary of current alerts.
    pub fn alert_line(&self) -> String {
        format_alert_line(&self.alerts)
    }

    /// Set the reference position to a named base.
    pub fn set_position_base(&mut self, name: &str, model: &GalaxyModel) -> Result<String, String> {
        let base = model
            .base(name)
            .ok_or_else(|| format!("Base not found: \"{name}\""))?;
        let address = base.address;
        let display_name = base.name.clone();
        self.position = Some(PositionContext::Base {
            name: display_name.clone(),
            address,
        });
        Ok(format!("Position set to {display_name}"))
    }

    /// Set the reference position to an explicit address.
    pub fn set_position_address(&mut self, address: GalacticAddress) -> String {
        let label = format!("0x{:012X}", address.packed());
        self.position = Some(PositionContext::Address(address));
        format!("Position set to {label}")
    }

    /// Reset position to the player's save file position.
    pub fn reset_position(&mut self, model: &GalaxyModel) -> String {
        self.position = model
            .player_state
            .as_ref()
            .map(|ps| PositionContext::PlayerPosition(ps.current_address));
        "Position reset to player location".into()
    }

    /// Set the active biome filter.
    pub fn set_biome_filter(&mut self, biome: Biome) -> String {
        let name = format!("{biome:?}");
        self.biome_filter = Some(biome);
        format!("Biome filter set to {name}")
    }

    /// Clear the active biome filter.
    pub fn clear_biome_filter(&mut self) -> &'static str {
        self.biome_filter = None;
        "Biome filter cleared"
    }

    /// Set the default warp range.
    pub fn set_warp_range(&mut self, ly: f64) -> String {
        self.warp_range = Some(ly);
        format!("Warp range set to {} ly", ly as u64)
    }

    /// Clear the warp range.
    pub fn clear_warp_range(&mut self) -> &'static str {
        self.warp_range = None;
        "Warp range cleared"
    }

    /// Reset all session state to defaults.
    pub fn reset_all(&mut self, model: &GalaxyModel) -> &'static str {
        self.reset_position(model);
        self.biome_filter = None;
        self.warp_range = None;
        "Session state reset"
    }

    /// Format the current session state for display.
    /// What bare `set` prints: everything `set`, `reset`, and `backup on|off` control.
    pub fn format_settings(&self) -> String {
        let mut lines = Vec::new();

        match &self.position {
            Some(pos) => lines.push(format!("Position:    {}", pos.label())),
            None => lines.push("Position:    unknown".into()),
        }

        match &self.biome_filter {
            Some(b) => lines.push(format!("Biome:       {b:?}")),
            None => lines.push("Biome:       (none)".into()),
        }

        match self.warp_range {
            Some(r) => lines.push(format!("Warp range:  {} ly", r as u64)),
            None => lines.push("Warp range:  (none)".into()),
        }

        match &self.backup_policy {
            Some(policy) if self.backup_enabled => {
                lines.push(format!("Backups:     on ({})", policy.root.display()))
            }
            Some(_) => lines.push("Backups:     off".into()),
            None => lines.push("Backups:     (not configured)".into()),
        }

        lines.join("\n") + "\n"
    }

    /// The current alerts as `info` prints them: one line, or one per alert.
    pub fn format_alerts(&self) -> String {
        if self.alerts.is_empty() {
            return "Alerts:      (none)\n".into();
        }
        let mut lines = vec!["Alerts:".to_string()];
        for alert in &self.alerts {
            lines.push(format!("  {}", alert.text()));
        }
        lines.join("\n") + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model() -> GalaxyModel {
        let json = r#"{
            "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 100, "VoxelY": 50, "VoxelZ": -200, "SolarSystemIndex": 42, "PlanetIndex": 0}},
                    "Units": 0, "Nanites": 0, "Specials": 0,
                    "PersistentPlayerBases": [
                        {"BaseVersion": 8, "GalacticAddress": "0x05000003AB8C07", "Position": [0.0,0.0,0.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 0, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"1","USN":"","PTK":"ST","TS":0}, "Name": "Home Base", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}
                    ]
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
                {"DD": {"UA": "0x05000003AB8C07", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}},
                {"DD": {"UA": "0x15000003AB8C07", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}}
            ]}}}
        }"#;
        let save = nms_save::parse_save(json.as_bytes()).unwrap();
        GalaxyModel::from_save(&save)
    }

    #[test]
    fn test_session_from_model() {
        let model = test_model();
        let session = SessionState::from_model(&model);
        assert!(session.position.is_some());
        assert_eq!(session.galaxy.name, "Euclid");
        assert!(session.system_count > 0);
    }

    #[test]
    fn test_set_position_base() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        let result = session.set_position_base("Home Base", &model);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Home Base"));
        match &session.position {
            Some(PositionContext::Base { name, .. }) => assert_eq!(name, "Home Base"),
            _ => panic!("Expected Base position"),
        }
    }

    #[test]
    fn test_set_position_unknown_base_errors() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        assert!(session.set_position_base("No Such Base", &model).is_err());
    }

    #[test]
    fn test_set_biome_filter() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        session.set_biome_filter(Biome::Lush);
        assert_eq!(session.biome_filter, Some(Biome::Lush));
    }

    #[test]
    fn test_clear_biome_filter() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        session.set_biome_filter(Biome::Lush);
        session.clear_biome_filter();
        assert!(session.biome_filter.is_none());
    }

    #[test]
    fn test_set_warp_range() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        session.set_warp_range(2500.0);
        assert_eq!(session.warp_range, Some(2500.0));
    }

    #[test]
    fn test_reset_all() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        session.set_biome_filter(Biome::Toxic);
        session.set_warp_range(1000.0);
        session.reset_all(&model);
        assert!(session.biome_filter.is_none());
        assert!(session.warp_range.is_none());
    }

    #[test]
    fn test_format_settings() {
        let model = test_model();
        let session = SessionState::from_model(&model);
        let output = session.format_settings();
        assert!(output.contains("Position:"));
        assert!(output.contains("Biome:       (none)"));
        assert!(output.contains("Warp range:  (none)"));
        assert!(output.contains("Backups:     (not configured)"));
        assert!(!output.contains("Alerts"), "alerts are info's, not set's");
    }

    #[test]
    fn test_position_context_label() {
        let addr = GalacticAddress::new(0, 0, 0, 0, 0, 0);
        let base = PositionContext::Base {
            name: "Test".into(),
            address: addr,
        };
        assert_eq!(base.label(), "Test");

        let player = PositionContext::PlayerPosition(addr);
        assert_eq!(player.label(), "player position");
    }

    #[test]
    fn test_set_position_address() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        let addr = GalacticAddress::new(100, 50, -200, 42, 0, 0);
        let msg = session.set_position_address(addr);
        assert!(msg.contains("Position set to"));
        assert!(matches!(
            &session.position,
            Some(PositionContext::Address(_))
        ));
    }

    #[test]
    fn test_reset_position() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        session.set_position_base("Home Base", &model).unwrap();
        session.reset_position(&model);
        assert!(matches!(
            &session.position,
            Some(PositionContext::PlayerPosition(_))
        ));
    }

    #[test]
    fn test_log_keeps_the_newest_lines() {
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        session.set_log_capacity(2);
        session.record(10, "one");
        session.record(20, "two");
        session.record(30, "three");
        let lines: Vec<(i64, &str)> = session.log().map(|l| (l.at, l.text.as_str())).collect();
        assert_eq!(lines, vec![(20, "two"), (30, "three")]);
        session.set_log_capacity(1);
        assert_eq!(session.log().count(), 1);
        assert_eq!(session.log().next().unwrap().text, "three");
    }

    #[test]
    fn test_refresh_alerts_announces_once_and_rearms() {
        use nms_core::player::{BaseType, PlayerBase};
        use nms_core::{BaseObjects, RawBaseObject};

        let snapshot = 1_789_402_087;
        let mut model = test_model();
        let farm = PlayerBase::new(
            "Farm".into(),
            BaseType::HomePlanetBase,
            GalacticAddress::new(0, 0, 0, 1, 0, 0),
            [0.0; 3],
            None,
        )
        .with_objects(BaseObjects::decode([RawBaseObject {
            object_id: "^SNOWPLANT",
            timestamp: snapshot,
            user_data: 3000 << 32,
            ..Default::default()
        }]));
        model.insert_base(farm);
        let mut session = SessionState::from_model(&model);

        assert!(session.refresh_alerts(&model, snapshot).is_empty());
        assert_eq!(session.alert_indicator(), "");
        assert!(session.format_alerts().contains("Alerts:      (none)"));

        let notices = session.refresh_alerts(&model, snapshot + 600);
        assert_eq!(notices, vec!["Farm: 1 Frost Crystal ready".to_string()]);
        assert!(
            session.refresh_alerts(&model, snapshot + 700).is_empty(),
            "announced once"
        );
        assert_eq!(session.alert_indicator(), "\u{1F331} 1 ready");
        assert!(
            session
                .format_alerts()
                .contains("Farm: 1 Frost Crystal ready")
        );
        assert_eq!(
            session.alert_line(),
            "Ready: 1 Frost Crystal at Farm. Full depots: none."
        );

        // Harvested: the plant restarts, so the alert clears and re-arms.
        let farm = PlayerBase::new(
            "Farm".into(),
            BaseType::HomePlanetBase,
            GalacticAddress::new(0, 0, 0, 1, 0, 0),
            [0.0; 3],
            None,
        )
        .with_objects(BaseObjects::decode([RawBaseObject {
            object_id: "^SNOWPLANT",
            timestamp: snapshot + 700,
            user_data: 0,
            ..Default::default()
        }]));
        model.insert_base(farm);
        assert!(session.refresh_alerts(&model, snapshot + 800).is_empty());
        assert_eq!(session.alert_indicator(), "");
        assert_eq!(
            session.refresh_alerts(&model, snapshot + 700 + 3600).len(),
            1,
            "re-armed"
        );
    }
}
