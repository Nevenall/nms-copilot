//! `list saves`: every account's save slots, with which files each holds and when it was last written.

use std::time::SystemTime;

use nms_save::locate::{
    AccountDir, SaveSlot, group_into_slots, list_accounts, list_saves, nms_save_dir_checked,
};

use crate::display::format_ago;
use crate::table::{Builder, TableStyleConfig, build_table};

/// The slot table for every account under the game's save folder.
pub fn format_save_slots(theme: &TableStyleConfig) -> Result<String, Box<dyn std::error::Error>> {
    let save_dir = nms_save_dir_checked()?;
    let accounts = list_accounts(&save_dir)?;
    let mut out = String::new();
    for account in &accounts {
        out.push_str(&format_account_slots(account, SystemTime::now(), theme));
    }
    Ok(out)
}

/// One account's heading and slot table, or a line saying it has no saves.
pub fn format_account_slots(
    account: &AccountDir,
    now: SystemTime,
    theme: &TableStyleConfig,
) -> String {
    let mut out = format!("Account: {} ({})\n", account.name(), account.kind());
    let slots = match list_saves(account.path()) {
        Ok(saves) => group_into_slots(&saves),
        Err(_) => Vec::new(),
    };
    if slots.is_empty() {
        out.push_str("  No save slots found.\n\n");
        return out;
    }
    out.push_str(&format_slots(&slots, now, theme));
    out
}

/// The slot table itself.
pub fn format_slots(slots: &[SaveSlot], now: SystemTime, theme: &TableStyleConfig) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Number", "Manual", "Auto", "Most Recent"]);
    for slot in slots {
        let manual = if slot.manual().is_some() { "yes" } else { "-" };
        let auto = if slot.auto().is_some() { "yes" } else { "-" };
        let recent = slot
            .most_recent()
            .map(|s| format!("{} ({})", s.save_type(), written_ago(s.modified(), now)))
            .unwrap_or_default();
        builder.push_record([
            slot.slot().to_string(),
            manual.to_string(),
            auto.to_string(),
            recent,
        ]);
    }
    builder.push_record(["", "", "", ""]);
    build_table(builder, &["SAVE", "SLOTS"], theme, "Slots")
}

fn written_ago(modified: SystemTime, now: SystemTime) -> String {
    match now.duration_since(modified) {
        Ok(d) => format_ago(d.as_secs() as i64),
        Err(_) => "just now".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::nms_theme_no_color;
    use nms_save::locate::SaveFile;
    use std::time::Duration;

    fn slots_in(dir: &std::path::Path, names: &[&str]) -> Vec<SaveSlot> {
        let files: Vec<SaveFile> = names
            .iter()
            .map(|name| {
                let path = dir.join(name);
                std::fs::write(&path, b"x").unwrap();
                SaveFile::from_path(&path).unwrap()
            })
            .collect();
        group_into_slots(&files)
    }

    #[test]
    fn test_format_slots_one_row_per_slot_with_files_and_age() {
        let dir = tempfile::tempdir().unwrap();
        let slots = slots_in(dir.path(), &["save.hg", "save2.hg", "save5.hg"]);
        let now = SystemTime::now() + Duration::from_secs(2 * 3600);
        let out = format_slots(&slots, now, &nms_theme_no_color());
        let rows: Vec<&str> = out
            .lines()
            .filter(|l| l.contains("yes") || l.contains(" - "))
            .collect();
        assert_eq!(rows.len(), 2, "{out}");
        assert!(
            rows[0].contains("1") && rows[0].contains("yes") && rows[0].matches("yes").count() == 2,
            "{}",
            rows[0]
        );
        assert!(
            rows[1].contains("3") && rows[1].contains("Manual") && rows[1].contains("2h 00m ago"),
            "{}",
            rows[1]
        );
    }

    #[test]
    fn test_written_ago_in_the_future_is_just_now() {
        let now = SystemTime::now();
        assert_eq!(written_ago(now + Duration::from_secs(60), now), "just now");
        assert_eq!(written_ago(now - Duration::from_secs(30), now), "just now");
    }
}
