use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod backup;
mod base;
mod completions;
mod convert;
mod export;
mod find;
mod fleet;
mod import;
mod info;
mod inventory;
mod list;
mod raw;
mod route;
mod saves;
mod show;
mod stats;

#[derive(Parser)]
#[command(
    name = "nms",
    about = "NMS Copilot CLI -- search planets, plan routes, convert glyphs",
    version
)]
pub(crate) struct Cli {
    /// Use a specific save slot (1-15) instead of most recent.
    #[arg(long, global = true)]
    slot: Option<u8>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display save file summary.
    Info {
        /// Path to a decompressed/deobfuscated save file (JSON).
        /// If omitted, auto-detects the most recent save.
        #[arg(long)]
        save: Option<PathBuf>,
    },

    /// Search planets by biome, distance, name.
    Find {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Filter by biome (e.g., Lush, Toxic, Scorched).
        #[arg(long)]
        biome: Option<String>,

        /// Only show infested planets.
        #[arg(long)]
        infested: bool,

        /// Only within this radius in light-years.
        #[arg(long)]
        within: Option<f64>,

        /// Show only the N nearest results.
        #[arg(long)]
        nearest: Option<usize>,

        /// Only show named planets/systems.
        #[arg(long)]
        named: bool,

        /// Filter by discoverer username (substring match).
        #[arg(long)]
        discoverer: Option<String>,

        /// Distance from this base name (default: current position).
        #[arg(long)]
        from: Option<String>,
    },

    /// Show detailed information about a system or base.
    Show {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        #[command(subcommand)]
        target: ShowTargetCmd,
    },

    /// Show crops, extraction networks, and power at your bases.
    Base {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Base name for the full view (exact, then substring match). Omit for a one-row-per-base overview.
        name: Option<String>,

        /// Layout width in columns for the full view (default: terminal width; sections stack when output is piped).
        #[arg(long)]
        width: Option<usize>,
    },

    /// Show frigate expeditions, the Navigator's offers, and the fleet.
    Fleet {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// An expedition number for the full view, or "frigates" for every frigate. Omit for the overview.
        target: Option<String>,
    },

    /// Do I have an item, how much, and where? Matches item names and IDs.
    Have {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Name or ID to look for (substring, case-insensitive), e.g. gold or ASTEROID2.
        #[arg(required = true, num_args = 1..)]
        pattern: Vec<String>,

        /// Only items of this type: substance, product, or technology.
        #[arg(long = "type")]
        kind: Option<String>,
    },

    /// Every container and how full it is, or one container's contents.
    Inventory {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// A container to show in full ("storage 3", exosuit, freighter, "ship 1") or a word its label contains (ship, storage).
        container: Vec<String>,

        /// Order by free slots, most first, and leave out full containers.
        #[arg(long)]
        free: bool,
    },

    /// Display aggregate galaxy statistics.
    Stats {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Show biome distribution table.
        #[arg(long)]
        biomes: bool,

        /// Show discovery counts by type.
        #[arg(long)]
        discoveries: bool,
    },

    /// Plan a route through discovered systems.
    Route {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Filter targets by biome (e.g., Lush, Toxic).
        #[arg(long)]
        biome: Option<String>,

        /// Named targets (bases or systems) to visit.
        #[arg(long = "target", num_args = 1)]
        targets: Vec<String>,

        /// Start from this base name (default: current position).
        #[arg(long)]
        from: Option<String>,

        /// Ship warp range in light-years (for hop constraints).
        #[arg(long)]
        warp_range: Option<f64>,

        /// Only consider targets within this radius in light-years.
        #[arg(long)]
        within: Option<f64>,

        /// Maximum number of targets to visit.
        #[arg(long)]
        max_targets: Option<usize>,

        /// Routing algorithm: nn, nearest-neighbor, 2opt, two-opt.
        #[arg(long)]
        algo: Option<String>,

        /// Return to starting system at the end.
        #[arg(long)]
        round_trip: bool,
    },

    /// List reference data or model collections.
    List {
        /// Path to save file (required for bases/systems, ignored for reference data).
        #[arg(long)]
        save: Option<PathBuf>,

        #[command(subcommand)]
        target: ListTargetCmd,
    },

    /// Convert between NMS coordinate formats.
    Convert {
        /// Portal glyphs as 12 hex digits (e.g., 01717D8A4EA2).
        #[arg(long, group = "input")]
        glyphs: Option<String>,

        /// Signal booster coordinates (XXXX:YYYY:ZZZZ:SSSS).
        #[arg(long, group = "input")]
        coords: Option<String>,

        /// Galactic address as hex (0x01717D8A4EA2).
        #[arg(long, group = "input")]
        ga: Option<String>,

        /// Voxel position as X,Y,Z (requires --ssi).
        #[arg(long, group = "input")]
        voxel: Option<String>,

        /// Solar system index (required with --voxel).
        #[arg(long)]
        ssi: Option<u16>,

        /// Planet index (0-15, defaults to 0).
        #[arg(long, default_value = "0")]
        planet: u8,

        /// Galaxy index (0-255) or name (e.g., "Euclid").
        #[arg(long, default_value = "0")]
        galaxy: String,
    },

    /// Export filtered planets as JSON or CSV.
    Export {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Filter by biome (e.g., Lush, Toxic, Scorched).
        #[arg(long)]
        biome: Option<String>,

        /// Only show infested planets.
        #[arg(long)]
        infested: bool,

        /// Only within this radius in light-years.
        #[arg(long)]
        within: Option<f64>,

        /// Show only the N nearest results.
        #[arg(long)]
        nearest: Option<usize>,

        /// Only show named planets/systems.
        #[arg(long)]
        named: bool,

        /// Filter by discoverer username (substring match).
        #[arg(long)]
        discoverer: Option<String>,

        /// Distance from this base name (default: current position).
        #[arg(long)]
        from: Option<String>,

        /// Output format: json, csv (default: json).
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Import community coordinate data.
    Import {
        /// Path to CSV file.
        file: PathBuf,
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,
        /// Source name for imported data (e.g., "NMSCE").
        #[arg(long, default_value = "community")]
        source: String,
    },

    /// Print any part of the decoded save as JSON (a probe for save research).
    Raw {
        /// Path to save file (auto-detects if omitted).
        #[arg(long)]
        save: Option<PathBuf>,

        /// Dotted path from the save root, e.g. BaseContext.PlayerStateData.FleetExpeditions[0].Events (omit for the root).
        path: Option<String>,

        /// Levels of nesting to print below the target (0 = unlimited).
        #[arg(long, default_value = "3")]
        depth: usize,

        /// Array items to print per array (0 = unlimited).
        #[arg(long, default_value = "10")]
        limit: usize,

        /// List the keys at the target with their types and sizes instead of printing it.
        #[arg(long)]
        keys: bool,

        /// Search key names below the target for this text (case-insensitive) and print where they occur.
        #[arg(long)]
        find: Option<String>,
    },

    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for: bash, zsh, fish, powershell, elvish.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// List all save slots.
    Saves,

    /// Copy save files into dated backup folders, list what is kept, or prune old snapshots.
    Backup {
        /// Snapshot every file of every slot instead of the most recent save.
        #[arg(long)]
        all: bool,

        /// Label for the snapshot folder; labelled snapshots are never pruned.
        #[arg(long)]
        label: Option<String>,

        /// Backup folder (default: ~/.nms-copilot/backups).
        #[arg(long, global = true, value_name = "DIR")]
        to: Option<PathBuf>,

        #[command(subcommand)]
        action: Option<BackupCmd>,
    },
}

#[derive(Subcommand)]
enum BackupCmd {
    /// List kept snapshots, newest first.
    List,

    /// Delete all but the newest N unlabelled snapshots of each slot.
    Prune {
        /// Snapshots to keep per slot; 0 keeps everything.
        #[arg(long, default_value = "20")]
        keep: usize,
    },
}

#[derive(Subcommand)]
enum ShowTargetCmd {
    /// Show system details.
    System {
        /// System name or hex address.
        name: String,
    },
    /// Show base details.
    Base {
        /// Base name (case-insensitive).
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ListTargetCmd {
    /// List all 256 galaxies.
    Galaxies {
        /// Filter by galaxy type (Normal, Lush, Harsh, Empty).
        #[arg(long = "type")]
        galaxy_type: Option<String>,
    },
    /// List biome types and their variants.
    Biomes,
    /// List portal glyphs.
    Glyphs,
    /// List player bases.
    Bases {
        /// Maximum number of bases to display (0 for all).
        #[arg(long, default_value = "0")]
        limit: usize,

        /// Show all bases (equivalent to --limit 0).
        #[arg(long)]
        all: bool,
    },
    /// List discovered systems.
    Systems {
        /// Maximum number of systems to display (0 for all).
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Show all systems (equivalent to --limit 0).
        #[arg(long)]
        all: bool,
    },
    /// List terrain generation types (GcBiomeSubType).
    #[command(name = "terrain-types")]
    TerrainTypes,
    /// List every item you hold with its total across all containers.
    Items {
        /// Only items of this type: substance, product, or technology.
        #[arg(long = "type")]
        kind: Option<String>,

        /// Only items with at least this many.
        #[arg(long, default_value = "0")]
        min: u32,

        /// Only items whose name or ID contains this.
        pattern: Option<String>,
    },
    /// List owned ships with type, class, slots, and bonuses.
    Ships,
    /// List owned exocraft and where each is parked.
    Exocraft,
    /// List owned multi-tools with class, slots, and bonuses.
    Multitools,
}

/// Resolve a save file path from --save, --slot, or auto-detect.
fn resolve_save(save: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = save {
        return Ok(path);
    }
    Ok(nms_save::locate::find_most_recent_save()?
        .path()
        .to_path_buf())
}

/// Resolve a save file path, checking --slot from the global CLI arg.
fn resolve_save_with_slot(
    save: Option<PathBuf>,
    slot: Option<u8>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = save {
        return Ok(path);
    }
    if let Some(slot_num) = slot {
        let save_dir = nms_save::locate::nms_save_dir_checked()?;
        let accounts = nms_save::locate::list_accounts(&save_dir)?;
        let saves = nms_save::locate::list_saves(accounts[0].path())?;
        let slots = nms_save::locate::group_into_slots(&saves);
        let target = slots
            .iter()
            .find(|s| s.slot() == slot_num)
            .ok_or_else(|| format!("save slot {slot_num} not found"))?;
        let file = target
            .most_recent()
            .ok_or_else(|| format!("save slot {slot_num} is empty"))?;
        return Ok(file.path().to_path_buf());
    }
    Ok(nms_save::locate::find_most_recent_save()?
        .path()
        .to_path_buf())
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let slot = cli.slot;

    match cli.command {
        Commands::Info { save } => {
            let path = resolve_save_with_slot(save, slot)?;
            info::run(Some(path))
        }
        Commands::Find {
            save,
            biome,
            infested,
            within,
            nearest,
            named,
            discoverer,
            from,
        } => {
            let path = resolve_save_with_slot(save, slot)?;
            find::run(find::FindArgs {
                save: Some(path),
                biome,
                infested,
                within,
                nearest,
                named,
                discoverer,
                from,
            })
        }
        Commands::Show { save, target } => {
            let path = resolve_save_with_slot(save, slot)?;
            let target = match target {
                ShowTargetCmd::System { name } => show::ShowTarget::System { name },
                ShowTargetCmd::Base { name } => show::ShowTarget::Base { name },
            };
            show::run(Some(path), target)
        }
        Commands::Base { save, name, width } => {
            let path = resolve_save_with_slot(save, slot)?;
            base::run(Some(path), name, width)
        }
        Commands::Fleet { save, target } => {
            let path = resolve_save_with_slot(save, slot)?;
            fleet::run(Some(path), target)
        }
        Commands::Have {
            save,
            pattern,
            kind,
        } => {
            let path = resolve_save_with_slot(save, slot)?;
            inventory::run_have(Some(path), pattern, kind)
        }
        Commands::Inventory {
            save,
            container,
            free,
        } => {
            let path = resolve_save_with_slot(save, slot)?;
            inventory::run_inventory(Some(path), container, free)
        }
        Commands::Stats {
            save,
            biomes,
            discoveries,
        } => {
            let path = resolve_save_with_slot(save, slot)?;
            stats::run(Some(path), biomes, discoveries)
        }
        Commands::Route {
            save,
            biome,
            targets,
            from,
            warp_range,
            within,
            max_targets,
            algo,
            round_trip,
        } => {
            let path = resolve_save_with_slot(save, slot)?;
            route::run(route::RouteArgs {
                save: Some(path),
                biome,
                targets,
                from,
                warp_range,
                within,
                max_targets,
                algo,
                round_trip,
            })
        }
        Commands::List { save, target } => list::run(target, save, slot),
        Commands::Convert {
            glyphs,
            coords,
            ga,
            voxel,
            ssi,
            planet,
            galaxy,
        } => convert::run(glyphs, coords, ga, voxel, ssi, planet, galaxy),
        Commands::Export {
            save,
            biome,
            infested,
            within,
            nearest,
            named,
            discoverer,
            from,
            format,
        } => {
            let path = resolve_save_with_slot(save, slot)?;
            export::run(export::ExportArgs {
                save: Some(path),
                biome,
                infested,
                within,
                nearest,
                named,
                discoverer,
                from,
                format,
            })
        }
        Commands::Import { file, save, source } => import::run(file, save, source),
        Commands::Raw {
            save,
            path,
            depth,
            limit,
            keys,
            find,
        } => {
            let save = resolve_save_with_slot(save, slot)?;
            raw::run(raw::RawArgs {
                save: Some(save),
                path,
                depth,
                limit,
                keys,
                find,
            })
        }
        Commands::Completions { shell } => completions::run(shell),
        Commands::Saves => saves::run(),
        Commands::Backup {
            all,
            label,
            to,
            action,
        } => match action {
            None => {
                let target = if all {
                    backup::Target::All
                } else if let Some(n) = slot {
                    backup::Target::Slot(n)
                } else {
                    backup::Target::MostRecent
                };
                backup::run_snapshot(target, label.as_deref(), to)
            }
            Some(BackupCmd::List) => backup::run_list(slot, to),
            Some(BackupCmd::Prune { keep }) => backup::run_prune(keep, to),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_save_explicit_path() {
        let path = PathBuf::from("/tmp/save.hg");
        let result = resolve_save(Some(path.clone())).unwrap();
        assert_eq!(result, path);
    }

    #[test]
    fn test_resolve_save_with_slot_explicit_path_takes_priority() {
        let path = PathBuf::from("/tmp/save.hg");
        let result = resolve_save_with_slot(Some(path.clone()), Some(2)).unwrap();
        assert_eq!(result, path);
    }
}
