//! `nms show` command -- one system in detail.

use std::path::PathBuf;

use nms_query::display::format_show_system;
use nms_query::show::show_system;
use nms_query::theme::{Theme, should_use_colors};

pub fn run(save: Option<PathBuf>, name: String) -> Result<(), Box<dyn std::error::Error>> {
    let path = match save {
        Some(p) => p,
        None => nms_save::locate::find_most_recent_save()?
            .path()
            .to_path_buf(),
    };
    let save = nms_save::parse_save_file(&path)?;
    let mut model = crate::build_model(&save);
    // The player's own system may be undiscovered; it is still worth showing.
    model.ensure_player_system();

    let result = show_system(&model, &name)?;
    let theme = if should_use_colors(true) {
        Theme::default_dark()
    } else {
        Theme::none()
    };
    print!("{}", format_show_system(&result, &theme));

    Ok(())
}
