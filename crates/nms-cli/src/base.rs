//! `nms base` command -- crops, extraction networks, and power at your bases.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nms_graph::GalaxyModel;
use nms_query::base::{BaseQuery, execute_base};
use nms_query::display::{format_base_detail, format_base_overview};
use nms_query::layout::terminal_width;
use nms_query::theme::{Theme, should_use_colors};

/// Current Unix time in seconds.
pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn run(
    save: Option<PathBuf>,
    name: Option<String>,
    width: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = match save {
        Some(p) => p,
        None => nms_save::locate::find_most_recent_save()?
            .path()
            .to_path_buf(),
    };
    let save = nms_save::parse_save_file(&path)?;
    let model = GalaxyModel::from_save(&save);

    let now = unix_now();
    let theme = if should_use_colors(true) {
        Theme::default_dark()
    } else {
        Theme::none()
    };
    let statuses = execute_base(&model, &BaseQuery { name: name.clone() }, now)?;

    if name.is_some() {
        let width = width.or_else(terminal_width);
        for (i, status) in statuses.iter().enumerate() {
            if i > 0 {
                println!();
            }
            print!("{}", format_base_detail(status, now, &theme, width));
        }
    } else {
        print!("{}", format_base_overview(&statuses, &theme));
    }
    Ok(())
}
