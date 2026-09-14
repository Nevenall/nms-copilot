# Game notes

Mechanics, object and item IDs, constants, and where to look them up. Tags are explained in [README.md](README.md). Encodings of these things in the save file are in [nms-save-notes.md](nms-save-notes.md).

## Sources, ranked

1. **The save plus an independent observation.** A second save a known time later, the in-game screen, or object positions. Outranks everything below.
2. **Game data by internal ID.** [nomansskyrecipes.com](https://nomansskyrecipes.com) keys its pages by internal ID (`^U_GENERATOR_S`, `^ASTEROID2`), so a lookup there is unambiguous; MBINCompiler's decompiled game data is the origin of those IDs.
3. **Community documentation.** The [No Man's Sky wiki](https://nomanssky.fandom.com) for mechanics and item names; [libNOM.io](https://github.com/zencq/libNOM.io) for the save container and slot layout; [NMSCD](https://github.com/NMSCD) for coordinate conversions. Item names there are keyed by display name, so match them to IDs with care.
4. **This project's own notes.** Weakest; anything here that is only `inferred` should move up the ladder when it can.

## Coordinates

- The galaxy is a grid of voxels 400 light-years on a side; system positions inside a voxel are not in the address, so distances between addresses are estimates with a mean error of about 265 ly within one voxel. `[community: NMSCD and wiki; the constants live in `crates/nms-core/src/address.rs`]`
- Portal glyph order is planet, system index (3 glyphs), Y (2), Z (3), X (3). The sixteen glyphs map to hex digits 0 to F in the order Sunset, Bird, Face, Diplo, Eclipse, Balloon, Boat, Bug, Dragonfly, Galaxy, Voxel, Whale, Tent, Rocket, Tree, Atlas. `[community: wiki; verified against known portal addresses in the glyph tests]`
- Signal booster coordinates (`XXXX:YYYY:ZZZZ:SSSS`) are the same fields in a different frame: add `0x801` to X and Z and `0x81` to Y to reach the portal frame. `[community: NMSCD converters; verified in the address tests]`
- Solar system index `0x079` is always a black hole and `0x07A` always an Atlas Interface; indices `0x3E8` to `0x429` are purple-star systems. `[community: wiki]`
- Galaxies are numbered by `RealityIndex`: 0 Euclid, 1 Hilbert Dimension, and on through all 256 (Odyalutai is index 255). `[community: wiki; the table is in `crates/nms-core/src/galaxy.rs`]`

## Bases

### Rooms and structures

| Object ID | Object | Evidence |
|-----------|--------|----------|
| `^MAINROOM` | Cylindrical Room | `[game-data: recipe database]` |
| `^MAINROOMCUBE` | Cuboid Room | `[game-data: recipe database]` |
| `^BIOROOM` | Bio-Dome | `[game-data: recipe database]` |
| `^CONTAINER0` … `^CONTAINER9` | Storage Container 1 to 10 at a planet base | `[verified: `^CONTAINER0` at two bases on the sample; the 0-to-1 numbering is `inferred`]` |
| `^FRE_ROOM_STORE0` … `^FRE_ROOM_STORE9` | Storage Room 1 to 10 on the freighter | `[verified: all ten on the sample freighter]` |
| `^FRE_ROOM_FLEET` | Fleet Command Room | `[inferred: name from the ID; 7 on the sample freighter, not yet counted in-game]` |
| `^FRE_ROOM_PLANT1` | freighter Cultivation Chamber (planter room) | `[inferred: name from the ID; not looked up]` |
| `^PLANTTUBE` | Hydroponic Tray | `[inferred: name from the ID]` |

### Crops

| Object ID | Crop | Growth time | Evidence |
|-----------|------|-------------|----------|
| `^LUSHPLANT` | Star Bulb | 4 h | `[verified: save value grew by exactly the interval between two saves and capped at 14,400]` |
| `^RADIOPLANT` | Gamma Root | 4 h | `[verified: capped at 14,400 in the sample]` |
| `^TOXICPLANT` | Fungal Mould | 4 h | `[verified: capped at 14,400 in the sample]` |
| `^POOPPLANT` | Coprite | 4 h | `[verified: capped at 14,400 in the sample]` |
| `^SNOWPLANT` | Frost Crystal | 1 h | `[verified: rose to 3,600 between two saves and stopped]` |
| `^BARRENPLANT` | Cactus Flesh | 16 h | `[verified: grew by the save interval; the 57,600 cap is `community` from the wiki, not yet seen reached]` |
| `^SCORCHEDPLANT` | Solanium | 16 h | `[community: wiki growth time; ID seen in the sample]` |
| `^CREATUREPLANT` | Mordite Root | 8 h | `[game-data: ID and time from game data; not seen in a save]` |

- Growth is real time: it accrued at wall-clock rate between the two saves. `[verified: two-save experiment]` Whether it continues with the game closed is `[community: wiki]`.

### Extraction

- Mineral Extractor `^U_EXTRACTOR_S` and Gas Extractor `^U_GASEXTRACTOR` each buffer 250 units; Supply Depot `^U_SILO_S` holds 1,000. `[verified: the sum of per-object caps reproduces the game's "of 4,750" capacity line exactly]`
- A pipe network shares one pool, split evenly across every member including extractors, each capped. When the smallest members fill, the rest keep filling. `[verified: in-game depot screen against the save, three networks, 2026-09-14]`
- Extractors produce in real time while the player is away. Observed rate on the sample farm: about 33 units per hour per gas extractor on a nitrogen hotspot. `[verified: 79 units in 48 minutes across 3 extractors; rate depends on hotspot class and is a single observation]`
- Only `^U_SILO_S` has been seen as a depot; whether every depot is 1,000 is `[open]`.

### Power

- Battery `^U_BATTERY_S`: the save stores charge, 45,000 when the game shows it full. `[inferred: see the save notes; part-charged reading not yet compared]`
- Electromagnetic Generator `^U_GENERATOR_S`: placed on power hotspots, so it sits in clusters away from the rooms. Reads 0 in the save. `[verified: recipe database by ID; positions on the sample]`
- Solar Panel `^U_SOLAR_S`. `[game-data: recipe database ID; not in the sample]`
- Biofuel Reactor `^U_BIOGENERATOR`. `[game-data: recipe database ID; save value not decoded]`
- Wires are `^U_POWERLINE` segments and pipes `^U_PIPELINE`. `[verified: sample]`
- Prefab rooms (Cylindrical, Cuboid, Bio-Dome) do not include solar panels; an earlier belief that they did came from mislabelling `^U_GENERATOR_S`. `[verified: as above]`
- `^U_PARAGON` sits at a corvette's origin with 1,000,000 in its value and is not a buildable part. `[open]`

## Fleet

- The freighter's Navigator offers five expeditions per day; the set refreshes at 00:00 UTC. `[community: wiki and forum consensus; the save's `LastKnownDay` is a UTC day number, consistent with it]`
- Each running expedition occupies one Fleet Command Room and takes up to five frigates. `[community: wiki; the sample's one expedition has five frigates]`
- Save category names differ from the game's labels: `Mining` shows as Industrial and `Diplomacy` as Trade; `Combat`, `Exploration`, and `Support` match. `[inferred: the sample's `Diplomacy` run has trading events and intervention IDs (`^INT_TRADING_*`); confirm on the bridge]`
- Duration classes `Short`, `Medium`, `Long`, `VeryLong`. Community figures: Short about an hour, Medium four to five, Long 21 to 24, Very Long up to 28 hours, growing with fleet size and shrinking with a Fuel Oxidiser. `[community: wiki and forums; too loose to present as fact]`
- Events happen at a steady cadence along the route; 55 minutes per event on the sample run. When an intervention event is reached the fleet holds until the player answers on the freighter bridge ("waiting for player"). `[verified: in-game state matched the save flags; the cadence is one observation]`
- Frigate classes match the expedition categories. The four main stats are Combat, Exploration, Industrial, Trade; Support frigates have a fifth. Grades run C, B, A, S. `[verified: 25 frigates, each class peaks in its own stat]`
- Trait IDs name a stat, a tier (PRI, SEC, TER), and a variant, for example `^EXPLORE_TER_4`. `[inferred: pattern across the sample's traits]`

## Items

| ID | Name | Evidence |
|----|------|----------|
| `^ASTEROID1` | Silver | `[community: refiner recipe gist]` |
| `^ASTEROID2` | Gold | `[community: refiner recipe gist]` |
| `^ASTEROID3` | Platinum | `[community: refiner recipe gist]` |
| `^STELLAR2` | Chromatic Metal | `[community: refiner recipe gist]` |
| `^LAND1` | Ferrite Dust | `[community: refiner recipe gist]` |
| `^CATALYST1` | Sodium | `[community: refiner recipe gist]` |
| `^OXYGEN` | Oxygen | `[community: refiner recipe gist]` |
| `^GAS1`, `^GAS2`, `^GAS3` | Sulphurine, Radon, Nitrogen | `[community: refiner recipe gist]` |

- The save carries no display names; a full ID-to-name table is planned (arc 01). A public refiner-recipe gist keys about 90 substances by ID, and the wiki's Item Id List covers products and technology. `[community]`
- Ship archetypes by the folder in `Resource.Filename`: `FIGHTERS` fighter, `DROPSHIPS` hauler, `SCIENTIFIC` explorer, `SAILSHIP` solar, `S-CLASS` exotic, `BIGGS` living ship. `[community: folder names as read by save editors; not yet checked against the in-game ship list]`
- The ten numbered storage containers are one shared inventory each, reachable from any base or freighter room with the matching number. `[community: wiki; matches play]`

## Time

- The game rewrites every base object's timestamp to the save time on every save, so "as of" for any base reading is the save time, not the base's last edit. `[verified: two saves, every base]`
- Crops, extractors, and fleet expeditions all run on real time. `[verified for crops and extractors: two-save experiment; `community` for expeditions]` That they continue while the game is closed is `[community: wiki]`.
- `LastKnownDay` in the save is a UTC day number (`floor(unix / 86400)`). `[verified: 20710 = 2026-09-14 UTC on a save taken that day]`
