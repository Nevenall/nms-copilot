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
- `PauseTime` most likely marks when that hold began: 07:42 local on the sample, before the fleet's last move at 08:01, with the in-game timer stopped. `[inferred: one sample; test by saving twice during a hold and seeing whether the in-game remaining time moves]`
- Remaining time is not stored, only the duration class. The run's own cadence (resolved events over active seconds; 55 minutes per event on the sample) gives an estimate. `[verified: no such field in the sample; the estimate is a method, not a fact]`
- `SpeedMultiplier` is 1.0 on the sample; consumables such as the Fuel Oxidiser presumably raise it. `[inferred]`
- Nothing seen so far marks an expedition as finished but not yet debriefed. The likely shape is `NextEventToTrigger == Events.len()` with `UA` back at the freighter. `[open: needs a save taken in that state]`
- `FreighterFleet[8]` holds eight entries with empty inventories and no home seed; not the frigates. `[open]`

### `PlayerStateData.FleetFrigates[]`

- Fields: `CustomName`, `FrigateClass.FrigateClass` (same five names), `Race.AlienRace`, `InventoryClass.InventoryClass` (C/B/A/S), `Stats[11]`, `TraitIDs[5]` (`^COMBAT_PRI`, `^EXPLORE_TER_4`, `^FUEL_SEC_1`, …: stat, PRI/SEC/TER tier, variant), `DamageTaken`, `NumberOfTimesDamaged`, `RepairsMade`, `TotalNumberOfExpeditions`, `TotalNumberOfSuccessfulEvents`, `TotalNumberOfFailedEvents`, `HomeSystemSeed`, `ResourceSeed`, `ForcedTraitsSeed`, `TimeOfLastIncomeCollection`. `[verified: 25 frigates on the sample]`
- `Stats[0..4]` are Combat, Exploration, Industrial, Trade: each class's frigates peak in their own slot (Combat 33, Exploration 36, Trade 20 to 26). `[verified: 25 frigates, every class peaks in its own index]`
- `Stats[5]` is the Support stat (22 and 23 on the two Support frigates, near zero elsewhere); `Stats[4]` is low on Support frigates and around 10 on the rest, probably the fuel figure; `Stats[6..11]` are zero on every frigate. `[inferred: open one frigate's detail screen and compare the order]`
- A frigate is at home when its index appears in no running expedition's `AllFrigateIndices`. `[inferred: follows from the index lists; 20 of 25 on the sample]`
- `TimeOfLastIncomeCollection` is years old on every frigate and does not change with expeditions. `[open]`

### Daily offers

- `ExpeditionSeedsSelectedToday[5]` holds the Navigator's current five offers; `LastKnownDay` is the UTC day number, `floor(unix / 86400)`, of the day they belong to (20710 = 2026-09-14 UTC on the sample). The running expedition's seed was the second of the five. `[verified: day number arithmetic against the save date; seed match]`
- Offers refresh at 00:00 UTC, so `next_refresh = (LastKnownDay + 1) × 86400`. `[community: wiki and forum consensus on the daily reset; the day-number field is consistent with it]`
- Whether `LastKnownDay` rolls on game load or only when the Navigator is used is `[open: save after 00:00 UTC before talking to the Navigator]`.
- A debriefed expedition is presumably removed from `FleetExpeditions`, so "5 minus running today" is only an upper bound on unused offers. `[inferred]`
- Fleet Command Rooms are `^FRE_ROOM_FLEET` objects at the freighter base (7 on the sample); one running expedition per room. `[inferred: name from the ID; count not yet compared with the freighter; the one-per-room rule is `community`]`

## 8. Inventory

- Grids under `PlayerStateData`: `Inventory` (exosuit general, 10x12), `Inventory_Cargo` (7x5), `Inventory_TechOnly` (10x6), `FreighterInventory` with `_Cargo` and `_TechOnly`, `Chest1Inventory` … `Chest10Inventory` (the ten storage containers, 10x6 each), `ShipOwnership[i].Inventory` with `_Cargo` and `_TechOnly`, `VehicleOwnership[i].Inventory`, `Multitools[i].Store`, `WeaponInventory` (equipped multi-tool), `RefinerBufferData[i].InventoryContainer`, `ChestMagicInventory`, `ChestMagic2Inventory`, `CookingIngredientsInventory`, `FishBaitBoxInventory`, `FoodUnitInventory`, `RocketLockerInventory`, `GraveInventory`. `[verified: sample save, 2026-09-12]`
- Every grid has `Width`, `Height`, `Class.InventoryClass` (C/B/A/S), `Slots`, `SpecialSlots`, `ValidSlotIndices`, `StackSizeGroup`, `Name`. `Slots` lists occupied cells only; each has `Id`, `Amount`, `MaxAmount`, `Index {X, Y}`, `Type.InventoryType` (`Substance`, `Product`, `Technology`), `DamageFactor`, `FullyInstalled`, `AddedAutomatically`. `[verified: sample]`
- Technology slots carry `Amount` as charge or condition, not a count. `[verified: sample values]`
- Currencies: `Units`, `Nanites`, `Specials` (quicksilver). `[verified: field names and values on the sample; `Specials` as quicksilver is `community`]`
- `PrimaryShip` is the index of the active ship in `ShipOwnership`. `ShipOwnership` has 12 entries; entries without `Resource.Filename` are empty stubs. Ship type is the path segment before the file name in `Resource.Filename`: `FIGHTERS`, `DROPSHIPS` (hauler), `SCIENTIFIC` (explorer), `SAILSHIP` (solar), `S-CLASS` (exotic), `BIGGS` (living ship). `[verified: 8 real ships on the sample carry these segments; the segment-to-type names are `community` from save editors, not yet checked against the in-game ship list]`
- Ship `Location` and `Position` read zero on every ship in the sample, so their meaning is `[open: leave a ship on a planet and re-read]`. Exocraft carry the address of the base they are parked at in `Location`. `[verified: four at the farm base]`
- Storage containers are one inventory per number reachable from every base that has the matching object placed: `^CONTAINER0` in a planet base and `^FRE_ROOM_STORE0` on the freighter both open `Chest1Inventory`. `[verified: object IDs in the sample; the 0-based ID to 1-based container numbering and the shared-inventory behaviour are `community` and match play]`
- `ChestMagicInventory` (11 stacks on the sample) and `ChestMagic2Inventory` (empty) are unidentified. Ruled out: the Nutrient Processor ingredient store (that is `CookingIngredientsInventory`, confirmed by adding ingredients in-game and re-reading) and settlement storage. `[open: move a distinctive item into a suspected container and diff the save]`
- `Inventory_Cargo` grids are all empty on the sample; presumably the high-capacity slots. `[inferred]`
- `KnownProducts` (835 entries) and `KnownTech` (137) are ID lists. `[verified: sample counts]`

## 9. Discoveries

- Discovery records live under `DiscoveryManagerData` with a `DD.UA` save-layout address (section 5), a discovery type, and ownership data whose platform token is `ST` for Steam, `PS` for PlayStation, and so on. `[verified: model module, sample save]`
