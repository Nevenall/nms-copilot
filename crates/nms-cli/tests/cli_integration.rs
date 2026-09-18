//! Integration tests for the `nms` CLI binary.
//!
//! Tests exercise the compiled binary via `assert_cmd`, verifying that
//! subcommands produce expected output and exit codes.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::PathBuf;

/// Helper: path to a test fixture relative to the workspace root.
fn fixture_path(name: &str) -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("data").join("test").join(name)
}

// ---- Convert command tests (no save file needed) ----

#[test]
fn test_nms_convert_glyphs_roundtrip_displays_all_formats() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--glyphs", "01717D8A4EA2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Portal Glyphs"))
        .stdout(predicate::str::contains("01717D8A4EA2"))
        .stdout(predicate::str::contains("Signal Booster"))
        .stdout(predicate::str::contains("Galactic Address"))
        .stdout(predicate::str::contains("Voxel Position"))
        .stdout(predicate::str::contains("System Index"))
        .stdout(predicate::str::contains("Planet Index"))
        .stdout(predicate::str::contains("Galaxy"));
}

#[test]
fn test_nms_convert_glyphs_lowercase_accepted() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--glyphs", "01717d8a4ea2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("01717D8A4EA2"));
}

#[test]
fn test_nms_convert_coords_displays_conversion() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--coords", "0EA2:007D:08A4:0171"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Signal Booster"))
        .stdout(predicate::str::contains("0EA2:007D:08A4:0171"))
        .stdout(predicate::str::contains("Portal Glyphs"))
        .stdout(predicate::str::contains("Galactic Address"));
}

#[test]
fn test_nms_convert_ga_displays_conversion() {
    cargo_bin_cmd!("nms")
        // Save-file layout: system index 46 at voxel (-532, -4, -1706), planet 0.
        .args(["convert", "--ga", "0x2E00FC956DEC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("002EFC956DEC"))
        .stdout(predicate::str::contains("0x2E00FC956DEC"))
        .stdout(predicate::str::contains("46"));
}

#[test]
fn test_nms_convert_voxel_with_ssi_displays_conversion() {
    cargo_bin_cmd!("nms")
        .args([
            "convert",
            "--voxel",
            "100,50,-200",
            "--ssi",
            "42",
            "--planet",
            "2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("X=100"))
        .stdout(predicate::str::contains("Y=50"))
        .stdout(predicate::str::contains("Z=-200"))
        .stdout(predicate::str::contains("System Index"))
        .stdout(predicate::str::contains("Planet Index"))
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_nms_convert_voxel_without_ssi_fails() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--voxel", "100,50,-200"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--ssi is required"));
}

#[test]
fn test_nms_convert_glyphs_invalid_length_fails() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--glyphs", "0171"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected 12 glyphs"));
}

#[test]
fn test_nms_convert_glyphs_invalid_hex_fails() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--glyphs", "ZZZZZZZZZZZZ"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid glyph"));
}

#[test]
fn test_nms_convert_galaxy_name_euclid() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--glyphs", "01717D8A4EA2", "--galaxy", "Euclid"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Euclid (0)"));
}

#[test]
fn test_nms_convert_galaxy_name_hilbert() {
    cargo_bin_cmd!("nms")
        .args([
            "convert",
            "--glyphs",
            "01717D8A4EA2",
            "--galaxy",
            "Hilbert Dimension",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hilbert Dimension (1)"));
}

#[test]
fn test_nms_convert_no_input_fails() {
    cargo_bin_cmd!("nms").args(["convert"]).assert().failure();
}

// ---- Completions command tests ----

#[test]
fn test_nms_completions_bash_generates_script() {
    cargo_bin_cmd!("nms")
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_nms"));
}

#[test]
fn test_nms_completions_zsh_generates_script() {
    cargo_bin_cmd!("nms")
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("nms"));
}

#[test]
fn test_nms_completions_fish_generates_script() {
    cargo_bin_cmd!("nms")
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("nms"));
}

// ---- General CLI tests ----

#[test]
fn test_nms_unknown_command_fails_with_error() {
    cargo_bin_cmd!("nms")
        .args(["nonexistent-command"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_nms_help_shows_usage() {
    cargo_bin_cmd!("nms")
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("NMS Copilot CLI"))
        .stdout(predicate::str::contains("convert"))
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("find"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("route"))
        .stdout(predicate::str::contains("export"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn test_nms_version_shows_version() {
    cargo_bin_cmd!("nms")
        .args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("nms"));
}

#[test]
fn test_nms_no_args_shows_error() {
    cargo_bin_cmd!("nms")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

// ---- Save-file-dependent tests (using fixtures) ----

#[test]
fn test_nms_info_with_fixture_shows_summary() {
    let fixture = fixture_path("minimal_save.json");
    cargo_bin_cmd!("nms")
        .args(["info", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("SAVE FILE SUMMARY"))
        .stdout(predicate::str::contains("Integration Test Save"))
        .stdout(predicate::str::contains("Normal"))
        .stdout(predicate::str::contains("Euclid"))
        .stdout(predicate::str::contains("5,000,000"));
}

#[test]
fn test_nms_info_with_multi_system_fixture_shows_discovery_counts() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["info", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("SAVE FILE SUMMARY"))
        .stdout(predicate::str::contains("Multi System Test"))
        .stdout(predicate::str::contains("Solar Systems"))
        .stdout(predicate::str::contains("Planets"));
}

#[test]
fn test_nms_info_nonexistent_save_fails() {
    cargo_bin_cmd!("nms")
        .args(["info", "--save", "/tmp/nonexistent_save_file.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_nms_have_gold_reports_total_and_locations() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["have", "gold", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Gold (ASTEROID2)"))
        .stdout(predicate::str::contains("11,297"))
        .stdout(predicate::str::contains("Storage 1"))
        .stdout(predicate::str::contains(
            "Lush Haven, Frost Outpost, Home Freighter",
        ));
}

#[test]
fn test_nms_have_unknown_item_says_so() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["have", "unobtainium", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing matching"));
}

#[test]
fn test_nms_inventory_overview_and_free() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["inventory", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Exosuit"))
        .stdout(predicate::str::contains("4 / 93"))
        .stdout(predicate::str::contains("Corvette parts"));
    cargo_bin_cmd!("nms")
        .args(["inventory", "--free", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Corvette parts"));
    cargo_bin_cmd!("nms")
        .args([
            "inventory",
            "storage",
            "2",
            "--save",
            fixture.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sulphurine"));
}

#[test]
fn test_nms_list_items_ships_exocraft_multitools() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["list", "--save", fixture.to_str().unwrap(), "items"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Gold"))
        .stdout(predicate::str::contains("'Apple' Roll"))
        .stdout(predicate::str::contains("Chromatic Metal"));
    cargo_bin_cmd!("nms")
        .args(["list", "--save", fixture.to_str().unwrap(), "ships"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Starbird *"))
        .stdout(predicate::str::contains("Fighter"));
    cargo_bin_cmd!("nms")
        .args(["list", "--save", fixture.to_str().unwrap(), "exocraft"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Lush Haven"));
    cargo_bin_cmd!("nms")
        .args(["list", "--save", fixture.to_str().unwrap(), "multitools"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Multi-tool 1 *"));
}

#[test]
fn test_nms_find_with_fixture_returns_results() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["find", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Portal Glyphs"));
}

#[test]
fn test_nms_find_with_biome_filter_lush() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "find",
            "--save",
            fixture.to_str().unwrap(),
            "--biome",
            "Lush",
        ])
        .assert()
        .success();
}

#[test]
fn test_nms_find_with_biome_filter_frozen() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "find",
            "--save",
            fixture.to_str().unwrap(),
            "--biome",
            "Frozen",
        ])
        .assert()
        .success();
}

#[test]
fn test_nms_find_with_nearest_limit() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "find",
            "--save",
            fixture.to_str().unwrap(),
            "--nearest",
            "2",
        ])
        .assert()
        .success();
}

#[test]
fn test_nms_find_invalid_biome_fails() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "find",
            "--save",
            fixture.to_str().unwrap(),
            "--biome",
            "NotABiome",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid biome"));
}

#[test]
fn test_nms_stats_with_fixture_shows_statistics() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["stats", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("GALAXY STATISTICS"));
}

#[test]
fn test_nms_stats_biomes_flag_shows_biome_table() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["stats", "--save", fixture.to_str().unwrap(), "--biomes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("BIOME DISTRIBUTION"));
}

#[test]
fn test_nms_export_json_produces_valid_json() {
    let fixture = fixture_path("multi_system_save.json");
    let output = cargo_bin_cmd!("nms")
        .args([
            "export",
            "--save",
            fixture.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Verify it is valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn test_nms_export_csv_produces_header_row() {
    let fixture = fixture_path("multi_system_save.json");
    let output = cargo_bin_cmd!("nms")
        .args([
            "export",
            "--save",
            fixture.to_str().unwrap(),
            "--format",
            "csv",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // CSV should have header row with known column names
    if !stdout.is_empty() {
        assert!(stdout.contains("planet_name"));
        assert!(stdout.contains("biome"));
        assert!(stdout.contains("portal_glyphs"));
    }
}

#[test]
fn test_nms_export_invalid_format_fails() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "export",
            "--save",
            fixture.to_str().unwrap(),
            "--format",
            "xml",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown format"));
}

#[test]
fn test_nms_convert_glyphs_roundtrip_coords_consistent() {
    // Convert glyphs, capture output, verify signal booster coords match
    let output = cargo_bin_cmd!("nms")
        .args(["convert", "--glyphs", "01717D8A4EA2"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Strip ANSI escape codes to extract signal booster coords
    let plain: String = stdout
        .chars()
        .fold((String::new(), false), |(mut s, in_esc), c| {
            if c == '\x1b' {
                (s, true)
            } else if in_esc {
                (s, c != 'm')
            } else {
                s.push(c);
                (s, false)
            }
        })
        .0;

    // Find the XXXX:XXXX:XXXX:XXXX pattern in the plain text
    let coords = plain
        .split_whitespace()
        .find(|w| w.matches(':').count() == 3 && w.len() >= 19)
        .expect("Signal booster coords not found in output");

    // Feed those coords back in and verify we get the same glyphs
    let output2 = cargo_bin_cmd!("nms")
        .args(["convert", "--coords", coords])
        .output()
        .unwrap();

    assert!(output2.status.success());
    let stdout2 = String::from_utf8(output2.stdout).unwrap();
    assert!(stdout2.contains("01717D8A4EA2"));
}

#[test]
fn test_nms_convert_help_shows_options() {
    cargo_bin_cmd!("nms")
        .args(["convert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--glyphs"))
        .stdout(predicate::str::contains("--coords"))
        .stdout(predicate::str::contains("--ga"))
        .stdout(predicate::str::contains("--voxel"))
        .stdout(predicate::str::contains("--galaxy"));
}

#[test]
fn test_nms_find_from_base_reference() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "find",
            "--save",
            fixture.to_str().unwrap(),
            "--from",
            "Lush Haven",
            "--nearest",
            "1",
        ])
        .assert()
        .success();
}

// ---- Raw command tests ----

#[test]
fn test_nms_raw_prints_value_at_path() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "raw",
            "BaseContext.PlayerStateData.PersistentPlayerBases[0].Name",
            "--save",
            fixture.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Lush Haven"));
}

#[test]
fn test_nms_raw_root_is_pruned_by_default() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["raw", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("BaseContext"))
        .stdout(predicate::str::contains("keys}"));
}

#[test]
fn test_nms_raw_keys_lists_children() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "raw",
            "BaseContext.PlayerStateData",
            "--keys",
            "--save",
            fixture.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("PersistentPlayerBases"))
        .stdout(predicate::str::contains("array (3 items)"));
}

#[test]
fn test_nms_raw_find_reports_key_paths() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "raw",
            "--find",
            "galacticaddress",
            "--save",
            fixture.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "PersistentPlayerBases[*].GalacticAddress",
        ));
}

#[test]
fn test_nms_raw_missing_key_lists_alternatives() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args([
            "raw",
            "BaseContext.Nope",
            "--save",
            fixture.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no key `Nope`"))
        .stderr(predicate::str::contains("PlayerStateData"));
}

#[test]
fn test_nms_base_overview_lists_bases() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["base", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("BASES"))
        .stdout(predicate::str::contains("Lush Haven"))
        .stdout(predicate::str::contains("Frost Outpost"))
        .stdout(predicate::str::contains(" / 5"))
        .stdout(predicate::str::contains("1 of 2 FULL"));
}

#[test]
fn test_nms_base_detail_shows_sections() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["base", "lush", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("CROPS"))
        .stdout(predicate::str::contains("Frost Crystal"))
        .stdout(predicate::str::contains("Gamma Root"))
        .stdout(predicate::str::contains("EXTRACTION"))
        .stdout(predicate::str::contains("2,250"))
        .stdout(predicate::str::contains("FULL"))
        .stdout(predicate::str::contains("POWER"))
        .stdout(predicate::str::contains("Solar Panel"))
        .stdout(predicate::str::contains("Electromagnetic Generator"))
        .stdout(predicate::str::contains("as of"));
}

#[test]
fn test_nms_base_unknown_name_fails() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["base", "no such base", "--save", fixture.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("base not found"));
}

#[test]
fn test_nms_base_detail_width_places_sections_side_by_side() {
    let fixture = fixture_path("multi_system_save.json");
    let output = cargo_bin_cmd!("nms")
        .args([
            "base",
            "lush",
            "--width",
            "200",
            "--save",
            fixture.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let title_line = stdout.lines().find(|l| l.contains("BASE")).unwrap();
    assert!(title_line.contains("CROPS"), "{stdout}");
    assert!(title_line.contains("EXTRACTION"), "{stdout}");
}

#[test]
fn test_nms_fleet_overview_lists_expeditions_and_navigator() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["fleet", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("FLEET"))
        .stdout(predicate::str::contains("Trade"))
        .stdout(predicate::str::contains("Very long"))
        .stdout(predicate::str::contains("2 / 3"))
        .stdout(predicate::str::contains("waiting for you"))
        .stdout(predicate::str::contains("Navigator: 5 new offers waiting"))
        .stdout(predicate::str::contains("1 of 2 command rooms free"))
        .stdout(predicate::str::contains("1 of 4 frigates at home"));
}

#[test]
fn test_nms_fleet_expedition_detail_shows_sections() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["fleet", "1", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("EXPEDITION"))
        .stdout(predicate::str::contains("FRIGATES"))
        .stdout(predicate::str::contains("EVENTS"))
        .stdout(predicate::str::contains("Combat frigate #1"))
        .stdout(predicate::str::contains("trading choose fund"))
        .stdout(predicate::str::contains("waiting for you"))
        .stdout(predicate::str::contains("unnamed system"))
        .stdout(predicate::str::contains("Location"));
}

#[test]
fn test_nms_fleet_frigates_lists_every_frigate() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["fleet", "frigates", "--save", fixture.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("FRIGATES"))
        .stdout(predicate::str::contains("Support frigate #4"))
        .stdout(predicate::str::contains("expedition 1"))
        .stdout(predicate::str::contains("home"))
        .stdout(predicate::str::contains("Korvax"));
}

#[test]
fn test_nms_fleet_bad_target_fails() {
    let fixture = fixture_path("multi_system_save.json");
    cargo_bin_cmd!("nms")
        .args(["fleet", "9", "--save", fixture.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expedition 9 not found"));
    cargo_bin_cmd!("nms")
        .args(["fleet", "ships", "--save", fixture.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected an expedition number"));
}

// ---- Backup command tests (no save file needed) ----

#[test]
fn test_nms_backup_list_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    cargo_bin_cmd!("nms")
        .args(["backup", "list", "--to"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No backups in"));
}

#[test]
fn test_nms_backup_prune_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    cargo_bin_cmd!("nms")
        .args(["backup", "prune", "--keep", "5", "--to"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No backups in"));
}

#[test]
fn test_nms_backup_help_lists_actions() {
    cargo_bin_cmd!("nms")
        .args(["backup", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("prune"))
        .stdout(predicate::str::contains("--label"))
        .stdout(predicate::str::contains("--all"));
}
