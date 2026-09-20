# Command words: one grammar across the CLI, the REPL, and the MCP tools

Make the command vocabulary one grammar again, after project 02 added noun-first commands beside the atlas's verb-first ones, and give the REPL the CLI commands it was missing.

**Status:** implemented 2026-09-19 on `feature/commands` as written below, with two additions found on the way: `export` and `raw` moved into `nms-query` (`export::render_export`, `raw::format_raw`) so both binaries share them, and `saves::format_save_slots` joined them for `list saves`; and `--format` completes `csv` and `json` at the prompt. Planned the same day from a review of every command word. The words were fine one feature at a time; read together they had three homes for "base", a noun-noun `fleet frigates` beside `list ships`, a bare `saves` beside `list galaxies`, three overview words (`info`, `stats`, `status`), two ways to ask for an item by name, one abbreviation (`dash`), and an MCP tool list that mixed `verb_noun`, `noun_noun`, and phrases.

**Depends on:** nothing new. Every change is a rename, an alias, a moved function, or a `list` target over query code that exists.

---

## The grammar

- **Verbs own the atlas:** `find` (planets), `show system`, `list <plural>`, `route`, `convert`, `export`, `import`, `raw`, `backup`.
- **A domain noun is a command whose bare form is the overview and whose argument is one thing in full:** `base` / `base Farm`, `fleet` / `fleet 2`, `inventory` / `inventory "storage 3"`. There is no second door to the same thing.
- **Collections are `list <plural>`**, whatever they are: reference data, atlas contents, holdings, the fleet, save slots.
- **`have <item>` is the one by-name item question.** `list items` is the totals table with its `--type` and `--min` filters and no name argument.
- **Overviews:** `info` is the save and the player (and, at the REPL, the alerts); `stats` is the aggregates; the session's own settings print from bare `set`. `status` goes.
- **Whole words**, with `dash` kept as an alias of `dashboard`.
- **MCP tool names mirror the command words** with a `verb_noun` or `noun_noun` shape: `find_planets`, `plan_route`, `player_position`, `nearby_planets`, `show_system`, `base_status`, `fleet_status`, `have_item`, `inventory`, `list_ships`, `convert_coordinates`, `galaxy_stats`.

## Changes

| Was | Is | Where |
|-----|----|-------|
| `show base X` and `base X` | `base X` alone; the detail table gains the `Planets` row `show base` had, and `show` has only `system` | `nms_query::show` loses its base half; CLI `show`, REPL `show`, completer, MCP |
| `fleet frigates` | `list frigates` | `FleetTarget` loses `Frigates`; `list` gains the target in both binaries |
| — | `list expeditions` | the `fleet` overview's table without the Navigator line |
| `saves` | `list saves` | the slot table moves into `nms-query` so both binaries print it |
| `list items <pattern>` | `list items` with `--type` and `--min` only | `ListItemsQuery` loses `pattern` |
| `status` | bare `set` prints position, biome, warp range, backups; `info` prints the alerts | `SessionState::format_status` splits |
| `dash` | `dashboard`, alias `dash` | `Action::Dashboard` |
| REPL without `export`, `raw`, `list saves`, `backup prune` | all four at the prompt | `export`'s records and writers and `raw`'s walker move into `nms-query`; the REPL's `raw` re-reads the followed save file; `export --to FILE` writes a file from either binary and prints without it |
| MCP `search_planets`, `where_am_i`, `whats_nearby`, `inventory_summary` | `find_planets`, `player_position`, `nearby_planets`, `inventory` | tool names, the guidance block, tests |
| MCP `show_base` | gone; `base_status` with a name carries the glyphs, the system, and its planet count | `build_base_status_json` |

REPL `info` keeps its model summary and position and adds the galaxy and the alert list that `status` printed. Bare `set` is the settings view; `set <thing> <value>` still sets.

## Verification

The existing parse, dispatch, completer, CLI, and MCP tests are updated to the new words; new ones cover `list frigates`, `list expeditions`, `list saves` (formatting only, over synthetic slots), bare `set`, `dashboard` and its alias, the REPL `export`, `raw`, and `backup prune` parses, and that `show base`, `fleet frigates`, `saves`, and `status` are refused.

## Not in this arc

- `show region` and `find` over undiscovered addresses, which the same grammar has room for.
- Making the REPL `info` print the CLI's full save summary; the REPL holds the model, not the save, and that is a model change rather than a naming one.
