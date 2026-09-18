//! The frigate fleet and its expeditions, decoded from the save.
//!
//! Expeditions run on real time. The save stores each running expedition's effective start time, the moment a hold for the player's decision began, the full event route, and the frigates assigned; it does not store the remaining time or the route's length. The Navigator's daily offers are recorded only as the seeds of the expeditions launched that day, plus the UTC day number they belong to. The decode rules and their evidence are in `docs/reference/nms-save-notes.md`, section 7.
//!
//! Nothing here reads the clock: every time-dependent method takes `now` as Unix seconds so results are deterministic in tests.

use serde::{Deserialize, Serialize};

use crate::address::GalacticAddress;

/// Expeditions the Navigator offers per day.
pub const OFFERS_PER_DAY: usize = 5;

/// Seconds in a UTC day; the offer set refreshes at 00:00 UTC.
pub const SECS_PER_DAY: i64 = 86_400;

/// Frigates an expedition can take.
pub const MAX_FRIGATES_PER_EXPEDITION: usize = 5;

/// An expedition's category as the save spells it; the game shows different labels for three of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum ExpeditionCategory {
    Combat,
    Exploration,
    Mining,
    Diplomacy,
    Support,
}

impl ExpeditionCategory {
    /// Parse the save's `ExpeditionCategory` string.
    pub fn from_save_name(name: &str) -> Option<Self> {
        match name {
            "Combat" => Some(Self::Combat),
            "Exploration" => Some(Self::Exploration),
            "Mining" => Some(Self::Mining),
            "Diplomacy" => Some(Self::Diplomacy),
            "Support" => Some(Self::Support),
            _ => None,
        }
    }

    /// The label the Navigator uses: Industrial for Mining, Trade for Diplomacy, Balanced for Support.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Combat => "Combat",
            Self::Exploration => "Exploration",
            Self::Mining => "Industrial",
            Self::Diplomacy => "Trade",
            Self::Support => "Balanced",
        }
    }
}

/// The save's duration class. The actual length is the route's distance at the fleet's speed; the class is a bucket over that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum DurationClass {
    Short,
    Medium,
    Long,
    VeryLong,
}

impl DurationClass {
    pub fn from_save_name(name: &str) -> Option<Self> {
        match name {
            "Short" => Some(Self::Short),
            "Medium" => Some(Self::Medium),
            "Long" => Some(Self::Long),
            "VeryLong" => Some(Self::VeryLong),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Short => "Short",
            Self::Medium => "Medium",
            Self::Long => "Long",
            Self::VeryLong => "Very long",
        }
    }
}

/// A frigate's class; the same five names as the expedition categories, but Support frigates are shown as Support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum FrigateClass {
    Combat,
    Exploration,
    Mining,
    Diplomacy,
    Support,
}

impl FrigateClass {
    pub fn from_save_name(name: &str) -> Option<Self> {
        match name {
            "Combat" => Some(Self::Combat),
            "Exploration" => Some(Self::Exploration),
            "Mining" => Some(Self::Mining),
            "Diplomacy" => Some(Self::Diplomacy),
            "Support" => Some(Self::Support),
            _ => None,
        }
    }

    /// The label the fleet screen uses.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Combat => "Combat",
            Self::Exploration => "Exploration",
            Self::Mining => "Industrial",
            Self::Diplomacy => "Trade",
            Self::Support => "Support",
        }
    }
}

/// A grade, C to S: a frigate's grade, or the class of a ship, multi-tool, or inventory grid (`InventoryClass` in the save).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum Grade {
    C,
    B,
    A,
    S,
}

/// The grade of a frigate; the same scale as every other C-to-S class in the save.
pub type FrigateGrade = Grade;

impl Grade {
    pub fn from_save_name(name: &str) -> Option<Self> {
        match name {
            "C" => Some(Self::C),
            "B" => Some(Self::B),
            "A" => Some(Self::A),
            "S" => Some(Self::S),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::B => "B",
            Self::A => "A",
            Self::S => "S",
        }
    }
}

/// One event on an expedition's route. The whole route is generated at launch; locations fill in as events are reached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Event {
    /// Event type and tier, for example `^DIPLOMATIC_2`.
    pub id: String,
    /// The decision offered when this is an intervention event, for example `^INT_TRADING_CHOOSE_FUND`.
    pub intervention_id: String,
    /// The fleet holds at this event until the player answers a call.
    pub is_intervention: bool,
    /// The event's outcome once resolved; `false` on events not yet reached.
    pub success: bool,
    /// Where it happens; `None` until reached.
    pub location: Option<GalacticAddress>,
    /// Frigates damaged by this event, as indices into the fleet.
    pub affected: Vec<u32>,
}

impl Event {
    /// The event's type in the game's words with its tier, for example `Trade 2`.
    pub fn label(&self) -> String {
        let raw = self.id.trim_start_matches('^');
        let (kind, tier) = match raw.rsplit_once('_') {
            Some((kind, tier)) if !tier.is_empty() && tier.bytes().all(|b| b.is_ascii_digit()) => {
                (kind, Some(tier))
            }
            _ => (raw, None),
        };
        let kind = match kind {
            "COMBAT" => "Combat",
            "EXPLORATION" => "Exploration",
            "MINING" => "Industrial",
            "DIPLOMATIC" => "Trade",
            other => other,
        };
        match tier {
            Some(tier) => format!("{kind} {tier}"),
            None => kind.to_string(),
        }
    }

    /// The decision's name in lowercase words, for example `trading choose fund`, or an empty string when this is not an intervention event. The save pre-fills an intervention ID on every event, so the flag decides.
    pub fn intervention_label(&self) -> String {
        let raw = self.intervention_id.trim_start_matches('^');
        if !self.is_intervention || raw.is_empty() {
            return String::new();
        }
        raw.strip_prefix("INT_")
            .unwrap_or(raw)
            .split('_')
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Where an expedition stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpeditionState {
    /// A frigate has called for a decision and the clock is stopped.
    Waiting,
    /// Every event is resolved; the fleet is back and awaits its debrief.
    Complete,
    /// Under way.
    Running,
}

/// A running expedition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Expedition {
    /// Identity; the same value is recorded in the day's launch list.
    pub seed: u64,
    /// Player-given name, usually empty.
    pub name: String,
    pub category: Option<ExpeditionCategory>,
    /// The save's category string, kept for categories not in the enum.
    pub category_raw: String,
    pub duration: Option<DurationClass>,
    pub duration_raw: String,
    /// Effective start, Unix seconds. The game shifts it forward by the length of each hold when the call is answered, so `now - start` is the active time while running.
    pub start: i64,
    /// Unix seconds the current hold began, or 0 while running.
    pub pause: i64,
    /// Product of the assigned frigates' duration modules; 1.0 with none, smaller is faster.
    pub speed_multiplier: f64,
    /// The fleet's location; `None` when the save holds 0, which it does once the fleet is back.
    pub location: Option<GalacticAddress>,
    /// Unix seconds of the last move.
    pub last_move: i64,
    /// Every assigned frigate, as indices into the fleet.
    pub frigates: Vec<u32>,
    pub active: Vec<u32>,
    pub damaged: Vec<u32>,
    pub destroyed: Vec<u32>,
    /// The whole route.
    pub events: Vec<Event>,
    /// Index of the next unresolved event; equal to the event count once the route is done.
    pub next_event: u32,
    /// A frigate has called for a decision.
    pub intervention_pending: bool,
    /// Resolved outcomes. An intervention counts its event outcome and its decision outcome separately, so the two can sum to more than the event count.
    pub successes: u32,
    pub failures: u32,
}

impl Expedition {
    /// The category as the Navigator labels it, or the raw save string.
    pub fn category_label(&self) -> &str {
        self.category
            .map(ExpeditionCategory::display_name)
            .unwrap_or(&self.category_raw)
    }

    /// The duration class label, or the raw save string.
    pub fn duration_label(&self) -> &str {
        self.duration
            .map(DurationClass::display_name)
            .unwrap_or(&self.duration_raw)
    }

    /// Events on the route.
    pub fn total(&self) -> usize {
        self.events.len()
    }

    /// Events resolved so far.
    pub fn resolved(&self) -> usize {
        (self.next_event as usize).min(self.events.len())
    }

    /// Events still ahead.
    pub fn remaining(&self) -> usize {
        self.events.len() - self.resolved()
    }

    /// The next unresolved event, if any.
    pub fn next(&self) -> Option<&Event> {
        self.events.get(self.next_event as usize)
    }

    /// The fleet is holding for the player's decision.
    pub fn is_waiting(&self) -> bool {
        self.intervention_pending && self.next().is_some_and(|e| e.is_intervention)
    }

    /// When the current hold began, while waiting.
    pub fn waiting_since(&self) -> Option<i64> {
        (self.is_waiting() && self.pause > 0).then_some(self.pause)
    }

    /// Every event is resolved.
    pub fn is_complete(&self) -> bool {
        !self.events.is_empty() && self.next_event as usize >= self.events.len()
    }

    pub fn state(&self) -> ExpeditionState {
        if self.is_waiting() {
            ExpeditionState::Waiting
        } else if self.is_complete() {
            ExpeditionState::Complete
        } else {
            ExpeditionState::Running
        }
    }

    /// Seconds the expedition has been under way at `now`, not counting the current hold.
    pub fn active_secs(&self, now: i64) -> i64 {
        let end = if self.pause > 0 { self.pause } else { now };
        (end - self.start).max(0)
    }

    /// Seconds the fleet has been holding at `now`, while waiting.
    pub fn waiting_secs(&self, now: i64) -> Option<i64> {
        self.waiting_since().map(|since| (now - since).max(0))
    }

    /// Average seconds per resolved event, from this run's own pace.
    pub fn secs_per_event(&self, now: i64) -> Option<f64> {
        let resolved = self.resolved();
        (resolved > 0).then(|| self.active_secs(now) as f64 / resolved as f64)
    }

    /// A rough estimate of the active seconds left, from the run's own pace: the remaining events at the average pace so far. `None` before the first event resolves. The real figure depends on the route's length, which the save does not hold, and the last leg has run longer than the average, so this is an estimate to show as "about".
    pub fn estimate_remaining_secs(&self, now: i64) -> Option<i64> {
        if self.is_complete() {
            return Some(0);
        }
        self.secs_per_event(now)
            .map(|per_event| (per_event * self.remaining() as f64).round() as i64)
    }

    /// The frigate is assigned to this expedition.
    pub fn has_frigate(&self, index: u32) -> bool {
        self.frigates.contains(&index)
    }
}

/// One frigate in the fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Frigate {
    /// Position in the save's fleet list; expeditions refer to frigates by it.
    pub index: u32,
    /// Player-given name; empty unless renamed. The game's generated name is not stored.
    pub name: String,
    pub class: Option<FrigateClass>,
    pub class_raw: String,
    /// The save's race name (`Traders`, `Explorers`, `Warriors`, ...).
    pub race: String,
    pub grade: Option<FrigateGrade>,
    /// The save's eleven stats. Indices 0 to 3 are Combat, Exploration, Industrial, and Trade; 5 is Support.
    pub stats: Vec<u32>,
    /// Module IDs, five slots, empty ones `^`.
    pub traits: Vec<String>,
    pub damage_taken: u32,
    pub times_damaged: u32,
    pub repairs: u32,
    /// Lifetime record.
    pub expeditions: u32,
    pub successes: u32,
    pub failures: u32,
    /// The system the frigate was recruited in.
    pub home: Option<GalacticAddress>,
}

impl Frigate {
    /// The player's name for the frigate, or `<Class> frigate #n` with its 1-based position.
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        format!("{} frigate #{}", self.class_label(), self.index + 1)
    }

    pub fn class_label(&self) -> &str {
        self.class
            .map(FrigateClass::display_name)
            .unwrap_or(&self.class_raw)
    }

    /// The race as the game names it.
    pub fn race_label(&self) -> &str {
        match self.race.as_str() {
            "Traders" => "Gek",
            "Explorers" => "Korvax",
            "Warriors" => "Vy'keen",
            other => other,
        }
    }

    pub fn grade_label(&self) -> &str {
        self.grade.map(FrigateGrade::display_name).unwrap_or("?")
    }

    /// The raw stat at `index` in the save's list, 0 when absent.
    pub fn stat(&self, index: usize) -> u32 {
        self.stats.get(index).copied().unwrap_or(0)
    }

    /// The save's fifth stat, index 4. Around 10 on most frigates and low on Support ones; not the fuel figure, and not yet identified.
    pub fn stat_five(&self) -> u32 {
        self.stat(4)
    }

    pub fn combat(&self) -> u32 {
        self.stat(0)
    }

    pub fn exploration(&self) -> u32 {
        self.stat(1)
    }

    pub fn industrial(&self) -> u32 {
        self.stat(2)
    }

    pub fn trade(&self) -> u32 {
        self.stat(3)
    }

    pub fn support(&self) -> u32 {
        self.stat(5)
    }

    pub fn is_damaged(&self) -> bool {
        self.damage_taken > 0
    }

    /// Installed modules, without the `^` prefix and empty slots.
    pub fn trait_labels(&self) -> Vec<String> {
        self.traits
            .iter()
            .map(|t| t.trim_start_matches('^'))
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Modules that shorten expeditions.
    pub fn speed_modules(&self) -> usize {
        self.traits
            .iter()
            .filter(|t| t.starts_with("^SPEED"))
            .count()
    }
    /// What the installed modules do, in the game's words: `3 combat, exploration, less fuel`. The save holds only each module's stat and tier, not its name, so this is as much as can be said.
    pub fn module_summary(&self) -> String {
        const KINDS: [(&str, &str); 7] = [
            ("COMBAT", "combat"),
            ("EXPLORE", "exploration"),
            ("MINING", "industrial"),
            ("TRADING", "trade"),
            ("FUEL", "less fuel"),
            ("SPEED", "faster"),
            ("INVULN", "protection"),
        ];
        let mut parts: Vec<String> = Vec::new();
        for (prefix, label) in KINDS {
            let count = self
                .traits
                .iter()
                .filter(|t| t.trim_start_matches('^').starts_with(prefix))
                .count();
            match count {
                0 => {}
                1 => parts.push(label.to_string()),
                n => parts.push(format!("{n} {label}")),
            }
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// The modules that do something other than add to a stat: fuel savings, speed, and damage protection. `-` when there are none. Stat modules are left out because their bonuses are already inside the stat numbers.
    pub fn perks(&self) -> String {
        const KINDS: [(&str, &str); 3] = [
            ("FUEL", "less fuel"),
            ("SPEED", "faster"),
            ("INVULN", "protection"),
        ];
        let mut parts: Vec<String> = Vec::new();
        for (prefix, label) in KINDS {
            let count = self
                .traits
                .iter()
                .filter(|t| t.trim_start_matches('^').starts_with(prefix))
                .count();
            match count {
                0 => {}
                1 => parts.push(label.to_string()),
                n => parts.push(format!("{label} \u{00D7}{n}")),
            }
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// The fleet, its running expeditions, and the state of the Navigator's daily offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Fleet {
    /// Running expeditions in save order; a debriefed expedition is removed.
    pub expeditions: Vec<Expedition>,
    /// Every frigate in save order.
    pub frigates: Vec<Frigate>,
    /// UTC day number of the current offer day, `floor(unix / 86400)`. Rolls on game load.
    pub offer_day: i64,
    /// Seeds of the expeditions launched on `offer_day`; cleared when the day rolls.
    pub launched_today: Vec<u64>,
    /// Fleet Command Rooms on the freighter; one running expedition per room.
    pub command_rooms: u32,
}

impl Fleet {
    /// The save holds no fleet at all.
    pub fn is_empty(&self) -> bool {
        self.expeditions.is_empty() && self.frigates.is_empty() && self.offer_day == 0
    }

    /// Unix seconds of the next 00:00 UTC after the recorded offer day.
    pub fn next_refresh(&self) -> i64 {
        (self.offer_day + 1) * SECS_PER_DAY
    }

    /// The offer day has rolled since the save; the game will start a fresh day when next loaded.
    pub fn offers_refreshed(&self, now: i64) -> bool {
        now >= self.next_refresh()
    }

    /// Expeditions launched on the current offer day.
    pub fn launched_today(&self, now: i64) -> usize {
        if self.offers_refreshed(now) {
            0
        } else {
            self.launched_today.len().min(OFFERS_PER_DAY)
        }
    }

    /// Offers still available today.
    pub fn offers_left(&self, now: i64) -> usize {
        OFFERS_PER_DAY - self.launched_today(now)
    }

    /// Command rooms without a running expedition.
    pub fn rooms_free(&self) -> u32 {
        self.command_rooms
            .saturating_sub(u32::try_from(self.expeditions.len()).unwrap_or(u32::MAX))
    }

    /// The running expedition a frigate is on, as a position in `expeditions`.
    pub fn expedition_of(&self, frigate: u32) -> Option<usize> {
        self.expeditions.iter().position(|e| e.has_frigate(frigate))
    }

    /// Frigates not assigned to any running expedition.
    pub fn frigates_at_home(&self) -> Vec<&Frigate> {
        self.frigates
            .iter()
            .filter(|f| self.expedition_of(f.index).is_none())
            .collect()
    }

    /// Frigates assigned to the expedition at `position`, in the order the expedition lists them.
    pub fn frigates_on(&self, position: usize) -> Vec<&Frigate> {
        self.expeditions
            .get(position)
            .map(|e| {
                e.frigates
                    .iter()
                    .filter_map(|i| self.frigates.get(*i as usize))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, intervention: bool, success: bool) -> Event {
        Event {
            id: id.into(),
            intervention_id: if intervention {
                "^INT_TRADING_CHOOSE_FUND".into()
            } else {
                "^".into()
            },
            is_intervention: intervention,
            success,
            location: None,
            affected: Vec::new(),
        }
    }

    /// The sample run at 09:08 on 2026-09-14: 16 of 18 resolved, holding at the intervention since 07:42.
    fn sample_waiting() -> Expedition {
        let mut events: Vec<Event> = (0..16)
            .map(|_| event("^DIPLOMATIC_2", false, true))
            .collect();
        events.push(event("^DIPLOMATIC_2", true, false));
        events.push(event("^DIPLOMATIC_2", false, false));
        Expedition {
            seed: 0x5F98_B405_C7B3_0D18,
            name: String::new(),
            category: Some(ExpeditionCategory::Diplomacy),
            category_raw: "Diplomacy".into(),
            duration: Some(DurationClass::VeryLong),
            duration_raw: "VeryLong".into(),
            start: 1_789_344_506,
            pause: 1_789_396_923,
            speed_multiplier: 1.0,
            location: Some(GalacticAddress::new(1733, -8, 10, 303, 0, 0)),
            last_move: 1_789_398_099,
            frigates: vec![9, 12, 20, 22, 24],
            active: vec![22, 20, 9, 12, 24],
            damaged: Vec::new(),
            destroyed: Vec::new(),
            events,
            next_event: 16,
            intervention_pending: true,
            successes: 16,
            failures: 0,
        }
    }

    fn frigate(index: u32, class: FrigateClass, traits: &[&str]) -> Frigate {
        Frigate {
            index,
            name: String::new(),
            class: Some(class),
            class_raw: format!("{class:?}"),
            race: "Traders".into(),
            grade: Some(FrigateGrade::S),
            stats: vec![33, 14, 8, 10, 10, 0, 0, 0, 0, 0, 0],
            traits: traits.iter().map(|t| t.to_string()).collect(),
            damage_taken: 0,
            times_damaged: 3,
            repairs: 0,
            expeditions: 34,
            successes: 286,
            failures: 9,
            home: None,
        }
    }

    #[test]
    fn test_category_labels_match_the_navigator() {
        assert_eq!(
            ExpeditionCategory::from_save_name("Diplomacy")
                .unwrap()
                .display_name(),
            "Trade"
        );
        assert_eq!(
            ExpeditionCategory::from_save_name("Mining")
                .unwrap()
                .display_name(),
            "Industrial"
        );
        assert_eq!(
            ExpeditionCategory::from_save_name("Support")
                .unwrap()
                .display_name(),
            "Balanced"
        );
        assert_eq!(
            FrigateClass::from_save_name("Support")
                .unwrap()
                .display_name(),
            "Support"
        );
        assert!(ExpeditionCategory::from_save_name("Other").is_none());
        assert_eq!(
            DurationClass::from_save_name("VeryLong")
                .unwrap()
                .display_name(),
            "Very long"
        );
        assert!(FrigateGrade::A < FrigateGrade::S);
    }

    #[test]
    fn test_event_label_uses_game_words_and_tier() {
        assert_eq!(event("^DIPLOMATIC_2", false, true).label(), "Trade 2");
        assert_eq!(event("^MINING_0", false, true).label(), "Industrial 0");
        assert_eq!(event("^COMBAT_3", false, true).label(), "Combat 3");
        assert_eq!(event("^WHALE", false, true).label(), "WHALE");
        assert_eq!(
            event("^DIPLOMATIC_2", true, false).intervention_label(),
            "trading choose fund"
        );
        assert_eq!(
            event("^DIPLOMATIC_2", false, false).intervention_label(),
            ""
        );
        let prefilled = Event {
            intervention_id: "^INT_MINING_CHOOSE_MINE_6".into(),
            ..event("^MINING_1", false, true)
        };
        assert_eq!(
            prefilled.intervention_label(),
            "",
            "not an intervention event"
        );
    }

    #[test]
    fn test_waiting_expedition_state_and_timing() {
        let e = sample_waiting();
        let now = 1_789_407_700; // 09:08 local
        assert_eq!(e.state(), ExpeditionState::Waiting);
        assert_eq!(e.resolved(), 16);
        assert_eq!(e.total(), 18);
        assert_eq!(e.remaining(), 2);
        assert_eq!(e.waiting_since(), Some(1_789_396_923));
        assert_eq!(e.waiting_secs(now), Some(10_777));
        assert_eq!(e.active_secs(now), 52_417, "active time stops at the hold");
        assert_eq!(e.secs_per_event(now).unwrap().round() as i64, 3_276);
        assert_eq!(e.estimate_remaining_secs(now), Some(6_552));
        assert_eq!(e.category_label(), "Trade");
        assert_eq!(e.duration_label(), "Very long");
    }

    #[test]
    fn test_answered_call_shifts_start_and_runs() {
        let mut e = sample_waiting();
        // The game moved the start forward by the hold and cleared the pause.
        e.start = 1_789_366_238;
        e.pause = 0;
        e.next_event = 17;
        e.intervention_pending = false;
        e.failures = 2;
        let now = 1_789_418_655;
        assert_eq!(e.state(), ExpeditionState::Running);
        assert_eq!(e.active_secs(now), 52_417);
        assert!(e.waiting_since().is_none());
        assert_eq!(e.remaining(), 1);
        assert_eq!(e.estimate_remaining_secs(now), Some(3_083));
    }

    #[test]
    fn test_complete_expedition() {
        let mut e = sample_waiting();
        e.pause = 0;
        e.next_event = 18;
        e.intervention_pending = false;
        e.location = None;
        assert_eq!(e.state(), ExpeditionState::Complete);
        assert!(e.is_complete());
        assert_eq!(e.remaining(), 0);
        assert_eq!(e.estimate_remaining_secs(1_789_434_496), Some(0));
        assert!(e.next().is_none());
    }

    #[test]
    fn test_fresh_expedition_has_no_estimate() {
        let mut e = sample_waiting();
        e.pause = 0;
        e.next_event = 0;
        e.intervention_pending = false;
        assert_eq!(e.state(), ExpeditionState::Running);
        assert!(e.estimate_remaining_secs(e.start + 300).is_none());
        assert_eq!(e.active_secs(e.start + 300), 300);
    }

    #[test]
    fn test_pending_flag_without_intervention_event_is_not_waiting() {
        let mut e = sample_waiting();
        e.next_event = 15;
        assert!(!e.is_waiting());
        assert_eq!(e.state(), ExpeditionState::Running);
    }

    #[test]
    fn test_frigate_labels_and_stats() {
        let f = frigate(
            0,
            FrigateClass::Combat,
            &["^COMBAT_PRI", "^EXPLORE_TER_4", "^", "^SPEED_TER_2", "^"],
        );
        assert_eq!(f.label(), "Combat frigate #1");
        assert_eq!(f.race_label(), "Gek");
        assert_eq!(f.grade_label(), "S");
        assert_eq!(
            (f.combat(), f.exploration(), f.industrial(), f.trade()),
            (33, 14, 8, 10)
        );
        assert_eq!(f.support(), 0);
        assert_eq!(
            f.trait_labels(),
            vec!["COMBAT_PRI", "EXPLORE_TER_4", "SPEED_TER_2"]
        );
        assert_eq!(f.speed_modules(), 1);
        assert_eq!(f.module_summary(), "combat, exploration, faster");
        assert!(!f.is_damaged());
        let bare = Frigate {
            traits: vec!["^".into(); 5],
            ..f.clone()
        };
        assert_eq!(bare.module_summary(), "-");
        let loaded = Frigate {
            traits: vec![
                "^COMBAT_PRI".into(),
                "^COMBAT_SEC_3".into(),
                "^COMBAT_SEC_1".into(),
                "^TRADING_TER_5".into(),
                "^FUEL_TER_2".into(),
            ],
            ..f.clone()
        };
        assert_eq!(loaded.module_summary(), "3 combat, trade, less fuel");
        assert_eq!(loaded.perks(), "less fuel");
        assert_eq!(f.perks(), "faster");
        assert_eq!(bare.perks(), "-");
        let shielded = Frigate {
            traits: vec![
                "^FUEL_PRI".into(),
                "^FUEL_SEC_2".into(),
                "^INVULN_TER_2".into(),
                "^SPEED_TER_1".into(),
                "^".into(),
            ],
            ..f.clone()
        };
        assert_eq!(shielded.perks(), "less fuel \u{00D7}2, faster, protection");
        assert_eq!(f.stat_five(), 10);
        let named = Frigate {
            name: "SV-8 Zuhotoh".into(),
            ..f
        };
        assert_eq!(named.label(), "SV-8 Zuhotoh");
    }

    #[test]
    fn test_fleet_offers_and_rooms() {
        let fleet = Fleet {
            expeditions: vec![sample_waiting()],
            frigates: (0..25)
                .map(|i| frigate(i, FrigateClass::Combat, &[]))
                .collect(),
            offer_day: 20710,
            launched_today: vec![0x5F98_B405_C7B3_0D18],
            command_rooms: 7,
        };
        let during = 20710 * SECS_PER_DAY + 3600;
        assert_eq!(fleet.next_refresh(), 1_789_430_400);
        assert!(!fleet.offers_refreshed(during));
        assert_eq!(fleet.launched_today(during), 1);
        assert_eq!(fleet.offers_left(during), 4);
        assert!(fleet.offers_refreshed(1_789_430_400));
        assert_eq!(fleet.offers_left(1_789_430_400), 5);
        assert_eq!(fleet.rooms_free(), 6);
        assert_eq!(fleet.frigates_at_home().len(), 20);
        assert_eq!(fleet.expedition_of(12), Some(0));
        assert!(fleet.expedition_of(0).is_none());
        assert_eq!(fleet.frigates_on(0).len(), 5);
        assert_eq!(fleet.frigates_on(0)[0].index, 9);
        assert!(fleet.frigates_on(3).is_empty());
        assert!(!fleet.is_empty());
    }

    #[test]
    fn test_empty_fleet() {
        let fleet = Fleet {
            expeditions: Vec::new(),
            frigates: Vec::new(),
            offer_day: 0,
            launched_today: Vec::new(),
            command_rooms: 0,
        };
        assert!(fleet.is_empty());
        assert_eq!(fleet.rooms_free(), 0);
        assert_eq!(fleet.offers_left(0), 5);
    }
}
