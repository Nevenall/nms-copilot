# Save file notes

What the No Man's Sky save file contains and how the values in it are encoded. Tags are explained in [README.md](README.md). Sample saves referred to below are the author's Steam save, format version 4735, read on 2026-09-13 and 2026-09-14; times are Unix seconds unless stated.

## 1. Files and slots

- Saves live under `%APPDATA%\HelloGames\NMS\<account>\` on Windows, where `<account>` is `st_<SteamID64>` for Steam or `DefaultUser` otherwise. `[verified: locate module, exercised on the author's machine]`
- Each slot has two files, an auto save and a manual save, numbered `save.hg`, `save2.hg`, `save3.hg`, … in pairs: files 1 and 2 are slot 1, 3 and 4 are slot 2, and so on. Each `saveN.hg` has a sibling `mf_saveN.hg` metadata file. `[verified: locate module against a real save directory]`
- Which file of the pair is the manual save is disputed. The locate module labels the odd file (`save.hg`) manual and the even file (`save2.hg`) auto. The author reports `save2.hg` as the manual save of slot 1, and libNOM.io documents `save.hg` as the auto save and `save2.hg` as the manual one. `[open: check by saving manually in-game and seeing which file's modification time changes; then fix `parse_save_filename` in `crates/nms-save/src/locate.rs` and its tests]`

## 2. Container format

- A save file is a sequence of LZ4 blocks. Each block has a 16-byte header starting with the little-endian magic `0xFEEDA1E5`, followed by the LZ4 payload; the decompressed blocks concatenate into one JSON document. A file whose first byte is `{` is plaintext JSON and is used as-is. `[verified: decompress module, run against real saves and the exported fixtures]`
- Saves of format 2002 and later (post-Frontiers) are not encrypted. The only cipher in play is XXTEA on the small `mf_save*.hg` metadata file, which carries a format number, a SHA-256 of the decompressed save, and the slot's summary fields; valid lengths are 104, 360, 384, and 432 bytes for formats 2001 through 2004. `[verified: metadata module; libNOM.io is the documentation it was written from]`
- String values can contain bytes that are not valid UTF-8 (item hashes, binary IDs). The reader replaces invalid sequences with U+FFFD before parsing. `[verified: real saves fail to parse without it]`

## 3. JSON keys

- Keys are obfuscated to three-character tokens (`F2P` is `Version`, and so on). The bundled mapping merges MBINCompiler's `mapping.json`, a legacy mapping for older keys, and SaveWizard name fixups. A key that is not in the mapping is kept as its raw token. `[verified: mapping module; a plaintext save starts with `{"Version"` and an obfuscated one with `{"F2P"`]`
- Save version 4735 contains keys the bundled mapping does not know, at least `9JR`, `A0K`, and `Anq`. Their meaning is unknown and the mapping wants a refresh from a current MBINCompiler. `[open: seen with `nms raw --find` on 2026-09-14]`

## 4. Top-level layout

- The root holds `Version` (integer, 4735 on the sample), `Platform` (`Win|Final`, `Mac|Final`), `CommonStateData`, `BaseContext`, `ExpeditionContext`, `ActiveContext`, and `DiscoveryManagerData`. `BaseContext.PlayerStateData` is the normal game; `ExpeditionContext.PlayerStateData` is the seasonal community expedition and has the same shape. `[verified: model module; both contexts parsed from the sample]`
- `PlayerStateData` carries the player's universe address, teleport endpoints, `PersistentPlayerBases`, every inventory grid, `FleetExpeditions`, `FleetFrigates`, currencies, and the known-item lists. Sections below cover the parts that have been decoded. `[verified: `nms raw BaseContext.PlayerStateData --keys`]`

## 5. Addresses

- A universe address stored as a single 64-bit value (`DD.UA` in discovery records, `GalacticAddress` on bases, teleport endpoints, expedition `UA`) uses this layout: bits 0-11 VoxelX (12-bit signed), 12-23 VoxelZ (12-bit signed), 24-31 VoxelY (8-bit signed), 32-39 a flag of unknown meaning, 40-51 SolarSystemIndex (12-bit), 52-55 PlanetIndex (4-bit). The galaxy is not in the value. `[verified: teleport endpoints carry the same address in expanded form; `0x2E00FC956DEC` is voxel (-532, -4, -1706), system 46, and the expanded record agrees]`
- Bits 32-39 read both 0 and 1 for records of the same Euclid system in a save that never left Euclid, so they are not the galaxy index. `[verified: same save, multiple records]` What they mean is `[open]`.
- The 48-bit portal-glyph layout is different: `P-SSS-YY-ZZZ-XXX`, planet in bits 44-47, system index in 32-43, VoxelY in 24-31, VoxelZ in 12-23, VoxelX in 0-11. Signal-booster coordinates are the same fields offset by `0x801` on X and Z and `0x81` on Y. `[community: NMSCD coordinate converters; verified against known portal addresses in the glyph tests]`
- The galaxy is stored separately as `RealityIndex` (0 is Euclid, 1 Hilbert Dimension, …) next to an expanded `GalacticAddress` object with `VoxelX`, `VoxelY`, `VoxelZ`, `SolarSystemIndex`, `PlanetIndex`. `[verified: model module, sample save]`
- The value written as a hex string (`"0x40050003AB8C07"`) or a bare integer is the same 64-bit number. `[verified: both forms appear in real saves and fixtures]`

## 6. Bases: `PlayerStateData.PersistentPlayerBases[]`

### Base record

- Fields: `Name`, `BaseType.PersistentBaseTypes` (seen: `HomePlanetBase`, `FreighterBase`, `PlayerShipBase`), `GalacticAddress` (save layout, section 5), `Position` and `Forward` (world-space origin and orientation), `LastUpdateTimestamp` (Unix seconds of the last edit, not the last save), `Objects[]`. `[verified: sample save; `LastUpdateTimestamp` sits days behind the object timestamps]`
- The corvette appears as a `PlayerShipBase` named `Default` with its own object list. `[verified: sample save]` Whether the tool should show it beside planet bases is an open product question, not a decode question.
- Each base also has its own top-level `UserData` field and a `BaseBuildingObjects` list (4 entries on the sample). Neither is decoded. `[open: seen with `nms raw --find userdata` on 2026-09-14]`

### Object record

- Each object: `ObjectID` (for example `^SNOWPLANT`), `Position` (base-relative), `Up`, `At`, `Timestamp` (Unix seconds), `UserData` (`u64`). `[verified: sample save]`
- **Objects with state carry a `Timestamp` equal to the save time.** The game rewrites their timestamps on every save, whether or not the base is loaded, so `Timestamp` is the snapshot instant for `UserData`. `[verified: two saves 48 minutes apart; crop, depot, extractor, and power timestamps moved to the new save time at every base]` One `^PLANTTUBE` with zero `UserData` kept a 2022 timestamp, so objects without state may be left alone. `[inferred: one object]`
- For crops, depots, extractors, and batteries the value is the high 32 bits, `UserData >> 32`; the low 32 bits are zero. Freighter planter rooms carry 51 or 83 in the low bits and biofuel reactors 256. `[verified: high bits, on the object types below]` `[open: the low bits]`

### Crops

- `UserData >> 32` is **seconds grown at the snapshot, capped at the crop's growth time**. `grown_now = min(growth, value + (now − Timestamp))`; ready when `grown_now >= growth`. `[verified: two saves 2,874 s apart: Star Bulb 1,004 → 3,878 and Cactus Flesh 1,053 → 3,927 (both +2,874); Frost Crystal 989 → 3,600 and stopped at its 1-hour growth time]`
- An earlier reading of the same field as "seconds remaining" was wrong; the 14,400 readings that suggested it were mature 4-hour crops waiting for harvest. `[verified: same experiment]`
Crop object IDs and growth times are in [nms-game-notes.md](nms-game-notes.md#crops).
- Freighter planter rooms (`^FRE_ROOM_PLANT1`) use a different encoding: high bits 0, low bits 51. `[open]`
- A hydroponic tray (`^PLANTTUBE`) with a 2022 timestamp and zero `UserData` was seen once, which looks like an empty tray. `[inferred: one object; plant something in a tray and re-read]`

### Supply depots and extractors

- `UserData >> 32` is **the units that object holds, times 1,440**. `[verified: in-game depot screen against the save, 2026-09-14, three networks]`
- The game keeps one pool per pipe network and splits it evenly across every member, extractors included, each capped at its own capacity (depot 1,000 units, extractor 250). Members below their cap therefore share one value. `network_stored = Σ(min(cap, value / 1440))`, `network_capacity = 250 × extractors + 1000 × depots`. `[verified: nitrogen network of 3 gas extractors and 4 depots read 154,712 on all seven objects, 7 × 154,712 / 1,440 = 752, and the game showed "750 of 4,750"; paraffinium 746 against "750 of 3,750"; oxygen 257 against "250 of 1,250"]`
- Which objects share a network is not stored on the objects. Grouping by equal raw value is right while members are below their caps and merges two full networks, where the totals are still right. `[inferred: follows from the pooling rule; a save with two networks at different fill levels would confirm the grouping]`
- Nothing on a depot or extractor says which resource it holds. Refiner contents are in `RefinerBufferData`; eleven `StoredInteractions` tables keyed by world-space position index into per-object records such as `MaintenanceInteractions`, which may carry the resource. Linking needs the object's world position from the base `Position`, `Forward`, and `Up`. `[open]`
- Production continues in real time while the player is away, so a depot value is a lower bound after the save. `[verified: the nitrogen network gained 79 units across the two saves]`

### Power

| Object ID | Object | `UserData >> 32` | Evidence |
|-----------|--------|------------------|----------|
| `^U_BATTERY_S` | Battery | charge; 45,000 full, 0 empty | `[inferred: 45,000 is the largest value read across the sample's batteries and the game reports those as full; check the reading of a battery the game shows part-charged]` |
| `^U_GENERATOR_S` | Electromagnetic Generator | 0 on all 16 seen | `[verified: recipe database by ID, and the objects sit in tight clusters up to 400 units from the rooms, where power hotspots are]` |
| `^U_SOLAR_S` | Solar Panel | not seen in a save | `[game-data: recipe database ID]` |
| `^U_BIOGENERATOR` | Biofuel Reactor | 87,267 seen once, 0 otherwise; low bits 256 | `[open: possibly fuel remaining in seconds]` |
| `^U_POWERLINE` | wire segment | 0 | `[verified: sample]` |
| `^U_PIPELINE` | pipe segment | 0 | `[verified: sample]` |
| `^U_PARAGON` | not a placeable part; one sits at the corvette's origin with 1,000,000 | | `[open: not in the recipe database]` |

- `^U_GENERATOR_S` was once labelled the Solar Panel in this project; that was wrong. Prefab rooms have no built-in solar panels. `[verified: as above]`

## 7. Fleet

### `PlayerStateData.FleetExpeditions[]`

- One entry per running expedition. Fields: `Seed` (matches one of `ExpeditionSeedsSelectedToday`), `CustomName`, `ExpeditionCategory.ExpeditionCategory` (`Combat`, `Exploration`, `Mining`, `Diplomacy`, `Support`), `ExpeditionDuration.ExpeditionDuration` (`Short`, `Medium`, `Long`, `VeryLong`), `StartTime`, `PauseTime`, `SpeedMultiplier`, `UA` (fleet location, save layout), `TimeOfLastUAChange`, `AllFrigateIndices`, `ActiveFrigateIndices`, `DamagedFrigateIndices`, `DestroyedFrigateIndices` (indices into `FleetFrigates`), `Events[]`, `NextEventToTrigger`, `NumberOfSuccessfulEventsThisExpedition`, `NumberOfFailedEventsThisExpedition`, `InterventionPhoneCallActivated`, `InterventionEventMissionID`, `Powerups[3]`. `[verified: sample save with one running expedition, 2026-09-14]`
- `Events[]` holds the whole route, generated at launch; each event has `EventID` (`^DIPLOMATIC_2`, `^MINING_0`, `^COMBAT_2`, …), `IsInterventionEvent`, `InterventionEventID` (`^INT_TRADING_CHOOSE_FUND`, `^INT_TRADING_PIRATES`, …), `Success`, `UA` (0 until reached), and damage bookkeeping lists. Progress is `NextEventToTrigger` of `Events.len()`. `[verified: 18 events, 16 resolved with `Success` true, the two unreached ones `false` with `UA` 0]`
- The fleet is waiting for the player when `InterventionPhoneCallActivated` is true and `Events[NextEventToTrigger].IsInterventionEvent` is true. `[verified: matched the in-game "waiting for player" state on the sample]`
- `PauseTime` is the Unix second the hold began and is 0 while the expedition is running. Answering the call sets it back to 0 and **shifts `StartTime` forward by the length of the hold**, so `StartTime` is the effective start and the active time is `PauseTime − StartTime` during a hold and `now − StartTime` otherwise. `[verified: 2026-09-14, saves before and after answering a call; `PauseTime` 1789396923 → 0, `StartTime` 1789344506 → 1789366238, a shift of 21,732 s, equal to the hold from `PauseTime` to the answer at `TimeOfLastUAChange` 1789418655]`
- Opening the Fleet Command Room during a hold goes straight to the intervention dialogue; the remaining time cannot be read until the call is answered. `[verified: 2026-09-14]`
- Answering the call advances `NextEventToTrigger` past the intervention event and sets `TimeOfLastUAChange` to the answer time. The intervention event's `Success` stayed `false` and `NumberOfFailedEventsThisExpedition` went from 0 to 2 while successes stayed at 16, so "resolved" is `index < NextEventToTrigger`, not `Success`. `[verified: same saves]` The call was `^INT_TRADING_CHOOSE_FUND`, answered by sending the credits requested for an investment, with no outcome shown yet. Why the failed counter rose by two for one event is `[open: see whether the debrief reports the investment's result and whether the counter changes]`.
- Remaining time is not stored, only the duration class. Right after the call was answered, with 52,417 s of active time elapsed and one event left, the game showed 1h 54m 15s (6,855 s) remaining, an implied total of 59,272 s (16h 28m) for a Very Long expedition with five frigates at `SpeedMultiplier` 1.0. `[verified: one reading, 2026-09-14]` The run's own cadence (resolved events over active seconds, 55 minutes per event on the sample) predicted 1h 50m for the two events outstanding before the answer, within five minutes of the game, but would give 51 minutes for the one event left afterwards, so the final leg is longer than a mid-route event. `[inferred: one run; the finish time, expected at Unix 1789425510, will show whether the total holds]`
- `SpeedMultiplier` is the product of the assigned frigates' duration traits: a run with no `^SPEED_*` traits aboard read 1.0, and a run carrying `^SPEED_TER_4` and `^SPEED_TER_2` on two of its five frigates read 0.97 with no consumables installed (`Powerups` all `^`). A smaller value is a shorter run. `[verified: 2026-09-14, two expeditions on the same fleet]` Whether consumables also change it is `[open]`.
- Two Very Long runs on the same fleet both had 18 events. `[verified: 2026-09-14; an earlier reading of 10 was the `nms raw` array limit, not the save]`
- Event `UA` values are all 0 at launch, so the route and its length are not in the save; the total duration the game shows cannot be computed from the file. `[verified: the new run's 18 events, seconds after launch]`
- A finished expedition that has not been debriefed stays in `FleetExpeditions` with `NextEventToTrigger == Events.len()`, every event resolved, `UA` **0** (not the freighter's address), `PauseTime` 0, and `TimeOfLastUAChange` set to the moment the return was registered, which on the sample was the game load nine seconds before the save. `[verified: 2026-09-14, save taken on load after the run ended, before the debrief]`
- Debriefing removes the entry from `FleetExpeditions`; nothing about the run survives on the player state apart from the frigates' lifetime counters. `[verified: 2026-09-14, the list went from one entry to empty across the debrief]`
- The success and failure counters can exceed the event count: the sample finished with 17 successes and 2 failures over 18 events. Both failures were counted the moment the intervention was answered, only the intervention event's `Success` is `false`, and the debrief listed two failure entries for it: the event itself went wrong with a small resource consolation, and the investment funded through the call was lost. So an intervention event yields an event outcome and an intervention outcome, each counted, while `Success` records the event outcome. `[verified: saves before and after the answer and the finish; inferred: the two-outcome reading, from one intervention against its debrief log]`
- `FreighterFleet[8]` holds eight entries with empty inventories and no home seed; not the frigates. `[open]`

### `PlayerStateData.FleetFrigates[]`

- Fields: `CustomName`, `FrigateClass.FrigateClass` (same five names), `Race.AlienRace`, `InventoryClass.InventoryClass` (C/B/A/S), `Stats[11]`, `TraitIDs[5]` (`^COMBAT_PRI`, `^EXPLORE_TER_4`, `^FUEL_SEC_1`, …: stat, PRI/SEC/TER tier, variant), `DamageTaken`, `NumberOfTimesDamaged`, `RepairsMade`, `TotalNumberOfExpeditions`, `TotalNumberOfSuccessfulEvents`, `TotalNumberOfFailedEvents`, `HomeSystemSeed`, `ResourceSeed`, `ForcedTraitsSeed`, `TimeOfLastIncomeCollection`. `[verified: 25 frigates on the sample]`
- `Stats[0..4]` are Combat, Exploration, Industrial, Trade in that order. `[verified: 2026-09-14, frigate 0's detail screen read 33, 14, 8, 10 against `Stats` `[33, 14, 8, 10, …]`]`
- `Stats[5]` is the Support stat (22 and 23 on the two Support frigates, near zero elsewhere). `[inferred: every class peaks in its own slot and Support has no slot in 0..4]` `Stats[4]` reads 10 and 11 on the two frigates checked while the screen showed fuel requirements of 7 tonnes per 250 ly, so it is not the fuel figure; `Stats[6..11]` are zero on every frigate. `[open: `Stats[4]`]`
- The detail screen's Expeditions and Times Damaged are `TotalNumberOfExpeditions` and `NumberOfTimesDamaged`, and its Successful Encounters is `TotalNumberOfSuccessfulEvents`. `[verified: 34, 3, and 286 on frigate 0's screen and record]`
- `TraitIDs` has five slots, empty ones `^`. Prefixes seen across 25 frigates: `COMBAT`, `EXPLORE`, `MINING`, `TRADING`, `FUEL` (the primary trait of every Support frigate), `SPEED` (expedition duration), `INVULN` (damage), each with a `PRI`, `SEC`, or `TER` tier and a variant number. `[verified: sample fleet, 2026-09-14]` The five `TraitIDs` are the five modules on the detail screen. On frigate 0: `^COMBAT_PRI` is Combat Specialist +15, `^COMBAT_SEC_3` Cloaking Device +6, `^COMBAT_SEC_1` Massive Guns +2, `^TRADING_TER_5` Negotiation Module +2 Trade, `^EXPLORE_TER_4` Fauna Analysis Device +1 Exploration. The tier does not fix the bonus (SEC_3 gives +6, SEC_1 +2), so a name and value table per ID is needed. `[verified: one frigate's screen against its record]`
- A frigate's displayed name (`SV-8 Zuhotoh`), captain, crew mood, and notes are not stored; `CustomName` is empty unless the player renamed it. `[verified: frigate 0]`
- `HomeSystemSeed` is not a seed: frigate 0's value `0x2E00FC956DEC` is the save-layout address of a known system (section 5), so it is the system the frigate was recruited in. `[verified: the same value appears as a universe address elsewhere in the save]`
- A frigate is at home when its index appears in no running expedition's `AllFrigateIndices`. `[inferred: follows from the index lists; 20 of 25 on the sample]`
- `TimeOfLastIncomeCollection` is years old on every frigate and does not change with expeditions. `[open]`

### Daily offers

- `LastKnownDay` is the UTC day number, `floor(unix / 86400)`, of the current offer day (20710 = 2026-09-14 UTC on the sample). `[verified: day number arithmetic against the save date]`
- `ExpeditionSeedsSelectedToday` (up to 5 entries, written as bare hex strings where an expedition's own `Seed` is a `[flag, value]` pair) holds the seeds of the expeditions **launched** today, not the offers on display: opening the Navigator with five offers showing left the list empty, and launching one added exactly that expedition's seed. `5 − len` is therefore the number of offers still available today. `[verified: 2026-09-14, saves before and after a launch; the list went `[]` → `[0x5F98B405C7B30D1B]` and the new `FleetExpeditions` entry carries the same seed]`
- Offer seeds are close neighbours across days: the previous day's launches were `…0D18`, `…0D19`, `…0D1A`, `…0D1E` and the next day's `…0D1B`, all on the base `0x5F98B405C7B30D1x`, with one unrelated seed (`0x50250529F5B0387`) among the earlier five. `[inferred: two days' lists; how offers are generated from the base is `open`]`
- Offers refresh at 00:00 UTC, so `next_refresh = (LastKnownDay + 1) × 86400`. `[community: wiki and forum consensus on the daily reset; the day-number field is consistent with it]`
- `LastKnownDay` rolls on game load and the seed list is cleared with it: a save taken on load after 00:00 UTC, before visiting the bridge, read the new day number and an empty `ExpeditionSeedsSelectedToday`, and it stayed empty after a debrief and after the Navigator's five offers were viewed. `[verified: 2026-09-14, `LastKnownDay` 20710 → 20711 and the list `[5 seeds]` → `[]`, three saves]` A running expedition keeps its seed on its own record after the list is cleared.
- Four of the five seeds on the sample day were consecutive values (`…0D18`, `…0D19`, `…0D1A`, `…0D1E`) and the fifth unrelated (`0x50250529F5B0387`), so the offers derive from one day seed with small offsets plus one of a different kind. `[inferred: one day's list]`
- A debriefed expedition is removed from `FleetExpeditions`; the seed list above, if it records launches, is what makes today's count exact. `[verified: 2026-09-14 debrief]`
- Fleet Command Rooms are `^FRE_ROOM_FLEET` objects at the freighter base (7 on the sample); one running expedition per room. `[inferred: name from the ID; count not yet compared with the freighter; the one-per-room rule is `community`]`

## 8. Inventory

- Grids under `PlayerStateData`: `Inventory` (exosuit general, 10x12), `Inventory_Cargo` (7x5), `Inventory_TechOnly` (10x6), `FreighterInventory` with `_Cargo` and `_TechOnly`, `Chest1Inventory` … `Chest10Inventory` (the ten storage containers, 10x6 each), `ShipOwnership[i].Inventory` with `_Cargo` and `_TechOnly`, `VehicleOwnership[i].Inventory`, `Multitools[i].Store`, `WeaponInventory` (equipped multi-tool), `RefinerBufferData[i].InventoryContainer`, `ChestMagicInventory`, `ChestMagic2Inventory`, `CookingIngredientsInventory`, `FishBaitBoxInventory`, `FoodUnitInventory`, `RocketLockerInventory`, `GraveInventory`. `[verified: sample save, 2026-09-12]`
- Every grid has `Width`, `Height`, `Class.InventoryClass` (C/B/A/S), `Slots`, `SpecialSlots`, `ValidSlotIndices`, `StackSizeGroup`, `Name`. `Slots` lists occupied cells only; each has `Id`, `Amount`, `MaxAmount`, `Index {X, Y}`, `Type.InventoryType` (`Substance`, `Product`, `Technology`), `DamageFactor`, `FullyInstalled`, `AddedAutomatically`. `[verified: sample]`
- Technology slots carry `Amount` as charge or condition, not a count. `[verified: sample values]`
- Currencies: `Units`, `Nanites`, `Specials` (quicksilver). `[verified: field names and values on the sample; `Specials` as quicksilver is `community`]`
- `PrimaryShip` is the index of the active ship in `ShipOwnership`. `ShipOwnership` has 12 entries; entries without `Resource.Filename` are empty stubs. Ship type is the path segment before the file name in `Resource.Filename`: `FIGHTERS`, `DROPSHIPS` (hauler), `SCIENTIFIC` (explorer), `SAILSHIP` (solar), `S-CLASS` (exotic), `BIGGS` (living ship). `[verified: 8 real ships on the sample carry these segments; the segment-to-type names are `community` from save editors, not yet checked against the in-game ship list]`
- Ship `Location` and `Position` read zero on every ship in the sample, so their meaning is `[open: leave a ship on a planet and re-read]`. Exocraft carry the address of the base they are parked at in `Location`. `[verified: four at the farm base]`
- A ship's `Name` is empty unless the player renamed it; the procedural name the game shows is generated from `Resource.Seed` and is not stored. `[verified: all eight real ships on the sample have an empty `Name` while the game shows names for each]`
- Ship class is `Inventory.Class.InventoryClass`; the unlocked slot counts are the lengths of `Inventory.ValidSlotIndices` (general) and `Inventory_TechOnly.ValidSlotIndices` (technology); `Inventory.BaseStatValues` carries the class bonuses as `^SHIP_DAMAGE`, `^SHIP_SHIELD`, `^SHIP_HYPERDRIVE`, `^SHIP_AGILE` percentages. `[verified: sample ships, 2026-09-14; an S-class fighter reads 32 general and 35 tech slots with damage 79.6, shield 26.5, agility 37.8; the slot-count reading is `inferred` until compared with the in-game ship screen]`
- `Multitools` has 6 entries, `ActiveMultioolIndex` (sic) the active one; entries without `Resource.Filename` are empty stubs. Each real one has `Store` (the tool's grid, `Class.InventoryClass` for grade, `ValidSlotIndices` for slots, `BaseStatValues` with `^WEAPON_DAMAGE`, `^WEAPON_MINING`, `^WEAPON_SCAN`), `Seed`, `IsLarge`, `PrimaryMode` and `SecondaryMode` (installed weapon modes as enum indices), `Name` (empty unless renamed), `CustomisationData`, `ScreenData`. `[verified: two real multitools on the sample]`
- A live multitool carries no type field (Pistol, Rifle, Experimental, Alien, Royal, Sentinel, Atlantid, Staff); `Resource.Filename` is the same generic scene for every tool, so the type comes from the seed. `[verified: both sample tools share the filename]` How to derive the type is `[open: the archived record below stores it explicitly, so archive one and compare]`.
- `ArchivedShipOwnership[18]` and `ArchivedMultitools[18]` are the Collected Ships and Collected Multi-Tools archives, fixed at 18 slots each. An archived ship is the live ownership record under `Ownership` plus `ArchivedClass.ShipClass` (`Fighter`, `Freighter`, …), `ArchivedInventoryClass`, `ArchivedName`, `Customisation`; an archived multitool is the live record under `MultitoolData` plus `WeaponClass.WeaponStatClass` (`Pistol`, …), `ArchivedInventoryClass`, `ArchivedName`. An empty slot has an empty `Resource.Filename` and the defaults `Freighter` and `Pistol`. `[verified: sample save, 2026-09-14, all 36 slots empty; the field meanings are `inferred` from the names until a real archive entry is seen]`
- Storage containers are one inventory per number reachable from every base that has the matching object placed: `^CONTAINER0` in a planet base and `^FRE_ROOM_STORE0` on the freighter both open `Chest1Inventory`. `[verified: object IDs in the sample; the 0-based ID to 1-based container numbering and the shared-inventory behaviour are `community` and match play]`
- `ChestMagicInventory` (11 stacks on the sample) and `ChestMagic2Inventory` (empty) are unidentified. Ruled out: the Nutrient Processor ingredient store (that is `CookingIngredientsInventory`, confirmed by adding ingredients in-game and re-reading) and settlement storage. `[open: move a distinctive item into a suspected container and diff the save]`
- `Inventory_Cargo` grids are all empty on the sample; presumably the high-capacity slots. `[inferred]`
- `KnownProducts` (835 entries) and `KnownTech` (137) are ID lists. `[verified: sample counts]`

## 9. Discoveries


- Discovery records live under `DiscoveryManagerData` with a `DD.UA` save-layout address (section 5), a discovery type, and ownership data whose platform token is `ST` for Steam, `PS` for PlayStation, and so on. `[verified: model module, sample save]`

## 10. Wonders: `PlayerStateData.Wonder*Records[]`

The Wonders catalogue (Catalogue & Guide, in-game menu) is stored as one fixed-length array per category, one slot per record type, in the order the game lists them. Slot orders are in [nms-game-notes.md](nms-game-notes.md#wonders).

- Arrays and lengths: `WonderPlanetRecords[11]`, `WonderCreatureRecords[15]`, `WonderFloraRecords[8]`, `WonderMineralRecords[8]`, `WonderTreasureRecords[13]`, `WonderWeirdBasePartRecords[11]`, `WonderCustomRecords[12]` with a parallel `WonderCustomRecordsExtraData[12]`. `[verified: sample save, 2026-09-14; the lengths match the wiki's category counts]`
- Each record: `GenerationID` (a pair of 64-bit values, written as hex strings or bare integers), `SeenInFrontend` (the player has opened the entry in the catalogue), `WonderStatValue` (the record's figure). An empty slot has `GenerationID` `[0, 0]` and a value of 0. `[verified: sample; the eleven glitch slots and four treasure slots are empty in that shape]`
- For planets, creatures, flora, minerals, and custom records, `GenerationID[0]` is the planet's address in the save layout (section 5) and `GenerationID[1]` is a seed. The first planet record decodes to voxel (-533, -4, -1706), system 85, planet 2, and the creature and custom records decode to neighbouring systems in the player's home region. `[verified: the address bits decode to sensible voxels next to the player's known systems; matching one against the atlas is still to do]`
- For treasures, the two values are the artifact's item ID as 16 little-endian ASCII bytes: `0x4F4F4C5F434F5250` then `0x36353234312354` read `PROC_LOOT#14256`, a procedural loot ID. `[verified: the bytes decode to a well-formed ID; the exact item is `[open]`]`
- `WonderStatValue` is the record's number in the game's display unit, so slot 0 of the planet array (hottest temperature) reads 316.04 °C, slot 5 (largest planet) 524,102 u, slot 9 (paradise quotient) 87.6 %, and slot 3 of the mineral array (metal content) 79.9 %. The first four creature slots are sizes in metres; treasure values are the artifact's age or worth as the game shows it. `[verified: planet and mineral values checked against the wiki's units and ranges; the creature and treasure units are `inferred`]`
- Custom records are the player's personal wonders: each entry in `WonderCustomRecordsExtraData` carries `ActualType.WonderType` (`Planet`, `Creature`, …) and the `CustomName` the player typed; its `WonderStatValue` is 0. `[verified: 12 named entries on the sample]`
