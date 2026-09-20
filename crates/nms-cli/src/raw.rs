//! `nms raw` command -- print any part of the decoded save as JSON.

use std::path::PathBuf;

use nms_query::raw::{RawQuery, format_raw};

/// Arguments for `nms raw`.
pub struct RawArgs {
    pub save: Option<PathBuf>,
    /// Dotted path from the save root, such as `BaseContext.PlayerStateData.FleetExpeditions[0].Events`. Empty means the root.
    pub path: Option<String>,
    /// Levels of nesting to print below the target; 0 means unlimited.
    pub depth: usize,
    /// Array items to print per array; 0 means unlimited.
    pub limit: usize,
    /// List the keys at the target instead of printing it.
    pub keys: bool,
    /// Search key names below the target for this text (case-insensitive).
    pub find: Option<String>,
}

pub fn run(args: RawArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = match args.save {
        Some(p) => p,
        None => nms_save::locate::find_most_recent_save()?
            .path()
            .to_path_buf(),
    };
    let root = nms_save::read_save_json(&path)?;
    let query = RawQuery {
        path: args.path,
        depth: args.depth,
        limit: args.limit,
        keys: args.keys,
        find: args.find,
    };
    println!("{}", format_raw(&root, &query)?);
    Ok(())
}
