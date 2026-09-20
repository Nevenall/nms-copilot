# 🚀 NMS Copilot

[![][build-badge]][build]
[![][crate-badge]][crate]
[![][tag-badge]][tag]
[![][docs-badge]][docs]
[![License](https://img.shields.io/crates/l/treadle.svg)](LICENSE-MIT)

**A real-time galactic copilot for [No Man's Sky](https://www.nomanssky.com/), built in Rust.**

[![][logo]][logo-large]

Search planets by biome. Plan warp routes through the stars. Convert portal glyphs with emoji. Watch your save file live as you play — and let an AI explore the galaxy *with* you.

```
[Euclid │ 644 planets │ 293 systems] 🚀 find --biome Lush --nearest 5

  #  Planet            Biome   System             Distance   Portal Glyphs
  1  Metok-Kalpa       Lush    Gugestor Colony       0 ly    🌅🕊️🐜🕊️🐜🌳🦋🕋🌜🔺🕋😑
  2  Sushimi           Lush    Esurad               18K ly   🌅🕊️🐜🦕🌜🎈⛵🐜🦋🌀🕋🐋
  3  (unnamed)         Lush    Ogsjov XV            42K ly   🌅😑🐜🕊️🐜🌳🌜🕋🌅🔺🕋🦕
  4  (unnamed)         Lush    Rastarc-Zukk         67K ly   🌅🦕🐜🕊️🐜🌅🦋🕋🌜🔺🕋🐜
  5  Dipadri Grosso    Lush    Ipswic               91K ly   🌅🌜🐜🕊️🐜🌳🌜🕋🌅🌀🕋😑
```

---

## What is this?

NMS Copilot reads your No Man's Sky save files — either the raw binary format (`save.hg`) directly or exported JSON — and builds a live, in-memory model of every system, planet, and base you've discovered. It's not a save editor. It's a **queryable atlas** of your personal galaxy.

Two ways to use it:

| Interface | What it does |
|-----------|-------------|
| **CLI** (`nms`) | One-shot commands for quick lookups and scripted pipelines |
| **REPL** (`nms-copilot`) | Interactive session with persistent state, built-in MCP server for AI co-exploration |

The copilot watches your save directory for changes. When you warp to a new system, scan a planet, or build a base, it detects the auto-save and updates the model automatically. The built-in MCP server shares the same live model, so your AI copilot knows where you are *right now*.

Use `--headless` to run just the MCP server without the REPL (e.g., for Claude Desktop integration).

---

## Features

- **Native save file parsing** -- reads `save.hg` directly (LZ4 block decompression + JSON key deobfuscation), no export step needed
- **Planet search** -- find planets by biome, distance, discoverer, infested status, or any combination
- **Route planning** -- nearest-neighbor and 2-opt TSP solvers with warp-range hop constraints
- **Portal glyph converter** -- fully multidirectional: hex, emoji, coordinates, signal booster, galactic address
- **Interactive galaxy map** -- full-screen TUI with galaxy, region, and local zoom levels
- **Live file watching** -- detects auto-saves while you play and updates the model in real time
- **rkyv cache** -- zero-copy serialization for near-instant startup after the first load
- **Inventory** -- do I have gold, how much, and where: every container, ship, exocraft, and multi-tool, with item names from the game's data
- **Base status** -- crops ready to harvest, supply depots and their pipe networks, power at every base, with alerts when something is ready or full
- **Fleet** -- frigate expeditions under way, returned, or waiting for your decision; every frigate's class, grade, and modules
- **Region names and system properties** -- region, generated system name, star colour, economy, wealth, conflict, and lifeform for any address, via the community `nms_namegen` tool
- **Dashboard** -- a live terminal view of you, your bases, and your fleet for a second monitor, redrawn as the game saves
- **Save backups** -- dated snapshots of a slot's two files, on demand or on every save the game writes; restore is a two-file copy
- **Raw save inspection** -- print any part of the decoded save as JSON
- **Multi-save support** -- switch between save slots (up to 15)
- **Export & import** -- JSON/CSV export of filtered data; CSV import of community coordinates
- **MCP server** -- stdio and HTTP transports for AI copilot integration (Claude Desktop, etc.)
- **Configurable color themes** -- ANSI terminal themes via `~/.nms-copilot/config.toml`
- **Shell completions** -- bash, zsh, fish, powershell, elvish
- **Multi-galaxy routing** -- per-galaxy spatial indexes across all 256 NMS galaxies

---

## Portal Glyphs

NMS Copilot renders portal addresses as emoji throughout all interfaces:

Portal Glyphs

| Index | Name  |      Hex | Emoji | Unicode|
|-------|-------|----------|-------|--------|
|  0 |   Sunset  |     0 |  🌅  |   U+1F305
|  1 |   Bird    |     1 |  🕊️  |   U+1F54A U+FE0F
|  2 |   Face    |     2 |  😑  |   U+1F611
|  3 |   Diplo   |     3 |  🦕  |   U+1F995
|  4 |   Eclipse |     4 |  🌜  |   U+1F31C
|  5 |   Balloon |     5 |  🎈  |   U+1F388
|  6 |   Boat    |     6 |  ⛵  |   U+26F5
|  7 |   Bug     |     7 |  🐜  |   U+1F41C
|  8 |   Dragonfly|    8 |  🦋  |   U+1F98B
|  9 |   Galaxy   |    9 |  🌀  |   U+1F300
| 10 |   Voxel    |    A |  🕋  |   U+1F54B
| 11 |   Whale    |    B |  🐋  |   U+1F40B
| 12 |   Tent     |    C |  ⛺  |   U+26FA
| 13 |   Rocket   |    D |  🚀  |   U+1F680
| 14 |   Tree     |    E |  🌳  |   U+1F333
| 15 |   Atlas    |    F |  🔺  |   U+1F53A

Convert freely between formats:

```bash
# Emoji → coordinates
nms convert --glyphs "🌅🕊️🐜🕊️🐜🌳🦋🕋🌜🔺🕋😑"

# Hex glyphs → coordinates
nms convert --glyphs 01717D8A4EA2

# Signal booster → emoji glyphs
nms convert --coords 0EA2:007D:08A4:0171

# Galactic address → everything
nms convert --ga 0x40050003AB8C07
```

---

## Commands

All commands below work with both the CLI (`nms`) and the REPL (`nms-copilot`), unless noted otherwise. The CLI accepts `--save` and `--slot` flags; the REPL uses its pre-loaded model.

### Search

Find planets matching any combination of criteria, sorted by distance:

```bash
nms find --biome Lush                          # all lush planets
nms find --biome Scorched --infested           # infested scorched only
nms find --biome Barren --within 100000        # within 100K ly
nms find --biome Lava --nearest 5              # 5 closest lava planets
nms find --biome Swamp --from "Sealab 2038"   # distance from a named base
nms find --named --discoverer oubiwann         # your named discoveries
```

### Route Planning

Plan optimal routes through the galaxy with warp range constraints:

```bash
nms route --biome Scorched                       # visit all scorched, nearest-neighbor
nms route --biome Scorched --within 500000       # only within radius
nms route --biome Lush,Swamp --warp-range 2500   # S-class hyperdrive hops
nms route --biome Frozen --algo 2opt             # improved TSP
nms route --target "Base A" --target "Base B"    # explicit waypoints
nms route --round-trip                           # return to start
```

### Info & Details

```bash
nms info                              # save overview, player location, discovery counts
nms show system 369                   # system details + all planets
nms show base "Acadia National Park"  # base details with portal glyphs
nms base                              # every base: crops ready, extraction fill, power
nms base "Farm"                       # one base: crops by type, extraction by pipe network, power
nms base "Farm" --width 160           # lay the sections out side by side at this width (default: terminal width)
nms fleet                             # frigate expeditions: waiting for you, returned, or under way; the Navigator's offers left
nms fleet 1                           # one expedition: its frigates and the event log
nms fleet frigates                    # every frigate: class, grade, stats, modules, and whether it is out
nms have gold                         # do I have it, how much, and in which container, reachable from where
nms have gas --type substance         # several matches, one block each; --type substance | product | technology
nms inventory                         # every container: class, used of unlocked slots, free, reachable from
nms inventory --free                  # most free slots first
nms inventory storage 3               # one container's contents; a word like "ship" lists every matching grid
nms stats --biomes                    # biome distribution table
nms stats --discoveries               # discovery counts by type
nms saves                             # list all save slots
```

`have` matches item names and the game's internal IDs (`have asteroid2` finds Gold), totals across every grid, and lists each stack with the bases a storage container can be opened from. Names come from a bundled table generated from the [AssistantNMS API](https://api.nmsassistant.com) by `scripts/gen-items.py`; an ID the table does not know is shown as itself. Free space is the unlocked slots minus the occupied ones, so a cargo grid nothing has been bought for reports nothing rather than an empty 7×5. Technology slots hold charge, not a count, and are never totalled.

`base "Name"` lists a base's supply networks as A, B, C. Nothing in the save says which depots and extractors share a network, so the networks are traced through the pipes: a supply pipe records its own run, pipes that meet end to end are one network, a pipe joins the machine it reaches, and depots built touching each other join without a pipe. See [docs/reference/nms-save-notes.md](docs/reference/nms-save-notes.md#wires-pipes-and-cables-what-is-connected-to-what).

### Raw Save Inspection

`nms raw` prints any part of the decoded save as JSON, for finding out what the file holds before the model does. Output is pruned by depth and array length so it stays readable; pruned parts show as `{… N keys}`, `[… N items]`, or `… N more`.

```bash
nms raw --keys                                                   # what is at the root
nms raw BaseContext.PlayerStateData --keys                       # every field of the player state, with types and sizes
nms raw BaseContext.PlayerStateData.FleetExpeditions[0] --depth 1
nms raw BaseContext.PlayerStateData.FleetFrigates --limit 0      # every element, not just the first 10
nms raw --find UserData                                          # every key containing "UserData", with its path
```

### Coordinate Conversion

```bash
nms convert --glyphs 01717D8A4EA2                  # hex glyphs to all formats
nms convert --glyphs "🌅🕊️🐜🕊️🐜🌳🦋🕋🌜🔺🕋😑"  # emoji glyphs to all formats
nms convert --coords 0EA2:007D:08A4:0171           # signal booster format
nms convert --ga 0x40050003AB8C07                   # galactic address (save-file format)
nms convert --voxel 100,50,-200 --ssi 42           # voxel coordinates
```

### Export & Import

```bash
nms export --format json                          # export all planets as JSON
nms export --biome Lush --format csv              # export filtered planets as CSV
nms import community_data.csv --source "NMSCE"    # import community coordinates
```

### Listing Data

Browse reference data and model collections:

```bash
nms list galaxies                      # all 256 galaxies
nms list galaxies --type Lush          # filter by galaxy type
nms list biomes                        # biome types and variants
nms list glyphs                        # portal glyph table
nms list terrain-types                 # terrain generation types
nms list bases                         # all player bases
nms list systems --limit 10            # first 10 discovered systems
nms list systems --all                 # all discovered systems
nms list items                         # everything you hold, largest total first
nms list items --min 1000 --type substance
nms list ships                         # type, class, slots, installed tech, class bonuses; the primary marked
nms list exocraft                      # each exocraft and the base it is parked at
nms list multitools                    # class, slots, installed tech, bonuses; the equipped one marked
```

### Shell Completions

```bash
nms completions bash > ~/.bash_completion.d/nms   # bash completions
nms completions zsh > ~/.zfunc/_nms               # zsh completions
nms completions fish > ~/.config/fish/completions/nms.fish  # fish completions
```

### Multi-Save Support

```bash
nms info --slot 3                   # use save slot 3 instead of most recent
nms find --slot 5 --biome Lush      # search slot 5's discoveries
```

### Save Backups

`nms backup` copies a save file and its `mf_` metadata sibling into a dated folder under `~/.nms-copilot/backups/<account>/`, keeping the original file names so a restore is a two-file copy. The folder name carries the save's own modification time, the slot, whether it was a manual or auto save, and any label. An unlabelled snapshot whose content matches the newest one kept is skipped.

```bash
nms backup                            # snapshot the most recent save of the most recent slot
nms backup --slot 3                   # a specific slot, its most recent file
nms backup --all                      # every file of every slot
nms backup --label before-call        # label the folder; labelled snapshots are never pruned
nms backup list                       # what is kept, newest first, with sizes and labels
nms backup prune --keep 10            # drop all but the newest 10 unlabelled snapshots per slot
nms backup --to D:/nms-backups        # any of the above against another folder
```

The REPL and the headless MCP server can snapshot automatically on every save the game writes, once turned on in the config or with `backup on` in the REPL. The watcher follows both files of a slot, so manual and auto saves are both caught.

```toml
[backup]
enabled = false                       # automatic snapshots while the watcher runs
dir = "~/.nms-copilot/backups"        # where snapshots go
keep = 20                             # unlabelled snapshots kept per slot; 0 keeps everything
```

Restore is manual, and the tool never writes into the game's folder. With the game closed, copy the two files from a snapshot folder back over the same names in the account folder (for example `%APPDATA%\HelloGames\NMS\st_<id>\` on Windows), then start the game and load the slot.

### Region names and system properties

The save never holds a region's name, the generated name of a system the player has not renamed or docked at, or the properties the galaxy map shows on hover (star colour, economy, wealth, conflict, lifeform). The game generates all of them from the address, and the community tool [nms_namegen](https://github.com/hadsh/nms_namegen) reproduces the generator. When it is installed, `show system`, `list systems`, `find`, `export`, `info`, the dashboard, and the `show_system` MCP tool show region names and fill in generated system names (marked `(generated)` or `*`), and `show system` adds the hover properties. Without it everything works as before with those columns empty.

Install it once (Python 3.13 or later with numpy):

```bash
git clone https://github.com/hadsh/nms_namegen ~/.nms-copilot/nms_namegen
git -C ~/.nms-copilot/nms_namegen checkout 52ad48a   # the commit this release was validated against
pip install numpy
```

Answers are cached in `~/.nms-copilot/namegen-cache.json`, so each address costs one run of the tool ever. The location can be changed with `NMS_NAMEGEN` or in the config:

```toml
[namegen]
enabled = true                        # use the tool when it is installed
path = "~/.nms-copilot/nms_namegen/namegen.py"
python = "python"                     # or "python3", or a full path
```

### Interactive REPL

The REPL (`nms-copilot`) opens into a dashboard (see below) and, at its prompt, supports all the commands above plus session management and an interactive galaxy map. It also watches your bases and your fleet: the prompt's right-hand side shows `🌱 16 ready · 📦 1 full · 🚀 1 waiting` while crops are ready to harvest, an extraction network is full, a frigate is waiting for your decision, or an expedition has returned, and a notice prints once when any of those happens. New Navigator offers after the 00:00 UTC reset are announced the same way.

```bash
nms-copilot

[Euclid │ 644 planets │ 293 systems] 🚀 set position "Acadia National Park"
📍 Position set to Acadia National Park (Lush, Gugestor Colony)

[Euclid │ 644 planets │ 293 systems] 🚀 find --biome Lava --nearest 3
  #  Planet       Biome  Distance   Portal Glyphs
  1  (unnamed)    Lava     127K ly  🌅🦕🌀🕊️🐜🌳🌜🕋🌅🌀🕋🦕
  2  (unnamed)    Lava     204K ly  🌅🌜🐜🕊️🐜🌳🦋🕋🌜🔺🕋😑
  3  (unnamed)    Lava     318K ly  🌅😑🐜🕊️🐜🌅🌜🕋🌅🔺🕋🐜

[Euclid │ 644 planets │ 293 systems] 🚀 list bases --limit 5
  #  Base                    System            Planet          Biome
  1  Acadia National Park    Gugestor Colony   Metok-Kalpa     Lush
  2  Sealab 2038             Esurad            Sushimi         Lush
  ...

[Euclid │ 644 planets │ 293 systems] 🚀 map
  (opens full-screen interactive galaxy map with zoom levels)
```

Commands at the prompt beyond the CLI's, and the CLI commands that gain completion or alerts there:

| Command | Description |
|---------|-------------|
| `set position <base>` | Set reference position for distance calculations |
| `set biome <biome>` | Set default biome filter for find/route |
| `set warp-range <ly>` | Set default warp range for route planning |
| `reset [position\|biome\|warp-range\|all]` | Reset session state |
| `status` | Show current session state and base and fleet alerts |
| `base [name] [--width N]` | Every base's crops, extraction, and power, or one base in full |
| `fleet [N\|frigates]` | Frigate expeditions, one expedition in full, or every frigate |
| `have <item> [--type T]` | Do I have it, how much, and where; completes item names |
| `inventory [container] [--free]` | Every container and how full it is, or one container's contents |
| `backup [--label L]` | Snapshot the save now |
| `backup on\|off\|list` | Automatic snapshots for this session, or what is kept |
| `map` | Interactive galaxy map (galaxy/region/local zoom) |
| `dash` | Return to the dashboard (an empty line does the same) |

### Dashboard

`nms-copilot` opens into a dashboard rather than a prompt: your own details, every base, the fleet, and a log of what happened, meant for a terminal on a second monitor so you never leave the game. Anything wanting your attention is coloured in place, so ready crops, a full extraction network, and an expedition holding for a decision stand out in the row they belong to. It uses the same deep-space palette as the tables, and follows `display.color`. It redraws only when something shown changes: a save write, an alert coming due, a key or a resize, or a countdown ticking over. Times are shown to the minute, and to five-minute steps above ten minutes, so the screen changes a handful of times an hour. A new alert rings the terminal bell, which Windows Terminal can turn into a sound or a taskbar flash according to its `bellStyle`.

```
 NMS Copilot   Euclid · in Lauderen · save 21:14 (10m ago) · watching slot 1        q quit  : prompt
┌ PLAYER ──────────────────────────────────────────────────────────────────────────────────────────┐
│System       Lauderen · 6 planets          Units        551,213,032                               │
│Address      2043FC956DEC                  Nanites      4,253                                     │
│From centre  127,412 ly                    Quicksilver  240                                       │
│Warped from  Ekitok                        Inventory    Exosuit 30/93 · Storage 1, Storage 3 full │
│Known        293 systems · 644 planets     Ships        10 · Ship 1 (Fighter S)                   │
│Bases        8                             Freighter    in this system                            │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ BASES ───────────────────────────────────────────────────────────────────────────────────────────┐
│Base                Type      Crops     Next    Extraction                 Power                  │
│Farm                home      16 / 142  now     8,898 / 9,750  2 of 3 FULL 1 battery full · 6 e   │
│Radon               home      -         -       4,750 / 4,750  FULL        1 battery full · 2 e   │
│Home Freighter      freighter -         -       -                          -                      │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ FLEET ───────────────────────────────────────────────────────────────────────────────────────────┐
│#  Type       Length     Frigates Events Elapsed  Status                                          │
│1  Trade      Very long  5        16/18  17h 30m  waiting since 20:58                             │
│2  Combat     Short      3        4/6    40m      about 25m left                                  │
│                                                                                                  │
│Offers: 3 of 5 left · new in 2h 45m (00:00 UTC)                                                   │
│Rooms: 6 of 8 free · 17 of 25 frigates at home                                                    │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ LOG ─────────────────────────────────────────────────────────────────────────────────────────────┐
│21:14  Save written: slot 1 Auto                                                                  │
│21:14  Snapshot saved: 2026-09-15T21-14-02-slot1-auto                                             │
│20:58  Fleet: expedition 1 (Trade) is waiting for your decision since 20:58                       │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

The player section carries where you are, how far out from the centre, where you warped from, and how much of the galaxy the atlas holds on the left; what you are carrying, how full the exosuit is and which storage containers are full, how many ships you own and which you are flying, and where the freighter is on the right. The rest are framed boxes holding the REPL's own tables: a bar of column names over rows on the deep-space navy. Sections share the height between them, so a screen too short for everything trims each one rather than starving the ones at the bottom. The log yields first, since it is history and the tables above it are the live state, and it takes the slack when everything fits. The bases and the fleet carry the same columns and the same cell text as the `base` and `fleet` overviews, so what you read on the dashboard is what those commands print, with a `Next` column added for the soonest harvest. Below about 150 columns the two panels stack so each keeps its full width; above that they sit side by side. Keys: `:` or Enter drops to the prompt and `q` quits.

The dashboard is drawn in the terminal's own buffer, not a separate screen, and leaves the bottom rows clear. Pressing `:` puts the prompt on the first of those rows with the dashboard still above it, so a command's output scrolls the dashboard up the way any other output would, and a short answer sits under a dashboard you can still read. A reminder of the way back prints with the prompt:

```
 NMS Copilot   Euclid · at Base Ferox · save 21:14 (10m ago) · watching slot 1                     q quit  : prompt
   ... dashboard ...
  An empty line or "dash" returns to the dashboard · "help" lists commands · "exit" quits
[Euclid | 644 planets] 🚀 fleet 1
```

An empty line or `dash` returns to the dashboard, which is redrawn at the top of a blank screen with everything before it kept in the scrollback. `exit` and `quit` end the program from either mode. `nms-copilot --prompt` starts at the prompt instead, as does `start = false` in the config; then an empty line stays at the prompt and only `dash` opens the dashboard.

```toml
[dashboard]
start = true                          # open into the dashboard; false starts at the prompt
tick_secs = 60                        # how often alerts are re-checked against the clock
bell = true                           # ring the terminal bell on a new alert
log_lines = 20                        # lines kept in the log panel
```

---

## Architecture

NMS Copilot is a Rust workspace of focused crates:

```
nms/
├─ nms-core       Types, enums, address math, glyph emoji
├─ nms-save       Raw binary save parser (LZ4 + XXTEA + key mapping)
├─ nms-compat     Format adapters (NomNom save format detection)
├─ nms-graph      petgraph spatial model, R-tree index, routing
├─ nms-query      Shared query engine (find, route, show, stats)
├─ nms-watch      File watcher, delta computation, live updates
├─ nms-cache      rkyv zero-copy serialization for fast startup
├─ nms-namegen    Region and system names and properties from the address, via the nms_namegen tool
├─ nms-cli        clap one-shot CLI (the `nms` binary)
└─ nms-copilot    reedline interactive REPL + MCP server (the `nms-copilot` binary)
```

The data flows in one direction:

```
save file → parser → galaxy model → query engine → CLI / REPL / MCP
                          ↑
               file watcher (live updates)
```

The galaxy model is the core: a petgraph of systems with an R-tree spatial index, incrementally updated as the game auto-saves. All three interfaces share the same query engine — no duplicated logic.

### How Save Parsing Works

NMS saves are **LZ4 block-compressed JSON** (not a proprietary binary format). The pipeline:

1. Read sequential 16-byte block headers (magic `0xFEEDA1E5`) + LZ4 payloads
2. Decompress and concatenate all blocks
3. Deobfuscate JSON keys using MBINCompiler's `mapping.json`
4. Deserialize into typed Rust structs via serde

No encryption on modern saves (format 2002+, post-Frontiers). The only crypto is XXTEA on the small metadata file (`mf_save.hg`), used for integrity verification.

### MCP Server

The REPL includes a built-in MCP server for AI co-exploration. It starts automatically on `http://127.0.0.1:3000` and shares the same live model as the REPL.

For headless operation (e.g., Claude Desktop integration):

```bash
nms-copilot --headless                           # stdio transport
nms-copilot --headless --http 127.0.0.1:3000    # HTTP transport
```

The MCP server exposes all query capabilities as tools — your AI copilot can search planets, plan routes, convert coordinates, track your position as you play, check which crops are ready and how full your supply depots are (`base_status`), see whether a frigate is waiting for your decision or an expedition is back (`fleet_status`), and answer "do I have gold and where is it" (`have_item`, `inventory_summary`, `list_ships`). `show_system` carries the region, the generated name, and the hover properties when the `nms_namegen` tool is installed.

---

## Installation

```bash
cargo install nms-copilot    # interactive REPL + MCP server
cargo install nms-cli        # one-shot CLI (the `nms` binary)
```

Or build from source:

```bash
git clone https://github.com/oxur/nms-copilot
cd nms-copilot
make build
```

---

## Requirements

- **Rust** 1.91+ (2024 edition)
- **No Man's Sky** save files (Steam, GOG, or Mac)
- A terminal with emoji support (most modern terminals)

---

## Acknowledgements

NMS Copilot builds on a decade of community reverse engineering. Special thanks to:

- **[libNOM.io](https://github.com/zencq/libNOM.io)** / **[NomNom](https://github.com/zencq/NomNom)** by zencq — the most complete save format implementation
- **[MBINCompiler](https://github.com/monkeyman192/MBINCompiler)** by monkeyman192 — game data decompilation and key mapping
- **[Chase-san](https://gist.github.com/Chase-san/704284e4acd841471d9836e6bc296f2f)** — the cleanest minimal save decoder
- **[MetaIdea/nms-savetool](https://github.com/MetaIdea/nms-savetool)** — definitive format 2001 encryption documentation
- **[NMSCD](https://github.com/NMSCD)** — community developer tools and coordinate converters
- The **NMS Modding Discord** community — collective format knowledge
- **Hello Games** — for building a universe worth exploring 🌌

---

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

---

*"The universe is a pretty big place. It's good to have a copilot."* 🚀🦀

[//]: ---Named-Links---

[logo]: assets/images/logo/v2-x250.jpg
[logo-large]: assets/images/logo/v2.jpg
[build]: https://github.com/oxur/nms-copilot/actions/workflows/ci.yml
[build-badge]: https://github.com/oxur/nms-copilot/actions/workflows/ci.yml/badge.svg
[crate]: https://crates.io/crates/nms-copilot
[crate-badge]: https://img.shields.io/crates/v/nms-copilot.svg
[docs]: https://docs.rs/nms-copilot/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
[tag-badge]: https://img.shields.io/github/tag/oxur/nms-copilot.svg
[tag]: https://github.com/oxur/nms-copilot/tags
