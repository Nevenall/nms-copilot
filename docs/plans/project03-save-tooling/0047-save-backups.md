# Save backups

Keep copies of the game's save files: on demand from the CLI or REPL, and automatically on every save the game makes while the REPL or MCP server is running.

**Status:** first draft, 2026-09-14. Prompted by the fleet verification work the same day: the game overwrote both files of the slot when an intervention call was answered, and the before-answer save was lost with them.

**Depends on:** the `locate` module in `nms-save` (accounts, slot pairs, metadata siblings) and the `nms-watch` file watcher. Both exist; the watcher needs one change described below.

---

## Context

A slot is two files, `saveN.hg` and `saveN+1.hg`, one written by manual saves and one by auto saves, each with a `mf_` metadata sibling the game needs to load it. The game alternates between them as the player saves, so any state older than the last write to each file is gone.

The tool already knows the account folder, the slot layout, and each file's metadata path, and the REPL already watches the save folder for changes. What it lacks is a way to keep what it sees. Restore stays manual: copy a snapshot's pair back into the account folder while the game is closed. The tool is not a save editor and will not write into the game's folder.

---

## Design

### Snapshots

A snapshot is a folder holding the save file and its metadata sibling under their original names, so restoring is a two-file copy with no renaming:

```
~/.nms-copilot/backups/<account>/2026-09-14T13-41-36-slot1-manual/save.hg
~/.nms-copilot/backups/<account>/2026-09-14T13-41-36-slot1-manual/mf_save.hg
~/.nms-copilot/backups/<account>/2026-09-14T13-46-43-slot1-auto-before-call/save2.hg
~/.nms-copilot/backups/<account>/2026-09-14T13-46-43-slot1-auto-before-call/mf_save2.hg
```

The timestamp is the save file's modification time in local time, not the moment the copy was made. An optional label goes on the end of the folder name. A `notes.txt` can be dropped in by hand; the tool never touches it.

Taking a snapshot: check the save file's size is stable (the watcher's existing check), copy the save, copy the metadata sibling if present, then compare the save's hash with the newest existing snapshot of the same file and delete the new folder if they match. The hash comparison costs nothing and stops a restarted REPL from duplicating the file it starts from.

### Retention

Per account, keep the newest `keep` unlabelled snapshots of each slot (default 20) and every labelled one. Pruning runs after each automatic snapshot and on `backup prune`; it never runs on a manual `backup`. `keep = 0` means keep everything, which is what a research session wants.

### Watching both files of a slot

`nms-watch` today follows one file: the path it was started with. The debouncer watches the account folder, but events for the other file of the slot are dropped, so an auto save is invisible while the model follows the manual file, and the reverse. That is also why REPL alerts fire on every other save.

Change: `WatchConfig` takes the account folder and the slot to follow. Events are filtered with `parse_save_filename`, so any `save*.hg` write in the folder counts. The watcher emits a `WatchEvent`:

```
WatchEvent::SaveWritten(SaveFile)        // any slot, after the stability check; drives backups
WatchEvent::Delta(SaveDelta)             // the followed slot only; the newer of its two files re-parsed and diffed
```

The delta side becomes slot-based: whichever file of the followed slot was just written is the new state of the game, so it is parsed and diffed against the last snapshot regardless of which file it is. Nothing downstream changes shape; `SaveDelta` is unchanged, and the base and fleet alerts simply start firing on every save.

### Surfaces

CLI, one-shot:

```
nms backup                          # snapshot the most recent save of the most recent slot
nms backup --slot 3                 # a specific slot, its most recent file
nms backup --all                    # every file of every slot
nms backup --label before-call      # label the snapshot
nms backup list [--slot N]          # what is kept, newest first, with sizes and labels
nms backup prune                    # apply the retention rule now
```

REPL:

```
backup                              # snapshot now
backup on | off                     # automatic snapshots for this session
backup list
```

Config, `~/.nms-copilot/config.toml`:

```toml
[backup]
enabled = false                     # automatic snapshots when the watcher is running
dir = "~/.nms-copilot/backups"      # override the location
keep = 20                           # unlabelled snapshots per slot; 0 keeps everything
```

The MCP server runs the same watcher, so `enabled = true` snapshots while it runs too. No MCP tool is planned; an assistant has no reason to trigger a backup that the watcher would not already take.

### Output

`backup list` is a table in the house style:

```
 BACKUPS  st_76561198028658595                         42 snapshots · 33.6 MB
 When                 Slot  Type    Label         Size
 2026-09-14 13:46:43  1     auto    before-call   779 KB
 2026-09-14 13:41:36  1     manual                779 KB
 ...
```

A snapshot taken by hand prints one line with the folder it went to. Automatic snapshots print nothing in the REPL unless the copy fails.

---

## Files to create or modify

| File | Change |
|------|--------|
| `crates/nms-save/src/backup.rs` | new: `snapshot(file, dir, label) -> Snapshot`, `list(dir)`, `prune(dir, keep)`, folder naming, hash check |
| `crates/nms-save/src/locate.rs` | `list_saves` already exposes the pieces; add `SaveSlot::files()` if the `--all` path wants it |
| `crates/nms-watch/src/watcher.rs` | watch the account folder for a slot; `WatchEvent` enum; delta from whichever file of the slot was written |
| `crates/nms-copilot/src/config.rs` | `BackupConfig { enabled, dir, keep }` |
| `crates/nms-copilot/src/paths.rs` | `backup_dir()` default |
| `crates/nms-copilot/src/watch.rs`, `session.rs` | handle `SaveWritten` by snapshotting when enabled; session toggle |
| `crates/nms-copilot/src/commands.rs`, `dispatch.rs`, `completer.rs` | `backup`, `backup on|off`, `backup list` |
| `crates/nms-copilot/src/mcp/*` | consume `WatchEvent` instead of bare deltas; no new tool |
| `crates/nms-cli/src/backup.rs`, `main.rs` | `nms backup [--slot N | --all] [--label] [--to DIR]`, `list`, `prune` |
| `README.md` | command and config docs; the manual restore procedure |

---

## Key design decisions

- **Restore is manual.** Writing into the game's folder while it runs can corrupt the slot. The tool copies out, never in; the README shows the two-file copy to do by hand with the game closed.
- **Original file names inside a dated folder**, so a restore is a copy, not a rename, and the pair stays together.
- **Both files of a slot are watched.** Needed for backups to catch auto saves, and it fixes the every-other-save gap in the existing alerts as a side effect.
- **Full copies in the game's own format.** No re-encoding, no deltas. Saves are about 800 KB; the retention rule, not compression, keeps the folder in check.
- **Timestamps come from the file**, so a snapshot's name says when the game saved, not when the tool noticed.
- **Automatic snapshots are off by default.** The feature exists for the player who turns it on, and for research sessions.

---

## Testing strategy

- `backup.rs` against a temp folder: naming from a fixed modification time, metadata sibling copied when present and skipped when absent, the duplicate-hash check removing an identical snapshot, labelled snapshots surviving `prune`, `keep = 0` keeping everything.
- Watcher: a write to the slot's other file produces both a `SaveWritten` and a `Delta`; a write to a different slot produces only `SaveWritten`; a `mf_` write alone produces nothing.
- Config round trip for `[backup]` with defaults.
- CLI: `backup list` on an empty folder, after one snapshot, and with `--slot`.
- REPL: `backup on` then a simulated write snapshots; `backup off` stops it.

---

## Open questions

1. Does the game write the `mf_` sibling before or after the save file? If after, the snapshot should wait for both to be stable, not just the save.
2. Should `backup --all` include slots the player has not opened in months? Probably yes; it is the backup command.
3. Is a `notes.txt` per snapshot enough for research, or should `backup list` show a first line from it?
