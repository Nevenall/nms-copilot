//! The dashboard's view: a plain value of strings and rows built from the model and the session.
//!
//! Building it is the only place the dashboard reads the model; drawing it is the only place ratatui appears. The redraw rule compares views, so anything that must not cause a redraw must not be in the view: every time shown here is at the coarse granularity of [`format_duration_coarse`].

use nms_core::address::{GalacticAddress, PortalAddress};
use nms_core::fleet::ExpeditionState;
use nms_core::system::SystemId;
use nms_graph::GalaxyModel;
use nms_query::base::{BaseQuery, BaseStatus, execute_base};
use nms_query::display::{
    base_type_label, crops_cell, extraction_cell, format_ago, format_clock, format_distance,
    format_duration_coarse, power_cell, thousands,
};
use nms_query::fleet::{ExpeditionRow, FleetStatus, execute_fleet};
use nms_query::inventory::holdings_summary;

use crate::session::{PositionContext, SessionState};
use crate::watch::system_name_near;

/// The keys the header advertises.
pub const KEYS: &str = "q quit  : prompt";

/// The longest base name shown before it is cut.
const NAME_WIDTH: usize = 24;

/// Everything on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub title: String,
    /// Galaxy, position, the last save write, and what is watched.
    pub header: String,
    pub keys: String,
    pub player: PlayerPanel,
    pub bases: Vec<BaseRow>,
    /// `None` when the save has no freighter.
    pub fleet: Option<FleetPanel>,
    /// Oldest first.
    pub log: Vec<String>,
}

/// What the save knows about the player: where they are, what they are carrying, and how much of the galaxy they have seen.
///
/// The facts are laid out down the left column and then down the right, so the first of each half is the one a squeezed screen keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerPanel {
    pub facts: Vec<Fact>,
}

/// One labelled detail about the player.
pub type Fact = (String, String);

impl PlayerPanel {
    /// Rows the facts take when laid out two to a line.
    pub fn rows(&self) -> usize {
        self.facts.len().div_ceil(2)
    }

    /// The pair on line `row`: the left fact and, when there is one, the right.
    pub fn line(&self, row: usize) -> (Option<&Fact>, Option<&Fact>) {
        (self.facts.get(row), self.facts.get(self.rows() + row))
    }
}

/// One base, carrying the same cells the `base` overview prints, plus the time to the next harvest.
///
/// The `_wants_you` flags mark what the alerts used to say in a section of their own: a cell carrying one is drawn in the attention colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseRow {
    pub name: String,
    /// Home, planet, or freighter.
    pub kind: String,
    /// Plants ready of plants planted.
    pub crops: String,
    /// Something is ready to harvest.
    pub crops_want_you: bool,
    /// When the next batch comes ready, at the coarse granularity.
    pub next: String,
    /// Units stored of capacity across the base's extraction networks, with how many are full.
    pub extraction: String,
    /// A network is at capacity and stops producing until it is emptied.
    pub extraction_wants_you: bool,
    /// Batteries and generators.
    pub power: String,
}

/// The fleet: one row per running expedition, then the Navigator and home lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetPanel {
    pub rows: Vec<FleetRow>,
    pub offers: String,
    /// The offer day has rolled and a fresh set waits at the Navigator.
    pub offers_want_you: bool,
    pub rooms: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRow {
    pub number: String,
    pub kind: String,
    pub length: String,
    pub frigates: String,
    pub events: String,
    pub elapsed: String,
    pub status: String,
    /// The expedition is holding for a decision or back and awaiting its debrief.
    pub status_wants_you: bool,
}

/// Build the view at `now`.
pub fn build(model: &GalaxyModel, session: &SessionState, now: i64) -> View {
    let bases = execute_base(model, &BaseQuery::default(), now)
        .unwrap_or_default()
        .iter()
        .map(base_row)
        .collect();
    let fleet = execute_fleet(model, now).ok().map(|s| fleet_panel(&s));
    let player = player_panel(model);
    let log = session
        .log()
        .map(|line| format!("{}  {}", format_clock(line.at), line.text))
        .collect();
    View {
        title: "NMS Copilot".to_string(),
        header: header(model, session, now),
        keys: KEYS.to_string(),
        player,
        bases,
        fleet,
        log,
    }
}

fn header(model: &GalaxyModel, session: &SessionState, now: i64) -> String {
    let save = match session.save_written {
        Some(written) => format!(
            "save {} ({})",
            format_clock(written),
            format_ago(now - written)
        ),
        None => "save time unknown".to_string(),
    };
    let watching = match (&session.save_file, session.watching) {
        (Some(file), true) => format!("watching slot {}", file.slot()),
        (None, true) => "watching".to_string(),
        (_, false) => "not watching".to_string(),
    };
    [
        session.galaxy.name.to_string(),
        whereabouts(model, session),
        save,
        watching,
    ]
    .join(" \u{00B7} ")
}

/// The player's own details, in the order they are shown.
fn player_panel(model: &GalaxyModel) -> PlayerPanel {
    let known = (
        "Known".to_string(),
        format!(
            "{} systems \u{00B7} {} planets",
            model.system_count(),
            model.planet_count()
        ),
    );
    let bases = ("Bases".to_string(), model.base_count().to_string());
    let Some(state) = model.player_state.as_ref() else {
        return PlayerPanel {
            facts: vec![known, bases],
        };
    };
    let here = state.current_address;
    let system = match model.system(&SystemId::from_address(&here)) {
        Some(system) => {
            let name = system
                .name
                .clone()
                .unwrap_or_else(|| format!("system {}", here.solar_system_index()));
            let planets = system.planets.len();
            format!(
                "{name} \u{00B7} {planets} planet{}",
                if planets == 1 { "" } else { "s" }
            )
        }
        None => "not in the atlas".to_string(),
    };
    let freighter = match state.freighter_address {
        Some(addr) if addr.same_system(&here) => "in this system".to_string(),
        Some(addr) => system_label(model, &addr),
        None => "-".to_string(),
    };
    let from = match state.previous_address {
        Some(addr) => system_label(model, &addr),
        None => "-".to_string(),
    };
    PlayerPanel {
        facts: vec![
            ("System".to_string(), system),
            (
                "Address".to_string(),
                PortalAddress::from_galactic_address(&here).to_hex_string(),
            ),
            (
                "From centre".to_string(),
                format_distance(here.distance_to_core_ly()),
            ),
            ("Warped from".to_string(), from),
            known,
            bases,
            ("Units".to_string(), thousands(state.units)),
            ("Nanites".to_string(), thousands(state.nanites)),
            ("Quicksilver".to_string(), thousands(state.quicksilver)),
            (
                "Inventory".to_string(),
                model
                    .holdings
                    .as_ref()
                    .map(holdings_summary)
                    .unwrap_or_else(|| "-".to_string()),
            ),
            ("Ships".to_string(), ships_fact(model)),
            ("Freighter".to_string(), freighter),
        ],
    }
}

/// How many ships the player owns and which one they are flying: `10 · Starbird (Exotic S)`.
fn ships_fact(model: &GalaxyModel) -> String {
    let Some(holdings) = model.holdings.as_ref() else {
        return "-".to_string();
    };
    match holdings.primary_ship() {
        Some(ship) => format!(
            "{} \u{00B7} {} ({} {})",
            holdings.ships.len(),
            ship.label(),
            ship.type_label(),
            ship.class_label()
        ),
        None if holdings.ships.is_empty() => "-".to_string(),
        None => holdings.ships.len().to_string(),
    }
}

/// A system's name, or its portal address when the atlas has no name for it.
fn system_label(model: &GalaxyModel, addr: &GalacticAddress) -> String {
    model
        .system(&SystemId::from_address(addr))
        .and_then(|system| system.name.clone())
        .unwrap_or_else(|| PortalAddress::from_galactic_address(addr).to_hex_string())
}

fn whereabouts(model: &GalaxyModel, session: &SessionState) -> String {
    match &session.position {
        Some(PositionContext::Base { name, .. }) => format!("at {name}"),
        Some(PositionContext::PlayerPosition(addr)) => {
            format!("in {}", system_name_near(model, addr))
        }
        Some(position @ PositionContext::Address(_)) => format!("at {}", position.label()),
        None => "position unknown".to_string(),
    }
}

fn base_row(status: &BaseStatus) -> BaseRow {
    let name = if status.base.name.is_empty() {
        format!("({})", base_type_label(&status.base.base_type))
    } else {
        status.base.name.clone()
    };
    let next = if status.crops_total == 0 {
        "-".to_string()
    } else if status.crops_ready > 0 {
        "now".to_string()
    } else {
        status
            .crops
            .iter()
            .filter_map(|row| row.next_ready_secs())
            .min()
            .map(format_duration_coarse)
            .unwrap_or_else(|| "?".to_string())
    };
    BaseRow {
        name: truncate(&name, NAME_WIDTH),
        kind: base_type_label(&status.base.base_type),
        crops: crops_cell(status),
        crops_want_you: status.crops_ready > 0,
        next,
        extraction: extraction_cell(status),
        extraction_wants_you: status.full_networks() > 0,
        power: power_cell(&status.power),
    }
}

fn fleet_panel(status: &FleetStatus) -> FleetPanel {
    let rows = status.expeditions.iter().map(fleet_row).collect();
    let offers = if status.offers.day == 0 {
        "Offers: none recorded yet".to_string()
    } else if status.offers.refreshed {
        format!(
            "Offers: {} new waiting at the Navigator",
            status.offers.left
        )
    } else {
        format!(
            "Offers: {} of {} left \u{00B7} new in {} (00:00 UTC)",
            status.offers.left,
            status.offers.per_day,
            format_duration_coarse(status.offers.secs_until_refresh)
        )
    };
    let rooms = format!(
        "Rooms: {} of {} free \u{00B7} {} of {} frigates at home",
        status.rooms_free,
        status.command_rooms,
        status.frigates_home,
        status.frigates.len()
    );
    FleetPanel {
        rows,
        offers,
        offers_want_you: status.offers.refreshed && status.offers.day > 0,
        rooms,
    }
}

fn fleet_row(row: &ExpeditionRow) -> FleetRow {
    let damaged = row.damaged();
    let frigates = if damaged > 0 {
        format!("{} ({damaged} dmg)", row.expedition.frigates.len())
    } else {
        row.expedition.frigates.len().to_string()
    };
    let status = match row.state {
        ExpeditionState::Waiting => match row.expedition.waiting_since() {
            Some(since) => format!("waiting since {}", format_clock(since)),
            None => "waiting for you".to_string(),
        },
        ExpeditionState::Complete => "returned, debrief".to_string(),
        ExpeditionState::Running => {
            let mut text = match row.estimate_remaining_secs {
                Some(left) => format!("about {} left", format_duration_coarse(left)),
                None => "under way".to_string(),
            };
            if damaged > 0 {
                text.push_str(&format!(", {damaged} damaged"));
            }
            text
        }
    };
    FleetRow {
        number: row.number.to_string(),
        kind: row.category_label().to_string(),
        length: row.duration_label().to_string(),
        frigates,
        events: format!("{}/{}", row.expedition.resolved(), row.expedition.total()),
        elapsed: format_duration_coarse(row.elapsed_secs),
        status,
        status_wants_you: matches!(
            row.state,
            ExpeditionState::Waiting | ExpeditionState::Complete
        ),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{head}\u{2026}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The multi-system fixture: three bases (one with crops, one empty, the freighter) and one expedition holding for a decision.
    fn fixture() -> GalaxyModel {
        let json = include_str!("../../../../data/test/multi_system_save.json");
        let save = nms_save::parse_save(json.as_bytes()).unwrap();
        GalaxyModel::from_save(&save)
    }

    /// An hour after the fixture's snapshot: its crops are ready and its expedition still waits.
    const NOW: i64 = 1_789_402_087 + 3_600;

    fn session_at(model: &GalaxyModel, now: i64) -> SessionState {
        let mut session = SessionState::from_model(model);
        session.save_written = Some(now - 12 * 60);
        session.watching = true;
        session.refresh_alerts(model, now);
        session
    }

    #[test]
    fn test_build_shows_bases_fleet_and_log() {
        let model = fixture();
        let mut session = session_at(&model, NOW);
        session.record(NOW - 60, "Warped: Here -> There");
        let view = build(&model, &session, NOW);

        assert_eq!(view.title, "NMS Copilot");
        assert_eq!(view.keys, KEYS);
        assert!(
            view.header.starts_with("Euclid \u{00B7} in "),
            "{}",
            view.header
        );
        assert!(
            view.header.contains("(10m ago) \u{00B7} watching"),
            "{}",
            view.header
        );

        let names: Vec<&str> = view.bases.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Lush Haven", "Frost Outpost", "Home Freighter"],
            "alerting base first, then by name"
        );
        let haven = &view.bases[0];
        assert_eq!(haven.kind, "home");
        assert_eq!(
            haven.crops, "2 / 5",
            "the same cell the base overview prints"
        );
        assert_eq!(haven.next, "now");
        assert!(haven.extraction.contains("FULL"), "{}", haven.extraction);
        let outpost = &view.bases[1];
        assert_eq!(
            (
                outpost.kind.as_str(),
                outpost.crops.as_str(),
                outpost.next.as_str(),
                outpost.extraction.as_str(),
                outpost.power.as_str()
            ),
            ("planet", "-", "-", "-", "-")
        );

        let fleet = view.fleet.as_ref().expect("the fixture has a freighter");
        assert_eq!(fleet.rows.len(), 1);
        let row = &fleet.rows[0];
        assert_eq!(row.number, "1");
        assert_eq!(
            row.kind, "Trade",
            "the save's Diplomacy category is the Trade expedition type"
        );
        assert_eq!(row.length, "Very long");
        assert_eq!(row.frigates, "3");
        assert_eq!(row.events, "2/3");
        assert!(row.status.starts_with("waiting since "), "{}", row.status);
        assert!(fleet.offers.starts_with("Offers: "), "{}", fleet.offers);
        assert!(
            fleet.rooms.contains("of 4 frigates at home"),
            "{}",
            fleet.rooms
        );

        assert_eq!(view.log.len(), 1);
        assert!(
            view.log[0].ends_with("  Warped: Here -> There"),
            "{}",
            view.log[0]
        );
    }

    #[test]
    fn test_player_facts_read_from_the_save() {
        let model = fixture();
        let session = session_at(&model, NOW);
        let view = build(&model, &session, NOW);
        let facts: Vec<(&str, &str)> = view
            .player
            .facts
            .iter()
            .map(|(label, value)| (label.as_str(), value.as_str()))
            .collect();

        let of = |label: &str| {
            facts
                .iter()
                .find(|(name, _)| *name == label)
                .unwrap_or_else(|| panic!("no {label} fact in {facts:?}"))
                .1
        };
        assert_eq!(of("Units"), "25,000,000");
        assert_eq!(of("Nanites"), "50,000");
        assert_eq!(of("Quicksilver"), "1,500");
        assert_eq!(
            of("Warped from"),
            "-",
            "the fixture records no previous system"
        );
        assert_eq!(of("Freighter"), "-", "and no freighter");
        assert_eq!(
            of("Inventory"),
            "Exosuit 4/93",
            "occupied of unlocked, and no storage container is full"
        );
        assert_eq!(of("Ships"), "2 \u{00B7} Starbird (Exotic S)");
        assert_eq!(of("Bases"), "3");
        assert!(of("Known").ends_with("planets"), "{}", of("Known"));
        assert!(of("From centre").ends_with("ly"), "{}", of("From centre"));
        assert_eq!(of("Address").len(), 12, "a portal address is twelve digits");

        // Twelve facts make six lines: where the player is on the left, what they hold on the right.
        assert_eq!(view.player.rows(), 6);
        let (left, right) = view.player.line(0);
        assert_eq!(left.map(|f| f.0.as_str()), Some("System"));
        assert_eq!(right.map(|f| f.0.as_str()), Some("Units"));
    }

    #[test]
    fn test_a_model_with_no_player_still_says_what_it_knows() {
        let model = GalaxyModel::new();
        let session = SessionState::from_model(&model);
        let view = build(&model, &session, NOW);
        let labels: Vec<&str> = view
            .player
            .facts
            .iter()
            .map(|(label, _)| label.as_str())
            .collect();
        assert_eq!(labels, vec!["Known", "Bases"]);
    }

    #[test]
    fn test_what_wants_the_player_is_flagged_on_the_cell() {
        let model = fixture();
        let session = session_at(&model, NOW);
        let view = build(&model, &session, NOW);

        let haven = &view.bases[0];
        assert!(haven.crops_want_you, "Lush Haven has plants ready at NOW");
        assert!(haven.extraction_wants_you, "and a network at capacity");
        let outpost = &view.bases[1];
        assert!(!outpost.crops_want_you);
        assert!(!outpost.extraction_wants_you);

        let fleet = view.fleet.as_ref().expect("the fixture has a freighter");
        assert!(
            fleet.rows[0].status_wants_you,
            "the expedition is holding for a decision"
        );
    }

    #[test]
    fn test_view_is_stable_across_seconds_and_changes_across_minutes() {
        let model = fixture();
        let session = session_at(&model, NOW);
        let first = build(&model, &session, NOW);
        let soon = build(&model, &session, NOW + 30);
        assert_eq!(first, soon, "thirty seconds change nothing shown");
        let later = build(&model, &session, NOW + 300);
        assert_ne!(first, later, "five minutes move the save age");
        assert!(later.header.contains("(15m ago)"), "{}", later.header);
    }

    #[test]
    fn test_view_changes_when_a_save_is_written() {
        let model = fixture();
        let mut session = session_at(&model, NOW);
        let before = build(&model, &session, NOW);
        session.save_written = Some(NOW);
        let after = build(&model, &session, NOW);
        assert_ne!(before, after);
        assert!(after.header.contains("(just now)"), "{}", after.header);
    }

    #[test]
    fn test_header_without_watcher_or_save_time() {
        let model = fixture();
        let session = SessionState::from_model(&model);
        let view = build(&model, &session, NOW);
        assert!(
            view.header
                .ends_with("save time unknown \u{00B7} not watching"),
            "{}",
            view.header
        );
    }

    #[test]
    fn test_truncate_counts_characters() {
        assert_eq!(truncate("short", 24), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcd\u{2026}");
        assert_eq!(truncate("ünïcödé nämé", 6), "ünïcö\u{2026}");
    }
}
