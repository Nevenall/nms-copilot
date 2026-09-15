# Inventory model and queries

Answer "do I have X, how much, and where?" from the save file, across every container the player owns.

**Status:** first draft 2026-09-14, not started. Revised 2026-09-15 against the reference notes, the code as left by arcs 02 and 03, the dashboard (0048), and a fresh read of the save: the cache format version and typed base objects already exist, currencies already live on `PlayerState`, the item-name source is now the AssistantNMS API, live updates follow the fleet's whole-replacement delta, `CorvetteStorageInventory` joins the container list, `WeaponInventory` leaves it as a copy of the active multi-tool, and `ValidSlotIndices` is the only source of unlocked slots. The sample numbers below are from the 2026-09-12 save and have drifted since; verification now runs against the committed fixture.

**Depends on:** the item name table described below, before any listing is readable; a `PlayerShipBase` variant on `BaseType` (see shared groundwork in the [project README](../README.md)), so a corvette-mounted container is not labelled as a planet base; the fleet's delta pattern from arc 03.

---

## Context

The atlas knows where the player has been but nothing about what they carry. The save file records every inventory grid the game has: exosuit, freighter, the ten storage containers, every owned ship, exocraft, and multi-tool, plus refiner buffers and a handful of minor containers. Each grid lists its occupied slots with the item ID, amount, and stack maximum. Nothing in the tool reads any of it.

The motivating question was "do I have gold and silver, and where?" The answer from the save is: Gold (`^ASTEROID2`) 11,297 total, in Storage 1 (9,999 and 927) and the exosuit (371); Silver (`^ASTEROID1`) 2,959 in Storage 1 and the freighter; Platinum (`^ASTEROID3`) 2,994 in Storage 1. That report is the target output of this arc.

---

## What the save contains (verified)

> The decode facts below are maintained in [docs/reference/nms-save-notes.md](../../../reference/nms-save-notes.md) with their evidence; this section is the snapshot the plan was written from.

All paths are under `BaseContext.PlayerStateData` (or the expedition context; the active one applies).

### Grids

| Key | Grid | Occupied in sample | Notes |
|-----|------|--------------------|-------|
| `Inventory` | 10x12 | 30 | exosuit general |
| `Inventory_Cargo` | 7x5 | 0 | exosuit cargo (high-capacity) |
| `Inventory_TechOnly` | 10x6 | 42 | exosuit technology |
| `FreighterInventory` (+`_Cargo`, `_TechOnly`) | 7x5 | 24 | class A |
| `Chest1Inventory` .. `Chest10Inventory` | 10x6 each | 5 to 49 | the ten storage containers |
| `ShipOwnership[i].Inventory` (+`_Cargo`, `_TechOnly`) | 7x5 | 6 on the primary ship | 12 entries, 8 real ships, 4 empty stubs with no `Resource.Filename` |
| `VehicleOwnership[i].Inventory` | | 0 | 7 exocraft; `Location` holds the base address where parked |
| `Multitools[i].Store` | | 17 to 18 | technology only |
| `WeaponInventory` | 10x6 | 18 | a copy of `Multitools[ActiveMultioolIndex].Store`; not a container of its own |
| `RefinerBufferData[i].InventoryContainer` | | | 38 refiners; 2 held Chromatic Metal (180 and 49) |
| `CorvetteStorageInventory` | 10x16 | 32 | corvette parts (`^B_CON_5`, `^B_WNG_G`, …), all products; not in the first draft |
| `ChestMagicInventory`, `ChestMagic2Inventory` | 10x6 | 11, 0 | meaning unknown, see open questions |
| `CookingIngredientsInventory`, `FishBaitBoxInventory`, `FoodUnitInventory`, `RocketLockerInventory`, `GraveInventory` | small | 0 to 1 | minor containers |

Currency sits beside them: `Units` (351,503,622 in the sample), `Nanites` (45,087), `Specials` (quicksilver, 240). These are already parsed onto `PlayerState` and shown by `info`, `status`, and the dashboard's player section; this arc does not touch them.

`ShipInventory` and `CurrentShip` at the top level are empty legacy fields (a 0x0 grid), not containers.

Every grid object has `Width`, `Height`, `Class.InventoryClass` (C/B/A/S), `Slots`, `SpecialSlots`, `ValidSlotIndices`, `StackSizeGroup`, and `Name` (empty in the sample). `ValidSlotIndices` lists the unlocked cells and is populated on every grid: the exosuit's 10x12 has 93 unlocked, each storage container's 10x6 has 50, and every `Inventory_Cargo` has **zero**, which is why the cargo grids are empty. `PrimaryShip` is the index of the active ship in `ShipOwnership`.

### Slots

`Slots` lists occupied cells only. An empty cell is simply absent, so free space is the length of `ValidSlotIndices` minus the occupied entries; `Width × Height` is the grid's shape, not its capacity, and using it would report 35 free cargo slots that do not exist. Each slot:

```json
{"Id": "^ASTEROID2", "Amount": 9999, "MaxAmount": 9999, "Index": {"X": 2, "Y": 3},
 "Type": {"InventoryType": "Substance"}, "DamageFactor": 0.0, "FullyInstalled": true, "AddedAutomatically": false}
```

`InventoryType` is one of `Substance`, `Product`, or `Technology`. Technology slots carry `Amount` as charge or condition, not a count, and are excluded from holdings totals.

### Where a container physically is

- **Storage containers** are one inventory per number, reachable from every base that has the matching object placed. `^CONTAINER0` in a planet base and `^FRE_ROOM_STORE0` on the freighter both open `Chest1Inventory`. The sample has `^CONTAINER0` at two planet bases and `^FRE_ROOM_STORE0` through `9` on the freighter. So "where" for storage is a list of access points, derived from base objects.
- **Exocraft** carry the address of the base they are parked at in `Location` (four at the farm base, one at another system, two at zero). Their `Resource.Filename` is empty on every entry, so the exocraft type is not in the record; `VehicleOwnership` is presumably a fixed seven-entry array in kind order (community, unverified). Until that is checked in-game the label is "Exocraft N".
- **Corvette parts** live in their own grid, `CorvetteStorageInventory`, beside the corvette editor's fields. The corvette itself is a `PlayerShipBase` in the base list; which `ShipOwnership` entry it is, if any, is open.
- **Ships** carry `Location` and `Position` too, but every ship in the sample reads zero. The non-primary ships are presumably aboard the freighter. Until the field is understood, ships are labelled "with you" (primary) or "on the freighter".
- **Ship type** comes from `Resource.Filename`: the path segment before the file name is `FIGHTERS`, `DROPSHIPS` (hauler), `SCIENTIFIC` (explorer), `SAILSHIP` (solar), `S-CLASS` (exotic), `BIGGS` (living ship). Ships in the sample are unnamed, so the type is the label.

### Item IDs and names

The save uses internal IDs and carries no display names. The 2026-09-12 sample held 267 distinct IDs across all containers, before the corvette-part IDs were counted. Known correspondences that matter for the first tests:

| ID | Name |
|----|------|
| `^ASTEROID1` | Silver |
| `^ASTEROID2` | Gold |
| `^ASTEROID3` | Platinum |
| `^STELLAR2` | Chromatic Metal |
| `^LAND1` | Ferrite Dust |
| `^CATALYST1` | Sodium |
| `^OXYGEN` | Oxygen |
| `^GAS1`, `^GAS2`, `^GAS3` | Sulphurine, Radon, Nitrogen |

`KnownProducts` (835 entries) and `KnownTech` (137) are ID lists too and could seed a "known recipes" view later, but they are out of scope here.

---

## Architecture

```
save (PlayerStateData) ──► nms-save::model::inventory ──► nms-core::holdings ──► GalaxyModel.holdings
                                                                                        │
                                       nms-query::inventory (have / inventory / items) ◄┘
                                                        │
                                   CLI `nms have` · REPL `have` · MCP `have_item`
```

- **nms-core** gains the domain types and the item name table. No serde of save shapes here.
- **nms-save** gains serde structs for the grids and owners, plus `to_holdings()`.
- **nms-graph** stores `Holdings` on the model so all three interfaces and the cache see one copy.
- **nms-query** gains `inventory.rs` with the three queries and display functions.
- **nms-cache** stores holdings alongside systems and the fleet, and bumps `CACHE_FORMAT_VERSION`.
- **nms-watch** replaces holdings whole in the delta when they changed, as it does the fleet.

### Core types (`nms-core/src/holdings.rs`)

```rust
pub struct ItemId(pub String);            // "^ASTEROID2", stored with the caret

pub enum ItemKind { Substance, Product, Technology }

pub struct ItemStack {
    pub id: ItemId,
    pub kind: ItemKind,
    pub amount: u32,
    pub max: u32,
    pub slot: (u8, u8),
}

pub enum ContainerKind {
    Exosuit, ExosuitCargo, ExosuitTech,
    Freighter, FreighterCargo, FreighterTech,
    Storage(u8),                          // 1..=10
    Ship { index: u8, primary: bool, ship_type: ShipType },
    ShipCargo { index: u8 },
    ShipTech { index: u8 },
    Exocraft { index: u8 },
    MultiTool { index: u8, active: bool },  // Multitools[i].Store; WeaponInventory is the active one's copy and is not read
    Refiner { index: u8 },
    CorvetteParts,                        // CorvetteStorageInventory
    Unidentified(u8),                     // ChestMagicInventory, ChestMagic2Inventory
    Other(String),                        // cooking, bait, grave, ...
}

pub struct Container {
    pub kind: ContainerKind,
    pub class: FrigateGrade,              // the C/B/A/S enum from fleet.rs, renamed to a shared `Grade` if that reads better
    pub width: u8,
    pub height: u8,
    pub unlocked_slots: u16,              // ValidSlotIndices.len(); zero is a real answer (cargo grids)
    pub stacks: Vec<ItemStack>,
}

pub struct Holdings {
    pub containers: Vec<Container>,
    pub ships: Vec<ShipSummary>,          // type, class, general and tech slot counts, class bonuses, location
    pub exocraft: Vec<VehicleSummary>,    // index, parked-at address; type once the order is verified
    pub multitools: Vec<MultiToolSummary>, // grade, slots, damage/mining/scan stats, active marker
}
```

Currencies stay on `PlayerState`; `Holdings` does not repeat them. Technology stacks are kept (they drive `list ships` tech counts and a later "installed tech" view) but every totalling query filters them out. `Holdings` derives `PartialEq` so the watcher can compare old and new by value.

### Item name table (`nms-core/src/items.rs`)

A bundled JSON file, `crates/nms-core/data/items.json`, parsed once into a `OnceLock<HashMap<ItemId, ItemInfo>>`:

```json
{"ASTEROID2": {"name": "Gold", "symbol": "Au", "kind": "Substance"}}
```

Lookup is by ID with the caret stripped. `ItemInfo::display(&ItemId)` returns the name or, when unknown, the raw ID without the caret. Searches accept either a display-name substring (case-insensitive) or an ID substring, so `have gold` and `have asteroid2` both work.

**Sources.** The AssistantNMS API serves the game's own item data by internal ID, free and without a key: `https://api.nmsassistant.com/ItemInfo/GameId/<ID>/en` returns the display name, group, and description for the ID minus its caret (see the ranked sources in [nms-game-notes.md](../../../reference/nms-game-notes.md)). A script under `scripts/` takes the set of IDs to resolve, queries the API for each, and writes `items.json`; a public refiner-recipe gist (about 90 substances with symbols) and the wiki's Item Id List are fallbacks for anything the API does not return, and the symbol column comes from the gist since the API does not carry it. The script, its ID list, and the fallback inputs are committed so the table is reproducible when the game adds items, and every entry records which source named it. Item names are facts about the game and are not a licensing concern.

**Coverage target.** Every ID present in the committed fixture resolves to a name, asserted by a test over `list items`; and every ID present in the author's current save resolves, checked by hand at implementation time with `nms list items` (the corvette-part IDs are the ones the gist and wiki will not have).

### Save structs (`nms-save/src/model.rs`)

```rust
#[serde(rename_all = "PascalCase")]
pub struct Inventory { width, height, class: InventoryClass, #[serde(default)] slots: Vec<InventorySlot>, #[serde(default)] valid_slot_indices: Vec<SlotIndex>, ... }

pub struct InventorySlot { id: String, amount: i64, max_amount: i64, index: SlotIndex, #[serde(rename = "Type")] slot_type: SlotType }

pub struct ShipOwnership { name, resource: ResourceRef, inventory, inventory_cargo, inventory_tech_only, location: PackedGalacticAddress, ... }
pub struct VehicleOwnership { ... same shape ... }
pub struct Multitool { name, store: Inventory, ... }
```

`PlayerStateData` gains the grid fields, `chest1_inventory` .. `chest10_inventory` (a small macro or an array built in `to_holdings()`), `corvette_storage_inventory`, `ship_ownership`, `vehicle_ownership`, `multitools`, `active_multiool_index` (the save's spelling), `primary_ship`, and `refiner_buffer_data`. `WeaponInventory`, `ShipInventory`, and `CurrentShip` are not read. All default when absent so older or partial saves still load. Amounts are `i64` in serde and clamped to `u32` on conversion; the game has written negative units before. Ship records also carry `Inventory.BaseStatValues` (`^SHIP_DAMAGE`, `^SHIP_SHIELD`, `^SHIP_HYPERDRIVE`, `^SHIP_AGILE`) and multi-tools `Store.BaseStatValues` (`^WEAPON_DAMAGE`, `^WEAPON_MINING`, `^WEAPON_SCAN`), which the summaries pick up.

### Queries (`nms-query/src/inventory.rs`)

```rust
pub struct HaveQuery { pub pattern: String, pub kind: Option<ItemKind> }
pub struct HaveResult { pub id: ItemId, pub name: String, pub total: u64, pub locations: Vec<HaveLocation> }
pub struct HaveLocation { pub container: ContainerKind, pub label: String, pub amount: u32, pub max: u32, pub access: Vec<String> }

pub struct InventoryQuery { pub container: Option<ContainerFilter>, pub free_only: bool }
pub struct ListItemsQuery { pub kind: Option<ItemKind>, pub min_amount: u32 }
```

`HaveLocation.access` is where the container can be opened: base names for storage containers, the parked base for exocraft, "with you" or "on the freighter" for ships. It is derived once when the model is built, from base objects, and stored on the container.

A pattern that matches several items (`have gas`) returns one `HaveResult` per item, sorted by total.

### Commands

CLI and REPL share the wording. MCP tools mirror the three queries and return JSON with the same fields.

```
nms have gold                        # total, then per-location rows
nms have gas --type substance        # several matches, one block each
nms inventory                        # every container: occupied/unlocked, class
nms inventory storage 3              # one container's contents as a grid listing
nms inventory --free                 # free slots per container, most free first
nms list items                       # everything, sorted by amount
nms list items --type product --min 100
nms list ships                       # index, type, class, general/tech slots, class bonuses, location, primary marker
nms list exocraft                    # index, parked at (type once the order is verified)
nms list multitools                  # index, grade, slots, damage/mining/scan, active marker
```

Sample `have gold` output, from the sample save:

```
 HOLDINGS: Gold (ASTEROID2)            Total: 11,297
 Location     Amount   Stack   Reachable from
 Storage 1     9,999   9,999   Radioactive Base, Smira Colony, Freighter
 Storage 1       927   9,999   Radioactive Base, Smira Colony, Freighter
 Exosuit         371   9,999   with you
```

Two stacks in the same container stay as two rows: they are two slots, and the stack headroom column is per slot.

### Live updates

The fleet set the pattern: `SaveDelta` gains `holdings: Option<Holdings>`, filled with the new value when it differs from the old one and left `None` otherwise, and `apply_delta` replaces the model's holdings whole. A per-stack diff (amounts that changed, stacks that appeared or vanished) would let the log say "Gold +2,831" and is a follow-up; the replacement alone means every `have` after a save write answers from the new state, and the dashboard's player section can show holdings facts without a special case. The log records nothing for a holdings change in this arc.

### Dashboard

The player section gains one line, `Exosuit 30/93 · Storage 1 full`: the exosuit's occupied of unlocked general slots, then the names of any storage containers with no free slot. Nothing else on the dashboard changes; the `have` and `inventory` views stay prompt commands. The line is built in `view::build` from the model's holdings like the currencies beside it.

### Cache

`CacheData` gains a `holdings` section next to `fleet`, and `CACHE_FORMAT_VERSION` goes up by one; the header check that rebuilds a stale cache already exists.

---

## Files to create or modify

| File | Change |
|------|--------|
| `crates/nms-core/src/holdings.rs` | new: `Holdings`, `Container`, `ItemStack`, `ContainerKind`, `ShipType` |
| `crates/nms-core/src/items.rs` | new: name table loader and display helper |
| `crates/nms-core/data/items.json` | new: generated ID-to-name table |
| `scripts/gen-items.py` | new: merges the public sources into `items.json` |
| `crates/nms-save/src/model.rs` | grids, slots, ship and vehicle ownership, multi-tools, refiner buffers |
| `crates/nms-save/src/convert.rs` | `PlayerStateData::to_holdings()` |
| `crates/nms-core/src/player.rs` | `BaseType::PlayerShipBase`, and the match arms that print it (`list.rs`, `dispatch.rs`, `display.rs`, `convert.rs`) |
| `crates/nms-core/src/delta.rs`; `crates/nms-watch/src/snapshot.rs`, `delta.rs` | `holdings: Option<Holdings>` replacement in the delta |
| `crates/nms-graph/src/model.rs` | `holdings` field; access points derived from base objects; `apply_delta` replaces holdings |
| `crates/nms-cache/src/data.rs`, `serialize.rs` | holdings in `CacheData`; bump `CACHE_FORMAT_VERSION` |
| `crates/nms-query/src/inventory.rs` | new: the three queries |
| `crates/nms-query/src/display.rs` | `format_have`, `format_inventory`, `format_items` |
| `crates/nms-cli/src/have.rs`, `inventory.rs`, `list.rs` | commands |
| `crates/nms-copilot/src/commands.rs`, `dispatch.rs`, `completer.rs` | REPL commands and completion of item names |
| `crates/nms-copilot/src/mcp/tools.rs` | `have_item`, `inventory_summary`, `list_ships` |
| `crates/nms-copilot/src/dashboard/view.rs` | the holdings line in the player section |
| `data/test/multi_system_save.json` | inventories, ownership, and a corvette-parts grid added to the fixture, with a cargo grid that has zero unlocked slots |
| `scripts/gen-items.py`, `README.md` | the generator's usage and the new commands |

---

## Key design decisions

- **Holdings live on the model, not in a side channel.** One copy, one cache, one reload path, same as systems. The REPL's live watcher then needs no special case.
- **Technology is stored but never totalled.** A technology slot's `Amount` is charge, and counting it as inventory would produce nonsense totals.
- **Unknown IDs are shown, not hidden.** The raw ID is the fallback so a missing name is visible and reportable rather than silently dropped.
- **Storage "location" means access points.** The game treats a numbered container as one inventory; reporting each placement as a separate location would double-count.
- **Search matches names and IDs.** Players know "Gold"; tests and power users know `ASTEROID2`.
- **Ship location stays conservative.** Report only what the save proves until the `Location` field is understood.
- **`ValidSlotIndices` is capacity.** A grid's shape is not its size; the cargo grids prove it with 35 cells and no slots.
- **One tool, one container.** `WeaponInventory` is a copy of the active multi-tool's store and is never read, so nothing is listed twice.
- **Holdings replace whole on a save.** Same as the fleet: small enough to clone, and a diff can come later without changing the shape.

---

## Testing strategy

- Serde: a `PlayerStateData` fixture with every grid key present and another with none, asserting defaults.
- Conversion: occupied-only slots produce correct free counts from `ValidSlotIndices`; a grid with none unlocked reports zero free and zero capacity; `WeaponInventory` present in the fixture produces no extra container; the corvette-parts grid becomes `CorvetteParts`.
- Delta: a changed stack amount produces `holdings: Some`, an unchanged save produces `None`, and `apply_delta` replaces the model's holdings.
- Dashboard: the player section line against the fixture with a full storage container.
- Name table: every ID in the sample fixture resolves; a made-up ID falls back to itself.
- Queries: `have` totals across containers, splits stacks, and filters technology; pattern matching by name and by ID; `inventory --free` ordering.
- Display: table contents for each formatter under the plain theme.
- Integration: CLI `nms have gold` against the fixture asserts the total and each location row.
- Cache: a cache with an older format version is rebuilt.

---

## Verification

The live save moves between drafting and implementation, so the fixed checks run against the committed fixture and the live checks are read fresh on the day. The 2026-09-12 figures in this document (Gold 11,297 in three stacks; 8 ships with index 3 primary; Storage 2 at 5 of 50) were already stale by 2026-09-15 (Gold 27,672 in five stacks across the exosuit, Storage 1, and the unidentified container; 10 ships with index 0, an S-class fighter, primary).

Fixture, asserted by tests:

- `nms have gold` reports the fixture's total with one row per stack and the storage rows' access lists naming every base that places the container.
- `nms list items` shows no raw IDs.
- `nms inventory --free` orders containers by free slots from `ValidSlotIndices`, and the cargo grid reports 0 of 0.

Live save, checked by hand at implementation time:

- `nms have gold` matches a hand sum of `nms raw …Slots` across every grid.
- `nms list items` shows no raw IDs, including the corvette parts.
- `nms list ships` shows every real ship once, the primary marked, and slot counts equal to the `ValidSlotIndices` lengths.
- `nms inventory` lists the active multi-tool once.

---

## Open questions

1. What do non-zero ship `Location` values mean, and does zero mean "aboard the freighter"? Needs a save where a ship was left on a planet.
2. What are `ChestMagicInventory` and `ChestMagic2Inventory`? Eleven stacks sit in the first (Ferrite Dust 1,186, Silicate Powder 353, Chromatic Metal 235, Pure Ferrite 203, Carbon 150, `FARMPROD3` 61, Sodium 21, Gold 15, Metal Plating 15, one Power Cell, one Microchip); the second is empty. Ruled out on 2026-09-14: not the Nutrient Processor ingredient store (that is `CookingIngredientsInventory`, confirmed by adding ingredients in-game and re-reading the save) and not settlement storage (the save's settlement table is a cache of other players' settlements and the player has none). Public save-editing guides mention the key without identifying it. Remaining candidates: the overflow store for cargo left in a traded-away ship or vehicle, or the container behind a base part such as the Item Vault. Identify it by moving a distinctive item into a suspected container in-game and diffing the save. Until then the tool should label it "Unidentified container 1" and still include its contents in totals.
3. Answered 2026-09-15: `ValidSlotIndices` is populated on every grid and is the unlocked-slot list (93 on the exosuit, 50 per storage container, 0 on every cargo grid). Still to do: compare one count with the in-game screen so the reference note moves from inferred to verified.
4. `SpecialSlots` marks supercharged slots. Worth surfacing in a later "installed tech" view, not here.
5. The `Inventory_Cargo` grids have zero unlocked slots on the sample, which is why they are empty. Whether they are the high-capacity slots from the inventory rework, and what unlocking one looks like in the save, needs a save where the player has bought one.
6. Where is the corvette in `ShipOwnership`? None of the ten real ships has a corvette folder in `Resource.Filename` and `CorvetteEditAssociatedShipIndex` is −1, yet a `PlayerShipBase` named `Default` exists. Until this is understood `list ships` shows only what `ShipOwnership` holds.
7. Is `VehicleOwnership` a fixed array in exocraft-kind order? `Resource.Filename` is empty on all seven, so the order is the only possible source of the type. Test by matching each entry's `Location` to where that exocraft was last parked.
