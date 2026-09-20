# Project 02 — Inventory and Bases

> What do I own, where is it, and what is happening at my bases?

Project 01 delivered the galactic atlas: systems, planets, routes, and the three interfaces that share one live model. Project 02 turns the same pipeline toward the player's holdings. The save file carries every inventory grid, every owned ship and vehicle, and every object placed at every base, and almost none of it is surfaced today.

Three arcs, each with its own plan document:

| Arc | Doc | Question it answers |
|-----|-----|---------------------|
| arc01-inventory | [0044](arc01-inventory/0044-inventory-model-and-queries.md) | Do I have this item, how much, and where is it? |
| arc02-farm-and-industry | [0045](arc02-farm-and-industry/0045-crops-and-supply-depots.md) | Are my crops ready, how full are my supply depots, and what powers the base? |
| arc03-fleet | [0046](arc03-fleet/0046-fleet-expeditions.md) | What are my frigates doing, is one waiting for me, and when can I send more? |

The arcs were scoped against a real save on 2026-09-12 to 2026-09-14. Every field and constant quoted in the plans was read from that save unless marked as inferred or unverified; the maintained record of those facts, with the evidence behind each, is [docs/reference](../../reference/README.md).

## Shared groundwork

The arcs share pieces that were built once; all four are done:

1. **Item name table.** Inventory slots, refiner buffers, and crop yields are all keyed by the game's internal item IDs (`^ASTEROID2` is Gold). The save contains no display names. Done in arc 01: `crates/nms-core/data/items.json`, generated from the AssistantNMS API by `scripts/gen-items.py` (3,323 names), with the raw ID as fallback. Sources are discussed in 0044 and ranked in [docs/reference/nms-game-notes.md](../../reference/nms-game-notes.md).
2. **Typed base objects.** Done in arc 02: `PersistentPlayerBases[].Objects[]` is parsed into `nms_core::BaseObjects` and stored on each `PlayerBase`; arc 01 uses it to say which base a storage container is reachable from.
3. **Model and cache growth.** Done in arcs 02 and 03: the rkyv cache carries base objects and the fleet and starts with a magic header and `CACHE_FORMAT_VERSION`, so a cache written by an older binary is rebuilt rather than misread; the watcher's delta replaces the fleet whole when it changed. Arc 01 added holdings to the model, the cache, and the delta the same way (format 6; project 03 later took it to 7).
4. **Ship bases.** The save's `PersistentBaseTypes` includes `PlayerShipBase` (the corvette). Done in arc 01: `nms_core::BaseType` has the variant, so a container on the corvette is labelled as such.

## Status

Complete. Arc 02 is implemented (2026-09-14) as the `base` command, REPL alerts, and the `base_status` MCP tool. Arc 03 is implemented (2026-09-15) as the `fleet` command, REPL alerts, and the `fleet_status` MCP tool; both plans record the verified decode rules. Arc 01 is implemented (2026-09-15) as the `have` and `inventory` commands, the `list items | ships | exocraft | multitools` targets, the `have_item`, `inventory_summary`, and `list_ships` MCP tools, and the dashboard's inventory line; the item name table is generated from the AssistantNMS API. Everything is merged into `main` as of 2026-09-19. Work that followed lives in [project 03](../project03-save-tooling/README.md).
