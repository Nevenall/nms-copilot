# Fleet expeditions

Report on frigate fleet expeditions: what each running expedition is doing, whether a frigate is waiting for the player, roughly when it will finish, and when the Navigator will offer new expeditions.

**Status:** implemented 2026-09-15 as `nms fleet [N | frigates]`, the REPL `fleet` command and alerts, and the `fleet_status` MCP tool (`fleet frigates` became `list frigates` in 0050, beside a new `list expeditions`). Drafted 2026-09-14 against a real save with one running expedition (Diplomacy, Very Long, five frigates, 16 of 18 events resolved, an intervention call pending) and a fleet of 25 frigates; the in-game checks below were run the same day and every decode rule the code depends on is `verified` in the reference notes. The one departure from the design below: the command-room count is taken straight from the freighter base's objects in the save conversion rather than as a field on `BaseObjects`.

**Depends on:** nothing new. The address decoder and the `now`-as-parameter pattern from arc 02 carry over.

---

## Context

Frigate expeditions run on real time. The freighter's Navigator offers five expeditions per day, each takes a Fleet Command Room and up to five frigates, and the fleet reports back through events along the way. The game shows this only on the freighter bridge. The save stores the whole picture: the expedition's start time, category, duration class, route, event log, and the frigates assigned, plus the day the current offer set was generated.

Before this arc the tool read none of it.

---

## What the save contains (verified)

> The decode facts below are maintained in [docs/reference/nms-save-notes.md](../../../reference/nms-save-notes.md) with their evidence; this section is the snapshot the plan was written from.

### Running expeditions: `PlayerStateData.FleetExpeditions[]`

| Field | Meaning | Sample |
|-------|---------|--------|
| `Seed` | expedition identity; matches one of `ExpeditionSeedsSelectedToday` | `0x5F98B405C7B30D18` |
| `CustomName` | player-given name, usually empty | `""` |
| `ExpeditionCategory.ExpeditionCategory` | `Combat`, `Exploration`, `Mining`, `Diplomacy`, `Support` (the game shows Industrial for Mining and Trade for Diplomacy) | `Diplomacy` |
| `ExpeditionDuration.ExpeditionDuration` | `Short`, `Medium`, `Long`, `VeryLong` | `VeryLong` |
| `StartTime` | Unix seconds | 1789344506 (2026-09-13 17:08 local) |
| `PauseTime` | Unix seconds; see below | 1789396923 (07:42) |
| `SpeedMultiplier` | 1.0 without upgrades; consumables such as the Fuel Oxidiser presumably raise it (inferred) | 1.0 |
| `UA` | the fleet's current location, save-layout address | voxel (1733, -8, 10), system 303, Euclid |
| `TimeOfLastUAChange` | Unix seconds of the last move | 1789398099 |
| `AllFrigateIndices`, `ActiveFrigateIndices`, `DamagedFrigateIndices`, `DestroyedFrigateIndices` | indices into `FleetFrigates` | 5 assigned, 5 active, none damaged |
| `Events[]` | the full route, generated at launch | 18 events |
| `NextEventToTrigger` | index of the next unresolved event | 16 |
| `NumberOfSuccessfulEventsThisExpedition`, `NumberOfFailedEventsThisExpedition` | resolved outcomes | 16, 0 |
| `InterventionPhoneCallActivated` | a frigate has called for a decision | `true` |
| `InterventionEventMissionID`, `Powerups[3]` | mission hook for the call; consumable upgrade slots, `^` when empty | `^`, all empty |

Each event:

| Field | Meaning | Sample |
|-------|---------|--------|
| `EventID` | event type and tier: `^DIPLOMATIC_2`, `^MINING_0`, `^COMBAT_2` | |
| `IsInterventionEvent` | the event asks the player for a decision | event 16: `true` |
| `InterventionEventID` | the decision offered: `^INT_TRADING_CHOOSE_FUND`, `^INT_TRADING_PIRATES`, ... | |
| `Success` | outcome; `false` on events not yet reached | 16 true, 2 false |
| `UA` | where it happens; `0` until reached | events 0 to 6 in the player's home region, 7 to 15 far out |
| `AffectedFrigateIndices`, `RepairingFrigateIndices`, `AffectedFrigateResponses` | damage bookkeeping | empty on this run |
| `WhaleEvent`, `AvoidedIntervention`, `Overridden*` | special cases and reward overrides (`^VALUE_UNITS`) | |

**Progress** is `NextEventToTrigger` of `Events.len()`, with `Success` on each resolved event. Events are generated up front, so the total is known from the start.

**Waiting for the player.** When `InterventionPhoneCallActivated` is true and `Events[NextEventToTrigger].IsInterventionEvent` is true, the fleet is holding for a decision, the state the game labels "waiting for player". `PauseTime` is the moment that hold began and is 0 otherwise. Answering the call zeroes it and shifts `StartTime` forward by the length of the hold, so active time is always `(PauseTime or now) − StartTime` (verified 2026-09-14, see the reference notes). An answered intervention event keeps `Success` false, so resolved means `index < NextEventToTrigger`.

**Time remaining is not stored, and neither is the route.** The game's duration is the offer's distance at about 284 light years per hour, times the fleet's `SpeedMultiplier` (verified 2026-09-14, see the game notes), but the distance is only shown on the Navigator screen and the event locations are zero until reached, so the tool cannot compute the total. What it has is the run's own cadence: resolved events divided by active time. On the first sample, 16 events in 52,417 seconds was 55 minutes per event and predicted the game's figure within five minutes with two events left, though the final leg ran longer than a mid-route event. Both Very Long runs seen had 18 events and totals near 16.5 hours, so a per-class table for this fleet is a fallback once more runs are seen. The tool shows the estimate as "about" and never as a countdown.

### The fleet: `PlayerStateData.FleetFrigates[]`

| Field | Meaning | Sample |
|-------|---------|--------|
| `CustomName` | player-given name | |
| `FrigateClass.FrigateClass` | `Combat`, `Exploration`, `Mining`, `Diplomacy`, `Support` | |
| `Race.AlienRace` | `Traders`, `Explorers`, ... | |
| `InventoryClass.InventoryClass` | grade C, B, A, S | S |
| `Stats[11]` | see below | `[33,14,8,10,10,0,0,0,0,0,0]` |
| `TraitIDs[5]` | `^COMBAT_PRI`, `^EXPLORE_TER_4`, `^FUEL_SEC_1`, ...: stat, then PRI/SEC/TER tier, then a variant | |
| `DamageTaken`, `NumberOfTimesDamaged`, `RepairsMade` | damage state and history | |
| `TotalNumberOfExpeditions`, `TotalNumberOfSuccessfulEvents`, `TotalNumberOfFailedEvents` | lifetime record | 34, 286, 9 |
| `HomeSystemSeed`, `ResourceSeed`, `ForcedTraitsSeed`, `TimeOfLastIncomeCollection` | generation seeds; the last is not understood | |

`Stats` indices 0 to 3 are Combat, Exploration, Industrial, Trade: each class's frigates peak in its own slot (a Combat frigate reads 33 there, an Exploration one 36, Trade frigates 20 to 26). Index 5 is the Support stat (22 and 23 on the two Support frigates, near zero elsewhere) and index 4 is low on Support frigates and around 10 on the rest, so it is probably the fuel-related figure the game shows. Indices 6 to 10 are zero on every frigate. Indices 4 to 10 are inferred.

A frigate is **at home** when its index appears in no running expedition's `AllFrigateIndices`.

### Daily offers: `ExpeditionSeedsSelectedToday[5]` and `LastKnownDay`

The Navigator offers five expeditions per day and the set refreshes at 00:00 UTC (community sources agree). The save records the offer set as five seeds and `LastKnownDay` as the UTC day number, `floor(unix_seconds / 86400)`, of the day the set belongs to. On the sample, `LastKnownDay` is 20710, which is 2026-09-14 UTC, and the running expedition's seed is the second of the five.

So the tool can say exactly when new offers arrive:

```
next_refresh   = (LastKnownDay + 1) * 86400            // 00:00 UTC after the recorded day
new_available  = now >= next_refresh                    // the game will roll a fresh set when next checked
```

The day rolls on game load: `LastKnownDay` moves to the new day and `ExpeditionSeedsSelectedToday` is cleared, and the five seeds appear once the Navigator is used (verified 2026-09-14, see the reference notes). So an empty list with the day current means all five offers are untouched. Once the list is filled, a running expedition whose seed is in it accounts for one, but an expedition launched and already debriefed today would be gone from `FleetExpeditions`, so "5 minus running today" is an upper bound. Verified later the same day: the list holds the seeds of expeditions launched today, so `5 − len` is exact and the tool shows "N of today's 5 offers left".

Capacity is exact: Fleet Command Rooms are `^FRE_ROOM_FLEET` objects at the freighter base (7 on the sample), one running expedition per room; frigates at home are those not assigned anywhere (20 of 25 on the sample).

### Not in the save, or not understood

- A finished, undebriefed expedition stays in the list with `NextEventToTrigger == Events.len()` and `UA` 0 (verified 2026-09-14). "Finished, awaiting debrief" is reported on that shape.
- `FreighterFleet[8]` holds eight entries with empty inventories and no home seed. Not fleet frigates; left alone.
- Event rewards are only described (`^VALUE_UNITS`), not quantified.
- `TimeOfLastIncomeCollection` on frigates is years old on every entry and does not change with expeditions.
- The seasonal community expedition lives in `ExpeditionContext` and is unrelated.

---

## Architecture

```
PlayerStateData.FleetExpeditions[] / FleetFrigates[] / LastKnownDay / ExpeditionSeedsSelectedToday
        ──► nms-save::model::{FleetExpedition, FleetEvent, FleetFrigate}
        ──► nms-core::fleet::{Fleet, Expedition, Event, Frigate}          (timestamps as i64, `now` as a parameter)
        ──► nms-graph::GalaxyModel.fleet: Option<Fleet>                    (cached by rkyv; bump CACHE_FORMAT_VERSION)
        ──► nms-query::fleet::{execute_fleet, FleetStatus, fleet_alerts}
        ──► CLI `nms fleet` · REPL `fleet` + alerts · MCP `fleet_status`
```

### Core types (`nms-core/src/fleet.rs`)

- `ExpeditionCategory`, `DurationClass`, `FrigateClass`, `FrigateGrade` enums with display names that match the game (Mining shows as Industrial, Diplomacy as Trade).
- `Expedition { seed, name, category, duration, start, pause, speed_multiplier, location: GalacticAddress, last_move, frigates: Vec<usize>, active, damaged, destroyed, events: Vec<Event>, next_event, intervention_pending }` with `resolved()`, `total()`, `failed()`, `is_waiting()`, `waiting_since()`, `active_secs(now)`, `estimate_remaining_secs(now)`, `is_complete()`.
- `Event { id, intervention_id, is_intervention, success, location: Option<GalacticAddress>, affected: Vec<usize> }`.
- `Frigate { name, class, race, grade, stats: [u32; 11], traits: Vec<String>, damage_taken, times_damaged, expeditions, successes, failures }` with `label()` (custom name or `<class> frigate #n`).
- `Fleet { expeditions, frigates, offer_day: i64, offer_seeds: Vec<u64>, command_rooms: usize }` with `next_refresh()`, `offers_unused_upper_bound()`, `frigates_at_home()`, `rooms_free()`.

The command-room count comes from the freighter base's objects, which arc 02 already decodes; `BaseObjects` gains a `fleet_rooms` count.

### Queries and views

`execute_fleet(model, now) -> FleetStatus` returns the expeditions with derived timing, the availability line, and the frigate list.

```
 FLEET EXPEDITIONS                                                     as of 09:08 (1h 32m ago)
 #  Type   Length     Frigates  Events   Elapsed   Status
 1  Trade  Very long  5         16 / 18  17h 34m   waiting for you since 07:42 (3h 00m), about 2h left once answered

  Navigator: up to 4 of today's 5 offers unused · new offers in 6h 17m (00:00 UTC) · 6 command rooms free · 20 frigates at home
```

`fleet 1` shows one expedition in full: the frigates on it with class, grade, and damage, then the event log with type, outcome, and where it happened, with the fleet's current system named when it is in the atlas and its distance from the player. `fleet frigates` lists the whole fleet: name, class, race, grade, the four main stats, traits, damage, lifetime expeditions, and whether it is out.

The REPL gets three alerts alongside the crop and depot ones, with the same once-per-event behaviour: a frigate is waiting for you, an expedition has resolved its last event, and the Navigator has new offers. The last one is time-based and needs no save change. The MCP tool `fleet_status` returns the same structures as JSON with the server's clock as `now`.

### Live refresh

The watcher's snapshot and delta carry systems, planets, player position, and bases. Fleet data is small and changes on every save, so the delta gains a `fleet: Option<Fleet>` replacement rather than a diff.

---

## Files to create or modify

| File | Change |
|------|--------|
| `crates/nms-save/src/model.rs` | `FleetExpedition`, `FleetEvent`, `FleetFrigate`; `fleet_expeditions`, `fleet_frigates`, `expedition_seeds_selected_today`, `last_known_day` on `PlayerStateData` |
| `crates/nms-save/src/convert.rs` | `to_core_fleet()` |
| `crates/nms-core/src/fleet.rs` | new: types, timing, estimates; `BaseObjects.fleet_rooms` in `base.rs` |
| `crates/nms-graph/src/model.rs`, `extract.rs` | `fleet: Option<Fleet>` on the model |
| `crates/nms-cache/src/data.rs`, `serialize.rs` | cache the fleet; bump `CACHE_FORMAT_VERSION` |
| `crates/nms-watch/src/snapshot.rs`, `delta.rs`; `nms-core/src/delta.rs` | fleet replacement in the delta |
| `crates/nms-query/src/fleet.rs`, `display.rs`, `base.rs` | queries, tables, alerts |
| `crates/nms-cli/src/fleet.rs`, `main.rs` | `nms fleet [N | frigates]` |
| `crates/nms-copilot/src/commands.rs`, `dispatch.rs`, `completer.rs`, `session.rs`, `mcp/tools.rs` | REPL command, alerts, `fleet_status` |
| `data/test/multi_system_save.json` | one running expedition, a small fleet, `LastKnownDay` |
| `README.md` | command docs |

---

## Key design decisions

- **Estimates are labelled.** Remaining time comes from the run's own event cadence and is shown as "about"; the game's remaining-time figure is not in the save and community duration tables are too loose to present as fact.
- **Waiting for the player is the headline.** It is the one state where the player can act, so it leads the status column and drives an alert.
- **Offers are counted as an upper bound** until the save shows how debriefed expeditions are recorded.
- **Game names over save names.** Trade and Industrial, not Diplomacy and Mining, in every table; the enums keep the save spelling for parsing.
- **`now` is a parameter** everywhere, as in arc 02.

---

## Testing strategy

- Parsing pinned to the sample: 18 events, `NextEventToTrigger` 16, five frigate indices, `PauseTime` 1789396923, `LastKnownDay` 20710, the running seed present in the five offers.
- Timing with a fixed `now`: elapsed, waiting-since, cadence estimate (55 minutes per event on the sample), `next_refresh` at 1789430400 and `new_available` flipping true after it.
- Fleet arithmetic: rooms free, frigates at home, offers unused upper bound.
- Alerts: waiting-for-player and new-offers keys, de-duplication, re-arm after the state clears.
- Display and CLI integration against the fixture.

---

## Verification

In-game checks against the sample save:

1. ~~Open the Fleet Command Room while the call is pending and note the remaining time; save; wait; save again.~~ Done 2026-09-14 from the save side: the room goes straight to the call, but no timing field moved in six hours of holding, and answering shifted `StartTime` by exactly the hold. The expedition pauses and `PauseTime` marks its start.
2. ~~Answer the call, then compare the resumed remaining time with the cadence estimate.~~ Done 2026-09-14: game showed 1h 54m 15s, cadence said 1h 50m for the two events outstanding. Expected finish Unix 1789425510; compare when it lands.
3. ~~Let the expedition finish without debriefing, save, and read the expedition's shape.~~ Done 2026-09-14: all events resolved, `UA` 0, entry still present; the debrief then removed it and left the seed list empty.
4. ~~After 00:00 UTC, save before talking to the Navigator.~~ Done 2026-09-14: the day rolled and the seed list emptied on load. Still to see: the seeds appearing after the Navigator is used.
5. ~~Open one frigate's details and confirm the order of the stats against `Stats[0..6]`.~~ Done 2026-09-14: Combat, Exploration, Industrial, Trade confirmed; `Stats[4]` is not the fuel figure and stays open.
6. ~~Launch one expedition, save, and check whether `ExpeditionSeedsSelectedToday` gains its seed.~~ Done 2026-09-14: it did. The list records launches and the offers-remaining count is exact.
7. ~~Read the new run's remaining time from the Fleet Command Room right after launch and compare with the Navigator's 16h 34m and the 0.97 speed multiplier.~~ Done 2026-09-14: 15h 58m 5s at 386 s after launch, matching 16h 34m × 0.97 within 20 s.

---

## Open questions

1. ~~What exactly does `PauseTime` mark, and is it reset when the call is answered?~~ Answered: the hold's start; reset to 0 and `StartTime` shifted forward by the hold length.
2. ~~How is a completed, undebriefed expedition represented, and does debriefing remove it from `FleetExpeditions`?~~ Answered: all events resolved with `UA` 0; the debrief removes it.
3. ~~Does `LastKnownDay` roll on game load or only when the Navigator is used?~~ Answered: on load, and the seed list empties with it.
4. What does `Stats[4]` hold? It is not the fuel figure (10 against 7 tonnes on the screen). `Stats[6..11]` are zero everywhere.
5. What is `FreighterFleet[8]`?
6. What is `TimeOfLastIncomeCollection`?
7. ~~Which consumable sets `SpeedMultiplier`, and to what?~~ Answered in part: the assigned frigates' `^SPEED_*` traits set it (0.97 with two aboard); consumables untested.
8. ~~Why did `NumberOfFailedEventsThisExpedition` rise from 0 to 2 when one intervention event was answered?~~ Answered: the debrief listed two failures for that one event, the event going wrong and the funded investment being lost, so an intervention counts its event outcome and its intervention outcome separately. Show the counters for totals and the flags per event.
