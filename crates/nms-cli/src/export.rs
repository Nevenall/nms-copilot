//! `nms export` command -- export filtered planets as JSON or CSV.

use std::io::{self, Write};
use std::path::PathBuf;

use nms_core::biome::Biome;
use nms_query::export::{ExportFormat, render_export};
use nms_query::find::{FindQuery, FindSort, ReferencePoint, execute_find};

/// Arguments for the export command.
pub struct ExportArgs {
    pub save: Option<PathBuf>,
    pub biome: Option<String>,
    pub infested: bool,
    pub within: Option<f64>,
    pub nearest: Option<usize>,
    pub named: bool,
    pub discoverer: Option<String>,
    pub from: Option<String>,
    pub sort: String,
    pub format: String,
    /// Write here instead of stdout.
    pub to: Option<PathBuf>,
}

pub fn run(args: ExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    let format = ExportFormat::parse(&args.format)?;

    let path = crate::resolve_save(args.save)?;
    let save = nms_save::parse_save_file(&path)?;
    let model = crate::build_model(&save);

    let biome = args
        .biome
        .map(|s| s.parse::<Biome>())
        .transpose()
        .map_err(|e| format!("Invalid biome: {e}"))?;

    let reference = match args.from {
        Some(name) => ReferencePoint::Base(name),
        None => ReferencePoint::CurrentPosition,
    };

    let query = FindQuery {
        biome,
        biome_subtype: None,
        infested: if args.infested { Some(true) } else { None },
        within_ly: args.within,
        nearest: args.nearest,
        name_pattern: None,
        discoverer: args.discoverer,
        named_only: args.named,
        from: reference,
        sort: FindSort::parse(&args.sort)?,
    };

    let results = execute_find(&model, &query)?;
    let text = render_export(&results, format)?;

    match args.to {
        Some(file) => {
            std::fs::write(&file, text)?;
            eprintln!("Wrote {} planets to {}", results.len(), file.display());
        }
        None => {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            out.write_all(text.as_bytes())?;
        }
    }
    Ok(())
}
