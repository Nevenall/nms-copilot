# Scanned counts: fauna, flora, and minerals recorded per planet

Count the species, plants, and minerals the player has scanned on each planet, carry the counts on the planet, and let `find` order planets by them, so "which planets am I closest to finishing for the fauna reward" is one command.

**Status:** implemented 2026-09-19 on `feature/commands` as designed below (cache format 8; the `find` and `export` `--sort` flag, completed at the prompt; the MCP `find_planets` `sort` argument and `scanned` objects). Planned the same day, prompted by the question of the same day: the player wanted fauna-rich planets to finish recording every species for the nanite bonus. The save answers half of it, and the half it answers is the useful half.

**Depends on:** the discovery records the model already parses (see [nms-save-notes.md](../../reference/nms-save-notes.md), section 9, for what `Animal`, `Flora`, and `Mineral` records carry).

---

## What the save gives

One discovery record per species, plant, or mineral scanned, at the planet's address. Counting them per planet is the number recorded there. The planet's total, the "of Y" the game shows on the Discoveries page, is not in the save anywhere, so "how many are left" cannot be said; but a planet with many recorded is one the player has worked on, and those are the ones to finish. The completion bonus scales with the planet's species count, so the richest planets are also the ones that pay.

## Design

- `nms_core::system::Scanned { fauna: u16, flora: u16, minerals: u16 }`, a field `scanned` on `Planet`, zero by default; `Planet::with_scanned` sets it. The struct is `#[non_exhaustive]`, so the new field costs no caller anything.
- The extractor counts `Animal`, `Flora`, and `Mineral` records per planet address in one pass and attaches the counts to the planets it built. A record for a planet with no `Planet` record is dropped, since the atlas has nothing to hang it on.
- The rkyv cache carries the field; `CACHE_FORMAT_VERSION` goes to 8 so an old cache is rebuilt rather than misread.
- `FindQuery` gains `sort: FindSort` (`Distance`, the default, or `Fauna`, `Flora`, `Minerals`); a scanned sort orders by that count descending, then distance, then system and planet as before. With a scanned sort, `nearest` is a plain limit over the sorted list rather than a spatial pre-fetch, so `--sort fauna --nearest 10` is the top ten by fauna.
- `find` and `export` take `--sort distance | fauna | flora | minerals`; the REPL's the same, with completion. The MCP `find_planets` takes `sort`.
- Display: `find` results and the `show system` planet table gain `Fauna`, `Flora`, and `Min.` columns; the export record gains `fauna_scanned`, `flora_scanned`, `minerals_scanned`; the MCP `find_planets` and `show_system` results gain `scanned: {fauna, flora, minerals}`.

## Verification

The extractor test fixture gains creature, plant, and mineral records for one planet and asserts the counts; a find test sorts by fauna and checks the order and the `nearest` limit; the display tests check the columns; the CLI test runs `find --sort fauna` on the fixture and checks the first row.

## Not in this arc

- The planet's total species count and a "complete" flag: not in the save, and not produced by the name generator either.
