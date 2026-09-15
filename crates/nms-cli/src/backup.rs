//! `nms backup` command -- copy save files into dated backup folders, list what is kept, prune old snapshots.

use std::path::PathBuf;

use nms_query::display::format_backup_list;
use nms_query::table::{TableStyleConfig, nms_theme, nms_theme_no_color};
use nms_query::theme::should_use_colors;
use nms_save::backup::{self, SnapshotOutcome};
use nms_save::locate::{
    AccountDir, SaveFile, find_most_recent_save, group_into_slots, list_accounts, list_saves,
    nms_save_dir_checked,
};

/// Which save files to snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The newest save file across accounts.
    MostRecent,
    /// The newest file of one slot in the first account.
    Slot(u8),
    /// Every file of every slot in the first account.
    All,
}

type CliResult = Result<(), Box<dyn std::error::Error>>;

fn root_or_default(to: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(match to {
        Some(dir) => dir,
        None => backup::default_root()?,
    })
}

fn theme() -> TableStyleConfig {
    if should_use_colors(true) {
        nms_theme()
    } else {
        nms_theme_no_color()
    }
}

fn first_account() -> Result<AccountDir, Box<dyn std::error::Error>> {
    let save_dir = nms_save_dir_checked()?;
    let mut accounts = list_accounts(&save_dir)?;
    if accounts.is_empty() {
        return Err(format!("no account folders in {}", save_dir.display()).into());
    }
    Ok(accounts.remove(0))
}

fn files_for(target: Target) -> Result<Vec<SaveFile>, Box<dyn std::error::Error>> {
    match target {
        Target::MostRecent => Ok(vec![find_most_recent_save()?]),
        Target::Slot(n) => {
            let account = first_account()?;
            let saves = list_saves(account.path())?;
            let slots = group_into_slots(&saves);
            let slot = slots
                .iter()
                .find(|s| s.slot() == n)
                .ok_or_else(|| format!("save slot {n} not found"))?;
            let file = slot
                .most_recent()
                .cloned()
                .ok_or_else(|| format!("save slot {n} is empty"))?;
            Ok(vec![file])
        }
        Target::All => {
            let account = first_account()?;
            Ok(list_saves(account.path())?)
        }
    }
}

/// `nms backup [--all] [--label L] [--to DIR]`: snapshot now and print where each copy went.
pub fn run_snapshot(target: Target, label: Option<&str>, to: Option<PathBuf>) -> CliResult {
    let root = root_or_default(to)?;
    for file in files_for(target)? {
        let name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("save");
        match backup::snapshot(&file, &root, label)? {
            SnapshotOutcome::Taken(s) => {
                println!(
                    "Saved {name} (slot {}, {}) to {}",
                    file.slot(),
                    file.save_type(),
                    s.path.display()
                );
            }
            SnapshotOutcome::Unchanged(s) => {
                println!("{name} unchanged since {}; nothing copied", s.folder_name());
            }
        }
    }
    Ok(())
}

/// `nms backup list [--slot N] [--to DIR]`: one table per account, newest first.
pub fn run_list(slot: Option<u8>, to: Option<PathBuf>) -> CliResult {
    let root = root_or_default(to)?;
    let accounts = backup::list_accounts(&root)?;
    if accounts.is_empty() {
        println!("No backups in {}", root.display());
        return Ok(());
    }
    let theme = theme();
    for account in &accounts {
        let mut snapshots = backup::list(&root, account)?;
        if let Some(n) = slot {
            snapshots.retain(|s| s.slot == n);
        }
        print!("{}", format_backup_list(account, &snapshots, &theme));
    }
    Ok(())
}

/// `nms backup prune [--keep N] [--to DIR]`: apply the retention rule to every account.
pub fn run_prune(keep: usize, to: Option<PathBuf>) -> CliResult {
    let root = root_or_default(to)?;
    let accounts = backup::list_accounts(&root)?;
    if accounts.is_empty() {
        println!("No backups in {}", root.display());
        return Ok(());
    }
    for account in &accounts {
        let removed = backup::prune(&root, account, keep)?;
        println!(
            "{account}: removed {} snapshot{}, keeping the newest {keep} unlabelled per slot",
            removed.len(),
            if removed.len() == 1 { "" } else { "s" }
        );
    }
    Ok(())
}
