//! `nms fleet` command -- frigate expeditions, the Navigator's offers, and the fleet.

use std::path::PathBuf;

use nms_graph::GalaxyModel;
use nms_query::display::format_fleet;
use nms_query::fleet::{FleetTarget, execute_fleet};
use nms_query::theme::{Theme, should_use_colors};

use crate::base::unix_now;

pub fn run(
    save: Option<PathBuf>,
    target: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let target = FleetTarget::parse(target.as_deref())?;
    let path = match save {
        Some(p) => p,
        None => nms_save::locate::find_most_recent_save()?
            .path()
            .to_path_buf(),
    };
    let save = nms_save::parse_save_file(&path)?;
    let model = GalaxyModel::from_save(&save);

    let theme = if should_use_colors(true) {
        Theme::default_dark()
    } else {
        Theme::none()
    };
    let status = execute_fleet(&model, unix_now())?;
    print!("{}", format_fleet(&status, target, &theme)?);
    Ok(())
}
