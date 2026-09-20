# Generated galaxy: names and attributes from the address

Give every system the tool knows its region name, its generated name, and the properties the game rolls from the address (star colour, economy, wealth, conflict, race, uncharted), by running the community `nms_namegen` generator as an external tool and caching what it says.

**Status:** implemented 2026-09-19 on `feature/namegen` as the `nms-namegen` crate, `nms_core::generated`, `GalaxyModel::enrich`, and the display changes below; the departures from the design are that the dashboard puts the region on the system line rather than its own row, to keep the panel's two columns even, and that `attributes` is a fourth batch kind, run for the rendered planet and moon counts. Planned the same day, prompted by the region question of the same day: the save carries the voxel of every address but never the region's name, and the same is true of the system name for any system the player has not renamed and whose station they have not visited, and of every property the galaxy map shows on hover. All of those are generated from the address and the galaxy by the game, and [nms_namegen](https://github.com/hadsh/nms_namegen) reproduces the generator (see [nms-game-notes.md](../../reference/nms-game-notes.md), Coordinates, for what was verified against the sample save).

**Depends on:** the galaxy carried in `SystemId` and read from bits 32-39 of the save address (done, e1fd0f4), Python 3.13 or later with numpy on the machine, and a checkout of `nms_namegen` at a pinned commit.

---

## Why external, not a port

The generator depends on exact float32 and float64 rounding at comparison edges, which is why it needs numpy; Rust would reproduce that natively, but a port would then have to be re-validated against the community corpora after every game update by us, where the upstream author already does that work. The name generator has not changed since Origins (2020); the attribute logic is the part that moves with game updates, and that is exactly the part best pulled rather than maintained. Two properties make the subprocess cheap: batch mode does about 10,000 addresses a second over stdin and stdout, and the output is a pure function of address and galaxy, so it can be cached forever and each address costs one run of the tool in the tool's lifetime.

The seam is a trait, so a port could replace the subprocess later and be checked against the same fixtures.

## Design

### The tool

`~/.nms-copilot/nms_namegen/namegen.py`, a git checkout pinned at `52ad48a` (2026-08-27, the commit validated against the sample save on 2026-09-19). The path can be overridden by `[namegen] path` in `config.toml` or `NMS_NAMEGEN`; the interpreter by `[namegen] python` (default `python`, then `python3`). Absent tool, missing Python, or a failed run degrade to no generated data: every column and line that would show it stays empty, nothing errors, and one debug log line says why.

The four batch kinds used are `region`, `system`, `system-attributes`, and `attributes`, one process per kind, each address written to stdin as `PSSSYYZZZXXX galaxy` with planet 0, one JSON object read back per line carrying the address and galaxy it answers for. A line with `error` is skipped. Stdin is fed from a thread and stderr drained on another, so a long list cannot fill a pipe and stall the tool.

### The data

`nms_core::generated`:

- `Generated { region: String, name: String, attributes: Option<SystemAttributes> }`.
- `SystemAttributes { star: StarColour, economy: Economy, wealth: Tier, conflict: Tier, race: Race, uncharted: bool, abandoned: bool, pirate: bool, planets: u8, moons: u8, gas_giant: bool }` with the enums mapped from the library's category numbers: star 0 yellow, 1 green, 2 blue, 3 red, 4 purple; economy 1 trading, 2 advanced materials, 3 scientific, 4 mining, 5 manufacturing, 6 technology, 7 power generation; tiers 1 low, 2 medium, 3 high; race 1 Gek, 2 Korvax, 3 Vy'keen, 0 none. Anything outside those ranges makes the whole attributes record `None` rather than a wrong label.
- Display names follow the game: wealth low, medium, high; conflict low, medium, high; the economy words as the galaxy map prints them.

`nms_namegen` crate: `NameGen::discover(config) -> Option<NameGen>`, `NameGen::generate(&[GalacticAddress]) -> HashMap<GalacticAddress, Generated>` with the disk cache at `~/.nms-copilot/namegen-cache.json` keyed by `galaxy:PSSSYYZZZXXX`, loaded on construction and written after each run that added entries. The trait `AddressGenerator` has the one method `generate`; `NameGen` is its subprocess implementation and `TableGenerator` the in-memory one for tests.

`GalaxyModel` gains `generated: HashMap<SystemId, Generated>` and `enrich(&dyn AddressGenerator)`, which asks for every system it holds plus the player's current system, keeps the answers, and enters each generated name in the name index when no save name has it, so `show system <generated name>` works. The REPL and the MCP delta loops enrich again after every applied delta, which is a cache read unless a new system arrived. The rkyv cache does not carry them: the JSON cache makes enrichment after a cache load a file read, and keeping one cache of generated data rather than one per save keeps it simple.

### Where it shows

- `nms show system`: a `Region` row, and when the system has no name a `Name` row reading the generated name marked `(generated)`; then `Star`, `Economy`, `Wealth`, `Conflict`, `Race` rows, `Uncharted` when true.
- `nms list systems`: `Region` and `Galaxy` columns; unnamed systems print the generated name in the name column marked with a trailing `*`, with a footnote.
- `nms info`: `Region` and `System` rows; the dashboard's player section: the region on the system line.
- The REPL `show` and `list` share the query code; the MCP `show_system` result gains the same fields as optional keys.
- `nms find` and `export`: the generated system name where the discovery has none, and a `region` field in the export.

`stats`'s "Named Systems" keeps counting player and station names only, so the figure keeps its meaning.

### Verification

Fixture: a `Generator` that returns fixed records for the test save's addresses, so display tests need no Python. Integration: an `#[ignore]` test that runs the real tool on the sample addresses and checks `Lauderen`, `Piponera Anomaly`, and the Hilbert names, which is the check already done by hand on 2026-09-19.

## Not in this arc

- Sifting undiscovered addresses for properties (wealthy systems near me): the same client makes it possible, since batch mode is fast enough to try every system index in a voxel, but the query and its display are their own piece of work.
- Dissonance: the library does not derive it and the community corpus barely labels it, so the galaxy map hover stays the only source.
- Planet names: available from the tool, but the discovery records already carry the planets the player has seen, which are the ones the tool lists.
