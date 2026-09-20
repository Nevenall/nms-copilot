# Project 03 — Save Tooling

> Keep the save safe, keep an eye on it without leaving the game, and fill in what the save leaves out.

Project 02 surfaced the player's holdings, bases, and fleet. Project 03 is the tooling around the save itself: snapshots so a bad decision or a game overwrite can be undone, a dashboard so the alerts project 02 added are seen without alt-tabbing, and the names and properties the game generates from an address but never writes to disk.

Five plans, each its own document:

| Plan | Doc | Question it answers |
|------|-----|---------------------|
| 0047 | [save backups](0047-save-backups.md) | Can I get yesterday's save back? |
| 0048 | [dashboard](0048-dashboard.md) | What needs me right now, on the other monitor? |
| 0049 | [generated galaxy](0049-generated-galaxy.md) | What region am I in, what is this system called, and what does the galaxy map say about it? |
| 0050 | [command words](0050-command-words.md) | Which word do I type, and is it the same one everywhere? |
| 0051 | [scanned counts](0051-scanned-counts.md) | Which planets have I nearly finished cataloguing? |

The facts these rest on are in [docs/reference](../../reference/README.md): the save's file pair and modification times for 0047, and the address layout, the galaxy byte, and the coordinate systems for 0049.

## Status

Complete. 0047 is implemented (2026-09-15) as `nms backup`, the REPL `backup` command, the `[backup]` config table, and the slot-wide watcher. 0048 is implemented (2026-09-15) as the dashboard the REPL opens into, the `dash` command, and `--prompt`. 0049 is implemented (2026-09-19) as the `nms-namegen` crate running the community `nms_namegen` tool as a subprocess, with the region and generated names and hover properties shown by `show system`, `list systems`, `find`, `export`, `info`, the dashboard, and the `show_system` MCP tool; the same day's galaxy fix (bits 32-39 of the save address) is recorded in the save notes. 0050 is implemented (2026-09-19) as one grammar across the CLI, the REPL, and the MCP tools: `base <name>` alone for a base in full, `list frigates | expeditions | saves`, `have` as the one by-name item question, bare `set` for the session settings and the alerts in `info`, `dashboard` with `dash` as an alias, `export`, `raw`, and `backup prune` at the prompt, and MCP tools named after the command words. 0051 is implemented (2026-09-19) as `Scanned` counts on every planet, counted from the `Animal`, `Flora`, and `Mineral` discovery records, shown by `find` and `show system` and carried by `export` and the MCP tools, with `find --sort fauna | flora | minerals`. Everything is merged into `main` as of 2026-09-19.

Left open on purpose, recorded in 0049: sifting undiscovered addresses for properties (wealthy systems near me) and dissonance, which the generator does not produce.
