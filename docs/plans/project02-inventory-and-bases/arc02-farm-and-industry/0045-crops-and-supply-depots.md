# Crops, supply depots, and power

Report crop readiness, supply-depot fill, and power equipment for every base, from the base objects stored in the save.

**Status:** implemented 2026-09-14 as the `base` command (CLI and REPL), the `base_status` MCP tool, and REPL alerts. The decode rules below were verified against two real saves 48 minutes apart and an in-game reading.

**Depends on:** typed base objects in `nms-save` (done in this arc). The item name table from arc01 is not needed; crop names come from a small table in `nms-core`.

---

## Context

A base in the save is a list of placed objects. Each object records what it is, where it sits relative to the base, a timestamp, and a 64-bit `UserData` field. For crops, extractors, supply depots, and batteries, `UserData` holds the object's simulation state at the moment the game last saved. That state answers "is anything ready to harvest?" exactly, and "how full are my depots?" as of the last save.

Before this arc the tool read a base's name, type, and address and discarded its objects.

---

## What the save contains (verified)

### Base objects

Under `PlayerStateData.PersistentPlayerBases[i]`:

| Field | Meaning |
|-------|---------|
| `Name`, `BaseType.PersistentBaseTypes` | as before; types seen: `HomePlanetBase`, `FreighterBase`, `PlayerShipBase` |
| `GalacticAddress` | save-layout universe address |
| `Position`, `Forward` | base origin and orientation in world space |
| `LastUpdateTimestamp` | Unix seconds; last edit, not last save |
| `Objects[]` | placed objects |

Each object:

```json
{"ObjectID": "^SNOWPLANT", "Position": [5.68, 3.86, -4.03], "Up": [..], "At": [..],
 "Timestamp": 1789402087, "UserData": 15461882265600}
```

`Position` is base-relative. `Timestamp` is Unix seconds. `UserData` is a `u64`; for every object type below the meaningful value is the high 32 bits (`UserData >> 32`) and the low 32 bits are zero, with the exceptions noted in open questions.

**Every object at every base carries the same `Timestamp`, equal to the save time.** The game rewrites all object timestamps on every save, whether or not the base is loaded. So `Timestamp` is the snapshot instant for `UserData`, and `LastUpdateTimestamp` is unrelated to it.

### Crops: `UserData >> 32` is seconds grown, capped at the growth time

| ObjectID | Crop | Growth time |
|----------|------|-------------|
| `^LUSHPLANT` | Star Bulb | 4 h (14,400 s) |
| `^RADIOPLANT` | Gamma Root | 4 h |
| `^TOXICPLANT` | Fungal Mould | 4 h |
| `^POOPPLANT` | Coprite | 4 h |
| `^SNOWPLANT` | Frost Crystal | 1 h (3,600 s) |
| `^BARRENPLANT` | Cactus Flesh | 16 h (57,600 s) |
| `^SCORCHEDPLANT` | Solanium | 16 h |
| `^CREATUREPLANT` | Mordite Root | 8 h; ID from game data, not yet seen in a save |

Evidence, from two saves of the same farm:

| Save time | Star Bulb | Frost Crystal | Cactus Flesh |
|-----------|-----------|---------------|--------------|
| 08:20 (1789399213) | 1,004 | 989 | 1,053 |
| 09:08 (1789402087) | 3,878 | 3,600 | 3,927 |

The saves are 2,874 seconds apart. Star Bulb and Cactus Flesh rose by exactly 2,874. Frost Crystal rose to 3,600, its growth time, and stopped. So the value is elapsed growth, capped at maturity, and the first draft of this document had it backwards ("seconds remaining"). The first draft's readings of 14,400 on every 4-hour crop were mature crops waiting to be harvested, not freshly planted ones.

```
grown_now = min(growth_time, (UserData >> 32) + (now - Timestamp))
ready     = grown_now >= growth_time
remaining = growth_time - grown_now
progress  = grown_now / growth_time
```

Crops need the growth table for everything but the raw elapsed time. A plant whose ID is not in the table (any `^...PLANT` other than freighter rooms) is still listed, with "grown 2h 10m" in place of a countdown.

### Depots and extractors: `UserData >> 32` is this object's units times 1,440

| ObjectID | Object | Capacity | Full reading |
|----------|--------|----------|--------------|
| `^U_SILO_S` | Supply depot | 1,000 units | 1,440,000 |
| `^U_EXTRACTOR_S` | Mineral extractor | 250 units | 360,000 |
| `^U_GASEXTRACTOR` | Gas extractor | 250 units | 360,000 |

The game keeps one pool per pipe network and splits it evenly across every member, extractors included, each capped at its own capacity. Members below their cap therefore carry the same value; members at their cap stop while the rest keep rising. Verified in one step: the farm's nitrogen network (3 gas extractors, 4 depots) read 154,712 on all seven objects; 7 × 154,712 ÷ 1,440 = 752, and the game's depot screen showed "750 of 4,750" moments earlier. The paraffinium network (3 mineral, 3 depots) gave 746 against "750 of 3,750", and the oxygen network (1 and 1) gave 257 against "250 of 1,250". The small gaps are production between the look and the save.

```
object_units      = min(capacity, (UserData >> 32) / 1440)
network_stored    = sum(object_units) over members
network_capacity  = 250 × extractors + 1000 × depots
```

Which objects share a network is not stored. The tool groups objects at one base by equal raw value, which is right whenever members are below their caps and merges two networks only when both are full, a case where the totals are still right.

Extractors produce in real time while the player is away, so depot numbers are a lower bound at any moment after the save. The tool reports "as of <time> (<age> ago)" and does not extrapolate. Two saves give a rate: the nitrogen network gained 79 units in 48 minutes, about 33 units per hour per extractor.

### Power

| ObjectID | Object | `UserData >> 32` |
|----------|--------|------------------|
| `^U_BATTERY_S` | Battery | charge; 45,000 is full, 0 empty |
| `^U_SOLAR_S` | Solar panel | not present in the sample save; ID from the recipe database |
| `^U_GENERATOR_S` | Electromagnetic generator | 0 on all 16; placed in clusters on power hotspots, up to 400 units from the base origin |
| `^U_BIOGENERATOR` | Biofuel reactor | 87,267 seen once, 0 otherwise; low bits 256; not decoded |
| `^U_PARAGON` | not a base part | one per corvette (`PlayerShipBase`), at the ship origin, 1,000,000; not in the recipe database |
| `^U_POWERLINE` | Wire segment | 0 |
| `^U_PIPELINE` | Pipe segment | 0 |

Battery charge is shown; a generator whose readings are all zero shows no state, and any other reading is shown raw. The first draft of this document had `^U_GENERATOR_S` as the solar panel; the recipe database and the object positions (tight clusters far from the rooms, on hotspots) both say electromagnetic generator.

### What is not in the object

Nothing on a depot or extractor says which resource it holds. Refiner contents are stored separately in `RefinerBufferData`, and there are eleven `StoredInteractions` tables keyed by world-space position that index into per-object state such as `MaintenanceInteractions`. Linking an object to those records means transforming its base-relative position into world space with the base's `Position`, `Forward`, and `Up`, then matching. That remains a research task.

---

## Architecture

```
PersistentPlayerBases[].Objects[] ──► nms-save::model::BaseObject ──► nms-core::base::BaseObjects {crops, depots, extractors, batteries, generators, wires, pipes, other}
                                                                                 │  (stored on nms-core::PlayerBase.objects, cached by rkyv)
                                                                nms-query::base::{execute_base, BaseStatus, current_alerts}
                                                                                 │
                                    CLI `nms base [name]` · REPL `base [name]` + alerts · MCP `base_status`
```

Time-dependent logic takes `now` as Unix seconds everywhere so tests are deterministic. `nms-core` keeps timestamps as `i64` rather than `DateTime` so the types can be archived by rkyv.

### Core types (`nms-core/src/base.rs`)

- `CropKind` with `from_object_id`, `growth_secs`, `display_name`.
- `Crop { kind: Option<CropKind>, object_id, grown_at_snapshot, snapshot }` with `grown_secs(now)`, `remaining_secs(now)`, `ready(now)`, `progress(now)`.
- `Depot`, `Extractor` (with `network` index), `Battery`, `Generator`.
- `BaseObjects::decode(objects)` and `BaseObjects::networks()`, which returns per-network `stored` and `capacity` totals.
- `PlayerBase.objects: BaseObjects`, filled by `nms-save` conversion.

### Cache

`CacheData` embeds `PlayerBase`, so adding objects changed the archived layout. Cache files now start with a 4-byte magic and a format version; a file without the current header is reported as unreadable and the model is rebuilt from the save.

### Queries (`nms-query/src/base.rs`)

- `BaseQuery { name: Option<String> }`: exact name first, then case-insensitive substring; `None` is every base.
- `BaseStatus`: location, crop rows (per type, batches clustered within 60 seconds, soonest first), networks, power summary, snapshot time.
- `current_alerts(model, now)`: one alert per crop type with ready plants and per full network, each with a stable key for de-duplication.

### Views

`base` with no name prints one row per base: crops ready of total, extraction stored of capacity with a FULL mark, battery and generator summary. Bases with an alert sort first. `base <name>` prints the location block, then CROPS, EXTRACTION, and POWER tables for the sections that apply, each with its snapshot time and age.

### REPL alerts

- The prompt's right-hand side shows `🌱 16 ready · 📦 1 full`, refreshed against the clock before every prompt; blank when nothing is pending.
- A notice prints between commands the first time an alert appears; the alert re-arms when it clears (harvested or emptied).
- One summary line prints on startup; `status` lists all current alerts.
- Nothing prints while idle at the prompt, since the REPL only runs between commands.

---

## Later steps

1. **Resource identity** for depots and extractors via the interaction tables, so a network reads "Nitrogen, 752 / 4,750". Base names carry this for now.
2. **Rate learning.** When two consecutive saves show a network's value rising, record units per hour and show a projected fill time.
3. **Refiners.** `RefinerBufferData` already holds contents; link buffers to `^BUILD_REFINER*` objects.
4. **Freighter farms.** `^FRE_ROOM_PLANT1` rooms carry a different encoding (low bits 51, high bits 0).
5. **Amenities.** Teleporters, landing pads, storage containers, trade terminals by object ID.

---

## Open questions

1. What do the low 32 bits of `UserData` mean? Zero on crops and industry, 51 or 83 on freighter rooms, 256 on biofuel reactors.
2. What does a biofuel reactor's 87,267 represent? Possibly fuel remaining in seconds.
3. What is `^U_PARAGON`? One sits at the origin of the corvette with 1,000,000 in its value. It is not a placeable part.
4. Is a depot's capacity always 1,000? The sample has only `^U_SILO_S`.
5. Which `StoredInteractions` table maps to which per-object record, and does `Value` index into `MaintenanceInteractions` directly? Needed for resource identity.
6. Do bio-dome crops and hydroponic trays (`^PLANTTUBE`) use the same encoding? The sample has one tray with a 2022 timestamp and zero `UserData`, which looks like an empty tray.
7. The corvette (`PlayerShipBase` "Default") appears in the base overview like a planet base. Should ship bases be labelled or left out?
