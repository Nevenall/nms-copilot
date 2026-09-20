# Project 03 — Save Tooling

> Keep the save safe, keep an eye on it without leaving the game, and fill in what the save leaves out.

Project 02 surfaced the player's holdings, bases, and fleet. Project 03 is the tooling around the save itself: snapshots so a bad decision or a game overwrite can be undone, a dashboard so the alerts project 02 added are seen without alt-tabbing, and the names and properties the game generates from an address but never writes to disk.

Three plans, each its own document:

| Plan | Doc | Question it answers |
|------|-----|---------------------|
| 0047 | [save backups](0047-save-backups.md) | Can I get yesterday's save back? |
| 0048 | [dashboard](0048-dashboard.md) | What needs me right now, on the other monitor? |
| 0049 | [generated galaxy](0049-generated-galaxy.md) | What region am I in, what is this system called, and what does the galaxy map say about it? |

The facts these rest on are in [docs/reference](../../reference/README.md): the save's file pair and modification times for 0047, and the address layout, the galaxy byte, and the coordinate systems for 0049.

## Status

Complete. 0047 is implemented (2026-09-15) as `nms backup`, the REPL `backup` command, the `[backup]` config table, and the slot-wide watcher. 0048 is implemented (2026-09-15) as the dashboard the REPL opens into, the `dash` command, and `--prompt`. 0049 is implemented (2026-09-19) as the `nms-namegen` crate running the community `nms_namegen` tool as a subprocess, with the region and generated names and hover properties shown by `show system`, `list systems`, `find`, `export`, `info`, the dashboard, and the `show_system` MCP tool; the same day's galaxy fix (bits 32-39 of the save address) is recorded in the save notes. Everything is merged into `main` as of 2026-09-19.

Left open on purpose, recorded in 0049: sifting undiscovered addresses for properties (wealthy systems near me) and dissonance, which the generator does not produce.
