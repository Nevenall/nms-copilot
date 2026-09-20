//! Fleet status queries: running expeditions, the Navigator's offers, and the frigates.
//!
//! Time-dependent values take `now` as Unix seconds; nothing here reads the clock.

use nms_core::fleet::{Expedition, ExpeditionState, Fleet, Frigate, OFFERS_PER_DAY};
use nms_core::system::System;
use nms_graph::spatial::SystemId;
use nms_graph::{GalaxyModel, GraphError};

use crate::base::{Alert, AlertKind};

/// What the `fleet` command should show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetTarget {
    /// Every running expedition and the Navigator line.
    Overview,
    /// One expedition in full, by its 1-based number in the overview.
    Expedition(usize),
}

impl FleetTarget {
    /// Parse the command's optional argument: nothing, or an expedition number. The frigates are `list frigates`.
    pub fn parse(arg: Option<&str>) -> Result<Self, String> {
        match arg.map(str::trim) {
            None | Some("") => Ok(Self::Overview),
            Some(s) => s
                .parse::<usize>()
                .ok()
                .filter(|n| *n >= 1)
                .map(Self::Expedition)
                .ok_or_else(|| {
                    format!("expected an expedition number, got \"{s}\" (the frigates are `list frigates`)")
                }),
        }
    }
}

/// Everything the fleet views show.
#[derive(Debug, Clone)]
pub struct FleetStatus {
    pub now: i64,
    /// Running expeditions in save order.
    pub expeditions: Vec<ExpeditionRow>,
    /// Every frigate in save order.
    pub frigates: Vec<FrigateRow>,
    pub offers: OfferStatus,
    pub command_rooms: u32,
    pub rooms_free: u32,
    pub frigates_home: usize,
}

impl FleetStatus {
    /// The expedition with this 1-based number.
    pub fn expedition(&self, number: usize) -> Option<&ExpeditionRow> {
        self.expeditions.iter().find(|e| e.number == number)
    }

    /// Expeditions holding for the player.
    pub fn waiting(&self) -> usize {
        self.expeditions
            .iter()
            .filter(|e| e.state == ExpeditionState::Waiting)
            .count()
    }

    /// Expeditions back and awaiting their debrief.
    pub fn returned(&self) -> usize {
        self.expeditions
            .iter()
            .filter(|e| e.state == ExpeditionState::Complete)
            .count()
    }
}

/// One running expedition with its derived timing.
#[derive(Debug, Clone)]
pub struct ExpeditionRow {
    /// 1-based position in the save's list; what `fleet N` refers to.
    pub number: usize,
    pub expedition: Expedition,
    pub state: ExpeditionState,
    /// Active seconds so far, not counting the current hold.
    pub elapsed_secs: i64,
    /// Seconds the fleet has been holding, while waiting.
    pub waiting_secs: Option<i64>,
    /// Rough active seconds left, from the run's own pace; see [`Expedition::estimate_remaining_secs`].
    pub estimate_remaining_secs: Option<i64>,
    /// The fleet's current system when it is in the atlas.
    pub system: Option<System>,
    pub distance_from_player: Option<f64>,
    /// Assigned frigates in the expedition's order.
    pub frigates: Vec<FrigateRow>,
    /// Where each event happened: the system's name when it is in the atlas, its hex address otherwise, `-` until reached.
    pub event_places: Vec<String>,
}

impl ExpeditionRow {
    pub fn category_label(&self) -> &str {
        self.expedition.category_label()
    }

    pub fn duration_label(&self) -> &str {
        self.expedition.duration_label()
    }

    /// Frigates the expedition reports as damaged.
    pub fn damaged(&self) -> usize {
        self.expedition.damaged.len()
    }
}

/// One frigate and where it is.
#[derive(Debug, Clone)]
pub struct FrigateRow {
    pub frigate: Frigate,
    /// 1-based number of the running expedition it is on, or `None` at home.
    pub out_on: Option<usize>,
}

/// The state of the Navigator's daily offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferStatus {
    /// UTC day number the save recorded.
    pub day: i64,
    /// Unix seconds of the next 00:00 UTC after that day.
    pub next_refresh: i64,
    /// Seconds until that refresh, zero once it has passed.
    pub secs_until_refresh: i64,
    /// The day has rolled since the save; a fresh set of five awaits on the next load.
    pub refreshed: bool,
    pub launched_today: usize,
    pub left: usize,
    pub per_day: usize,
}

/// Report on the fleet at `now`.
pub fn execute_fleet(model: &GalaxyModel, now: i64) -> Result<FleetStatus, GraphError> {
    let fleet = model.fleet.as_ref().ok_or(GraphError::NoFleet)?;
    Ok(fleet_status(model, fleet, now))
}

/// Build the status of a fleet.
pub fn fleet_status(model: &GalaxyModel, fleet: &Fleet, now: i64) -> FleetStatus {
    let frigate_row = |frigate: &Frigate| FrigateRow {
        frigate: frigate.clone(),
        out_on: fleet.expedition_of(frigate.index).map(|p| p + 1),
    };
    let expeditions = fleet
        .expeditions
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let system = e
                .location
                .and_then(|addr| model.system(&SystemId::from_address(&addr)))
                .cloned();
            let distance_from_player = e
                .location
                .and_then(|addr| model.player_position().map(|pos| pos.distance_ly(&addr)));
            let event_places = e
                .events
                .iter()
                .map(|ev| match ev.location {
                    Some(addr) => model
                        .system(&SystemId::from_address(&addr))
                        .and_then(|s| s.name.clone())
                        .unwrap_or_else(|| format!("{:012X}", addr.packed())),
                    None => "-".to_string(),
                })
                .collect();
            ExpeditionRow {
                number: i + 1,
                expedition: e.clone(),
                state: e.state(),
                elapsed_secs: e.active_secs(now),
                waiting_secs: e.waiting_secs(now),
                estimate_remaining_secs: e.estimate_remaining_secs(now),
                system,
                distance_from_player,
                frigates: fleet.frigates_on(i).into_iter().map(frigate_row).collect(),
                event_places,
            }
        })
        .collect();
    let frigates: Vec<FrigateRow> = fleet.frigates.iter().map(frigate_row).collect();
    let frigates_home = frigates.iter().filter(|f| f.out_on.is_none()).count();
    let next_refresh = fleet.next_refresh();
    FleetStatus {
        now,
        expeditions,
        frigates,
        offers: OfferStatus {
            day: fleet.offer_day,
            next_refresh,
            secs_until_refresh: (next_refresh - now).max(0),
            refreshed: fleet.offers_refreshed(now),
            launched_today: fleet.launched_today(now),
            left: fleet.offers_left(now),
            per_day: OFFERS_PER_DAY,
        },
        command_rooms: fleet.command_rooms,
        rooms_free: fleet.rooms_free(),
        frigates_home,
    }
}

/// Alerts for an already-computed status: expeditions waiting for the player or back from their run, and a rolled offer day.
pub fn fleet_alerts(status: &FleetStatus) -> Vec<Alert> {
    let mut alerts = Vec::new();
    for row in &status.expeditions {
        match row.state {
            ExpeditionState::Waiting => alerts.push(Alert {
                base: "Fleet".into(),
                kind: AlertKind::FleetWaiting {
                    seed: row.expedition.seed,
                    number: row.number,
                    category: row.category_label().to_string(),
                    since: row.expedition.waiting_since(),
                },
            }),
            ExpeditionState::Complete => alerts.push(Alert {
                base: "Fleet".into(),
                kind: AlertKind::FleetReturned {
                    seed: row.expedition.seed,
                    number: row.number,
                    category: row.category_label().to_string(),
                },
            }),
            ExpeditionState::Running => {}
        }
    }
    // A save that has never used the Navigator records no offer day; nothing to announce there.
    if status.offers.refreshed && status.offers.day > 0 {
        alerts.push(Alert {
            base: "Fleet".into(),
            kind: AlertKind::NewOffers {
                count: status.offers.left,
                day: status.offers.day,
            },
        });
    }
    alerts
}

/// Every current fleet alert, or none when the model has no fleet.
pub fn current_fleet_alerts(model: &GalaxyModel, now: i64) -> Vec<Alert> {
    match execute_fleet(model, now) {
        Ok(status) => fleet_alerts(&status),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::address::GalacticAddress;
    use nms_core::fleet::{
        DurationClass, Event, ExpeditionCategory, FrigateClass, FrigateGrade, SECS_PER_DAY,
    };

    fn event(intervention: bool, success: bool, location: Option<GalacticAddress>) -> Event {
        Event {
            id: "^DIPLOMATIC_2".into(),
            intervention_id: if intervention {
                "^INT_TRADING_CHOOSE_FUND".into()
            } else {
                "^".into()
            },
            is_intervention: intervention,
            success,
            location,
            affected: Vec::new(),
        }
    }

    fn frigate(index: u32, class: FrigateClass) -> Frigate {
        Frigate {
            index,
            name: String::new(),
            class: Some(class),
            class_raw: format!("{class:?}"),
            race: "Traders".into(),
            grade: Some(FrigateGrade::A),
            stats: vec![10; 11],
            traits: vec!["^COMBAT_PRI".into()],
            damage_taken: 0,
            times_damaged: 0,
            repairs: 0,
            expeditions: 1,
            successes: 10,
            failures: 0,
            home: None,
        }
    }

    /// Two expeditions: one waiting at its intervention, one back home.
    fn model_with_fleet() -> GalaxyModel {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/test/multi_system_save.json"
        ))
        .unwrap();
        let save = nms_save::parse_save(json.as_bytes()).unwrap();
        let mut model = GalaxyModel::from_save(&save);
        let here = model.player_position().copied().unwrap();
        let waiting = Expedition {
            seed: 0x5F98_B405_C7B3_0D18,
            name: String::new(),
            category: Some(ExpeditionCategory::Diplomacy),
            category_raw: "Diplomacy".into(),
            duration: Some(DurationClass::VeryLong),
            duration_raw: "VeryLong".into(),
            start: 1_789_344_506,
            pause: 1_789_396_923,
            speed_multiplier: 1.0,
            location: Some(here),
            last_move: 1_789_398_099,
            frigates: vec![0, 1],
            active: vec![0, 1],
            damaged: vec![1],
            destroyed: Vec::new(),
            events: vec![
                event(false, true, Some(here)),
                event(false, true, Some(here)),
                event(true, false, None),
                event(false, false, None),
            ],
            next_event: 2,
            intervention_pending: true,
            successes: 2,
            failures: 0,
        };
        let returned = Expedition {
            seed: 0x5F98_B405_C7B3_0D1B,
            category: Some(ExpeditionCategory::Mining),
            category_raw: "Mining".into(),
            pause: 0,
            location: None,
            frigates: vec![2],
            active: vec![2],
            damaged: Vec::new(),
            events: vec![
                event(false, true, Some(here)),
                event(false, false, Some(here)),
            ],
            next_event: 2,
            intervention_pending: false,
            successes: 1,
            failures: 1,
            ..waiting.clone()
        };
        model.fleet = Some(Fleet {
            expeditions: vec![waiting, returned],
            frigates: vec![
                frigate(0, FrigateClass::Combat),
                frigate(1, FrigateClass::Diplomacy),
                frigate(2, FrigateClass::Mining),
                frigate(3, FrigateClass::Support),
            ],
            offer_day: 20710,
            launched_today: vec![0x5F98_B405_C7B3_0D18, 0x5F98_B405_C7B3_0D1B],
            command_rooms: 3,
        });
        model
    }

    #[test]
    fn test_fleet_target_parse() {
        assert_eq!(FleetTarget::parse(None), Ok(FleetTarget::Overview));
        assert_eq!(FleetTarget::parse(Some(" ")), Ok(FleetTarget::Overview));
        assert!(
            FleetTarget::parse(Some("frigates")).is_err(),
            "the frigates moved to `list frigates`"
        );
        assert_eq!(
            FleetTarget::parse(Some("2")),
            Ok(FleetTarget::Expedition(2))
        );
        assert!(FleetTarget::parse(Some("0")).is_err());
        assert!(
            FleetTarget::parse(Some("ships"))
                .unwrap_err()
                .contains("ships")
        );
    }

    #[test]
    fn test_execute_fleet_rows_offers_and_rooms() {
        let model = model_with_fleet();
        let now = 20710 * SECS_PER_DAY + 16 * 3600; // 16:00 UTC on the offer day, after the hold began at 14:42
        let status = execute_fleet(&model, now).unwrap();
        assert_eq!(status.expeditions.len(), 2);
        assert_eq!(status.waiting(), 1);
        assert_eq!(status.returned(), 1);

        let first = status.expedition(1).unwrap();
        assert_eq!(first.state, ExpeditionState::Waiting);
        assert_eq!(first.category_label(), "Trade");
        assert_eq!(first.duration_label(), "Very long");
        assert_eq!(first.elapsed_secs, 52_417);
        assert_eq!(first.waiting_secs, Some(now - 1_789_396_923));
        assert_eq!(first.estimate_remaining_secs, Some(52_417));
        assert!(
            first.system.is_some(),
            "the player's system is in the atlas"
        );
        assert_eq!(first.distance_from_player, Some(0.0));
        assert_eq!(first.frigates.len(), 2);
        assert_eq!(first.frigates[1].out_on, Some(1));
        assert_eq!(first.damaged(), 1);
        assert_eq!(first.event_places.len(), 4);
        assert_ne!(first.event_places[0], "-");
        assert_eq!(first.event_places[2], "-");

        let second = status.expedition(2).unwrap();
        assert_eq!(second.state, ExpeditionState::Complete);
        assert_eq!(second.category_label(), "Industrial");
        assert!(second.system.is_none());
        assert_eq!(second.estimate_remaining_secs, Some(0));
        assert!(status.expedition(3).is_none());

        assert_eq!(status.offers.day, 20710);
        assert!(!status.offers.refreshed);
        assert_eq!(status.offers.launched_today, 2);
        assert_eq!(status.offers.left, 3);
        assert_eq!(status.offers.secs_until_refresh, 8 * 3600);
        assert_eq!(status.command_rooms, 3);
        assert_eq!(status.rooms_free, 1);
        assert_eq!(status.frigates.len(), 4);
        assert_eq!(status.frigates_home, 1);
        assert_eq!(status.frigates[3].out_on, None);
        assert_eq!(status.frigates[2].out_on, Some(2));
    }

    #[test]
    fn test_fleet_alerts_and_refreshed_day() {
        let model = model_with_fleet();
        let during = 20710 * SECS_PER_DAY + 8 * 3600;
        let status = execute_fleet(&model, during).unwrap();
        let alerts = fleet_alerts(&status);
        assert_eq!(alerts.len(), 2);
        assert!(matches!(
            alerts[0].kind,
            AlertKind::FleetWaiting { number: 1, .. }
        ));
        assert!(matches!(
            alerts[1].kind,
            AlertKind::FleetReturned { number: 2, .. }
        ));
        assert_eq!(alerts[0].key(), "fleet:waiting:5f98b405c7b30d18");
        assert!(
            alerts[0]
                .text()
                .contains("expedition 1 (Trade) is waiting for your decision")
        );
        assert!(
            alerts[1]
                .text()
                .contains("expedition 2 (Industrial) has returned")
        );

        let after = 20711 * SECS_PER_DAY + 60;
        let status = execute_fleet(&model, after).unwrap();
        assert!(status.offers.refreshed);
        assert_eq!(status.offers.left, 5);
        assert_eq!(status.offers.launched_today, 0);
        assert_eq!(status.offers.secs_until_refresh, 0);
        let alerts = current_fleet_alerts(&model, after);
        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[2].key(), "offers:20710");
        assert_eq!(alerts[2].text(), "Navigator: 5 new expeditions available");
    }

    #[test]
    fn test_unused_navigator_yields_no_offer_alert() {
        let mut model = model_with_fleet();
        let fleet = model.fleet.as_mut().unwrap();
        fleet.offer_day = 0;
        fleet.expeditions.clear();
        let alerts = current_fleet_alerts(&model, 1_789_434_496);
        assert!(alerts.is_empty(), "{alerts:?}");
    }

    #[test]
    fn test_no_fleet_errors_and_yields_no_alerts() {
        let model = GalaxyModel::new();
        assert!(matches!(execute_fleet(&model, 0), Err(GraphError::NoFleet)));
        assert!(current_fleet_alerts(&model, 0).is_empty());
    }
}
