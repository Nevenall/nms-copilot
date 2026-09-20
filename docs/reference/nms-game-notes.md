# Game notes

Mechanics, object and item IDs, constants, and where to look them up. Tags are explained in [README.md](README.md). Encodings of these things in the save file are in [nms-save-notes.md](nms-save-notes.md).

## Sources, ranked

1. **The save plus an independent observation.** A second save a known time later, the in-game screen, or object positions. Outranks everything below.
2. **Game data by internal ID.** The [AssistantNMS API](https://api.nmsassistant.com/index.html) serves data extracted from the game files, free and without a key: `https://api.nmsassistant.com/ItemInfo/GameId/<ID>/en` with the ID minus its `^` returns the display name, group, and description (`U_GENERATOR_S` is the Electromagnetic Generator, `FRE_ROOM_FLEET` the Fleet Command Room); the OpenAPI spec is at `/swagger/public-basic/swagger.json`. [nomansskyrecipes.com](https://nomansskyrecipes.com) keys its pages by internal ID too. [MBINCompiler](https://github.com/monkeyman192/MBINCompiler) decompiles the game's own data files and is the origin of the IDs and of the key mapping the save parser uses. The tables sit inside the game's `.pak` archives (`HGPAK` format since 5.50), which [HGPAKtool](https://github.com/monkeyman192/HGPAKtool) unpacks: `hgpaktool.exe -U -f="*reality/tables/inventorytable*" -O out <PCBANKS>\NMSARC.Precache.pak`, then `MBINCompiler.exe -q out/metadata/reality/tables/inventorytable.mbin` writes the `.MXML`. The Steam install is at `C:\Program Files (x86)\Steam\steamapps\common\No Man's Sky`; its build id is in `steamapps\appmanifest_275850.acf`. This is how the class-bonus tables below were read.
3. **Community documentation.** Two wikis carry the same body of articles: the [No Man's Sky wiki on Fandom](https://nomanssky.fandom.com) and the ad-free [independent wiki on Miraheze](https://nomanssky.miraheze.org). Prefer the Miraheze copy for lookups from a tool: Fandom answers scripted requests with 403, Miraheze serves them, and article content is the same. Check the page's last-edit date against the game version in play. [libNOM.io](https://github.com/zencq/libNOM.io) for the save container and slot layout; [NMSCD](https://github.com/NMSCD) for coordinate conversions. Item names on the wikis are keyed by display name, so match them to IDs with care, or resolve the ID through the API first.
4. **This project's own notes.** Weakest; anything here that is only `inferred` should move up the ladder when it can.

## Coordinates

- The galaxy is a grid of voxels 400 light-years on a side; system positions inside a voxel are not in the address, so distances between addresses are estimates with a mean error of about 265 ly within one voxel. `[community: NMSCD and wiki; the constants live in `crates/nms-core/src/address.rs`]`
- Portal glyph order is planet, system index (3 glyphs), Y (2), Z (3), X (3). The sixteen glyphs map to hex digits 0 to F in the order Sunset, Bird, Face, Diplo, Eclipse, Balloon, Boat, Bug, Dragonfly, Galaxy, Voxel, Whale, Tent, Rocket, Tree, Atlas. `[community: wiki; verified against known portal addresses in the glyph tests]`
- Signal booster coordinates (`XXXX:YYYY:ZZZZ:SSSS`) are the same fields in a different frame: add `0x801` to X and Z and `0x81` to Y to reach the portal frame. `[community: NMSCD converters; verified in the address tests]`
- Solar system index `0x079` is always a black hole and `0x07A` always an Atlas Interface; indices `0x3E8` to `0x429` are purple-star systems. `[community: wiki]`
- Galaxies are numbered by `RealityIndex`: 0 Euclid, 1 Hilbert Dimension, and on through all 256 (Odyalutai is index 255). `[community: wiki; the table is in `crates/nms-core/src/galaxy.rs`]`
- Region, system, and planet names, and a system's star colour, economy, wealth, conflict, race, and uncharted/abandoned/pirate flags, are all generated from the address and galaxy alone, and [nms_namegen](https://github.com/hadsh/nms_namegen) (MIT, Python 3.13+, numpy) reproduces the generator: `namegen.py region -p <PSSSYYZZZXXX> -g <galaxy>`, `system`, `planet`, `system-attributes`, and a `batch` mode at about 10,000 addresses a second. Its README scores the attributes at 98.4 to 99.8 percent against 1,000 wiki-recorded systems. `[verified: 2026-09-19, the system names it generates match all 44 unrenamed Euclid systems and all 8 Hilbert systems in the sample save exactly; the 13 non-matches are player-renamed (PIRATEBAY, Best Rest, CLASUIS-SYSTEM, Strange r) or named by another discoverer]`
- Region names are not in the save; the sample's home voxel (-532, -4, -1706) in Euclid is the `Piponera Anomaly`, and its current system 202 is `Dockbu`. `[game-data: nms_namegen from the address; the region name is unchecked against the in-game map]`
- Hello Games' [Galactic Atlas](https://galacticatlas.nomanssky.com) publishes 523 points of interest as JSON (`https://galactic-atlas.azureedge.net/assets/json/poi.json`), 53 of them named regions keyed by a 12-hex portal address, all community hubs and expedition sites. It is not a general region lookup. `[verified: fetched 2026-09-19]`

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

- Mineral Extractor `^U_EXTRACTOR_S` and Gas Extractor `^U_GASEXTRACTOR` each buffer 250 units; Supply Depot `^U_SILO_S` holds 1,000. `[verified: the sum of per-object caps reproduces the game's "of 4,750" capacity line exactly]` `[community: the Miraheze Supply Depot article, "Each Supply Depot adds 1000 units of storage capacity (the extractors themselves store 250)"]`
- A gas extractor and a mineral extractor share the same 250 cap. `[verified: both kinds plateau at exactly 360,000 = 250 × 1,440 across four bases of the sample, and the game's capacity line counts 250 for either kind]`
- `^U_SILO_S` is the only depot part in the game, so every depot holds 1,000. `[game-data: AssistantNMS knows `U_SILO_S` as the Supply Depot and has no `U_SILO_M` or `U_SILO_L`, 2026-09-15]`
- A pipe network is a Supply Grid in the game's own words, and resources reach the depots "in timed batches", so a depot and an extractor on one network need not hold the same amount at any instant. `[community: Miraheze wiki, Supply Depot article, current to Singularity]` `[verified: the save shows the two kinds carrying different values on one traced network]`
- One Supply Pipe segment reaches at most 240 u; a longer run is a chain of segments, and a chain can also pass through a depot. `[community: Miraheze wiki, Supply Pipe article]` `[verified: the longest segment in the sample is 196.3 u]`
- Which objects share a network is worked out from the pipe segments' endpoints, not from the depots and extractors; the rule is in [nms-save-notes.md](nms-save-notes.md#wires-pipes-and-cables-what-is-connected-to-what). `[verified: the reconstruction reproduces the Farm's three in-game networks, 2026-09-15]`
- Depots built close together join into one network with no pipe between them, so a network can spread along a touching row. `[community: the save's owner reports having had to rebuild after placing two depots of different networks too close together, 2026-09-15]` `[verified: depots seen touching sit 1.40 to 1.83 apart and no reconstruction reproduces the Farm without treating them as joined]`
- Extractors produce in real time while the player is away. Observed rate on the sample farm: about 33 units per hour per gas extractor on a nitrogen hotspot. `[verified: 79 units in 48 minutes across 3 extractors; rate depends on hotspot class and is a single observation]`

### Power

- Battery `^U_BATTERY_S`: the save stores charge, 45,000 when the game shows it full. `[inferred: see the save notes; part-charged reading not yet compared]`
- Electromagnetic Generator `^U_GENERATOR_S`: placed on power hotspots, so it sits in clusters away from the rooms. Reads 0 in the save. `[verified: recipe database by ID; positions on the sample]`
- Solar Panel `^U_SOLAR_S`. `[game-data: recipe database ID; not in the sample]`
- Biofuel Reactor `^U_BIOGENERATOR`. `[game-data: recipe database ID; save value not decoded]`
- Wires are `^U_POWERLINE` (Electrical Wiring), pipes `^U_PIPELINE` (Supply Pipe), and teleporter links `^U_PORTALLINE` (Teleport Cable). All three store their run as a vector instead of an orientation, which is what makes the base's graph readable; see the save notes. `[verified: sample]` `[game-data: the three names by ID from AssistantNMS, 2026-09-15]`
- Prefab rooms (Cylindrical, Cuboid, Bio-Dome) do not include solar panels; an earlier belief that they did came from mislabelling `^U_GENERATOR_S`. `[verified: as above]`
- `^U_PARAGON` sits at a corvette's origin with 1,000,000 in its value and is not a buildable part. `[open]`

## Fleet

- The freighter's Navigator offers five expeditions per day; the set refreshes at 00:00 UTC. `[community: wiki and forum consensus; the save's `LastKnownDay` is a UTC day number, consistent with it]`
- Each running expedition occupies one Fleet Command Room and takes up to five frigates. `[community: wiki; the sample's one expedition has five frigates]`
- Save category names differ from the game's labels. The Navigator lists Combat Patrol, Trade Expedition, Balanced Expedition, Voyage of Discovery, and Industrial Expedition, so `Diplomacy` shows as Trade, `Mining` as Industrial, `Exploration` as a Voyage of Discovery, and `Support` most likely as Balanced. `[verified: the five labels on the Navigator screen, 2026-09-14; the `Diplomacy` run had `^INT_TRADING_*` interventions; the `Support` to Balanced pairing is `inferred` until a Balanced run is seen in the save]`
- The Navigator shows each offer's name, type, difficulty (one to five stars), duration, and distance before any frigates are assigned. One day's set: 5h 33m and 1,578 ly, 5h 8m and 1,458 ly, 5h 33m and 1,578 ly, 8h 53m and 2,520 ly, 16h 34m and 4,716 ly. The 16h 34m offer is within six minutes of the 16h 28m a Very Long run took the same day, so the displayed duration is close to the actual run for this fleet. `[verified: Navigator screen, 2026-09-14; which duration class each figure belongs to is `inferred`]`
- The displayed duration is the offer's distance at a constant speed: all five offers above come to 284 to 285 light years per hour, so `hours = distance / 284`. The speed modules on the assigned frigates are not in that figure, since it is shown before frigates are chosen; the run's total is that figure times `SpeedMultiplier`. `[verified: five offers on one screen agree within a minute; the 16h 34m offer launched with a 0.97 multiplier showed 15h 58m 5s left 386 s after launch, a total of 57,871 s against 59,640 × 0.97 = 57,851 s]`
- Duration classes `Short`, `Medium`, `Long`, `VeryLong`. Community figures: Short about an hour, Medium four to five, Long 21 to 24, Very Long up to 28 hours, growing with fleet size; fleets of three or fewer frigates run much shorter, one to four and a half hours. `[community: wiki; too loose to present as fact]` One measured Very Long run with five frigates and no consumables came to 16h 28m of active time. `[verified: 2026-09-14, the game's remaining time plus the save's active time; see the save notes]`
- Expedition length is modified by frigate traits, each a percentage of the duration: Alcubierre Drive, Dynamic Ballast, Expert Navigator, and Motivated Crew give −2%, Experimental Impulse Drive −3%, Local Time Dilator and Mass Driver −1%; harmful traits such as Inefficient Engine, Leaky Fuel Tubes, Oil Burner, Poorly-Aligned Ballast, and Thirsty Crew add. The Fuel Oxidiser consumable also shortens the run. `[community: Miraheze wiki, Frigate article, edited 2025-05-03]`
- Events happen at a steady cadence along the route; 55 minutes per event on the sample run, though the final leg ran longer. When an intervention event is reached the fleet holds until the player answers on the freighter bridge ("waiting for player"), and the clock stops during the hold. `[verified: in-game state matched the save flags; the pause is confirmed by the `StartTime` shift in the save notes; the cadence is one observation]`
- Frigate classes match the expedition categories. The four main stats are Combat, Exploration, Industrial, Trade; Support frigates have a fifth. Grades run C, B, A, S. `[verified: 25 frigates, each class peaks in its own stat]`
- Trait IDs name a stat (`COMBAT`, `EXPLORE`, `MINING`, `TRADING`, `FUEL`, `SPEED`, `INVULN`), a tier (PRI, SEC, TER), and a variant, for example `^EXPLORE_TER_4`. Support frigates carry `^FUEL_PRI`. `[verified: 25 frigates' traits, 2026-09-14]`
- The module families and what they do: `COMBAT`, `EXPLORE`, `MINING`, and `TRADING` modules add to the matching stat and the fleet screen's stat numbers include them; `FUEL` modules cut the expedition's fuel cost by a fixed amount per run; `SPEED` modules shorten the run by 1 to 3 percent each; `INVULN` modules are the damage-reduction family (Advanced Maintenance Drones, Holographic Components, Self-Repairing Hull), which lessens the damage and negative traits a failed encounter can inflict. `[community: Miraheze wiki, Frigate article, trait lists; the family-to-effect pairing for `INVULN` is inferred from the ID name against the wiki's damage-reduction group]`

## Wonders

The Wonders catalogue has seven categories plus personal records. The save stores each category as a fixed array in the order below, which is the order the game and the wiki list them; see [nms-save-notes.md](nms-save-notes.md) section 10.

- Planet records, 11: Hottest Temperature (°C), Coldest Temperature (°C), Most Toxic Atmosphere (Tox), Highest Radiation Level (Rads), Strongest Reality Distortion (rQ), Largest Planet (u), Smallest Planet (u), Highest Peak (u), Deepest Ocean (u below), Most Perfect (Paradise Quotient %), Least Hospitable (Hostility Quotient %). `[community: Miraheze wiki, Wonders Catalogue article, current to Aquarius; verified: the sample's eleven values sit in these ranges in this order, 316 °C, −123 °C, 217 Tox, 31.9 Rads, 0.8 rQ, 524,102 u, 14,624 u, 3,505 u, 106 u, 87.6 %, 26.6 %]`
- Fauna records, 15: Largest Herbivore (m), Smallest Herbivore (m), Largest Carnivore (m), Smallest Carnivore (m), Most Intelligent Being (iep), Most Vicious Hunter (pav), Highest Body Temperature (°C), Greatest Freeze Tolerance (ppu), Most Corrosive Blood (pH), Most Radiation Resistant (uav), Strongest Psionic Field (hertz/u), Largest Aquatic Lifeform (kg), Convergence Potential (ieq), Heaviest Flying Lifeform (kg), Most Pressure Resistant (bar). `[community: same article; inferred: the sample's sizes 7.6, 0.3, 7.0, 1.2 m and weights 157 and 191 kg fall where this order puts them]`
- Flora records, 8: Strongest Photoactivity, Longest Lived, Most Invasive, Deepest Roots, Strongest Cold Resistance, Highest Drought Tolerance, Greatest Radiation Resistance, Most Toxic If Eaten. The game shows no figure for these; the save holds one anyway (about 4.1 to 4.6 on the sample). `[community: same article; the stored figure is `[open]`]`
- Mineral records, 8: Greatest Specific Gravity (kg/u³), Highest Electric Potential (mV), Strongest Magnetic Field (A/m), Highest Metal Content (%), Most Crystalline Structure (%), Highest Salt Content (%), Longest Half-Life (years), Highest Trace Organic Content (%). `[community: same article; verified: slot 3 reads 79.9 on the sample, a percentage, where the others read about 4]`
- Treasures, 13: the most valuable found of each of the 13 artifact variations, shown in years, lifetimes, or mutated genes. `[community: same article; the sample has nine of thirteen filled]`
- Collected glitches, 11: the Stabilised Reality Glitches picked up on exotic planets. `[community: same article; the sample has none]`
- Personal records, up to 12: any planet, creature, flora, or mineral the player assigns to a named category of their own. `[community: same article; verified: twelve named entries on the sample]`
- Records only go back to update 3.30 (June 2021), and a record holds the value encountered, not the planet's true extreme, so a hottest-temperature entry needs a firestorm to have been seen. `[community: same article]`

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

- The save carries no display names; a full ID-to-name table is planned (arc 01), generated from the AssistantNMS API above by internal ID, with a public refiner-recipe gist (about 90 substances) and the wiki's Item Id List (products and technology) as fallbacks. `[game-data: the API resolves `ASTEROID2`, `U_GENERATOR_S`, and `FRE_ROOM_FLEET` by ID; the gist and wiki are `community`]`
- Corvette parts are products whose IDs start with `^B_` (`^B_CON_5`, `^B_WNG_G`, `^B_STR_A_N`, `^B_HAB_B`); they live in `CorvetteStorageInventory`, not in a general grid. The AssistantNMS API does not know them (its full ID list has no `B_` entries as of 2026-09-15), so the tool shows the raw ID; nor does it know `EGG2` to `EGG4`, `HULK1`, or the `CV_*` corvette weapons. `[inferred: from the IDs and the key name on the sample, 2026-09-15; the API gap is `verified` by `scripts/gen-items.py`, whose misses are listed in `scripts/item-ids-missing.txt`]`
- The bundled name table (`crates/nms-core/data/items.json`, 3,323 entries) resolves every ID the API lists; `ASTEROID2` is Gold, `STELLAR2` Chromatic Metal, `LAND1` Ferrite Dust, `PRODFUEL2` Life Support Gel, `FOOD_B_APPLE` 'Apple' Roll. `[game-data: generated from the API on 2026-09-15]`
- Ship archetypes by the folder in `Resource.Filename`: `FIGHTERS` fighter, `DROPSHIPS` hauler, `SCIENTIFIC` explorer, `SAILSHIP` solar, `S-CLASS` exotic, `BIGGS` living ship, `SENTINELSHIP` interceptor. `[community: folder names as read by save editors; not yet checked against the in-game ship list. `SENTINELSHIP` is `verified`: the sentinel ship claimed on 2026-09-18 carries that folder and a `^ROBOT_SHIP` 1 stat, matching the Interceptor row of the ship table]`
- The ten numbered storage containers are one shared inventory each, reachable from any base or freighter room with the matching number. `[community: wiki; matches play]`

## Class bonuses

Multi-tools and ships carry class bonuses in `Store.BaseStatValues` / `Inventory.BaseStatValues` (see [nms-save-notes.md](nms-save-notes.md), Inventory). The ranges they are rolled from live in the game's `METADATA/REALITY/TABLES/INVENTORYTABLE.MBIN` (`GcInventoryTable`): `WeaponBaseStatsData[type][class].BaseStats[]` and `ShipBaseStatsData[type][class].BaseStats[]`, each entry `BaseStatID`, `Min`, `Max`, `MinFixedAdd`, `MaxFixedAdd`. The tables below were read from that file on 2026-09-18 by unpacking `NMSARC.Precache.pak` with HGPAKtool 1.1.3 and decompiling with MBINCompiler 7.02.0-pre2, from Steam build 25351301 (updated 2026-09-16), the build that wrote the sample save (`Version` 4737).

- The save's `Value` is in the table's units, a percentage: every fixed-range entry matches exactly (Rifle mining `0–0` reads `0.0` on both Rifle tools of the sample; Experimental/Royal S scan `100–100` reads `100.0`), and every ranged entry falls inside its range (S Rifle damage 19.91 in 15–20, scan 15.83 in 15–20; A Rifle damage 13.07 in 10–15, scan 14.03 in 10–15; S Experimental/Royal damage 15.09 in 15–25, mining 25.90 in 25–30; the S fighter of the save notes, damage 79.6, shield 26.5, agility 37.8, sits in Fighter S 70–90, 24–38, 35–45). `[verified: three multitools and one ship on the 2026-09-17 save against the game table, 2026-09-18]`
- Rolls are continuous, not whole percentages (19.9056, 15.8340, 13.0698, 25.8963 on the sample), and every `MinFixedAdd`/`MaxFixedAdd` in the table is zero, so a `Value` is simply a point between `Min` and `Max`. Whether the point is uniform is not known. `[verified: values on the sample; the 524 FixedAdd fields read zero in the decompiled table]`
- Multi-tool type is not stored in the live record, but the stat triple narrows it: a mining bonus of exactly `0.0` on an S/A/B tool means Rifle (only Rifle has mining fixed at 0 above C); scan `100.0` means Experimental or Royal (which share every range and cannot be told apart by stats); damage above 20 on an S tool rules out Pistol, Rifle, Experimental, Royal, and Atlantid. Sentinel and Staff overlap (S damage 32–50, mining 10–15, scan 45–50 fit both), as do the three Atlantid variants. On the sample: tool 0 (S, 15.1/25.9/100.0) is Experimental or Royal, tool 1 (S, 19.9/0.0/15.8) is a Rifle, tool 2 (A, 13.1/0.0/14.0) is a Rifle. `[inferred: from the ranges below; confirm by archiving a tool, which stores `WeaponClass.WeaponStatClass`, or by reading the type on the trade screen]`
- A roll is fixed for the life of the tool: the same values are written on every save (identical on the 2026-09-16 and 2026-09-17 saves) and the wiki says class bonuses "are static and do not change". `[community: Fandom wiki, Multi-Tool, Class Bonuses; stability across two saves with no upgrade between them is `verified` but does not test an upgrade]`
- Whether the Multi-tool Upgrade Station rerolls the bonuses into the new class's range, scales them, or leaves them is `[open: tool 2 of the sample is an A-class Rifle at damage 13.07, scan 14.03; upgrade it to S (50,000 nanites) and re-read. A fresh roll lands anywhere in 15–20 / 15–20; unchanged values would sit below the S minimum. Forum reports say the stats "are most likely going to see an improvement" and that a good A roll has "worse odds" of a big S gain, which reads as a fresh roll but is not a measurement]`.
- The internal type names are `GcWeaponClasses.WeaponStatClassEnum`: `Pistol`, `Rifle`, `Pristine` (Experimental), `Alien`, `Royal`, `Robot` (Sentinel), `Atlas`, `AtlasYellow`, `AtlasBlue` (Atlantid variants), `Staff`. The same enum names the archived-tool `WeaponClass.WeaponStatClass` field. `[game-data: libMBIN `GcWeaponClasses`; the display-name pairing for Pristine, Robot, and Atlas is `community` from the wiki's type list matched to the ranges]`
- `WeaponInventoryMaxUpgradeSize` reads C 21, B 30, A 45, S 60, but it is not a cap on a tool's slots: a C-class Sentinel tool claimed on 2026-09-18 has 25 unlocked slots. What it bounds is `[open: probably the slots the Upgrade Station can add, not the generated count]`. Slot ranges at generation (`WeaponCostData` `MinSlots`–`MaxSlots`): Pistol 10–30, Experimental and Alien 11–30, Royal 13–30, Rifle, Sentinel, Atlantid, and Staff 17–30. `[game-data: same table]`
- Class odds at generation by system wealth (`ClassProbabilityData` C/B/A/S): Poor 60/30/10/0, Average 49/35/15/1, Wealthy 30/40/28/2, Pirate 5/5/5/5. `[game-data: same table; whether the Pirate row is weights or percentages is not known]`
- The wiki's datamined table (labelled 4.41) differs from the current build in three cells: Sentinel S damage is now 32–50 (wiki 25–50) and scan 45–55 (wiki 40–50); Staff S scan is 40–50 (wiki 40–55). Its footnote that Royal, Sentinel, and Atlantid have zero class multipliers and a 20% `CoolMultiplier` is also stale: the current table has `ClassMultiplier` 0/10/25/50 for Royal and Sentinel, 0/1/2/3 for Atlantid, and `CoolMultiplier` 0 everywhere. `[game-data: the decompiled table wins over the wiki copy]`

Multi-tool ranges, `damage / mining / scan` percentages, `Min–Max`:

| Type (enum) | C | B | A | S | Evidence |
|-------------|---|---|---|---|----------|
| Pistol | 0 / 5–10 / 10–20 | 0 / 10–15 / 25–30 | 0 / 15–20 / 35–40 | 0 / 20–35 / 45–50 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Rifle | 0–5 / 0 / 0–5 | 5–10 / 0 / 5–10 | 10–15 / 0 / 10–15 | 15–20 / 0 / 15–20 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Experimental (`Pristine`) | 0–5 / 5–10 / 40–50 | 5–10 / 10–20 / 60–70 | 10–15 / 20–25 / 80–90 | 15–25 / 25–30 / 100 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Alien | 10–15 / 0–5 / 20–25 | 15–20 / 5–10 / 30–35 | 20–25 / 10–15 / 40–45 | 25–35 / 15–20 / 50–60 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Royal | 0–5 / 5–10 / 40–50 | 5–10 / 10–20 / 60–70 | 10–15 / 20–25 / 80–90 | 15–25 / 25–30 / 100 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Sentinel (`Robot`) | 10–20 / 0–5 / 20–25 | 15–25 / 5–10 / 30–35 | 20–30 / 5–10 / 35–45 | 32–50 / 10–15 / 45–55 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Atlantid (`Atlas`, `AtlasYellow`, `AtlasBlue`, identical) | 5–10 / 0–15 / 20–25 | 5–10 / 15–25 / 30–35 | 10–15 / 20–30 / 35–45 | 10–20 / 35–50 / 40–50 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Staff | 10–20 / 0–5 / 20–25 | 15–25 / 5–10 / 30–35 | 20–30 / 5–10 / 35–45 | 25–50 / 10–15 / 40–50 | `[game-data: INVENTORYTABLE, build 25351301]` |

Ship ranges, `damage / shield / hyperdrive / agility` percentages (`^SHIP_DAMAGE`, `^SHIP_SHIELD`, `^SHIP_HYPERDRIVE`, `^SHIP_AGILE`), `Min–Max`:

| Type (enum) | C | B | A | S | Evidence |
|-------------|---|---|---|---|----------|
| Fighter | 8–15 / 0 / 0 / 5–15 | 24–45 / 8–15 / 0 / 15–20 | 50–75 / 24–30 / 0 / 20–30 | 70–90 / 24–38 / 0 / 35–45 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Hauler (`Dropship`) | 0 / 18–30 / 5–10 / 0 | 3–8 / 40–55 / 8–15 / 0–5 | 8–15 / 60–75 / 23–37 / 5–10 | 15–30 / 95–125 / 45–58 / 10–15 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Explorer (`Scientific`) | 0 / 0 / 15–25 / 5–10 | 0 / 4–16 / 30–45 / 10–20 | 0 / 15–25 / 50–85 / 20–25 | 0 / 30–40 / 90–120 / 25–32 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Shuttle | 0 / 0 / 0 / 0 | 3–8 / 3–8 / 3–8 / 3–5 | 8–15 / 8–15 / 8–15 / 8–15 | 25–30 / 25–30 / 25–30 / 15–35 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Exotic (`Royal`) | 35–50 / 55–60 / 50–65 / – | 35–50 / 55–60 / 50–65 / – | 35–50 / 55–60 / 50–65 / – | 50–75 / 70–90 / 75–95 / 40–50 | `[game-data: INVENTORYTABLE, build 25351301; C to A carry no agility entry]` |
| Living ship (`Alien`) | – | – | – | 50–75 / 15–38 / 75–95 / 10–15 | `[game-data: INVENTORYTABLE, build 25351301; only S has entries, plus `ALIEN_SHIP` 1]` |
| Solar (`Sail`) | 8–15 / 8–15 / 8–15 / 5–10 | 23–45 / 8–30 / 15–30 / 10–20 | 45–60 / 23–38 / 30–45 / 20–30 | 60–75 / 23–50 / 38–50 / 35–40 | `[game-data: INVENTORYTABLE, build 25351301]` |
| Interceptor (`Robot`) | 8–15 / 3–8 / 0 / 8–15 | 24–45 / 8–15 / 3–8 / 20–30 | 50–75 / 12–20 / 3–10 / 20–30 | 60–80 / 20–35 / 5–10 / 40–50 | `[game-data: INVENTORYTABLE, build 25351301; every class also carries `ROBOT_SHIP` 1]` |
| Corvette | 0 / 18–30 / 5–10 / 0 | 3–8 / 40–55 / 8–15 / 0–5 | 8–15 / 60–75 / 23–37 / 5–10 | 15–30 / 95–125 / 45–58 / 10–15 | `[game-data: INVENTORYTABLE, build 25351301; identical to Hauler]` |
| Freighter | hyperdrive 7–15, fleet 1–10 | 10–19, 10–20 | 40–60, 20–40 | 60–80, 40–60 | `[game-data: INVENTORYTABLE, build 25351301; stats are `FREI_HYPERDRIVE` and `FREI_FLEET`]` |

## Time

- The game rewrites every base object's timestamp to the save time on every save, so "as of" for any base reading is the save time, not the base's last edit. `[verified: two saves, every base]`
- Crops, extractors, and fleet expeditions all run on real time. `[verified for crops and extractors: two-save experiment; `community` for expeditions]` That they continue while the game is closed is `[community: wiki]`.
- `LastKnownDay` in the save is a UTC day number (`floor(unix / 86400)`). `[verified: 20710 = 2026-09-14 UTC on a save taken that day]`
