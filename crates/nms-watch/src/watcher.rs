//! File system watcher for NMS save files.
//!
//! Uses `notify` with debouncing to detect save file modifications in the
//! account folder. Every finished write to a save file in that folder is
//! reported; writes to either file of the followed slot are also re-parsed
//! and diffed. Includes robustness features: file stability checks,
//! consecutive failure counting, and graceful error recovery.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use nms_core::delta::SaveDelta;
use nms_save::locate::{SaveFile, parse_save_filename};

use crate::delta::compute_delta;
use crate::error::WatchError;
use crate::snapshot::SaveSnapshot;

/// After this many consecutive parse failures, log a warning.
const MAX_CONSECUTIVE_FAILURES: usize = 5;

/// Milliseconds to wait between file size checks for stability.
const FILE_STABILITY_CHECK_MS: u64 = 100;

/// Something the watcher saw.
#[derive(Debug)]
pub enum WatchEvent {
    /// A save file in the watched folder finished being written, whatever its slot. Drives backups.
    SaveWritten(SaveFile),
    /// The followed slot changed: the file just written was re-parsed and diffed against the last state seen.
    Delta(SaveDelta),
}

/// Handle to a running file watcher.
///
/// Dropping this stops the watcher thread.
pub struct WatchHandle {
    /// Receive events from the watcher thread.
    pub receiver: mpsc::Receiver<WatchEvent>,
    /// Keep the debouncer alive (dropping it stops watching).
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatchConfig {
    /// Path to the save file to follow. When it is a `save*.hg` file, both files of its slot are followed and every save file in its folder is reported; any other file is followed alone.
    pub save_path: PathBuf,
    /// Debounce duration (default: 500ms).
    pub debounce: Duration,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            save_path: PathBuf::new(),
            debounce: Duration::from_millis(500),
        }
    }
}

/// What the watcher follows for deltas.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Followed {
    /// A slot in an account folder: writes to either of its files are diffed.
    Slot { dir: PathBuf, slot: u8 },
    /// One file that is not named like a game save (a JSON export, say).
    File(PathBuf),
}

impl Followed {
    fn from_path(path: &Path) -> Self {
        let parsed = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(parse_save_filename);
        match (parsed, path.parent()) {
            (Some((slot, _)), Some(dir)) => Self::Slot {
                dir: dir.to_path_buf(),
                slot,
            },
            _ => Self::File(path.to_path_buf()),
        }
    }
}

/// How a written path relates to what is followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relevance {
    /// Not a save file we care about.
    Ignore,
    /// A save file in the folder, but another slot: report the write only.
    Other,
    /// A file of the followed slot (or the followed file itself): report and diff.
    Followed,
}

fn relevance(followed: &Followed, path: &Path) -> Relevance {
    match followed {
        Followed::File(p) => {
            if path == p {
                Relevance::Followed
            } else {
                Relevance::Ignore
            }
        }
        Followed::Slot { dir, slot } => {
            if path.parent() != Some(dir.as_path()) {
                return Relevance::Ignore;
            }
            match path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(parse_save_filename)
            {
                Some((s, _)) if s == *slot => Relevance::Followed,
                Some(_) => Relevance::Other,
                None => Relevance::Ignore,
            }
        }
    }
}

/// Check that a file's size is stable (not being actively written).
///
/// Takes two size readings separated by `FILE_STABILITY_CHECK_MS` and
/// returns `true` only if both readings succeed and match.
fn is_file_stable(path: &Path) -> bool {
    let size1 = std::fs::metadata(path).map(|m| m.len()).ok();
    thread::sleep(Duration::from_millis(FILE_STABILITY_CHECK_MS));
    let size2 = std::fs::metadata(path).map(|m| m.len()).ok();
    size1 == size2 && size1.is_some()
}

/// Start watching a save file for changes.
///
/// Returns a `WatchHandle` whose `receiver` yields a `WatchEvent::SaveWritten`
/// for every finished write to a save file in the folder and a
/// `WatchEvent::Delta` whenever the followed slot changes.
///
/// # Errors
///
/// Returns `WatchError::SaveNotFound` if the save file does not exist,
/// `WatchError::ParseError` if the initial parse fails, or
/// `WatchError::NotifyError` if the file watcher cannot be created.
pub fn start_watching(config: WatchConfig) -> Result<WatchHandle, WatchError> {
    if !config.save_path.exists() {
        return Err(WatchError::SaveNotFound(config.save_path));
    }

    let (event_tx, event_rx) = mpsc::channel();
    let save_path = config.save_path.clone();

    // Take an initial snapshot for diffing
    let initial_snapshot =
        SaveSnapshot::from_file(&save_path).map_err(|e| WatchError::ParseError(e.to_string()))?;

    // Channel for notify events (internal)
    let (notify_tx, notify_rx) = mpsc::channel();

    let mut debouncer = new_debouncer(config.debounce, notify_tx)
        .map_err(|e| WatchError::NotifyError(e.to_string()))?;

    // Watch the parent directory (more reliable than watching the file directly,
    // since games often write to a temp file and rename).
    let watch_dir = save_path.parent().unwrap_or(&save_path);

    debouncer
        .watcher()
        .watch(watch_dir, notify::RecursiveMode::NonRecursive)
        .map_err(|e| WatchError::NotifyError(e.to_string()))?;

    // Background thread: receive notify events, report writes, re-parse the followed slot, diff, send
    let followed = Followed::from_path(&save_path);
    thread::spawn(move || {
        let mut snapshot = initial_snapshot;
        let mut consecutive_failures: usize = 0;

        for events in notify_rx {
            let events = match events {
                Ok(evts) => evts,
                Err(_) => continue,
            };

            let mut written: Vec<(PathBuf, Relevance)> = Vec::new();
            for event in events.iter().filter(|e| e.kind == DebouncedEventKind::Any) {
                let relevance = relevance(&followed, &event.path);
                if relevance != Relevance::Ignore && !written.iter().any(|(p, _)| *p == event.path)
                {
                    written.push((event.path.clone(), relevance));
                }
            }

            for (path, relevance) in written {
                // File stability check: ensure file size is stable before reading
                if !is_file_stable(&path) {
                    continue;
                }

                if let Ok(file) = SaveFile::from_path(&path)
                    && event_tx.send(WatchEvent::SaveWritten(file)).is_err()
                {
                    return;
                }

                if relevance != Relevance::Followed {
                    continue;
                }

                // Re-parse whichever file of the slot was written; it is the game's newest state
                match SaveSnapshot::from_file(&path) {
                    Ok(new_snapshot) => {
                        consecutive_failures = 0;
                        let delta = compute_delta(&snapshot, &new_snapshot);

                        if !delta.is_empty() && event_tx.send(WatchEvent::Delta(delta)).is_err() {
                            return;
                        }

                        snapshot = new_snapshot;
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        if consecutive_failures == MAX_CONSECUTIVE_FAILURES {
                            eprintln!(
                                "Warning: {MAX_CONSECUTIVE_FAILURES} consecutive parse failures. \
                                 Save file may be corrupt or format changed: {e}"
                            );
                        }
                        // Keep watching -- the next save might succeed
                    }
                }
            }
        }
    });

    Ok(WatchHandle {
        receiver: event_rx,
        _debouncer: debouncer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_save::locate::SaveType;

    #[test]
    fn test_watch_config_default_debounce() {
        let config = WatchConfig::default();
        assert_eq!(config.debounce, Duration::from_millis(500));
    }

    #[test]
    fn test_start_watching_nonexistent_file_errors() {
        let config = WatchConfig {
            save_path: PathBuf::from("/tmp/nonexistent_nms_save_12345.json"),
            ..Default::default()
        };
        assert!(matches!(
            start_watching(config),
            Err(WatchError::SaveNotFound(_))
        ));
    }

    #[test]
    fn test_is_file_stable_for_static_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stable.json");
        std::fs::write(&path, "{}").unwrap();
        assert!(is_file_stable(&path));
    }

    #[test]
    fn test_is_file_stable_nonexistent_returns_false() {
        assert!(!is_file_stable(Path::new(
            "/tmp/nonexistent_nms_stable_check_xyz"
        )));
    }

    #[test]
    fn test_max_consecutive_failures_constant() {
        const { assert!(MAX_CONSECUTIVE_FAILURES > 0) };
    }

    #[test]
    fn test_file_stability_check_ms_constant() {
        const { assert!(FILE_STABILITY_CHECK_MS > 0) };
        const { assert!(FILE_STABILITY_CHECK_MS <= 1000) };
    }

    #[test]
    fn test_followed_from_save_name_is_a_slot() {
        let followed = Followed::from_path(Path::new("/acct/save4.hg"));
        assert_eq!(
            followed,
            Followed::Slot {
                dir: PathBuf::from("/acct"),
                slot: 2
            }
        );
    }

    #[test]
    fn test_followed_from_other_name_is_the_file() {
        let followed = Followed::from_path(Path::new("/tmp/export.json"));
        assert_eq!(followed, Followed::File(PathBuf::from("/tmp/export.json")));
    }

    #[test]
    fn test_relevance_for_a_followed_slot() {
        let followed = Followed::from_path(Path::new("/acct/save.hg"));
        assert_eq!(
            relevance(&followed, Path::new("/acct/save.hg")),
            Relevance::Followed
        );
        assert_eq!(
            relevance(&followed, Path::new("/acct/save2.hg")),
            Relevance::Followed
        );
        assert_eq!(
            relevance(&followed, Path::new("/acct/save3.hg")),
            Relevance::Other
        );
        assert_eq!(
            relevance(&followed, Path::new("/acct/mf_save.hg")),
            Relevance::Ignore
        );
        assert_eq!(
            relevance(&followed, Path::new("/acct/accountdata.hg")),
            Relevance::Ignore
        );
        assert_eq!(
            relevance(&followed, Path::new("/other/save.hg")),
            Relevance::Ignore
        );
    }

    #[test]
    fn test_relevance_for_a_followed_file() {
        let followed = Followed::from_path(Path::new("/tmp/export.json"));
        assert_eq!(
            relevance(&followed, Path::new("/tmp/export.json")),
            Relevance::Followed
        );
        assert_eq!(
            relevance(&followed, Path::new("/tmp/save.hg")),
            Relevance::Ignore
        );
    }

    fn fixture_json() -> Vec<u8> {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        std::fs::read(
            workspace
                .join("data")
                .join("test")
                .join("multi_system_save.json"),
        )
        .unwrap()
    }

    fn next_event(rx: &mpsc::Receiver<WatchEvent>) -> WatchEvent {
        rx.recv_timeout(Duration::from_secs(15))
            .expect("watcher event")
    }

    #[test]
    fn test_watcher_reports_every_save_write_and_diffs_the_followed_slot() {
        let dir = tempfile::tempdir().unwrap();
        let json = fixture_json();
        let followed = dir.path().join("save.hg");
        std::fs::write(&followed, &json).unwrap();

        let handle = start_watching(WatchConfig {
            save_path: followed.clone(),
            debounce: Duration::from_millis(100),
        })
        .unwrap();

        // The other file of the followed slot, same content: reported, not diffed.
        std::fs::write(dir.path().join("save2.hg"), &json).unwrap();
        match next_event(&handle.receiver) {
            WatchEvent::SaveWritten(file) => {
                assert_eq!(file.slot(), 1);
                assert_eq!(file.save_type(), SaveType::Auto);
            }
            other => panic!("expected SaveWritten, got {other:?}"),
        }

        // Another slot: reported only.
        std::fs::write(dir.path().join("save3.hg"), &json).unwrap();
        match next_event(&handle.receiver) {
            WatchEvent::SaveWritten(file) => assert_eq!(file.slot(), 2),
            other => panic!("expected SaveWritten, got {other:?}"),
        }

        // Metadata alone: nothing.
        std::fs::write(dir.path().join("mf_save.hg"), b"meta").unwrap();
        assert!(
            handle
                .receiver
                .recv_timeout(Duration::from_millis(1500))
                .is_err()
        );

        // The followed slot with a change: reported and diffed.
        let moved =
            String::from_utf8(json.clone())
                .unwrap()
                .replacen("\"VoxelX\": ", "\"VoxelX\": 1", 1);
        std::fs::write(dir.path().join("save2.hg"), moved.as_bytes()).unwrap();
        match next_event(&handle.receiver) {
            WatchEvent::SaveWritten(file) => assert_eq!(file.slot(), 1),
            other => panic!("expected SaveWritten, got {other:?}"),
        }
        match next_event(&handle.receiver) {
            WatchEvent::Delta(delta) => assert!(!delta.is_empty()),
            other => panic!("expected Delta, got {other:?}"),
        }
    }
}
