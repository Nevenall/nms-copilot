use nms_graph::GalaxyModel;
use nms_query::base::{BaseQuery, execute_base};
use nms_query::display::{format_base_detail, format_show_system};
use nms_query::show::show_system;
use nms_query::theme::Theme;

fn test_save_json() -> &'static str {
    r#"{
        "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
        "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
        "BaseContext": {
            "GameMode": 1,
            "PlayerStateData": {
                "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 100, "VoxelY": 50, "VoxelZ": -200, "SolarSystemIndex": 42, "PlanetIndex": 0}},
                "Units": 0, "Nanites": 0, "Specials": 0,
                "PersistentPlayerBases": [
                    {"BaseVersion": 8, "GalacticAddress": "0x05000003AB8C07", "Position": [100.0,200.0,300.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 1700000000, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"123","USN":"TestUser","PTK":"ST","TS":0}, "Name": "Sealab 2038", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}
                ]
            }
        },
        "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
        "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
            {"DD": {"UA": "0x05000003AB8C07", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":1700000000}, "FL": {"U": 1}},
            {"DD": {"UA": "0x15000003AB8C07", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":1700000000}, "FL": {"U": 1}}
        ]}}}
    }"#
}

#[test]
fn base_detail_end_to_end() {
    let save = nms_save::parse_save(test_save_json().as_bytes()).unwrap();
    let model = GalaxyModel::from_save(&save);

    let statuses = execute_base(
        &model,
        &BaseQuery {
            name: Some("Sealab 2038".into()),
        },
        1_700_000_000,
    )
    .unwrap();
    assert_eq!(statuses.len(), 1);
    let output = format_base_detail(&statuses[0], 1_700_000_000, &Theme::none(), Some(120));

    assert!(output.contains("Sealab 2038"));
    assert!(output.contains("home"), "{output}");
    assert!(output.contains("Euclid"));
    assert!(
        output.contains("Planets"),
        "the row show base used to have: {output}"
    );
}

#[test]
fn base_detail_case_insensitive() {
    let save = nms_save::parse_save(test_save_json().as_bytes()).unwrap();
    let model = GalaxyModel::from_save(&save);

    assert!(
        execute_base(
            &model,
            &BaseQuery {
                name: Some("sealab 2038".into())
            },
            0
        )
        .is_ok()
    );
}

#[test]
fn show_system_with_planets() {
    let save = nms_save::parse_save(test_save_json().as_bytes()).unwrap();
    let model = GalaxyModel::from_save(&save);

    // Look up the system by hex address
    let sys_id = model.systems.keys().next().unwrap();
    let hex = format!("{:012X}", sys_id.0);
    let s = show_system(&model, &hex).unwrap();

    assert!(!s.system.planets.is_empty());
    let output = format_show_system(&s, &Theme::none());
    assert!(output.contains("SYSTEM DETAIL"));
}

#[test]
fn show_nonexistent_base_errors() {
    let save = nms_save::parse_save(test_save_json().as_bytes()).unwrap();
    let model = GalaxyModel::from_save(&save);

    assert!(
        execute_base(
            &model,
            &BaseQuery {
                name: Some("No Such Base".into())
            },
            0
        )
        .is_err()
    );
}

#[test]
fn show_nonexistent_system_errors() {
    let save = nms_save::parse_save(test_save_json().as_bytes()).unwrap();
    let model = GalaxyModel::from_save(&save);

    assert!(show_system(&model, "NoSystem").is_err());
}
