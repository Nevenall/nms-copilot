//! Watch integration for the REPL.
//!
//! Both modes of the REPL, the prompt and the dashboard, keep the model and the session current through the same two steps: [`drain`] takes what the watcher has reported, and [`sync`] applies it, snapshots save writes when automatic backups are on, re-checks the alerts against the clock, and records every notice in the session log. Notices are returned as strings for the caller to show.

use std::path::Path;
use std::sync::mpsc;

use nms_core::delta::SaveDelta;
use nms_graph::GalaxyModel;
use nms_save::backup::SnapshotOutcome;
use nms_save::locate::SaveFile;
use nms_watch::WatchEvent;
use tokio::sync::RwLock;

use crate::session::{PositionContext, SessionState, unix_secs};

/// What a mode needs to keep the model current: the watcher's receiver, when there is one, and where to write the cache.
#[derive(Debug, Clone, Copy)]
pub struct WatchContext<'a> {
    pub receiver: Option<&'a mpsc::Receiver<WatchEvent>>,
    pub cache_path: Option<&'a Path>,
    pub save_version: u32,
}

impl WatchContext<'_> {
    /// Drain and sync in one step, for callers with nothing to do in between.
    pub fn sync_now(
        &self,
        model: &RwLock<GalaxyModel>,
        session: &mut SessionState,
        now: i64,
    ) -> SyncReport {
        let events = drain(self.receiver);
        sync(
            model,
            session,
            events,
            self.cache_path,
            self.save_version,
            now,
        )
    }
}

/// Everything the watcher has reported since the last drain, in order. Takes no lock and never blocks.
pub fn drain(receiver: Option<&mpsc::Receiver<WatchEvent>>) -> Vec<WatchEvent> {
    let mut events = Vec::new();
    if let Some(receiver) = receiver {
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
    }
    events
}

/// What one sync produced.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Notices in the order they happened: save writes, snapshots, warps, discoveries, then alerts that just came due.
    pub notes: Vec<String>,
    /// How many of the notes are alerts that had not been announced before.
    pub new_alerts: usize,
}

/// Apply drained events and re-check the alerts at `now`.
///
/// Deltas are applied under the model's write lock, held only for as long as that takes; the alert refresh takes the read lock. When any delta was applied and `cache_path` is given, the cache is rewritten. Every notice is recorded in the session log before it is returned.
pub fn sync(
    model: &RwLock<GalaxyModel>,
    session: &mut SessionState,
    events: Vec<WatchEvent>,
    cache_path: Option<&Path>,
    save_version: u32,
    now: i64,
) -> SyncReport {
    let mut notes = Vec::new();
    let mut any_delta = false;
    let deltas: Vec<SaveDelta> = events
        .into_iter()
        .filter_map(|event| match event {
            WatchEvent::SaveWritten(file) => {
                notes.extend(save_written(session, &file));
                None
            }
            WatchEvent::Delta(delta) => Some(*delta),
        })
        .collect();

    if !deltas.is_empty() {
        let mut guard = model.blocking_write();
        for delta in &deltas {
            notes.extend(
                apply_and_notify(&mut guard, session, delta)
                    .into_iter()
                    .map(|n| n.trim_start().to_string()),
            );
            any_delta = true;
        }
        if any_delta && let Some(path) = cache_path {
            let data = nms_cache::extract_cache_data(&guard, save_version);
            if let Err(e) = nms_cache::write_cache(&data, path) {
                notes.push(format!("Warning: could not update cache: {e}"));
            }
        }
    }

    let alerts = session.refresh_alerts(&model.blocking_read(), now);
    let new_alerts = alerts.len();
    notes.extend(alerts);

    for note in &notes {
        session.record(now, note.clone());
    }
    SyncReport { notes, new_alerts }
}

/// Note a save the watcher saw written and snapshot it when automatic backups are on.
fn save_written(session: &mut SessionState, file: &SaveFile) -> Vec<String> {
    let mut notes = Vec::new();
    let followed = session
        .save_file
        .as_ref()
        .is_some_and(|f| f.slot() == file.slot());
    if followed {
        session.save_written = Some(unix_secs(file.modified()));
    }
    notes.push(format!(
        "Save written: slot {} {}{}",
        file.slot(),
        file.save_type(),
        if followed { "" } else { " (not followed)" }
    ));
    notes.extend(auto_backup(session, file));
    notes
}

/// Apply a delta to the model and generate human-readable notifications.
///
/// Notifications are generated *before* the delta is applied so that
/// system name lookups for the "from" position still resolve correctly.
pub fn apply_and_notify(
    model: &mut GalaxyModel,
    session: &mut SessionState,
    delta: &SaveDelta,
) -> Vec<String> {
    let mut notes = Vec::new();

    if let Some(ref moved) = delta.player_moved {
        let from_name = system_name_near(model, &moved.from);
        let to_name = system_name_near(model, &moved.to);
        notes.push(format!("  Warped: {from_name} -> {to_name}"));
    }

    for system in &delta.new_systems {
        let name = system.name.as_deref().unwrap_or("-");
        let planets = system.planets.len();
        notes.push(format!(
            "  New system: {name} ({planets} planet{})",
            if planets == 1 { "" } else { "s" }
        ));
    }

    for (sys_id, planet) in &delta.new_planets {
        let sys_name = model
            .system(sys_id)
            .and_then(|s| s.name.as_deref())
            .unwrap_or("-");
        let biome = planet
            .biome
            .map(|b| b.to_string())
            .unwrap_or_else(|| "?".into());
        let planet_name = planet.name.as_deref().unwrap_or("-");
        notes.push(format!(
            "  New scan: \"{planet_name}\" ({biome}) in {sys_name}"
        ));
    }

    for base in &delta.new_bases {
        notes.push(format!("  New base: {}", base.name));
    }

    // Apply delta to model
    model.apply_delta(delta);

    // Update session counts
    session.system_count = model.system_count();
    session.planet_count = model.planet_count();

    // Update position if player moved
    if let Some(ref moved) = delta.player_moved {
        session.position = Some(PositionContext::PlayerPosition(moved.to));
    }

    notes
}

/// Snapshot a save the watcher saw written, when automatic backups are on.
///
/// A copy that was taken is noted; an unchanged file is silent; the returned line is a warning for a failed copy.
pub fn auto_backup(session: &SessionState, file: &SaveFile) -> Option<String> {
    if !session.backup_enabled {
        return None;
    }
    let policy = session.backup_policy.as_ref()?;
    match policy.auto_snapshot(file) {
        Ok(SnapshotOutcome::Taken(snapshot)) => {
            Some(format!("Snapshot saved: {}", snapshot.folder_name()))
        }
        Ok(SnapshotOutcome::Unchanged(_)) => None,
        Err(e) => Some(format!(
            "Warning: backup of {} failed: {e}",
            file.path().display()
        )),
    }
}

/// Look up the name of the nearest system to an address.
pub fn system_name_near(model: &GalaxyModel, addr: &nms_core::address::GalacticAddress) -> String {
    model
        .nearest_systems(addr, 1)
        .first()
        .and_then(|(id, _)| model.system(id))
        .and_then(|s| s.name.as_deref())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::address::GalacticAddress;
    use nms_core::delta::PlayerMoved;
    use nms_core::system::System;
    use nms_watch::WatchEvent;

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
    fn test_apply_and_notify_empty_delta() {
        let mut model = test_model();
        let mut session = SessionState::from_model(&model);
        let delta = SaveDelta::empty();

        let notes = apply_and_notify(&mut model, &mut session, &delta);
        assert!(notes.is_empty());
    }

    #[test]
    fn test_apply_and_notify_new_system() {
        let mut model = test_model();
        let mut session = SessionState::from_model(&model);
        let delta = SaveDelta {
            new_systems: vec![System::new(
                GalacticAddress::new(500, 10, -300, 0x999, 0, 0),
                Some("New System".into()),
                None,
                None,
                vec![],
            )],
            ..SaveDelta::empty()
        };

        let notes = apply_and_notify(&mut model, &mut session, &delta);
        assert!(notes.iter().any(|n| n.contains("New system")));
        assert!(notes.iter().any(|n| n.contains("New System")));
    }

    #[test]
    fn test_apply_and_notify_player_moved() {
        let mut model = test_model();
        let mut session = SessionState::from_model(&model);
        let from = *model.player_position().unwrap();
        let to = GalacticAddress::new(999, 0, 0, 1, 0, 0);

        let delta = SaveDelta {
            player_moved: Some(PlayerMoved { from, to }),
            ..SaveDelta::empty()
        };

        let notes = apply_and_notify(&mut model, &mut session, &delta);
        assert!(notes.iter().any(|n| n.contains("Warped")));
        assert_eq!(model.player_position().unwrap().voxel_x(), 999);
    }

    #[test]
    fn test_apply_and_notify_updates_session_counts() {
        let mut model = test_model();
        let mut session = SessionState::from_model(&model);
        let sys_before = session.system_count;

        let delta = SaveDelta {
            new_systems: vec![System::new(
                GalacticAddress::new(500, 10, -300, 0x999, 0, 0),
                None,
                None,
                None,
                vec![],
            )],
            ..SaveDelta::empty()
        };

        apply_and_notify(&mut model, &mut session, &delta);
        assert_eq!(session.system_count, sys_before + 1);
    }

    #[test]
    fn test_apply_and_notify_updates_session_position() {
        let mut model = test_model();
        let mut session = SessionState::from_model(&model);
        let from = *model.player_position().unwrap();
        let to = GalacticAddress::new(42, 0, 0, 1, 0, 0);

        let delta = SaveDelta {
            player_moved: Some(PlayerMoved { from, to }),
            ..SaveDelta::empty()
        };

        apply_and_notify(&mut model, &mut session, &delta);
        match &session.position {
            Some(PositionContext::PlayerPosition(addr)) => {
                assert_eq!(addr.voxel_x(), 42);
            }
            _ => panic!("Expected PlayerPosition after warp"),
        }
    }

    #[test]
    fn test_apply_and_notify_new_base() {
        use nms_core::player::{BaseType, PlayerBase};

        let mut model = test_model();
        let mut session = SessionState::from_model(&model);
        let addr = GalacticAddress::new(100, 50, -200, 42, 0, 0);
        let base = PlayerBase::new(
            "New Outpost".into(),
            BaseType::HomePlanetBase,
            addr,
            [0.0, 0.0, 0.0],
            None,
        );

        let delta = SaveDelta {
            new_bases: vec![base],
            ..SaveDelta::empty()
        };

        let notes = apply_and_notify(&mut model, &mut session, &delta);
        assert!(notes.iter().any(|n| n.contains("New base")));
        assert!(notes.iter().any(|n| n.contains("New Outpost")));
    }

    fn shared(model: GalaxyModel) -> RwLock<GalaxyModel> {
        RwLock::new(model)
    }

    #[test]
    fn test_drain_without_receiver_or_events_is_empty() {
        assert!(drain(None).is_empty());
        let (_tx, rx) = mpsc::channel::<WatchEvent>();
        assert!(drain(Some(&rx)).is_empty());
    }

    #[test]
    fn test_sync_with_nothing_pending_changes_nothing() {
        let model = shared(test_model());
        let mut session = SessionState::from_model(&model.blocking_read());
        let count_before = session.system_count;
        let report = sync(&model, &mut session, Vec::new(), None, 0, 1_000);
        assert_eq!(report, SyncReport::default());
        assert_eq!(session.system_count, count_before);
        assert_eq!(session.log().count(), 0);
    }

    #[test]
    fn test_sync_applies_a_delta_once_and_logs_it() {
        let (tx, rx) = mpsc::channel();
        let model = shared(test_model());
        let mut session = SessionState::from_model(&model.blocking_read());
        let count_before = session.system_count;

        let delta = SaveDelta {
            new_systems: vec![System::new(
                GalacticAddress::new(500, 10, -300, 0x999, 0, 0),
                Some("Drain Test".into()),
                None,
                None,
                vec![],
            )],
            ..SaveDelta::empty()
        };
        tx.send(WatchEvent::Delta(Box::new(delta))).unwrap();

        let report = sync(&model, &mut session, drain(Some(&rx)), None, 0, 1_000);
        assert_eq!(
            report.notes,
            vec!["New system: Drain Test (0 planets)".to_string()]
        );
        assert_eq!(report.new_alerts, 0);
        assert_eq!(session.system_count, count_before + 1);
        let logged: Vec<(i64, String)> = session.log().map(|l| (l.at, l.text.clone())).collect();
        assert_eq!(
            logged,
            vec![(1_000, "New system: Drain Test (0 planets)".to_string())]
        );

        let again = sync(&model, &mut session, drain(Some(&rx)), None, 0, 1_060);
        assert!(again.notes.is_empty(), "applied exactly once");
        assert_eq!(session.system_count, count_before + 1);
        assert_eq!(session.log().count(), 1);
    }

    #[test]
    fn test_sync_announces_a_new_alert_once() {
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
        let model = shared(model);
        let mut session = SessionState::from_model(&model.blocking_read());

        let before = sync(&model, &mut session, Vec::new(), None, 0, snapshot);
        assert_eq!(before.new_alerts, 0);

        let due = sync(&model, &mut session, Vec::new(), None, 0, snapshot + 600);
        assert_eq!(due.new_alerts, 1);
        assert_eq!(due.notes, vec!["Farm: 1 Frost Crystal ready".to_string()]);
        assert_eq!(session.log().count(), 1);

        let later = sync(&model, &mut session, Vec::new(), None, 0, snapshot + 700);
        assert_eq!(later, SyncReport::default());
        assert_eq!(session.log().count(), 1);
    }

    fn backup_fixture() -> (tempfile::TempDir, SaveFile, nms_save::backup::BackupPolicy) {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("st_1");
        std::fs::create_dir_all(&account).unwrap();
        let path = account.join("save.hg");
        std::fs::write(&path, b"save").unwrap();
        let file = SaveFile::from_path(&path).unwrap();
        let policy = nms_save::backup::BackupPolicy {
            root: dir.path().join("backups"),
            keep: 20,
        };
        (dir, file, policy)
    }

    #[test]
    fn test_sync_notes_a_save_write_and_snapshots_it_when_backups_on() {
        let (_dir, file, policy) = backup_fixture();
        let model = shared(test_model());
        let mut session = SessionState::from_model(&model.blocking_read());
        session.configure_backups(policy.clone(), true, Some(file.path()));
        let written = crate::session::unix_secs(file.modified());

        let report = sync(
            &model,
            &mut session,
            vec![WatchEvent::SaveWritten(file)],
            None,
            0,
            5_000,
        );
        assert_eq!(report.notes.len(), 2, "{:?}", report.notes);
        assert_eq!(report.notes[0], "Save written: slot 1 Manual");
        assert!(
            report.notes[1].starts_with("Snapshot saved: "),
            "{}",
            report.notes[1]
        );
        assert_eq!(session.save_written, Some(written));
        assert_eq!(
            nms_save::backup::list(&policy.root, "st_1").unwrap().len(),
            1
        );
        assert_eq!(session.log().count(), 2);
    }

    #[test]
    fn test_sync_notes_a_save_write_without_snapshot_when_backups_off() {
        let (_dir, file, policy) = backup_fixture();
        let model = shared(test_model());
        let mut session = SessionState::from_model(&model.blocking_read());
        session.configure_backups(policy.clone(), false, Some(file.path()));

        let report = sync(
            &model,
            &mut session,
            vec![WatchEvent::SaveWritten(file)],
            None,
            0,
            5_000,
        );
        assert_eq!(
            report.notes,
            vec!["Save written: slot 1 Manual".to_string()]
        );
        assert!(
            nms_save::backup::list(&policy.root, "st_1")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_sync_marks_a_write_to_another_slot() {
        let (dir, file, policy) = backup_fixture();
        let other_path = dir.path().join("st_1").join("save3.hg");
        std::fs::write(&other_path, b"other").unwrap();
        let other = SaveFile::from_path(&other_path).unwrap();
        let model = shared(test_model());
        let mut session = SessionState::from_model(&model.blocking_read());
        session.configure_backups(policy, false, Some(file.path()));
        let written_before = session.save_written;

        let report = sync(
            &model,
            &mut session,
            vec![WatchEvent::SaveWritten(other)],
            None,
            0,
            5_000,
        );
        assert_eq!(
            report.notes,
            vec!["Save written: slot 2 Manual (not followed)".to_string()]
        );
        assert_eq!(
            session.save_written, written_before,
            "another slot does not move the followed save's time"
        );
    }

    #[test]
    fn test_auto_backup_reports_failure() {
        let (dir, file, _policy) = backup_fixture();
        let model = test_model();
        let mut session = SessionState::from_model(&model);
        let blocked = dir.path().join("blocked");
        std::fs::write(&blocked, b"not a folder").unwrap();
        session.configure_backups(
            nms_save::backup::BackupPolicy {
                root: blocked,
                keep: 20,
            },
            true,
            Some(file.path()),
        );
        let warning = auto_backup(&session, &file).expect("a warning");
        assert!(warning.contains("backup of"));
        assert!(warning.contains("failed"));
    }
}
