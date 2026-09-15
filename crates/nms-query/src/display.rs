//! Output formatting for query results.
//!
//! All formatters return `String` -- the caller prints to stdout.
//! Each formatter accepts a [`Theme`] for API compatibility but uses
//! `nms_theme()` / `nms_theme_no_color()` for table styling internally.

use crate::base::{Alert, AlertKind, AlertSummary, BaseStatus, CropRow, PowerSummary};
use crate::find::FindResult;
use crate::fleet::{ExpeditionRow, FleetStatus, FleetTarget};
use crate::layout::side_by_side;
use crate::route::RouteResult;
use crate::show::{ShowBaseResult, ShowResult, ShowSystemResult};
use crate::stats::StatsResult;
use crate::table::{
    Builder, TableStyleConfig, build_grouped_table, build_table, nms_theme, nms_theme_no_color,
};
use crate::theme::Theme;
use nms_core::fleet::ExpeditionState;
use nms_save::backup::Snapshot;
use nms_save::locate::SaveType;

/// Format a distance in light-years for display.
///
/// - 0.0 (same system): `"< 1 ly"`
/// - ~265 ly (same voxel): `"~265 ly (±100)"` — SD from Robbins' variance
/// - Cross-voxel: `"~800 ly (±163)"` — projection SD, √(2/12) × 400
/// - Large distances: `"~40.0K ly"` — ± dropped when < 5% of distance
///
/// All non-zero distances are prefixed with `~` to indicate approximation.
/// The `±` uncertainty (1σ) is shown when it represents ≥ 5% of the distance.
pub fn format_distance(ly: f64) -> String {
    use nms_core::address::{CROSS_VOXEL_SD, SAME_VOXEL_SD, VOXEL_UNCERTAINTY};

    if ly == 0.0 {
        return "< 1 ly".to_string();
    }

    // Same-voxel estimates use their own SD; cross-voxel uses the projection SD.
    let sd = if (ly - VOXEL_UNCERTAINTY).abs() < 1.0 {
        SAME_VOXEL_SD
    } else {
        CROSS_VOXEL_SD
    };
    let show_error = sd / ly >= 0.05; // Drop ± when < 5% of distance

    let dist_str = if ly < 1_000.0 {
        format!("{:.0} ly", ly)
    } else if ly < 1_000_000.0 {
        format!("{:.1}K ly", ly / 1_000.0)
    } else {
        format!("{:.1}M ly", ly / 1_000_000.0)
    };

    if show_error {
        format!("~{dist_str} (\u{00B1}{:.0})", sd)
    } else {
        format!("~{dist_str}")
    }
}

/// Convert a 12-digit hex portal address to emoji string.
///
/// Uses `nms_core::glyph::Glyph::new()` and `emoji()`.
pub fn hex_to_emoji(hex: &str) -> String {
    use nms_core::glyph::Glyph;
    hex.chars()
        .filter_map(|ch| {
            let idx = ch.to_digit(16)? as u8;
            Some(Glyph::new(idx).emoji().to_string())
        })
        .collect()
}

/// Display label for a planet: its name, or `Planet N` from its index when unnamed.
///
/// The index is the planet's fixed slot in its system (the first portal glyph), so
/// the label is stable across players and scans. The save does not record whether a
/// body is a moon, so moons are labelled the same way.
pub fn planet_label(planet: &nms_core::system::Planet) -> String {
    planet
        .name
        .clone()
        .unwrap_or_else(|| format!("Planet {}", planet.index))
}

/// Select the appropriate table theme based on whether the `Theme` has colors.
fn table_theme_for(theme: &Theme) -> crate::table::TableStyleConfig {
    if theme.header.fg.is_some() || theme.header.bold {
        nms_theme()
    } else {
        nms_theme_no_color()
    }
}

/// Format find results as a themed table.
///
/// Planets are grouped by system: the system label and distance are printed on the
/// first row of each run of planets sharing a system, and left blank on the rows
/// that follow, so a block of rows with one label is one system. In the colour
/// theme alternating groups are shaded as bands. Systems without a name are
/// labelled with their portal address (planet digit zero). The Address column is
/// the planet's own portal address, so each row is a usable portal destination.
pub fn format_find_results(results: &[FindResult], theme: &Theme) -> String {
    if results.is_empty() {
        return "  No results found.\n".to_string();
    }

    let table_theme = table_theme_for(theme);
    let mut builder = Builder::default();
    builder.push_record([
        "#",
        "Planet",
        "Biome",
        "System",
        "Distance",
        "Address",
        "Portal Glyphs",
    ]);

    let mut previous_system: Option<&str> = None;
    let mut groups: Vec<usize> = Vec::with_capacity(results.len());
    let mut group = 0usize;
    for (i, r) in results.iter().enumerate() {
        let first_in_group = previous_system != Some(r.system_hex.as_str());
        if first_in_group && previous_system.is_some() {
            group += 1;
        }
        groups.push(group);
        previous_system = Some(r.system_hex.as_str());
        let planet_name = planet_label(&r.planet);
        let biome_str = r
            .planet
            .biome
            .map(|b| {
                let mut s = b.to_string();
                if let Some(sub) = r.planet.biome_subtype {
                    let sub_name = format!("{sub:?}");
                    if let Some(variant) = sub_name.strip_prefix(&s) {
                        s.push_str(&format!(" ({variant})"));
                    }
                }
                if r.planet.infested {
                    s.push('*');
                }
                s
            })
            .unwrap_or_else(|| "?".to_string());
        let system_label = if first_in_group {
            r.system.name.as_deref().unwrap_or(&r.system_hex)
        } else {
            ""
        };
        let distance = if first_in_group {
            format_distance(r.distance_ly)
        } else {
            String::new()
        };
        let address = r.portal_hex.clone();
        let glyphs = hex_to_emoji(&r.portal_hex);

        builder.push_record([
            (i + 1).to_string(),
            truncate(&planet_name, 20),
            truncate(&biome_str, 22),
            truncate(system_label, 22),
            distance,
            address,
            glyphs,
        ]);
    }
    builder.push_record(["", "", "", "", "", "", ""]);

    build_grouped_table(
        builder,
        &["SEARCH RESULTS"],
        &table_theme,
        "Results",
        &groups,
    )
}

/// Format a system detail view.
pub fn format_show_system(result: &ShowSystemResult, theme: &Theme) -> String {
    let table_theme = table_theme_for(theme);
    let sys = &result.system;
    let name = sys.name.as_deref().unwrap_or("-");

    let mut builder = Builder::default();
    builder.push_record(["Property", "Detail"]);
    builder.push_record(["Name", name]);
    builder.push_record(["Galaxy", &result.galaxy_name]);
    builder.push_record(["Portal Glyphs", &hex_to_emoji(&result.portal_hex)]);
    builder.push_record(["Hex Address", &result.portal_hex]);
    builder.push_record(["Discoverer", sys.discoverer.as_deref().unwrap_or("unknown")]);
    if let Some(dist) = result.distance_from_player {
        builder.push_record(["Distance", &format_distance(dist)]);
    }
    builder.push_record([
        "Voxel Position",
        &format!(
            "X={}, Y={}, Z={}",
            sys.address.voxel_x(),
            sys.address.voxel_y(),
            sys.address.voxel_z()
        ),
    ]);
    builder.push_record([
        "System Index",
        &format!(
            "{} (0x{:03X})",
            sys.address.solar_system_index(),
            sys.address.solar_system_index()
        ),
    ]);
    builder.push_record(["", ""]);

    let mut out = String::new();
    out.push_str(&build_table(builder, &["SYSTEM DETAIL"], &table_theme, ""));

    if sys.planets.is_empty() {
        out.push_str("\n  No planets discovered.\n");
    } else {
        out.push('\n');
        let mut pbuilder = Builder::default();
        pbuilder.push_record(["Index", "Name", "Biome", "Flags"]);
        for p in &sys.planets {
            let pname = planet_label(p);
            let biome_str = p
                .biome
                .map(|b| {
                    let mut s = b.to_string();
                    if let Some(sub) = p.biome_subtype {
                        let sub_name = format!("{sub:?}");
                        if let Some(variant) = sub_name.strip_prefix(&s) {
                            s.push_str(&format!(" ({variant})"));
                        }
                    }
                    s
                })
                .unwrap_or_else(|| "?".to_string());
            let flags = if p.infested { "infested" } else { "" };
            pbuilder.push_record([&p.index.to_string(), &pname, &biome_str, flags]);
        }
        pbuilder.push_record(["", "", "", ""]);
        out.push_str(&build_table(
            pbuilder,
            &[&format!("PLANETS ({})", sys.planets.len())],
            &table_theme,
            "Planets",
        ));
    }

    out
}

/// Format a base detail view.
pub fn format_show_base(result: &ShowBaseResult, theme: &Theme) -> String {
    let table_theme = table_theme_for(theme);
    let base = &result.base;

    let mut builder = Builder::default();
    builder.push_record(["Property", "Detail"]);
    builder.push_record(["Name", &base.name]);
    builder.push_record(["Type", &base.base_type.to_string()]);
    builder.push_record(["Galaxy", &result.galaxy_name]);
    builder.push_record(["Portal Glyphs", &hex_to_emoji(&result.portal_hex)]);
    builder.push_record(["Hex Address", &result.portal_hex]);
    if let Some(dist) = result.distance_from_player {
        builder.push_record(["Distance", &format_distance(dist)]);
    }
    if let Some(ref system) = result.system {
        builder.push_record(["System", system.name.as_deref().unwrap_or("-")]);
        builder.push_record(["Planets", &system.planets.len().to_string()]);
    }
    builder.push_record(["", ""]);

    let mut out = String::new();
    out.push_str(&build_table(builder, &["BASE DETAIL"], &table_theme, ""));
    out
}

/// Format a show result (dispatches to system or base).
pub fn format_show_result(result: &ShowResult, theme: &Theme) -> String {
    match result {
        ShowResult::System(s) => format_show_system(s, theme),
        ShowResult::Base(b) => format_show_base(b, theme),
    }
}

/// Format statistics output.
pub fn format_stats(result: &StatsResult, theme: &Theme) -> String {
    let table_theme = table_theme_for(theme);

    let mut builder = Builder::default();
    builder.push_record(["Metric", "Count"]);
    builder.push_record(["Systems", &result.system_count.to_string()]);
    builder.push_record(["Planets", &result.planet_count.to_string()]);
    builder.push_record(["Bases", &result.base_count.to_string()]);
    builder.push_record(["Named Systems", &result.named_system_count.to_string()]);
    builder.push_record(["Named Planets", &result.named_planet_count.to_string()]);
    builder.push_record(["Infested", &result.infested_count.to_string()]);
    builder.push_record(["", ""]);

    let mut out = String::new();
    out.push_str(&build_table(
        builder,
        &["GALAXY STATISTICS"],
        &table_theme,
        "",
    ));

    // Biome distribution table
    if !result.biome_counts.is_empty() || result.unknown_biome_count > 0 {
        out.push('\n');
        let mut bbuilder = Builder::default();
        bbuilder.push_record(["Name", "Count"]);

        let mut biomes: Vec<_> = result.biome_counts.iter().collect();
        biomes.sort_by_key(|item| std::cmp::Reverse(*item.1));

        for (biome, count) in biomes {
            bbuilder.push_record([biome.to_string(), count.to_string()]);
        }

        if result.unknown_biome_count > 0 {
            bbuilder.push_record([
                "(unknown)".to_string(),
                result.unknown_biome_count.to_string(),
            ]);
        }
        bbuilder.push_record(["".to_string(), "".to_string()]);
        out.push_str(&build_table(
            bbuilder,
            &["BIOME DISTRIBUTION"],
            &table_theme,
            "Biomes",
        ));
    }

    out
}

/// Format a route result as an itinerary table.
pub fn format_route(result: &RouteResult, model: &nms_graph::GalaxyModel, theme: &Theme) -> String {
    if result.route.hops.is_empty() {
        return "  No route computed.\n".to_string();
    }

    let table_theme = table_theme_for(theme);
    let mut builder = Builder::default();
    builder.push_record([
        "Hop",
        "System",
        "Distance",
        "Cumulative",
        "Address",
        "Portal Glyphs",
    ]);

    let mut hop_number = 0u32;
    for hop in &result.route.hops {
        let portal_hex = model
            .system(&hop.system_id)
            .map(|s| format!("{:012X}", s.address.packed()))
            .unwrap_or_else(|| format!("{:012X}", hop.system_id.0));

        // Unnamed systems are labelled by address, as in the find table.
        let system_hex = format!("{:012X}", hop.system_id.0);
        let system_name = model
            .system(&hop.system_id)
            .and_then(|s| s.name.as_deref())
            .unwrap_or(&system_hex);
        let glyphs = hex_to_emoji(&portal_hex);

        let distance = format_distance(hop.leg_distance_ly);
        let cumulative = format_distance(hop.cumulative_ly);

        if hop.is_waypoint {
            builder.push_record([
                "*".to_string(),
                format!("\u{21B3} {}", truncate(system_name, 19)),
                distance,
                cumulative,
                portal_hex,
                glyphs,
            ]);
        } else {
            hop_number += 1;
            builder.push_record([
                hop_number.to_string(),
                truncate(system_name, 21),
                distance,
                cumulative,
                portal_hex,
                glyphs,
            ]);
        }
    }
    builder.push_record(["", "", "", "", "", ""]);

    let mut out = build_table(builder, &["ROUTE ITINERARY"], &table_theme, "Hops");

    // Summary line
    out.push('\n');
    let total = format_distance(result.route.total_distance_ly);
    match (result.warp_jumps, result.warp_range) {
        (Some(jumps), Some(range)) => {
            out.push_str(&format!(
                "  Route: {} targets, {} total ({} warp jumps at {} range)\n",
                result.targets_visited,
                total,
                jumps,
                format_distance(range),
            ));
        }
        _ => {
            out.push_str(&format!(
                "  Route: {} targets, {} total\n",
                result.targets_visited, total,
            ));
        }
    }

    let algo_name = match result.algorithm {
        nms_graph::RoutingAlgorithm::NearestNeighbor => "nearest-neighbor",
        nms_graph::RoutingAlgorithm::TwoOpt => "2-opt",
    };
    out.push_str(&format!("  Algorithm: {algo_name}\n"));

    out
}

// ── Base status ─────────────────────────────────────────────────

/// Format a duration in seconds as `1d 03h`, `2h 05m`, `45m`, or `< 1m`.
pub fn format_duration(secs: i64) -> String {
    let secs = secs.max(0);
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours:02}h")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        "< 1m".to_string()
    }
}

/// Format a snapshot instant as local time plus its age: `2026-09-14 09:08 (12m ago)`.
pub fn format_snapshot(snapshot: i64, now: i64) -> String {
    let local = chrono::DateTime::<chrono::Utc>::from_timestamp(snapshot, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "unknown time".to_string());
    format!("{local} ({} ago)", format_duration(now - snapshot))
}

/// Insert thousands separators: `4750` becomes `4,750`.
pub fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Short label for a base type: `home`, `planet`, `freighter`.
pub fn base_type_label(bt: &nms_core::BaseType) -> String {
    match bt {
        nms_core::BaseType::HomePlanetBase => "home".to_string(),
        nms_core::BaseType::ExternalPlanetBase => "planet".to_string(),
        nms_core::BaseType::FreighterBase => "freighter".to_string(),
        other => other.to_string(),
    }
}

fn crops_cell(status: &BaseStatus) -> String {
    if status.crops_total == 0 {
        "-".to_string()
    } else {
        format!("{} / {}", status.crops_ready, status.crops_total)
    }
}

fn extraction_cell(status: &BaseStatus) -> String {
    if status.networks.is_empty() {
        return "-".to_string();
    }
    let mut cell = format!(
        "{} / {}",
        thousands(status.extraction_stored()),
        thousands(status.extraction_capacity())
    );
    let full = status.full_networks();
    if full == status.networks.len() {
        cell.push_str("  FULL");
    } else if full > 0 {
        cell.push_str(&format!("  {full} of {} FULL", status.networks.len()));
    }
    cell
}

fn power_cell(power: &PowerSummary) -> String {
    let mut parts: Vec<String> = Vec::new();
    if power.batteries > 0 {
        let noun = if power.batteries == 1 {
            "battery"
        } else {
            "batteries"
        };
        let state = if power.batteries_full == power.batteries {
            "full".to_string()
        } else if power.batteries_empty == power.batteries {
            "empty".to_string()
        } else {
            format!("{} full", power.batteries_full)
        };
        parts.push(format!("{} {noun} {state}", power.batteries));
    }
    for g in &power.generators {
        let word = match g.kind {
            nms_core::GeneratorKind::SolarPanel => "solar",
            nms_core::GeneratorKind::BiofuelReactor => "biofuel",
            nms_core::GeneratorKind::ElectromagneticGenerator => "electromagnetic",
            _ => "generator",
        };
        parts.push(format!("{} {word}", g.count));
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" \u{00B7} ")
    }
}

/// One row per base: crops ready of total, extraction stored of capacity, power state.
pub fn format_base_overview(statuses: &[BaseStatus], theme: &Theme) -> String {
    if statuses.is_empty() {
        return "  No bases found.\n".to_string();
    }
    let table_theme = table_theme_for(theme);
    let mut builder = Builder::default();
    builder.push_record(["Name", "Type", "Crops", "Extraction", "Power"]);
    for status in statuses {
        let name = if status.base.name.is_empty() {
            format!("({})", base_type_label(&status.base.base_type))
        } else {
            status.base.name.clone()
        };
        builder.push_record([
            truncate(&name, 28),
            base_type_label(&status.base.base_type),
            crops_cell(status),
            extraction_cell(status),
            power_cell(&status.power),
        ]);
    }
    builder.push_record(["", "", "", "", ""]);
    build_table(builder, &["BASES"], &table_theme, "Bases")
}

fn next_ready_cell(row: &CropRow) -> String {
    let multi = row.batches.len() > 1;
    row.batches
        .iter()
        .map(|b| {
            let when = match b.remaining_secs {
                Some(0) => "now".to_string(),
                Some(secs) => format!("in {}", format_duration(secs)),
                None => format!("grown {}", format_duration(row.grown_secs)),
            };
            if multi {
                format!("{when} ({})", b.count)
            } else {
                when
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Full view of one base: location, then crops, extraction, and power sections where present.
///
/// With a terminal `width` the sections are placed side by side as far as they fit; with `None` they are stacked.
pub fn format_base_detail(
    status: &BaseStatus,
    now: i64,
    theme: &Theme,
    width: Option<usize>,
) -> String {
    let table_theme = table_theme_for(theme);
    let base = &status.base;
    let name = if base.name.is_empty() {
        format!("({})", base_type_label(&base.base_type))
    } else {
        base.name.clone()
    };
    let mut blocks: Vec<String> = Vec::new();

    let mut builder = Builder::default();
    builder.push_record(["Property", "Detail"]);
    builder.push_record(["Name", &name]);
    builder.push_record(["Type", &base_type_label(&base.base_type)]);
    builder.push_record(["Galaxy", &status.galaxy_name]);
    if let Some(ref system) = status.system {
        builder.push_record(["System", system.name.as_deref().unwrap_or("-")]);
    }
    builder.push_record(["Portal Glyphs", &hex_to_emoji(&status.portal_hex)]);
    builder.push_record(["Hex Address", &status.portal_hex]);
    if let Some(dist) = status.distance_from_player {
        builder.push_record(["Distance", &format_distance(dist)]);
    }
    builder.push_record(["Objects", &base.objects.total().to_string()]);
    builder.push_record(["", ""]);
    blocks.push(build_table(builder, &["BASE"], &table_theme, ""));

    // A table followed by its snapshot line, as one block.
    let with_snapshot = |table: String| -> String {
        match status.snapshot {
            Some(snapshot) => format!(
                "{}\n  as of {}\n\n",
                table.trim_end_matches('\n'),
                format_snapshot(snapshot, now)
            ),
            None => table,
        }
    };

    if !status.crops.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["Crop", "Count", "Ready", "Next ready", "Progress"]);
        for row in &status.crops {
            let progress = row
                .progress
                .map(|p| format!("{:.0}%", p * 100.0))
                .unwrap_or_else(|| "-".to_string());
            builder.push_record([
                row.label.clone(),
                row.count.to_string(),
                row.ready.to_string(),
                next_ready_cell(row),
                progress,
            ]);
        }
        builder.push_record(["", "", "", "", ""]);
        blocks.push(with_snapshot(build_table(
            builder,
            &["CROPS"],
            &table_theme,
            "",
        )));
    }

    if !status.networks.is_empty() {
        let mut builder = Builder::default();
        builder.push_record([
            "Network",
            "Extractors",
            "Depots",
            "Stored",
            "Capacity",
            "Fill",
        ]);
        for net in &status.networks {
            let mut extractors: Vec<String> = Vec::new();
            if net.gas_extractors > 0 {
                extractors.push(format!("{} gas", net.gas_extractors));
            }
            if net.mineral_extractors > 0 {
                extractors.push(format!("{} mineral", net.mineral_extractors));
            }
            let extractors = if extractors.is_empty() {
                "-".to_string()
            } else {
                extractors.join(", ")
            };
            let fill = if net.is_full() {
                "FULL".to_string()
            } else {
                format!("{:.0}%", net.fill() * 100.0)
            };
            builder.push_record([
                net.index.to_string(),
                extractors,
                net.depots.to_string(),
                thousands(net.stored),
                thousands(net.capacity),
                fill,
            ]);
        }
        builder.push_record(["", "", "", "", "", ""]);
        blocks.push(with_snapshot(build_table(
            builder,
            &["EXTRACTION"],
            &table_theme,
            "",
        )));
    }

    let power = &status.power;
    if power.batteries > 0 || !power.generators.is_empty() || power.wires > 0 {
        let mut builder = Builder::default();
        builder.push_record(["Source", "Count", "State"]);
        for g in &power.generators {
            // Every kind seen so far reads 0 when it has nothing to say; anything else is shown raw until it is decoded.
            let state = if g.raw.iter().all(|r| *r == 0) {
                "-".to_string()
            } else {
                format!(
                    "raw {}",
                    g.raw
                        .iter()
                        .map(|r| thousands(*r))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            builder.push_record([
                g.kind.display_name().to_string(),
                g.count.to_string(),
                state,
            ]);
        }
        if power.batteries > 0 {
            let mut state = format!(
                "{} / {}",
                thousands(power.battery_charge),
                thousands(power.battery_capacity)
            );
            if power.batteries_full == power.batteries {
                state.push_str(", full");
            } else if power.batteries_empty == power.batteries {
                state.push_str(", empty");
            } else {
                state.push_str(&format!(
                    ", {} full, {} empty",
                    power.batteries_full, power.batteries_empty
                ));
            }
            builder.push_record(["Battery".to_string(), power.batteries.to_string(), state]);
        }
        if power.wires > 0 {
            builder.push_record([
                "Wires".to_string(),
                power.wires.to_string(),
                "-".to_string(),
            ]);
        }
        builder.push_record(["", "", ""]);
        blocks.push(build_table(builder, &["POWER"], &table_theme, ""));
    }

    side_by_side(&blocks, width, 2)
}

/// One-line summary of current alerts, for startup and `status`.
pub fn format_alert_line(alerts: &[Alert]) -> String {
    let ready: Vec<String> = alerts
        .iter()
        .filter_map(|a| match &a.kind {
            AlertKind::CropsReady { crop, count } => Some(format!("{count} {crop} at {}", a.base)),
            _ => None,
        })
        .collect();
    let full: Vec<String> = alerts
        .iter()
        .filter_map(|a| match &a.kind {
            AlertKind::DepotsFull { network, .. } => {
                Some(format!("{} (network {network})", a.base))
            }
            _ => None,
        })
        .collect();
    let fleet: Vec<String> = alerts
        .iter()
        .filter_map(|a| match &a.kind {
            AlertKind::FleetWaiting { number, .. } => {
                Some(format!("expedition {number} waiting for you"))
            }
            AlertKind::FleetReturned { number, .. } => {
                Some(format!("expedition {number} returned"))
            }
            AlertKind::NewOffers { count, .. } => Some(format!("{count} new offers")),
            _ => None,
        })
        .collect();
    let ready = if ready.is_empty() {
        "Ready: nothing".to_string()
    } else {
        format!("Ready: {}", ready.join(", "))
    };
    let full = if full.is_empty() {
        "Full depots: none".to_string()
    } else {
        format!("Full depots: {}", full.join(", "))
    };
    let mut line = format!("{ready}. {full}.");
    if !fleet.is_empty() {
        line.push_str(&format!(" Fleet: {}.", fleet.join(", ")));
    }
    line
}

/// Compact indicator for the prompt: `🌱 16 ready · 📦 1 full`, or empty when nothing is pending.
pub fn format_alert_indicator(summary: &AlertSummary) -> String {
    let mut parts: Vec<String> = Vec::new();
    if summary.crops_ready > 0 {
        parts.push(format!("\u{1F331} {} ready", summary.crops_ready));
    }
    if summary.networks_full > 0 {
        parts.push(format!("\u{1F4E6} {} full", summary.networks_full));
    }
    if summary.fleet_waiting > 0 {
        parts.push(format!("\u{1F680} {} waiting", summary.fleet_waiting));
    }
    if summary.fleet_returned > 0 {
        parts.push(format!("\u{2693} {} returned", summary.fleet_returned));
    }
    if summary.new_offers > 0 {
        parts.push(format!("\u{1F9ED} {} offers", summary.new_offers));
    }
    parts.join(" \u{00B7} ")
}

// ── Fleet ───────────────────────────────────────────────────────

/// Local wall-clock time, `HH:MM`.
pub fn format_clock(ts: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "?".to_string())
}

/// The status column of the fleet overview: what the expedition is doing and, as an estimate, how long it has left.
pub fn expedition_status_cell(row: &ExpeditionRow) -> String {
    let estimate = row.estimate_remaining_secs.map(format_duration);
    match row.state {
        ExpeditionState::Waiting => {
            let since = row
                .expedition
                .waiting_since()
                .map(|s| {
                    format!(
                        " since {} ({})",
                        format_clock(s),
                        format_duration(row.waiting_secs.unwrap_or(0))
                    )
                })
                .unwrap_or_default();
            match estimate {
                Some(left) => format!("waiting for you{since}, about {left} left once answered"),
                None => format!("waiting for you{since}"),
            }
        }
        ExpeditionState::Complete => "returned, awaiting debrief".to_string(),
        ExpeditionState::Running => {
            let mut text = match estimate {
                Some(left) => format!("about {left} left"),
                None => "under way".to_string(),
            };
            if row.damaged() > 0 {
                text.push_str(&format!(", {} damaged", row.damaged()));
            }
            text
        }
    }
}

/// The Navigator line under the overview: offers left, the next refresh, free command rooms, and frigates at home.
pub fn format_navigator_line(status: &FleetStatus) -> String {
    let offers = if status.offers.day == 0 {
        "Navigator: no offers recorded yet".to_string()
    } else if status.offers.refreshed {
        format!(
            "Navigator: {} new offers waiting (day rolled at 00:00 UTC)",
            status.offers.left
        )
    } else {
        format!(
            "Navigator: {} of today's {} offers left \u{00B7} new offers in {} (00:00 UTC)",
            status.offers.left,
            status.offers.per_day,
            format_duration(status.offers.secs_until_refresh)
        )
    };
    format!(
        "{offers} \u{00B7} {} of {} command rooms free \u{00B7} {} of {} frigates at home",
        status.rooms_free,
        status.command_rooms,
        status.frigates_home,
        status.frigates.len()
    )
}

/// One row per running expedition, then the Navigator line.
pub fn format_fleet_overview(status: &FleetStatus, theme: &Theme) -> String {
    let table_theme = table_theme_for(theme);
    let mut out = String::new();
    if status.expeditions.is_empty() {
        out.push_str("  No expeditions running.\n");
    } else {
        let mut builder = Builder::default();
        builder.push_record([
            "#", "Type", "Length", "Frigates", "Events", "Elapsed", "Status",
        ]);
        for row in &status.expeditions {
            let frigates = if row.damaged() > 0 {
                format!(
                    "{} ({} damaged)",
                    row.expedition.frigates.len(),
                    row.damaged()
                )
            } else {
                row.expedition.frigates.len().to_string()
            };
            builder.push_record([
                row.number.to_string(),
                row.category_label().to_string(),
                row.duration_label().to_string(),
                frigates,
                format!("{} / {}", row.expedition.resolved(), row.expedition.total()),
                format_duration(row.elapsed_secs),
                expedition_status_cell(row),
            ]);
        }
        builder.push_record(["", "", "", "", "", "", ""]);
        out.push_str(
            build_table(builder, &["FLEET"], &table_theme, "Expeditions").trim_end_matches('\n'),
        );
        out.push('\n');
    }
    out.push_str(&format!("\n  {}\n", format_navigator_line(status)));
    out
}

/// One expedition in full: its timing, the frigates on it, and the event log.
pub fn format_expedition_detail(row: &ExpeditionRow, now: i64, theme: &Theme) -> String {
    let table_theme = table_theme_for(theme);
    let e = &row.expedition;
    let mut blocks: Vec<String> = Vec::new();

    let mut builder = Builder::default();
    builder.push_record(["Property", "Detail"]);
    builder.push_record(["Expedition", &row.number.to_string()]);
    if !e.name.is_empty() {
        builder.push_record(["Name", &e.name]);
    }
    builder.push_record(["Type", row.category_label()]);
    builder.push_record(["Length", row.duration_label()]);
    builder.push_record(["Status", &expedition_status_cell(row)]);
    builder.push_record(["Started", &format_snapshot(e.start, now)]);
    builder.push_record(["Elapsed", &format_duration(row.elapsed_secs)]);
    if let Some(since) = e.waiting_since() {
        builder.push_record(["Waiting since", &format_snapshot(since, now)]);
    }
    builder.push_record([
        "Events",
        &format!("{} of {} resolved", e.resolved(), e.total()),
    ]);
    builder.push_record([
        "Outcomes",
        &format!("{} succeeded, {} failed", e.successes, e.failures),
    ]);
    let speed_modules: usize = row.frigates.iter().map(|f| f.frigate.speed_modules()).sum();
    let speed = match speed_modules {
        0 => format!("{:.2}", e.speed_multiplier),
        1 => format!("{:.2} (1 speed module aboard)", e.speed_multiplier),
        n => format!("{:.2} ({n} speed modules aboard)", e.speed_multiplier),
    };
    builder.push_record(["Speed", &speed]);
    let distance = row
        .distance_from_player
        .map(|d| format!(", {} away", format_distance(d)))
        .unwrap_or_default();
    let location = match (&row.system, e.location) {
        (Some(system), Some(_)) => format!(
            "{}{distance}",
            system.name.as_deref().unwrap_or("unnamed system")
        ),
        (None, Some(addr)) => format!("{:012X}{distance}", addr.packed()),
        (_, None) => "back at the freighter".to_string(),
    };
    builder.push_record(["Location", &location]);
    builder.push_record(["", ""]);
    blocks.push(build_table(builder, &["EXPEDITION"], &table_theme, ""));

    if !row.frigates.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["#", "Frigate", "Class", "Grade", "State"]);
        for f in &row.frigates {
            let state = if e.destroyed.contains(&f.frigate.index) {
                "destroyed"
            } else if e.damaged.contains(&f.frigate.index) {
                "damaged"
            } else {
                "active"
            };
            builder.push_record([
                (f.frigate.index + 1).to_string(),
                f.frigate.label(),
                f.frigate.class_label().to_string(),
                f.frigate.grade_label().to_string(),
                state.to_string(),
            ]);
        }
        builder.push_record(["", "", "", "", ""]);
        blocks.push(build_table(builder, &["FRIGATES"], &table_theme, ""));
    }

    if !e.events.is_empty() {
        let mut builder = Builder::default();
        builder.push_record(["#", "Event", "Decision", "Outcome", "Where"]);
        for (i, ev) in e.events.iter().enumerate() {
            let outcome = if i < e.resolved() {
                if ev.success { "ok" } else { "failed" }
            } else if i == e.next_event as usize && row.state == ExpeditionState::Waiting {
                "waiting for you"
            } else {
                "pending"
            };
            builder.push_record([
                (i + 1).to_string(),
                ev.label(),
                ev.intervention_label(),
                outcome.to_string(),
                row.event_places
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "-".to_string()),
            ]);
        }
        builder.push_record(["", "", "", "", ""]);
        blocks.push(build_table(builder, &["EVENTS"], &table_theme, ""));
    }

    blocks.join("\n")
}

/// The view `fleet` was asked for, or why it cannot be shown.
pub fn format_fleet(
    status: &FleetStatus,
    target: FleetTarget,
    theme: &Theme,
) -> Result<String, String> {
    match target {
        FleetTarget::Overview => Ok(format_fleet_overview(status, theme)),
        FleetTarget::Frigates => Ok(format_frigates(status, theme)),
        FleetTarget::Expedition(n) => status
            .expedition(n)
            .map(|row| format_expedition_detail(row, status.now, theme))
            .ok_or_else(|| match status.expeditions.len() {
                0 => "no expeditions running".to_string(),
                len => format!("expedition {n} not found; {len} running"),
            }),
    }
}

/// Every frigate: class, race, grade, the four main stats and the undecoded fifth, its non-stat perks, damage, lifetime runs, and where it is.
pub fn format_frigates(status: &FleetStatus, theme: &Theme) -> String {
    if status.frigates.is_empty() {
        return "  No frigates.\n".to_string();
    }
    let table_theme = table_theme_for(theme);
    let mut builder = Builder::default();
    builder.push_record([
        "#", "Frigate", "Class", "Race", "Grade", "Cbt", "Exp", "Ind", "Trd", "St5", "Perks",
        "Damage", "Runs", "Where",
    ]);
    for row in &status.frigates {
        let f = &row.frigate;
        let perks = f.perks();
        let location = match row.out_on {
            Some(n) => format!("expedition {n}"),
            None => "home".to_string(),
        };
        builder.push_record([
            (f.index + 1).to_string(),
            f.label(),
            f.class_label().to_string(),
            f.race_label().to_string(),
            f.grade_label().to_string(),
            f.combat().to_string(),
            f.exploration().to_string(),
            f.industrial().to_string(),
            f.trade().to_string(),
            f.stat_five().to_string(),
            perks,
            f.damage_taken.to_string(),
            f.expeditions.to_string(),
            location,
        ]);
    }
    builder.push_record([""; 14]);
    build_table(builder, &["FRIGATES"], &table_theme, "Frigates")
}

/// Truncate a string to `max_len` characters, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::address::GalacticAddress;
    use nms_core::biome::Biome;
    use nms_core::system::{Planet, System};

    fn plain() -> Theme {
        Theme::none()
    }

    #[test]
    fn test_format_distance_zero() {
        assert_eq!(format_distance(0.0), "< 1 ly");
    }

    #[test]
    fn test_format_distance_small_with_uncertainty() {
        // 42 ly (cross-voxel): SD=163, 163/42 > 5%, ± shown
        assert_eq!(format_distance(42.0), "~42 ly (\u{00B1}163)");
        assert_eq!(format_distance(999.0), "~999 ly (\u{00B1}163)");
    }

    #[test]
    fn test_format_distance_same_voxel() {
        // ~265 ly (same-voxel Robbins' estimate): SD=100, ± shown
        let voxel = nms_core::address::VOXEL_UNCERTAINTY;
        assert_eq!(format_distance(voxel), "~265 ly (\u{00B1}100)");
    }

    #[test]
    fn test_format_distance_thousands() {
        // 1000 ly (cross-voxel): SD=163, 163/1000 = 16% >= 5%, ± shown
        assert_eq!(format_distance(1000.0), "~1.0K ly (\u{00B1}163)");
        // 3000 ly: 163/3000 = 5.4% >= 5%, ± shown
        assert_eq!(format_distance(3000.0), "~3.0K ly (\u{00B1}163)");
        // 3200 ly: 163/3200 ≈ 5.1% >= 5%, ± shown
        assert_eq!(format_distance(3200.0), "~3.2K ly (\u{00B1}163)");
        // 3300 ly: 163/3300 < 5%, ± dropped
        assert_eq!(format_distance(3300.0), "~3.3K ly");
        // Large thousands: no ±
        assert_eq!(format_distance(40_000.0), "~40.0K ly");
    }

    #[test]
    fn test_format_distance_millions() {
        assert_eq!(format_distance(1_000_000.0), "~1.0M ly");
        assert_eq!(format_distance(1_500_000.0), "~1.5M ly");
    }

    #[test]
    fn test_hex_to_emoji_basic() {
        let emoji = hex_to_emoji("000000000000");
        // All zeros = 12 Sunset glyphs
        assert!(emoji.contains('\u{1F305}')); // Sunset emoji
    }

    #[test]
    fn test_hex_to_emoji_length() {
        let emoji = hex_to_emoji("01717D8A4EA2");
        // Should produce 12 emoji (some multi-byte)
        assert!(!emoji.is_empty());
    }

    #[test]
    fn test_format_find_results_empty() {
        let output = format_find_results(&[], &plain());
        assert!(output.contains("No results found"));
    }

    #[test]
    fn test_format_find_results_with_data() {
        let addr = GalacticAddress::new(100, 50, -200, 0x123, 0, 0);
        let results = vec![FindResult {
            planet: Planet::new(0, Some(Biome::Lush), None, false, Some("Eden".into()), None),
            system: System::new(
                addr,
                Some("Sol".into()),
                Some("Explorer".into()),
                None,
                vec![],
            ),
            distance_ly: 42_000.0,
            portal_hex: format!("{:012X}", addr.packed()),
            system_hex: format!("{:012X}", addr.packed()),
        }];
        let output = format_find_results(&results, &plain());
        assert!(output.contains("Eden"));
        assert!(output.contains("Lush"));
        assert!(output.contains("Sol"));
        assert!(output.contains("~42.0K ly"));
    }

    #[test]
    fn test_format_find_results_infested_marker() {
        let addr = GalacticAddress::new(0, 0, 0, 1, 0, 0);
        let results = vec![FindResult {
            planet: Planet::new(0, Some(Biome::Toxic), None, true, None, None),
            system: System::new(addr, None, None, None, vec![]),
            distance_ly: 0.0,
            portal_hex: "000000000001".into(),
            system_hex: "000000000001".into(),
        }];
        let output = format_find_results(&results, &plain());
        assert!(output.contains("Toxic*"));
    }

    #[test]
    fn test_format_find_results_with_dark_theme_contains_ansi() {
        let addr = GalacticAddress::new(100, 50, -200, 0x123, 0, 0);
        let results = vec![FindResult {
            planet: Planet::new(0, Some(Biome::Lush), None, false, Some("Eden".into()), None),
            system: System::new(
                addr,
                Some("Sol".into()),
                Some("Explorer".into()),
                None,
                vec![],
            ),
            distance_ly: 42_000.0,
            portal_hex: format!("{:012X}", addr.packed()),
            system_hex: format!("{:012X}", addr.packed()),
        }];
        let output = format_find_results(&results, &Theme::default_dark());
        // Should contain ANSI escape sequences (from hex color theme)
        assert!(output.contains("\x1b["));
        // But still contain the data
        assert!(output.contains("Eden"));
        assert!(output.contains("Sol"));
    }

    #[test]
    fn test_format_find_results_groups_planets_by_system() {
        let sol = GalacticAddress::new(100, 50, -200, 0x123, 0, 0);
        let unnamed = GalacticAddress::new(100, 50, -200, 0x456, 0, 0);
        let sol_hex = format!("{:012X}", sol.packed());
        let unnamed_hex = format!("{:012X}", unnamed.packed());
        let row = |addr: GalacticAddress, name: Option<&str>, index: u8, dist: f64| FindResult {
            planet: Planet::new(index, Some(Biome::Lush), None, false, None, None),
            system: System::new(addr, name.map(str::to_string), None, None, vec![]),
            distance_ly: dist,
            portal_hex: format!("{:X}{}", index, &format!("{:012X}", addr.packed())[1..]),
            system_hex: format!("{:012X}", addr.packed()),
        };
        let results = vec![
            row(sol, Some("Sol"), 1, 100.0),
            row(sol, Some("Sol"), 2, 100.0),
            row(sol, Some("Sol"), 3, 100.0),
            row(unnamed, None, 1, 400.0),
            row(unnamed, None, 2, 400.0),
        ];
        let output = format_find_results(&results, &plain());

        // The system name appears once for its block of three planets.
        assert_eq!(output.matches("Sol").count(), 1, "{output}");
        // The unnamed system is labelled with its own portal address, once.
        assert_eq!(output.matches(&unnamed_hex).count(), 1, "{output}");
        assert!(
            !output.contains(&sol_hex),
            "system hex must not replace a real name"
        );
        // Distance is printed once per group.
        assert_eq!(output.matches("~100 ly").count(), 1, "{output}");
        assert_eq!(output.matches("~400 ly").count(), 1, "{output}");
        // Each row carries its own planet address.
        for r in &results {
            assert!(output.contains(&r.portal_hex), "missing {}", r.portal_hex);
        }
        // No row falls back to a dash for the system column.
        let lines: Vec<&str> = output.lines().collect();
        assert!(
            lines
                .iter()
                .all(|l| !l.contains(" - ") || l.contains("Lush")),
            "{output}"
        );
    }

    #[test]
    fn test_format_find_results_shades_alternate_groups() {
        let a = GalacticAddress::new(1, 1, 1, 0x001, 0, 0);
        let b = GalacticAddress::new(1, 1, 1, 0x002, 0, 0);
        let c = GalacticAddress::new(1, 1, 1, 0x003, 0, 0);
        let row = |addr: GalacticAddress, name: &str, dist: f64| FindResult {
            planet: Planet::new(1, Some(Biome::Lush), None, false, None, None),
            system: System::new(addr, Some(name.into()), None, None, vec![]),
            distance_ly: dist,
            portal_hex: format!("{:012X}", addr.packed()),
            system_hex: format!("{:012X}", addr.packed()),
        };
        let results = vec![
            row(a, "Alpha", 100.0),
            row(a, "Alpha", 100.0),
            row(b, "Bravo", 200.0),
            row(c, "Charlie", 300.0),
        ];
        let output = format_find_results(&results, &Theme::default_dark());
        let shaded: Vec<&str> = output
            .lines()
            .filter(|l| l.contains("48;2;19;47;77m"))
            .collect();
        assert_eq!(shaded.len(), 1, "{output}");
        assert!(shaded[0].contains("Bravo"));
        let plain_output = format_find_results(&results, &plain());
        assert!(!plain_output.contains("48;2;19;47;77m"));
    }

    #[test]
    fn test_planet_label_uses_index_when_unnamed() {
        let named = Planet::new(2, Some(Biome::Lush), None, false, Some("Eden".into()), None);
        let unnamed = Planet::new(3, Some(Biome::Lush), None, false, None, None);
        assert_eq!(planet_label(&named), "Eden");
        assert_eq!(planet_label(&unnamed), "Planet 3");
    }

    #[test]
    fn test_format_find_results_labels_unnamed_planets_by_index() {
        let addr = GalacticAddress::new(1, 1, 1, 0x001, 0, 0);
        let results = vec![FindResult {
            planet: Planet::new(3, Some(Biome::Lush), None, false, None, None),
            system: System::new(addr, Some("Alpha".into()), None, None, vec![]),
            distance_ly: 100.0,
            portal_hex: format!("3{}", &format!("{:012X}", addr.packed())[1..]),
            system_hex: format!("{:012X}", addr.packed()),
        }];
        let output = format_find_results(&results, &plain());
        assert!(output.contains("Planet 3"), "{output}");
    }

    #[test]
    fn test_format_show_system_labels_unnamed_planets_by_index() {
        let addr = GalacticAddress::new(100, 50, -200, 0x123, 0, 0);
        let planets = vec![
            Planet::new(1, Some(Biome::Toxic), None, false, None, None),
            Planet::new(4, Some(Biome::Lush), None, false, Some("Eden".into()), None),
        ];
        let result = ShowSystemResult {
            system: System::new(addr, Some("Test System".into()), None, None, planets),
            portal_hex: format!("{:012X}", addr.packed()),
            galaxy_name: "Euclid".into(),
            distance_from_player: None,
        };
        let output = format_show_system(&result, &plain());
        assert!(output.contains("Planet 1"), "{output}");
        assert!(output.contains("Eden"), "{output}");
        assert!(!output.contains("Planet 4"), "{output}");
    }

    #[test]
    fn test_format_show_system_output() {
        let addr = GalacticAddress::new(100, 50, -200, 0x123, 0, 0);
        let result = ShowSystemResult {
            system: System::new(
                addr,
                Some("Test System".into()),
                Some("Explorer".into()),
                None,
                vec![Planet::new(0, Some(Biome::Lush), None, false, None, None)],
            ),
            portal_hex: format!("{:012X}", addr.packed()),
            galaxy_name: "Euclid".into(),
            distance_from_player: Some(5000.0),
        };
        let output = format_show_system(&result, &plain());
        assert!(output.contains("Test System"));
        assert!(output.contains("Euclid"));
        assert!(output.contains("Explorer"));
        assert!(output.contains("~5.0K ly"));
        assert!(output.contains("Lush"));
    }

    #[test]
    fn test_format_stats_output() {
        let mut biome_counts = std::collections::HashMap::new();
        biome_counts.insert(Biome::Lush, 10);
        biome_counts.insert(Biome::Toxic, 5);

        let result = StatsResult {
            system_count: 50,
            planet_count: 120,
            base_count: 3,
            biome_counts,
            unknown_biome_count: 5,
            named_planet_count: 20,
            named_system_count: 30,
            infested_count: 2,
        };
        let output = format_stats(&result, &plain());
        assert!(output.contains("50"));
        assert!(output.contains("120"));
        assert!(output.contains("Lush"));
        assert!(output.contains("10"));
        assert!(output.contains("(unknown)"));
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world this is long", 10), "hello w...");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    fn test_route_model() -> nms_graph::GalaxyModel {
        let json = r#"{
            "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 1, "PlanetIndex": 0}},
                    "Units": 0, "Nanites": 0, "Specials": 0,
                    "PersistentPlayerBases": []
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
                {"DD": {"UA": "0x00100000000064", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}}
            ]}}}
        }"#;
        nms_save::parse_save(json.as_bytes())
            .map(|save| nms_graph::GalaxyModel::from_save(&save))
            .unwrap()
    }

    #[test]
    fn test_format_route_empty() {
        let model = test_route_model();
        let result = RouteResult {
            route: nms_graph::Route {
                hops: vec![],
                total_distance_ly: 0.0,
            },
            warp_range: None,
            warp_jumps: None,
            algorithm: nms_graph::RoutingAlgorithm::TwoOpt,
            targets_visited: 0,
        };
        let output = format_route(&result, &model, &plain());
        assert!(output.contains("No route computed"));
    }

    #[test]
    fn test_format_route_header_present() {
        let model = test_route_model();
        let result = RouteResult {
            route: nms_graph::Route {
                hops: vec![nms_graph::RouteHop {
                    system_id: nms_graph::SystemId(0x001000000064),
                    leg_distance_ly: 0.0,
                    cumulative_ly: 0.0,
                    is_waypoint: false,
                }],
                total_distance_ly: 0.0,
            },
            warp_range: None,
            warp_jumps: None,
            algorithm: nms_graph::RoutingAlgorithm::TwoOpt,
            targets_visited: 0,
        };
        let output = format_route(&result, &model, &plain());
        assert!(output.contains("Hop"));
        assert!(output.contains("System"));
        assert!(output.contains("Distance"));
        assert!(output.contains("Cumulative"));
        assert!(output.contains("Portal Glyphs"));
    }

    #[test]
    fn test_format_route_algorithm_shown() {
        let model = test_route_model();
        let result = RouteResult {
            route: nms_graph::Route {
                hops: vec![nms_graph::RouteHop {
                    system_id: nms_graph::SystemId(0x001000000064),
                    leg_distance_ly: 0.0,
                    cumulative_ly: 0.0,
                    is_waypoint: false,
                }],
                total_distance_ly: 0.0,
            },
            warp_range: None,
            warp_jumps: None,
            algorithm: nms_graph::RoutingAlgorithm::NearestNeighbor,
            targets_visited: 0,
        };
        let output = format_route(&result, &model, &plain());
        assert!(output.contains("Algorithm: nearest-neighbor"));

        let result_2opt = RouteResult {
            algorithm: nms_graph::RoutingAlgorithm::TwoOpt,
            ..result
        };
        let output_2opt = format_route(&result_2opt, &model, &plain());
        assert!(output_2opt.contains("Algorithm: 2-opt"));
    }

    #[test]
    fn test_format_route_warp_jumps_shown() {
        let model = test_route_model();
        let result = RouteResult {
            route: nms_graph::Route {
                hops: vec![
                    nms_graph::RouteHop {
                        system_id: nms_graph::SystemId(0x001000000064),
                        leg_distance_ly: 0.0,
                        cumulative_ly: 0.0,
                        is_waypoint: false,
                    },
                    nms_graph::RouteHop {
                        system_id: nms_graph::SystemId(0x002000000C80),
                        leg_distance_ly: 5000.0,
                        cumulative_ly: 5000.0,
                        is_waypoint: false,
                    },
                ],
                total_distance_ly: 5000.0,
            },
            warp_range: Some(2000.0),
            warp_jumps: Some(3),
            algorithm: nms_graph::RoutingAlgorithm::TwoOpt,
            targets_visited: 1,
        };
        let output = format_route(&result, &model, &plain());
        assert!(output.contains("3 warp jumps"));
        assert!(output.contains("~2.0K ly") && output.contains("range"));
    }

    #[test]
    fn test_format_route_waypoint_marker() {
        let model = test_route_model();
        let result = RouteResult {
            route: nms_graph::Route {
                hops: vec![
                    nms_graph::RouteHop {
                        system_id: nms_graph::SystemId(0x001000000064),
                        leg_distance_ly: 0.0,
                        cumulative_ly: 0.0,
                        is_waypoint: false,
                    },
                    nms_graph::RouteHop {
                        system_id: nms_graph::SystemId(0x001000000064),
                        leg_distance_ly: 1000.0,
                        cumulative_ly: 1000.0,
                        is_waypoint: true,
                    },
                    nms_graph::RouteHop {
                        system_id: nms_graph::SystemId(0x001000000064),
                        leg_distance_ly: 1000.0,
                        cumulative_ly: 2000.0,
                        is_waypoint: false,
                    },
                ],
                total_distance_ly: 2000.0,
            },
            warp_range: None,
            warp_jumps: None,
            algorithm: nms_graph::RoutingAlgorithm::TwoOpt,
            targets_visited: 1,
        };
        let output = format_route(&result, &model, &plain());
        assert!(output.contains("*"));
        assert!(output.contains("\u{21B3}"));
    }
    mod base_status {
        use super::*;
        use crate::base::{BaseQuery, execute_base};
        use nms_core::player::{BaseType, PlayerBase};
        use nms_core::{BaseObjects, RawBaseObject};
        use nms_graph::GalaxyModel;

        const SNAPSHOT: i64 = 1_789_402_087;

        fn raw(object_id: &str, hi: u64) -> RawBaseObject<'_> {
            RawBaseObject {
                object_id,
                timestamp: SNAPSHOT,
                user_data: hi << 32,
            }
        }

        fn model() -> GalaxyModel {
            let mut raws = Vec::new();
            raws.extend(vec![raw("^SNOWPLANT", 3600); 16]);
            raws.extend(vec![raw("^BARRENPLANT", 3927); 13]);
            raws.extend(vec![raw("^BARRENPLANT", 30_000); 3]);
            raws.extend(vec![raw("^U_GASEXTRACTOR", 154_712); 3]);
            raws.extend(vec![raw("^U_SILO_S", 154_712); 4]);
            raws.push(raw("^U_SILO_S", 1_440_000));
            raws.push(raw("^U_BATTERY_S", 45_000));
            raws.extend(vec![raw("^U_GENERATOR_S", 0); 4]);
            raws.extend(vec![raw("^U_POWERLINE", 0); 30]);
            let farm = PlayerBase::new(
                "Farm".into(),
                BaseType::HomePlanetBase,
                GalacticAddress::new(0, 0, 0, 1, 0, 0),
                [0.0; 3],
                None,
            )
            .with_objects(BaseObjects::decode(raws));
            let bare = PlayerBase::new(
                "Outpost".into(),
                BaseType::ExternalPlanetBase,
                GalacticAddress::new(0, 0, 0, 2, 0, 0),
                [0.0; 3],
                None,
            );
            let mut model = GalaxyModel::new();
            model.insert_base(farm);
            model.insert_base(bare);
            model
        }

        #[test]
        fn duration_and_thousands() {
            assert_eq!(format_duration(0), "< 1m");
            assert_eq!(format_duration(59), "< 1m");
            assert_eq!(format_duration(2_700), "45m");
            assert_eq!(format_duration(7_260), "2h 01m");
            assert_eq!(format_duration(97_200), "1d 03h");
            assert_eq!(format_duration(-5), "< 1m");
            assert_eq!(thousands(0), "0");
            assert_eq!(thousands(999), "999");
            assert_eq!(thousands(4_750), "4,750");
            assert_eq!(thousands(1_440_000), "1,440,000");
        }

        #[test]
        fn snapshot_shows_age() {
            let text = format_snapshot(SNAPSHOT, SNAPSHOT + 720);
            assert!(text.ends_with("(12m ago)"), "{text}");
            assert!(text.starts_with("2026-09-14"), "{text}");
        }

        #[test]
        fn overview_lists_every_base_with_cells() {
            let statuses = execute_base(&model(), &BaseQuery::default(), SNAPSHOT).unwrap();
            let out = format_base_overview(&statuses, &plain());
            assert!(out.contains("BASES"));
            assert!(out.contains("16 / 32"), "{out}");
            assert!(out.contains("1,749 / 5,750  1 of 2 FULL"), "{out}");
            assert!(
                out.contains("1 battery full \u{00B7} 4 electromagnetic"),
                "{out}"
            );
            assert!(out.contains("Outpost"));
            let farm_line = out.lines().find(|l| l.contains("Farm")).unwrap();
            let outpost_line = out.lines().find(|l| l.contains("Outpost")).unwrap();
            assert!(
                out.find(farm_line).unwrap() < out.find(outpost_line).unwrap(),
                "alerting base first"
            );
        }

        #[test]
        fn detail_has_all_sections_and_batches() {
            let statuses = execute_base(
                &model(),
                &BaseQuery {
                    name: Some("farm".into()),
                },
                SNAPSHOT,
            )
            .unwrap();
            let out = format_base_detail(&statuses[0], SNAPSHOT + 60, &plain(), None);
            assert!(out.contains("BASE"));
            assert!(out.contains("CROPS"));
            assert!(out.contains("EXTRACTION"));
            assert!(out.contains("POWER"));
            assert!(out.contains("Frost Crystal"));
            assert!(out.contains("now"));
            assert!(out.contains("in 7h 40m (3), in 14h 54m (13)"), "{out}");
            assert!(out.contains("3 gas"));
            assert!(out.contains("4,750"));
            assert!(out.contains("FULL"));
            assert!(out.contains("Electromagnetic Generator"));
            assert!(out.contains("45,000 / 45,000, full"));
            assert!(out.contains("as of"));
            assert!(out.contains("(1m ago)"));
        }

        #[test]
        fn detail_places_sections_side_by_side_when_wide() {
            let statuses = execute_base(
                &model(),
                &BaseQuery {
                    name: Some("farm".into()),
                },
                SNAPSHOT,
            )
            .unwrap();
            let wide = format_base_detail(&statuses[0], SNAPSHOT, &plain(), Some(400));
            let title_line = wide.lines().find(|l| l.contains("BASE")).unwrap();
            assert!(
                title_line.contains("CROPS")
                    && title_line.contains("EXTRACTION")
                    && title_line.contains("POWER"),
                "{wide}"
            );
            assert!(
                wide.lines()
                    .any(|l| l.contains("Frost Crystal") && l.contains("3 gas")),
                "{wide}"
            );

            let medium = format_base_detail(&statuses[0], SNAPSHOT, &plain(), Some(120));
            let titles: Vec<&str> = medium
                .lines()
                .filter(|l| l.contains("BASE") || l.contains("EXTRACTION"))
                .collect();
            assert_eq!(titles.len(), 2, "{medium}");
            assert!(titles[0].contains("CROPS"), "{medium}");
            assert!(titles[1].contains("POWER"), "{medium}");

            let narrow = format_base_detail(&statuses[0], SNAPSHOT, &plain(), Some(40));
            let stacked = format_base_detail(&statuses[0], SNAPSHOT, &plain(), None);
            for title in ["BASE", "CROPS", "EXTRACTION", "POWER"] {
                assert_eq!(
                    narrow.lines().filter(|l| l.contains(title)).count(),
                    1,
                    "{narrow}"
                );
                assert!(stacked.contains(title));
            }
            assert!(
                !narrow
                    .lines()
                    .any(|l| l.contains("BASE") && l.contains("CROPS"))
            );
        }

        #[test]
        fn detail_omits_sections_that_do_not_apply() {
            let statuses = execute_base(
                &model(),
                &BaseQuery {
                    name: Some("Outpost".into()),
                },
                SNAPSHOT,
            )
            .unwrap();
            let out = format_base_detail(&statuses[0], SNAPSHOT, &plain(), None);
            assert!(out.contains("BASE"));
            assert!(!out.contains("CROPS"));
            assert!(!out.contains("EXTRACTION"));
            assert!(!out.contains("POWER"));
        }

        #[test]
        fn alert_line_and_indicator() {
            let alerts = crate::base::current_alerts(&model(), SNAPSHOT);
            assert_eq!(
                format_alert_line(&alerts),
                "Ready: 16 Frost Crystal at Farm. Full depots: Farm (network 2)."
            );
            assert_eq!(format_alert_line(&[]), "Ready: nothing. Full depots: none.");
            let indicator = format_alert_indicator(&AlertSummary::from_alerts(&alerts));
            assert_eq!(indicator, "\u{1F331} 16 ready \u{00B7} \u{1F4E6} 1 full");
            assert_eq!(format_alert_indicator(&AlertSummary::default()), "");
        }
    }
}

// ── Backups ─────────────────────────────────────────────────────

/// Bytes as a short human figure: `779 KB`, `33.6 MB`.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.0} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.2} GB", b / (KB * KB * KB))
    }
}

/// The `backup list` table for one account: when each snapshot was taken, its slot, type, label, and size, newest first.
pub fn format_backup_list(
    account: &str,
    snapshots: &[Snapshot],
    theme: &TableStyleConfig,
) -> String {
    if snapshots.is_empty() {
        return format!("No backups for {account}.\n");
    }
    let mut builder = Builder::default();
    builder.push_record(["When", "Slot", "Type", "Label", "Size"]);
    for snapshot in snapshots {
        let kind = match snapshot.save_type {
            SaveType::Manual => "manual",
            SaveType::Auto => "auto",
        };
        builder.push_record([
            snapshot.taken.format("%Y-%m-%d %H:%M:%S").to_string(),
            snapshot.slot.to_string(),
            kind.to_string(),
            snapshot.label.clone().unwrap_or_default(),
            format_size(snapshot.size),
        ]);
    }
    builder.push_record(["", "", "", "", ""]);
    let total: u64 = snapshots.iter().map(|s| s.size).sum();
    let table = build_table(builder, &["BACKUPS", account], theme, "snapshots");
    format!("{table}{} in all\n", format_size(total))
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn snapshot(label: Option<&str>, size: u64) -> Snapshot {
        Snapshot {
            path: PathBuf::from("/b/st_1/2026-09-14T13-41-36-slot1-manual"),
            taken: NaiveDate::from_ymd_opt(2026, 9, 14)
                .unwrap()
                .and_hms_opt(13, 41, 36)
                .unwrap(),
            slot: 1,
            save_type: SaveType::Manual,
            label: label.map(str::to_string),
            size,
        }
    }

    #[test]
    fn test_format_size_steps() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(797_696), "779 KB");
        assert_eq!(format_size(35_232_153), "33.6 MB");
        assert_eq!(format_size(2_500_000_000), "2.33 GB");
    }

    #[test]
    fn test_format_backup_list_rows_and_total() {
        let out = format_backup_list(
            "st_1",
            &[snapshot(Some("before-call"), 797_696), snapshot(None, 1024)],
            &nms_theme_no_color(),
        );
        assert!(out.contains("BACKUPS"));
        assert!(out.contains("st_1"));
        assert!(out.contains("2026-09-14 13:41:36"));
        assert!(out.contains("manual"));
        assert!(out.contains("before-call"));
        assert!(out.contains("779 KB"));
        assert!(out.contains("2 snapshots"));
        assert!(out.ends_with("780 KB in all\n"));
    }

    #[test]
    fn test_format_backup_list_empty() {
        assert_eq!(
            format_backup_list("st_1", &[], &nms_theme_no_color()),
            "No backups for st_1.\n"
        );
    }
}
