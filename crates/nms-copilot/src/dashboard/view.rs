//! The dashboard's view: a plain value of strings and rows built from the model and the session.
//!
//! Building it is the only place the dashboard reads the model; drawing it is the only place ratatui appears. The redraw rule compares views, so anything that must not cause a redraw must not be in the view: every time shown here is at the coarse granularity of [`format_duration_coarse`].

use std::collections::HashSet;

use nms_core::fleet::ExpeditionState;
use nms_graph::GalaxyModel;
use nms_query::base::{Alert, BaseQuery, BaseStatus, execute_base};
use nms_query::display::{
    base_type_label, crops_cell, extraction_cell, format_ago, format_clock, format_duration_coarse,
    power_cell,
};
use nms_query::fleet::{ExpeditionRow, FleetStatus, execute_fleet};

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
    pub alerts: Vec<AlertLine>,
    pub bases: Vec<BaseRow>,
    /// `None` when the save has no freighter.
    pub fleet: Option<FleetPanel>,
    /// Oldest first.
    pub log: Vec<String>,
}

/// One alert; `fresh` marks one that arrived since the last keypress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertLine {
    pub fresh: bool,
    pub text: String,
}

/// One base, carrying the same cells the `base` overview prints, plus the time to the next harvest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseRow {
    pub name: String,
    /// Home, planet, or freighter.
    pub kind: String,
    /// Plants ready of plants planted.
    pub crops: String,
    /// When the next batch comes ready, at the coarse granularity.
    pub next: String,
    /// Units stored of capacity across the base's extraction networks, with how many are full.
    pub extraction: String,
    /// Batteries and generators.
    pub power: String,
}

/// The fleet: one row per running expedition, then the Navigator and home lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetPanel {
    pub rows: Vec<FleetRow>,
    pub offers: String,
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
}

/// Build the view at `now`. Alerts whose key is not in `seen` are marked fresh.
pub fn build(
    model: &GalaxyModel,
    session: &SessionState,
    seen: &HashSet<String>,
    now: i64,
) -> View {
    let bases = execute_base(model, &BaseQuery::default(), now)
        .unwrap_or_default()
        .iter()
        .map(base_row)
        .collect();
    let fleet = execute_fleet(model, now).ok().map(|s| fleet_panel(&s));
    let alerts = session
        .alerts
        .iter()
        .map(|alert| AlertLine {
            fresh: !seen.contains(&alert.key()),
            text: alert.text(),
        })
        .collect();
    let log = session
        .log()
        .map(|line| format!("{}  {}", format_clock(line.at), line.text))
        .collect();
    View {
        title: "NMS Copilot".to_string(),
        header: header(model, session, now),
        keys: KEYS.to_string(),
        alerts,
        bases,
        fleet,
        log,
    }
}

/// The keys of the current alerts, for the caller's `seen` set.
pub fn alert_keys(session: &SessionState) -> HashSet<String> {
    session.alerts.iter().map(Alert::key).collect()
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
        next,
        extraction: extraction_cell(status),
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
    fn test_build_shows_bases_fleet_alerts_and_log() {
        let model = fixture();
        let mut session = session_at(&model, NOW);
        session.record(NOW - 60, "Warped: Here -> There");
        let view = build(&model, &session, &HashSet::new(), NOW);

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

        assert!(!view.alerts.is_empty());
        assert!(view.alerts.iter().all(|a| a.fresh), "nothing seen yet");
        assert!(view.alerts.iter().any(|a| a.text.contains("expedition 1")));

        assert_eq!(view.log.len(), 1);
        assert!(
            view.log[0].ends_with("  Warped: Here -> There"),
            "{}",
            view.log[0]
        );
    }

    #[test]
    fn test_seen_alerts_lose_their_marker() {
        let model = fixture();
        let session = session_at(&model, NOW);
        let view = build(&model, &session, &alert_keys(&session), NOW);
        assert!(!view.alerts.is_empty());
        assert!(view.alerts.iter().all(|a| !a.fresh));
    }

    #[test]
    fn test_view_is_stable_across_seconds_and_changes_across_minutes() {
        let model = fixture();
        let session = session_at(&model, NOW);
        let seen = alert_keys(&session);
        let first = build(&model, &session, &seen, NOW);
        let soon = build(&model, &session, &seen, NOW + 30);
        assert_eq!(first, soon, "thirty seconds change nothing shown");
        let later = build(&model, &session, &seen, NOW + 300);
        assert_ne!(first, later, "five minutes move the save age");
        assert!(later.header.contains("(15m ago)"), "{}", later.header);
    }

    #[test]
    fn test_view_changes_when_a_save_is_written() {
        let model = fixture();
        let mut session = session_at(&model, NOW);
        let seen = alert_keys(&session);
        let before = build(&model, &session, &seen, NOW);
        session.save_written = Some(NOW);
        let after = build(&model, &session, &seen, NOW);
        assert_ne!(before, after);
        assert!(after.header.contains("(just now)"), "{}", after.header);
    }

    #[test]
    fn test_header_without_watcher_or_save_time() {
        let model = fixture();
        let session = SessionState::from_model(&model);
        let view = build(&model, &session, &HashSet::new(), NOW);
        assert!(
            view.header
                .ends_with("save time unknown \u{00B7} not watching"),
            "{}",
            view.header
        );
        assert!(view.alerts.is_empty(), "no refresh, no alerts");
    }

    #[test]
    fn test_truncate_counts_characters() {
        assert_eq!(truncate("short", 24), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcd\u{2026}");
        assert_eq!(truncate("ünïcödé nämé", 6), "ünïcö\u{2026}");
    }
}
