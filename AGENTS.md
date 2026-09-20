# AGENTS.md

This file provides guidance to LLM-based coding agents when working with code in this repository.

## Project Overview

NMS Copilot is a real-time galactic copilot for No Man's Sky, built in Rust. It reads NMS save files (raw binary `save.hg` or exported JSON), builds a live in-memory model of discovered systems/planets/bases, and exposes it through three interfaces: a one-shot CLI (`nms`), an interactive REPL (`nms-copilot`), and an MCP server (`nms-mcp`). It is **not** a save editor — it is a queryable atlas.

**Status:** All 7 phases complete. Core types (`nms-core`), binary save parser (`nms-save`), galaxy model with multi-galaxy routing (`nms-graph`), query engine with color themes (`nms-query`), CLI (`nms-cli`) with `info`, `convert`, `find`, `show system`, `base`, `fleet`, `have`, `inventory`, `stats`, `route`, `export`, `import`, `backup`, `raw`, `list` (reference data, atlas contents, holdings, frigates, expeditions, save slots), and `completions` commands, interactive REPL (`nms-copilot`) with the same words plus `set`, `reset`, `map`, and `dashboard`, MCP server (`nms-mcp`) with stdio and HTTP transports and tools named after the command words, live file watching, and rkyv cache are implemented and tested (1065 tests). The grammar the words follow is in `docs/plans/project03-save-tooling/0050-command-words.md`.

## Document Hierarchy

For Rust code quality:

1. `/collaboration-framework` — Session discipline, project management, verification, and closure guidance
2. `/rust-guidelines` — Advanced Rust programming guidance and anti-patterns
3. This file — Project-specific conventions

Use the referenced guidance by name; do not assume local copies exist in this
repository.

## Design Documents

Plans live under `docs/plans/`, one numbered document per feature, each opening with a **Status** line that says when and on which branch it was implemented and how the result departs from the design. Each project directory has a `README.md` index with the status of its arcs:

- `docs/plans/project02-inventory-and-bases/` — inventory (0044), crops and supply depots (0045), fleet expeditions (0046); all implemented.
- `docs/plans/project03-save-tooling/` — save backups (0047), dashboard (0048), generated galaxy names and properties (0049), command words (0050), scanned counts per planet (0051); all implemented.

A new feature gets the next number and a plan before the code; when the code lands, the plan's Status line is updated rather than the plan rewritten. Facts the plans rest on live in `docs/reference/` (see below); `docs/mcp-http.md` documents the MCP HTTP transport. The upstream project's ODM design documents (`crates/design/`) were never part of this fork's tree.

## Crate Architecture

```
nms/                    Workspace root
├─ nms-core             Types, enums, address math, glyph emoji (zero heavy deps)
├─ nms-save             Raw binary save parser (LZ4 + XXTEA + key mapping)
├─ nms-compat           Format adapters (goatfungus JSON fixer)
├─ nms-graph            petgraph spatial model, R-tree index, routing (the brain)
├─ nms-query            Shared query engine (find, route, show, stats)
├─ nms-watch            notify file watcher, delta computation, event stream
├─ nms-cache            rkyv zero-copy serialization for fast startup
├─ nms-namegen          Region and system names and hover properties from the address, by running the nms_namegen Python tool
├─ nms-cli              clap one-shot CLI (`nms` binary)
└─ nms-copilot          reedline interactive REPL + MCP server (`nms-copilot` binary)
```

Data flow: `save file → parser → galaxy model → query engine → CLI / REPL / MCP`

**nms-graph is the core.** Everything upstream feeds into it; everything downstream queries from it. All three interfaces share `nms-query` — no duplicated logic.

## Build & Test Commands

```bash
make build          # Build all crates
make test           # Run all tests
make lint           # Clippy linting
make lint-docs      # Evidence tags and links in docs/reference
make format         # rustfmt formatting
make coverage       # Code coverage (target: 95%+)
cargo test -p nms-core          # Test a single crate
cargo test -p nms-save -- test_name  # Run a single test
```

Always run `make format` after changes, then `make lint` before testing.

### Dependency Updates

Run `cargo update` for in-range bumps and check `cargo info <crate>` for newer majors; the resolver honours `rust-version` in the workspace `Cargo.toml`. Known constraints as of 2026-09-15:

- **Re-pin rmcp after every `cargo update`:** `cargo update rmcp --precise 1.2.0`. `fabryk-mcp-core 0.4.2` declares `rmcp = "1"` but fails to compile against rmcp 1.3 and later (`StreamableHttpService` gained a second type parameter). Only the lockfile holds the pin; drop it once fabryk-mcp releases a fix.
- **tabled must match oxur-cli's version** (0.17 at oxur-cli 0.2.1): `nms-query`'s table `Builder` is passed into oxur-cli's `TableStyleConfig`. Only `nms-query` depends on tabled. The `proc-macro-error2` future-incompat warning comes from that tabled version and goes away when oxur-cli moves.
- **reedline 0.50 and later require Rust 1.95**, above the workspace `rust-version`; raise the MSRV before bumping past 0.49. 0.49 made `Signal` non-exhaustive, which `main.rs` handles with a wildcard arm.
- **rstar 0.13** takes the query point by value in `nearest_neighbor` and `nearest_neighbor_iter`.
- **notify 9** was still a release candidate, and `notify-debouncer-mini 0.7` needs notify 8.

After updating: `make format`, `make lint`, `make test`, then run `nms fleet` and `nms backup list` against a real save.

## Key Technical Details

### Save File Parsing Pipeline

1. Read `save.hg` — detect format (plaintext JSON vs LZ4 compressed)
2. Parse sequential LZ4 blocks (magic `0xFEEDA1E5`), decompress, concatenate
3. Deobfuscate JSON keys using MBINCompiler's `mapping.json`
4. Deserialize into typed Rust structs via serde

No encryption on modern saves (format 2002+, post-Frontiers). XXTEA only on metadata file `mf_save.hg`.

### Portal Glyph System

16 glyphs (0-F) rendered as emoji throughout all interfaces. The converter is fully multidirectional: index, name, hex, emoji, coordinates, signal booster format — all interconvertible. See README.md for the full glyph table.

### Galactic Address

`GalacticAddress` — 48-bit packed coordinate: VoxelX/Y/Z (signed), SolarSystemIndex, PlanetIndex, RealityIndex (galaxy 0-255). Distance = Euclidean voxel distance × 400 ly. The save file stores addresses in a different bit layout; both are in `docs/reference/nms-save-notes.md`.

## Game and Save Facts

Facts about the save file's encodings and the game's mechanics, IDs, and constants live in `docs/reference/` (`nms-save-notes.md`, `nms-game-notes.md`), each line tagged with how it is known: `verified`, `game-data`, `community`, `inferred`, or `open`. `docs/reference/README.md` defines the tags and the method that earns each one. A save reading plus an independent observation outranks any document, including those.

- Before writing code that depends on a decode rule, an object or item ID, or a constant, read the relevant line and check its tag. Do not build on an `inferred` line as if it were `verified`; either verify it first or make the code's caveat match the tag.
- Any session that establishes a new fact, changes one, or moves one up the ladder records it in `docs/reference` before the work is called done. Plan documents under `docs/plans` and code comments cite the reference notes rather than restating facts.
- `nms raw <path>` dumps any part of the decoded save for this work; `make lint-docs` checks that every fact line in the notes carries a tag and every link resolves.
- The `nms-expert` agent in `.claude/agents/` carries this method; use it for save-decoding and game-mechanics questions.

## Conventions

- Test naming: `test_<fn>_<scenario>_<expectation>`
- Load `/rust-guidelines` before writing Rust code, including its anti-patterns guidance
- Reference data ships in `data/` directory: `biomes.toml`, `glyphs.toml`, `galaxies.toml`, `mapping.json`
- Config location: `~/.nms-copilot/config.toml`
- Table output uses `oxur-table` crate (see `oxur-odm` for usage examples)
- Logging via `twyg`, config via `confyg`

## Commit Message Guidance

Every assistant-authored commit message must include these trailers:

```text
Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>
```

## Workbench

`workbench/` is gitignored and contains local tools and reference implementations (e.g., NMSSaveEditor). Not part of the project source.
