# Dashboard

Make the REPL open into a full-screen dashboard that shows the bases and the fleet and redraws when the game writes a save or an alert comes due. The prompt stays, as a mode the player drops into for questions, rather than the screen they watch.

**Status:** first draft, 2026-09-15. Prompted by the fleet arc: the alerts it added only print when the player presses enter, and checking them means switching windows out of the game, which the game handles badly. The player keeps a terminal on a second monitor, so the terminal itself is the notification surface.

**Depends on:** the `base` and `fleet` queries and alerts from project 02 (done), the ratatui screen pattern from the map, and the `nms-watch` watcher. The slot-wide watching in [0047](0047-save-backups.md) is not required, but until it lands the dashboard sees only every other save, as the REPL alerts do today.

---

## Context

Today `nms-copilot` starts at a prompt. Between commands it drains the watcher, recomputes the crop, depot, and fleet alerts against the clock, and prints any new ones (`main.rs`, the loop around `read_line`). Nothing happens while the prompt is waiting, so an expedition that comes back at 21:40 is announced at whatever time the player next types.

Everything the dashboard needs to show already exists as data: `BaseStatus` (crops with progress and next-ready time, pipe networks with stored and capacity, power), `FleetStatus` (expeditions with state, elapsed, hold time, and the estimated remaining time, plus the offer count, free rooms, and frigates at home), and `Alert` with its once-per-event announcement in `SessionState::refresh_alerts`. The map already shows how a ratatui screen enters the alternate screen, runs an event loop, and restores the terminal. The dashboard is a rendering and scheduling change, not a new query.

---

## Design

### The screen

```
 NMS Copilot   Euclid · at Base Ferox (Ushtar XI) · save 21:14 (12m ago) · watching slot 1                  q quit  : prompt
┌ Alerts ──────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● Fleet: expedition 1 is waiting for you (since 20:58)                                                               │
│   Base Ferox: 14 Gamma Weed ready                                                                                     │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌ Bases ─────────────────────────────────────────┐ ┌ Fleet ─────────────────────────────────────────────────────────────┐
│ Base            Crops        Next    Depots  Pwr│ │ #  Type    Length     Frig  Events   Elapsed  Status              │
│ Base Ferox      14/40 ready  now     3/4 full ok│ │ 1  Trade   Very long  5     16/18    17h 34m  waiting since 20:58 │
│ Ionised Rain    0/12         2h 10m  0/2     ok │ │ 2  Combat  Short      3     4/6      0h 41m   about 25m left      │
│ Freighter       -            -       -       -  │ │                                                                    │
│                                                 │ │ Offers: 3 of 5 left · new at 00:00 UTC (in 2h 46m)                 │
│                                                 │ │ Rooms: 6 free · 17 frigates at home                                │
└────────────────────────────────────────────────┘ └────────────────────────────────────────────────────────────────────┘
┌ Log ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 21:14  Save written (auto)                                                                                           │
│ 21:02  Warped: Ushtar -> Ekitok                                                                                      │
│ 20:58  Fleet: expedition 1 is waiting for you                                                                        │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Five regions, fixed layout, no tabs and no cursor:

- **Header**: galaxy, position, when the save was last written and how long ago, what is being watched, and the two keys that matter.
- **Alerts**: the current alerts, the same list `refresh_alerts` maintains, alerting bases first as in the `base` command. Alerts that arrived since the last keypress carry a marker so the eye finds them; any key clears the markers. Empty shows "Nothing waiting".
- **Bases**: one row per base from `execute_base` with no name filter: crops ready of total, time to the next harvest, full networks of total, and a one-word power state. Bases with no crops and no networks show dashes rather than being hidden, so the table is stable and the player learns what the tool knows about each base.
- **Fleet**: the `fleet` overview table as it already prints, then the Navigator and home lines. No fleet shows "No freighter".
- **Log**: the last twenty lines the REPL would have printed: save writes, warps, discoveries, alerts as they fired. Newest at the bottom.

Widths adapt to the terminal; below about 100 columns the bases and fleet panels stack instead of sitting side by side. Detail views (`base <name>`, `fleet N`) stay prompt commands.

### When it redraws

The player asked for a calm screen. The dashboard redraws on exactly four occasions and otherwise leaves the terminal alone:

1. **A save write.** The watcher's delta is applied under a short write lock, the log gains its lines, both panels are rebuilt.
2. **An alert coming due.** A timer wakes every 60 seconds (`[dashboard] tick_secs`), recomputes the alerts with the current clock, and redraws only if the alert set changed. A new alert also rings the terminal bell, so Windows Terminal can beep or flash the taskbar according to its own `bellStyle`.
3. **A keypress or a resize.**
4. **A coarse clock change.** Countdowns and "ago" times are shown to the minute, and only above ten minutes to five-minute steps ("2h 10m", "in 25m", "now"). The 60-second timer also redraws when any displayed time string changed, which at that granularity is a handful of times an hour. No spinners, no seconds, no animation.

The rule is implemented as: build the view, compare it with the last one drawn, draw only on difference. That makes "calm" a property of the view, not of the trigger code, and it is easy to test.

### Modes

`nms-copilot` starts in the dashboard. Pressing `:` or Enter leaves the alternate screen and runs the existing reedline loop unchanged: history, completion, every command, the `map`. From the prompt, an empty line or the `dash` command returns to the dashboard; `exit` and `quit` end the program from either mode. `--prompt` on the command line, or `[dashboard] start = false`, starts at the prompt for anyone who prefers the old behaviour.

Whichever mode is active drains the watcher and refreshes the alerts, through one shared function that replaces the two copies in the main loop today. The prompt mode keeps printing notices between commands as it does now; the dashboard shows the same notices in its log when the player comes back to it, because the alert state is shared.

### Locks

The dashboard takes the model read lock only while building a view and the write lock only while applying a delta, never across a poll or a draw, so the MCP server keeps answering while the screen is up.

### Config

```toml
[dashboard]
start = true        # open into the dashboard; false starts at the prompt
tick_secs = 60      # how often alerts are re-checked against the clock
bell = true         # ring the terminal bell on a new alert
log_lines = 20
```

---

## Architecture

```
nms-watch receiver ─┐
60 s timer          ├─► dashboard::run(model, session, watcher, config)   (owns the loop while the dashboard is up)
crossterm events   ─┘         │
                              ├─► view::build(&model, &session, now) -> View     (pure: BaseStatus, FleetStatus, Alerts, log)
                              └─► render::draw(frame, &View)                   (ratatui, no model access)
```

`View` is a plain struct of strings and rows. Building it is the only place the model is read; drawing it is the only place ratatui appears. The redraw rule compares `View` values.

### Files to create or modify

| File | Change |
|------|--------|
| `crates/nms-copilot/src/dashboard/mod.rs` | `run`: enter and restore the alternate screen like `map::run_map`, the wake loop, mode switching result |
| `crates/nms-copilot/src/dashboard/view.rs` | `View` and `build`; time strings at the coarse granularity |
| `crates/nms-copilot/src/dashboard/render.rs` | layout and widgets |
| `crates/nms-copilot/src/dashboard/input.rs` | keys: `q`, `:`, Enter, any key clears markers |
| `crates/nms-copilot/src/session.rs` | log ring buffer; `record` used by both modes |
| `crates/nms-copilot/src/watch.rs` | `sync(model, session, receiver, cache, now)`: the shared drain-and-refresh, returning the new notes |
| `crates/nms-copilot/src/main.rs` | mode loop: dashboard, prompt, back; `--prompt` flag; `dash` command |
| `crates/nms-copilot/src/commands.rs`, `dispatch.rs`, `completer.rs` | `dash` |
| `crates/nms-copilot/src/config.rs` | `[dashboard]` |
| `crates/nms-query/src/display.rs` | `format_duration_coarse`, `format_ago` if the coarse forms do not already exist |
| `README.md` | the dashboard, keys, and config |

---

## Key design decisions

- **Calm over live.** Redraw on save writes, alert changes, keys, and coarse time changes only. A dashboard that flickers every second on a second monitor is worse than the prompt it replaces.
- **The prompt survives intact.** A command line inside ratatui would mean replacing reedline and its history and completion; leaving the alternate screen for the existing loop costs nothing and keeps every command working.
- **One drain path.** Both modes call the same sync function, so the watcher and alert behaviour cannot drift between them.
- **Pure view, dumb renderer.** Everything worth testing is in `build`; the renderer only lays out strings.
- **The bell is the only sound.** The terminal decides whether that is a beep, a flash, or nothing; the tool does not depend on audio or OS notification APIs.

---

## Testing strategy

- `build` against the multi-system fixture with a fixed `now`: base rows, fleet rows, offer line, alerts, header strings.
- Redraw rule: two views a few seconds apart compare equal; views across a harvest becoming ready, a save write, and a five-minute boundary compare different.
- Coarse time formatting at the boundaries: 59 s, 61 s, 9m 59 s, 10m, 2h 10m.
- The shared sync function: a delta on the receiver updates the model, the log, and the alert state exactly once.
- Mode switching in `main` is kept thin enough not to need a test; `dash` parses like the other commands.

---

## Open questions

- Whether the log should also record prompt commands run, so the dashboard shows what the player did while away from it. Leaning no.
- The corvette's `PlayerShipBase` entry named "Default" will appear in the bases panel with dashes, like the freighter. Deciding how ship bases are labelled is still deferred from arc 02.
- Once [0047](0047-save-backups.md) lands, the header can say which file of the slot was written and the log can note snapshots taken.
