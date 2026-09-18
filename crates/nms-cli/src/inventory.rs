//! `nms have` and `nms inventory` -- what the player owns and where it is.

use std::path::PathBuf;

use nms_core::ItemKind;
use nms_graph::GalaxyModel;
use nms_query::display::{format_have, format_inventory};
use nms_query::inventory::{HaveQuery, InventoryQuery, execute_have, execute_inventory};
use nms_query::theme::{Theme, should_use_colors};

fn load(save: Option<PathBuf>) -> Result<GalaxyModel, Box<dyn std::error::Error>> {
    let path = match save {
        Some(p) => p,
        None => nms_save::locate::find_most_recent_save()?
            .path()
            .to_path_buf(),
    };
    let save = nms_save::parse_save_file(&path)?;
    Ok(GalaxyModel::from_save(&save))
}

fn theme() -> Theme {
    if should_use_colors(true) {
        Theme::default_dark()
    } else {
        Theme::none()
    }
}

/// Parse the `--type` word, or fail with the accepted spellings.
pub fn parse_kind(kind: Option<String>) -> Result<Option<ItemKind>, String> {
    match kind {
        None => Ok(None),
        Some(word) => ItemKind::parse(&word).map(Some).ok_or_else(|| {
            format!("unknown item type \"{word}\": use substance, product, or technology")
        }),
    }
}

pub fn run_have(
    save: Option<PathBuf>,
    pattern: Vec<String>,
    kind: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pattern = pattern.join(" ");
    let query = HaveQuery {
        pattern: pattern.clone(),
        kind: parse_kind(kind)?,
    };
    let model = load(save)?;
    let results = execute_have(&model, &query)?;
    print!("{}", format_have(&results, &pattern, &theme()));
    Ok(())
}

pub fn run_inventory(
    save: Option<PathBuf>,
    container: Vec<String>,
    free: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let container = (!container.is_empty()).then(|| container.join(" "));
    let query = InventoryQuery {
        container,
        free_only: free,
    };
    let model = load(save)?;
    let result = execute_inventory(&model, &query)?;
    print!("{}", format_inventory(&result, &theme()));
    Ok(())
}
