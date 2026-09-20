use nms_copilot::session::SessionState;
use nms_graph::GalaxyModel;

fn test_save_json() -> &'static str {
    r#"{
        "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
        "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
        "BaseContext": {
            "GameMode": 1,
            "PlayerStateData": {
                "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 1, "PlanetIndex": 0}},
                "Units": 0, "Nanites": 0, "Specials": 0,
                "PersistentPlayerBases": [
                    {"BaseVersion": 8, "GalacticAddress": "0x05000003AB8C07", "Position": [0.0,0.0,0.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 0, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"1","USN":"","PTK":"ST","TS":0}, "Name": "Test Base", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}
                ]
            }
        },
        "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
        "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
            {"DD": {"UA": "0x05000003AB8C07", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}},
            {"DD": {"UA": "0x15000003AB8C07", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}}
        ]}}}
    }"#
}

fn setup() -> (GalaxyModel, SessionState) {
    let save = nms_save::parse_save(test_save_json().as_bytes()).unwrap();
    let model = GalaxyModel::from_save(&save);
    let session = SessionState::from_model(&model);
    (model, session)
}

#[test]
fn dispatch_find_returns_results() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("find").unwrap().unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(!output.is_empty());
}

#[test]
fn dispatch_stats_returns_output() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("stats").unwrap().unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("GALAXY STATISTICS") || output.contains("system"));
}

#[test]
fn dispatch_info_returns_summary() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("info").unwrap().unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("systems"));
    assert!(output.contains("planets"));
}

#[test]
fn dispatch_help_returns_text() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("help").unwrap().unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("Commands:"));
}

#[test]
fn dispatch_convert_glyphs() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("convert --glyphs 01717D8A4EA2")
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("01717D8A4EA2"));
    assert!(output.contains("Signal Booster"));
}

#[test]
fn dispatch_base_by_name() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("base \"Test Base\"")
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("Test Base"));
    assert!(output.contains("Portal Glyphs"));
}

#[test]
fn dispatch_set_biome_filter() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("set biome Lush")
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("Lush"));
    assert!(session.biome_filter.is_some());
}

#[test]
fn dispatch_bare_set_shows_settings_and_info_shows_alerts() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("set").unwrap().unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("Position:"));
    assert!(output.contains("Warp range:"));
    assert!(!output.contains("Alerts"));
    let action = nms_copilot::commands::parse_line("info").unwrap().unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("Euclid"));
    assert!(output.contains("Alerts:"));
}

#[test]
fn dispatch_export_and_raw() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("export --format csv")
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.starts_with("planet_name,"), "{output}");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("out.json");
    let action = nms_copilot::commands::parse_line(&format!("export --to \"{}\"", file.display()))
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.starts_with("Wrote "), "{output}");
    assert!(std::fs::read_to_string(&file).unwrap().starts_with('['));
    let action = nms_copilot::commands::parse_line("raw --keys")
        .unwrap()
        .unwrap();
    let err = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap_err();
    assert!(err.contains("save.hg"), "no followed save file: {err}");
}

#[test]
fn dispatch_reset_clears_state() {
    let (model, mut session) = setup();

    // Set a biome filter first
    let set_action = nms_copilot::commands::parse_line("set biome Toxic")
        .unwrap()
        .unwrap();
    nms_copilot::dispatch::dispatch(&set_action, &model, &mut session).unwrap();
    assert!(session.biome_filter.is_some());

    // Reset it
    let reset_action = nms_copilot::commands::parse_line("reset biome")
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&reset_action, &model, &mut session).unwrap();
    assert!(output.contains("cleared"));
    assert!(session.biome_filter.is_none());
}

#[test]
fn dispatch_set_position_to_base() {
    let (model, mut session) = setup();
    let action = nms_copilot::commands::parse_line("set position \"Test Base\"")
        .unwrap()
        .unwrap();
    let output = nms_copilot::dispatch::dispatch(&action, &model, &mut session).unwrap();
    assert!(output.contains("Test Base"));
}

mod inventory {
    use nms_copilot::commands::parse_line;
    use nms_copilot::dispatch::dispatch;
    use nms_copilot::session::SessionState;
    use nms_graph::GalaxyModel;

    fn fixture() -> (GalaxyModel, SessionState) {
        let json = include_str!("../../../data/test/multi_system_save.json");
        let save = nms_save::parse_save(json.as_bytes()).unwrap();
        let model = GalaxyModel::from_save(&save);
        let session = SessionState::from_model(&model);
        (model, session)
    }

    fn run(line: &str) -> Result<String, String> {
        let (model, mut session) = fixture();
        let action = parse_line(line).unwrap().unwrap();
        dispatch(&action, &model, &mut session)
    }

    fn plain(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn test_have_gold_totals_and_locations() {
        let out = plain(&run("have gold").unwrap());
        assert!(out.contains("Gold (ASTEROID2)"), "{out}");
        assert!(out.contains("11,297"), "{out}");
        assert!(
            out.contains("Lush Haven, Frost Outpost, Home Freighter"),
            "{out}"
        );
        assert!(out.contains("with you"), "{out}");
    }

    #[test]
    fn test_have_by_id_and_type() {
        let out = plain(&run("have asteroid1").unwrap());
        assert!(out.contains("Silver"), "{out}");
        assert!(out.contains("2,959"), "{out}");
        let out = plain(&run("have gold --type product").unwrap());
        assert!(out.contains("Nothing matching"), "{out}");
        assert!(
            run("have gold --type ore")
                .unwrap_err()
                .contains("unknown item type")
        );
    }

    #[test]
    fn test_inventory_overview_contents_and_free() {
        let out = plain(&run("inventory").unwrap());
        assert!(out.contains("Storage 1"), "{out}");
        assert!(out.contains("4 / 93"), "{out}");
        assert!(
            !out.contains("Exosuit cargo"),
            "nothing unlocked, so not listed: {out}"
        );
        let out = plain(&run("inventory storage 1").unwrap());
        assert!(out.contains("Gold"), "{out}");
        assert!(out.contains("Platinum"), "{out}");
        let out = plain(&run("inventory --free").unwrap());
        let corvette = out.find("Corvette parts").unwrap();
        let storage2 = out.find("Storage 2").unwrap();
        assert!(corvette < storage2, "most free first: {out}");
        assert!(
            run("inventory locker")
                .unwrap_err()
                .contains("no container matches")
        );
    }

    #[test]
    fn test_list_items_ships_exocraft_multitools() {
        let out = plain(&run("list items --min 1000").unwrap());
        assert!(out.contains("Gold"), "{out}");
        assert!(out.contains("Ferrite Dust"), "{out}");
        assert!(!out.contains("Oxygen"), "under the minimum: {out}");
        let out = plain(&run("list ships").unwrap());
        assert!(out.contains("Starbird *"), "{out}");
        assert!(out.contains("Exotic"), "{out}");
        let out = plain(&run("list exocraft").unwrap());
        assert!(out.contains("Lush Haven"), "{out}");
        let out = plain(&run("list multitools").unwrap());
        assert!(out.contains("Multi-tool 1 *"), "{out}");
    }
}

mod backup {
    use super::setup;
    use nms_copilot::commands::parse_line;
    use nms_copilot::dispatch::dispatch;
    use nms_save::backup::{BackupPolicy, list};

    fn run(
        line: &str,
        model: &nms_graph::GalaxyModel,
        session: &mut nms_copilot::session::SessionState,
    ) -> Result<String, String> {
        let action = parse_line(line).unwrap().unwrap();
        dispatch(&action, model, session)
    }

    #[test]
    fn test_backup_without_configuration_is_an_error() {
        let (model, mut session) = setup();
        let err = run("backup", &model, &mut session).unwrap_err();
        assert!(err.contains("not configured"));
    }

    #[test]
    fn test_backup_on_off_and_status() {
        let (model, mut session) = setup();
        let dir = tempfile::tempdir().unwrap();
        session.configure_backups(
            BackupPolicy {
                root: dir.path().to_path_buf(),
                keep: 20,
            },
            false,
            None,
        );
        assert!(session.format_settings().contains("Backups:     off"));
        let out = run("backup on", &model, &mut session).unwrap();
        assert!(out.contains("on"));
        assert!(session.backup_enabled);
        assert!(session.format_settings().contains("Backups:     on"));
        let out = run("backup off", &model, &mut session).unwrap();
        assert!(out.contains("off"));
        assert!(!session.backup_enabled);
        let err = run("backup sideways", &model, &mut session).unwrap_err();
        assert!(err.contains("sideways"));
    }

    #[test]
    fn test_backup_snapshot_and_list_follow_the_save() {
        let (model, mut session) = setup();
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("st_9");
        std::fs::create_dir_all(&account).unwrap();
        let save = account.join("save4.hg");
        std::fs::write(&save, b"save bytes").unwrap();
        let root = dir.path().join("backups");
        session.configure_backups(
            BackupPolicy {
                root: root.clone(),
                keep: 20,
            },
            false,
            Some(&save),
        );

        let out = run("backup list", &model, &mut session).unwrap();
        assert_eq!(out, "No backups for st_9.\n");

        let out = run("backup --label first", &model, &mut session).unwrap();
        assert!(out.starts_with("Snapshot saved to "));
        let kept = list(&root, "st_9").unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].slot, 2);
        assert_eq!(kept[0].label.as_deref(), Some("first"));

        let out = run("backup", &model, &mut session).unwrap();
        assert!(out.contains("nothing copied"));

        let out = run("backup list", &model, &mut session).unwrap();
        assert!(out.contains("BACKUPS"));
        assert!(out.contains("first"));
        assert!(out.contains("1 snapshots"));
    }

    #[test]
    fn test_backup_snapshot_needs_a_game_save() {
        let (model, mut session) = setup();
        let dir = tempfile::tempdir().unwrap();
        let export = dir.path().join("export.json");
        std::fs::write(&export, b"{}").unwrap();
        session.configure_backups(
            BackupPolicy {
                root: dir.path().join("b"),
                keep: 20,
            },
            true,
            Some(&export),
        );
        let err = run("backup", &model, &mut session).unwrap_err();
        assert!(err.contains("save.hg"));
    }
}
