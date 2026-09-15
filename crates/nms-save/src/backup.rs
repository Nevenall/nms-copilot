//! Save backups: copy a save file and its metadata sibling into a dated folder, list what is kept, and prune by a retention rule.
//!
//! A snapshot is a folder under `<root>/<account>/` holding the save file and its `mf_` sibling under their original names, so restoring is a two-file copy with no renaming. Restore is manual and done with the game closed; nothing here writes into the game's folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime};
use sha2::{Digest, Sha256};

use crate::locate::{SaveFile, SaveType};

/// Folder names start with the save's modification time in this form, which sorts chronologically as text.
const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H-%M-%S";
const TIMESTAMP_LEN: usize = 19;

/// Errors from taking, listing, or pruning snapshots.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackupError {
    #[error("could not determine home directory")]
    NoHomeDir,

    #[error("{0} is not inside an account folder")]
    NoAccount(PathBuf),

    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Where snapshots go unless configured otherwise: `~/.nms-copilot/backups`.
pub fn default_root() -> Result<PathBuf, BackupError> {
    dirs::home_dir()
        .map(|home| home.join(".nms-copilot").join("backups"))
        .ok_or(BackupError::NoHomeDir)
}

/// The account folder a save file lives in, by name (`st_76561198028658595` or `DefaultUser`).
pub fn account_name(save_path: &Path) -> Result<String, BackupError> {
    save_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .ok_or_else(|| BackupError::NoAccount(save_path.to_path_buf()))
}

/// One kept snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The snapshot folder.
    pub path: PathBuf,
    /// The save's modification time when it was copied, in local time.
    pub taken: NaiveDateTime,
    pub slot: u8,
    pub save_type: SaveType,
    pub label: Option<String>,
    /// Bytes across the files in the folder.
    pub size: u64,
}

impl Snapshot {
    /// The folder's name.
    pub fn folder_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string()
    }

    /// Labelled snapshots survive pruning.
    pub fn is_labelled(&self) -> bool {
        self.label.is_some()
    }
}

/// What a snapshot request did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotOutcome {
    /// A new folder was written.
    Taken(Snapshot),
    /// The newest kept copy of this file has the same content, so nothing was written; the existing snapshot is returned.
    Unchanged(Snapshot),
}

impl SnapshotOutcome {
    pub fn snapshot(&self) -> &Snapshot {
        match self {
            Self::Taken(s) | Self::Unchanged(s) => s,
        }
    }
}

/// Settings for automatic snapshots: where they go and how many unlabelled ones to keep per slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPolicy {
    pub root: PathBuf,
    /// Unlabelled snapshots kept per slot; zero keeps everything.
    pub keep: usize,
}

impl BackupPolicy {
    /// Snapshot a save the watcher saw written, then apply the retention rule to its account.
    pub fn auto_snapshot(&self, save: &SaveFile) -> Result<SnapshotOutcome, BackupError> {
        let outcome = snapshot(save, &self.root, None)?;
        if matches!(outcome, SnapshotOutcome::Taken(_)) {
            prune(&self.root, &account_name(save.path())?, self.keep)?;
        }
        Ok(outcome)
    }
}

/// Reduce a label to letters, digits, `-` and `_`; runs of anything else become one `-`.
pub fn sanitize_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut pending_dash = false;
    for c in label.trim().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    out
}

/// The folder name for a snapshot: `2026-09-14T13-41-36-slot1-manual`, plus `-<label>` when labelled.
pub fn folder_name(
    taken: NaiveDateTime,
    slot: u8,
    save_type: SaveType,
    label: Option<&str>,
) -> String {
    let mut name = format!(
        "{}-slot{slot}-{}",
        taken.format(TIMESTAMP_FORMAT),
        type_word(save_type)
    );
    if let Some(label) = label.map(sanitize_label).filter(|l| !l.is_empty()) {
        name.push('-');
        name.push_str(&label);
    }
    name
}

/// Read a folder name back; `None` for folders this module did not write.
pub fn parse_folder_name(name: &str) -> Option<(NaiveDateTime, u8, SaveType, Option<String>)> {
    if name.len() < TIMESTAMP_LEN || !name.is_char_boundary(TIMESTAMP_LEN) {
        return None;
    }
    let (stamp, rest) = name.split_at(TIMESTAMP_LEN);
    let taken = NaiveDateTime::parse_from_str(stamp, TIMESTAMP_FORMAT).ok()?;
    let rest = rest.strip_prefix("-slot")?;
    let digits_end = rest.find(|c: char| !c.is_ascii_digit())?;
    let slot: u8 = rest[..digits_end].parse().ok()?;
    let rest = rest[digits_end..].strip_prefix('-')?;
    let (save_type, rest) = match rest.strip_prefix("manual") {
        Some(r) => (SaveType::Manual, r),
        None => (SaveType::Auto, rest.strip_prefix("auto")?),
    };
    let label = match rest {
        "" => None,
        r => Some(r.strip_prefix('-')?.to_string()),
    };
    Some((taken, slot, save_type, label))
}

fn type_word(save_type: SaveType) -> &'static str {
    match save_type {
        SaveType::Manual => "manual",
        SaveType::Auto => "auto",
    }
}

fn local_time(time: SystemTime) -> NaiveDateTime {
    DateTime::<Local>::from(time).naive_local()
}

/// Copy `save` and its metadata sibling into a new folder under `<root>/<account>/`.
///
/// An unlabelled request whose content matches the newest kept copy of the same file writes nothing and reports `Unchanged`, so a restarted watcher does not duplicate the file it starts from. A labelled request always keeps its folder. A folder that already exists under the computed name is treated as the same snapshot.
pub fn snapshot(
    save: &SaveFile,
    root: &Path,
    label: Option<&str>,
) -> Result<SnapshotOutcome, BackupError> {
    let account = account_name(save.path())?;
    let account_dir = root.join(&account);
    let taken = local_time(save.modified());
    let folder = account_dir.join(folder_name(taken, save.slot(), save.save_type(), label));

    if folder.is_dir() {
        return Ok(SnapshotOutcome::Unchanged(read_snapshot(&folder)?));
    }

    let file_name = save
        .path()
        .file_name()
        .ok_or_else(|| BackupError::NoAccount(save.path().to_path_buf()))?;

    if label.is_none()
        && let Some(newest) = list(root, &account)?
            .into_iter()
            .find(|s| s.slot == save.slot() && s.save_type == save.save_type())
    {
        let kept = newest.path.join(file_name);
        if kept.is_file() && file_hash(&kept)? == file_hash(save.path())? {
            return Ok(SnapshotOutcome::Unchanged(newest));
        }
    }

    fs::create_dir_all(&folder)?;
    let copied = copy_pair(save, &folder, file_name);
    if let Err(e) = copied {
        let _ = fs::remove_dir_all(&folder);
        return Err(e);
    }
    Ok(SnapshotOutcome::Taken(read_snapshot(&folder)?))
}

fn copy_pair(
    save: &SaveFile,
    folder: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(), BackupError> {
    fs::copy(save.path(), folder.join(file_name))?;
    let metadata = save.metadata_path();
    if metadata.is_file()
        && let Some(name) = metadata.file_name()
    {
        fs::copy(&metadata, folder.join(name))?;
    }
    Ok(())
}

fn file_hash(path: &Path) -> Result<[u8; 32], BackupError> {
    let bytes = fs::read(path)?;
    Ok(Sha256::digest(&bytes).into())
}

fn folder_size(folder: &Path) -> Result<u64, BackupError> {
    let mut size = 0;
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

fn read_snapshot(folder: &Path) -> Result<Snapshot, BackupError> {
    let name = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let (taken, slot, save_type, label) = parse_folder_name(name).ok_or_else(|| {
        BackupError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("not a snapshot folder: {name}"),
        ))
    })?;
    Ok(Snapshot {
        path: folder.to_path_buf(),
        taken,
        slot,
        save_type,
        label,
        size: folder_size(folder)?,
    })
}

/// Every snapshot kept for an account, newest first. A missing folder lists as empty.
pub fn list(root: &Path, account: &str) -> Result<Vec<Snapshot>, BackupError> {
    let account_dir = root.join(account);
    if !account_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in fs::read_dir(&account_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        let recognised = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| parse_folder_name(n).is_some());
        if recognised {
            snapshots.push(read_snapshot(&path)?);
        }
    }
    snapshots.sort_by_key(|s| std::cmp::Reverse(s.folder_name()));
    Ok(snapshots)
}

/// The account folders under `root`, sorted by name.
pub fn list_accounts(root: &Path) -> Result<Vec<String>, BackupError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = fs::read_dir(root)?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    Ok(names)
}

/// Delete all but the newest `keep` unlabelled snapshots of each slot; labelled ones always stay. Zero keeps everything. Returns what was removed.
pub fn prune(root: &Path, account: &str, keep: usize) -> Result<Vec<Snapshot>, BackupError> {
    if keep == 0 {
        return Ok(Vec::new());
    }
    let mut seen_per_slot: std::collections::HashMap<u8, usize> = std::collections::HashMap::new();
    let mut removed = Vec::new();
    for snapshot in list(root, account)? {
        if snapshot.is_labelled() {
            continue;
        }
        let seen = seen_per_slot.entry(snapshot.slot).or_insert(0);
        *seen += 1;
        if *seen > keep {
            fs::remove_dir_all(&snapshot.path)?;
            removed.push(snapshot);
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::time::Duration;

    fn stamp(secs_after_epoch_day: u64) -> SystemTime {
        // A fixed instant well after the epoch so local-time conversion never goes negative.
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + secs_after_epoch_day)
    }

    fn write_save(account: &Path, name: &str, content: &[u8], modified: SystemTime) -> SaveFile {
        let path = account.join(name);
        fs::write(&path, content).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        SaveFile::from_path(&path).unwrap()
    }

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("st_1");
        let root = dir.path().join("backups");
        fs::create_dir_all(&account).unwrap();
        (dir, account, root)
    }

    #[test]
    fn test_folder_name_roundtrip_with_label() {
        let taken = NaiveDate::from_ymd_opt(2026, 9, 14)
            .unwrap()
            .and_hms_opt(13, 41, 36)
            .unwrap();
        let name = folder_name(taken, 1, SaveType::Manual, Some("before call"));
        assert_eq!(name, "2026-09-14T13-41-36-slot1-manual-before-call");
        assert_eq!(
            parse_folder_name(&name),
            Some((taken, 1, SaveType::Manual, Some("before-call".into())))
        );
    }

    #[test]
    fn test_folder_name_roundtrip_without_label() {
        let taken = NaiveDate::from_ymd_opt(2026, 9, 14)
            .unwrap()
            .and_hms_opt(13, 46, 43)
            .unwrap();
        let name = folder_name(taken, 12, SaveType::Auto, Some("   "));
        assert_eq!(name, "2026-09-14T13-46-43-slot12-auto");
        assert_eq!(
            parse_folder_name(&name),
            Some((taken, 12, SaveType::Auto, None))
        );
    }

    #[test]
    fn test_parse_folder_name_rejects_strangers() {
        assert!(parse_folder_name("notes").is_none());
        assert!(parse_folder_name("2026-09-14T13-46-43-slot1").is_none());
        assert!(parse_folder_name("2026-09-14T13-46-43-slotx-auto").is_none());
        assert!(parse_folder_name("2026-09-14T13-46-43-slot1-weekly").is_none());
        assert!(parse_folder_name("2026-09-14T13-46-43-slot1-autox").is_none());
    }

    #[test]
    fn test_sanitize_label_collapses_runs() {
        assert_eq!(sanitize_label("before the call!!"), "before-the-call");
        assert_eq!(sanitize_label("--a--b--"), "a-b");
        assert_eq!(sanitize_label("ok_1"), "ok_1");
    }

    #[test]
    fn test_snapshot_copies_save_and_metadata_into_dated_folder() {
        let (_dir, account, root) = fixture();
        let save = write_save(&account, "save2.hg", b"one", stamp(0));
        fs::write(account.join("mf_save2.hg"), b"meta").unwrap();

        let outcome = snapshot(&save, &root, None).unwrap();
        let SnapshotOutcome::Taken(taken) = outcome else {
            panic!("expected a new snapshot")
        };
        assert_eq!(taken.slot, 1);
        assert_eq!(taken.save_type, SaveType::Auto);
        assert_eq!(taken.label, None);
        assert_eq!(taken.size, 7);
        assert_eq!(taken.taken, local_time(stamp(0)));
        assert!(taken.path.starts_with(root.join("st_1")));
        assert_eq!(fs::read(taken.path.join("save2.hg")).unwrap(), b"one");
        assert_eq!(fs::read(taken.path.join("mf_save2.hg")).unwrap(), b"meta");
    }

    #[test]
    fn test_snapshot_without_metadata_copies_one_file() {
        let (_dir, account, root) = fixture();
        let save = write_save(&account, "save.hg", b"solo", stamp(0));
        let outcome = snapshot(&save, &root, None).unwrap();
        let taken = outcome.snapshot();
        assert_eq!(fs::read_dir(&taken.path).unwrap().count(), 1);
        assert_eq!(taken.save_type, SaveType::Manual);
    }

    #[test]
    fn test_snapshot_identical_content_is_unchanged_until_content_differs() {
        let (_dir, account, root) = fixture();
        let save = write_save(&account, "save.hg", b"same", stamp(0));
        let first = snapshot(&save, &root, None).unwrap();
        assert!(matches!(first, SnapshotOutcome::Taken(_)));

        let again = write_save(&account, "save.hg", b"same", stamp(60));
        let second = snapshot(&again, &root, None).unwrap();
        assert!(
            matches!(second, SnapshotOutcome::Unchanged(ref s) if s.path == first.snapshot().path)
        );
        assert_eq!(list(&root, "st_1").unwrap().len(), 1);

        let changed = write_save(&account, "save.hg", b"different", stamp(120));
        let third = snapshot(&changed, &root, None).unwrap();
        assert!(matches!(third, SnapshotOutcome::Taken(_)));
        assert_eq!(list(&root, "st_1").unwrap().len(), 2);
    }

    #[test]
    fn test_snapshot_same_folder_name_is_unchanged() {
        let (_dir, account, root) = fixture();
        let save = write_save(&account, "save.hg", b"x", stamp(0));
        let first = snapshot(&save, &root, Some("tag")).unwrap();
        let second = snapshot(&save, &root, Some("tag")).unwrap();
        assert!(
            matches!(second, SnapshotOutcome::Unchanged(ref s) if s.path == first.snapshot().path)
        );
    }

    #[test]
    fn test_labelled_snapshot_keeps_its_folder_even_when_identical() {
        let (_dir, account, root) = fixture();
        let save = write_save(&account, "save.hg", b"same", stamp(0));
        snapshot(&save, &root, None).unwrap();
        let labelled = snapshot(&save, &root, Some("before-call")).unwrap();
        assert!(
            matches!(labelled, SnapshotOutcome::Taken(ref s) if s.label.as_deref() == Some("before-call"))
        );
        assert_eq!(list(&root, "st_1").unwrap().len(), 2);
    }

    #[test]
    fn test_list_is_newest_first_and_ignores_other_folders() {
        let (_dir, account, root) = fixture();
        let older = write_save(&account, "save.hg", b"a", stamp(0));
        snapshot(&older, &root, None).unwrap();
        let newer = write_save(&account, "save2.hg", b"bb", stamp(3600));
        snapshot(&newer, &root, None).unwrap();
        fs::create_dir_all(root.join("st_1").join("scratch")).unwrap();
        fs::write(root.join("st_1").join("README.txt"), b"hi").unwrap();

        let listed = list(&root, "st_1").unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].save_type, SaveType::Auto);
        assert_eq!(listed[0].size, 2);
        assert_eq!(listed[1].save_type, SaveType::Manual);
        assert_eq!(list_accounts(&root).unwrap(), vec!["st_1".to_string()]);
    }

    #[test]
    fn test_list_missing_root_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            list(&dir.path().join("nowhere"), "st_1")
                .unwrap()
                .is_empty()
        );
        assert!(
            list_accounts(&dir.path().join("nowhere"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_prune_keeps_newest_per_slot_and_every_labelled() {
        let (_dir, account, root) = fixture();
        for i in 0..4u64 {
            let save = write_save(
                &account,
                "save.hg",
                format!("slot1-{i}").as_bytes(),
                stamp(i * 60),
            );
            snapshot(&save, &root, None).unwrap();
        }
        let labelled = write_save(&account, "save.hg", b"labelled", stamp(1000));
        snapshot(&labelled, &root, Some("keep-me")).unwrap();
        for i in 0..3u64 {
            let save = write_save(
                &account,
                "save3.hg",
                format!("slot2-{i}").as_bytes(),
                stamp(i * 60),
            );
            snapshot(&save, &root, None).unwrap();
        }
        assert_eq!(list(&root, "st_1").unwrap().len(), 8);

        let removed = prune(&root, "st_1", 2).unwrap();
        assert_eq!(removed.len(), 3);
        assert!(removed.iter().all(|s| !s.is_labelled()));
        let kept = list(&root, "st_1").unwrap();
        assert_eq!(kept.len(), 5);
        assert_eq!(
            kept.iter()
                .filter(|s| s.slot == 1 && !s.is_labelled())
                .count(),
            2
        );
        assert_eq!(kept.iter().filter(|s| s.slot == 2).count(), 2);
        assert!(kept.iter().any(|s| s.label.as_deref() == Some("keep-me")));
        let newest_slot1 = kept
            .iter()
            .find(|s| s.slot == 1 && !s.is_labelled())
            .unwrap();
        assert_eq!(
            fs::read(newest_slot1.path.join("save.hg")).unwrap(),
            b"slot1-3"
        );
    }

    #[test]
    fn test_prune_zero_keeps_everything() {
        let (_dir, account, root) = fixture();
        for i in 0..3u64 {
            let save = write_save(
                &account,
                "save.hg",
                format!("{i}").as_bytes(),
                stamp(i * 60),
            );
            snapshot(&save, &root, None).unwrap();
        }
        assert!(prune(&root, "st_1", 0).unwrap().is_empty());
        assert_eq!(list(&root, "st_1").unwrap().len(), 3);
    }

    #[test]
    fn test_policy_auto_snapshot_prunes_after_taking() {
        let (_dir, account, root) = fixture();
        let policy = BackupPolicy {
            root: root.clone(),
            keep: 1,
        };
        for i in 0..3u64 {
            let save = write_save(
                &account,
                "save.hg",
                format!("{i}").as_bytes(),
                stamp(i * 60),
            );
            let outcome = policy.auto_snapshot(&save).unwrap();
            assert!(matches!(outcome, SnapshotOutcome::Taken(_)));
        }
        let kept = list(&root, "st_1").unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(fs::read(kept[0].path.join("save.hg")).unwrap(), b"2");
    }

    #[test]
    fn test_account_name_from_path() {
        assert_eq!(
            account_name(Path::new("/saves/st_42/save.hg")).unwrap(),
            "st_42"
        );
        assert!(account_name(Path::new("save.hg")).is_err());
    }
}
