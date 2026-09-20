//! Detail view for one system.
//!
//! A base in full is the `base` command's business (`crate::base`), which carries the same location rows and the crops, networks, and power besides.

use nms_core::generated::Generated;
use nms_core::system::System;
use nms_graph::spatial::SystemId;
use nms_graph::{GalaxyModel, GraphError};

/// Everything `show system` prints.
#[derive(Debug, Clone)]
pub struct ShowSystemResult {
    pub system: System,
    pub portal_hex: String,
    pub galaxy_name: String,
    pub distance_from_player: Option<f64>,
    /// The region, the generated name, and the hover properties, when a generator answered.
    pub generated: Option<Generated>,
}

/// One system by name (a player's or a generated one) or by its packed hex address.
pub fn show_system(model: &GalaxyModel, name_or_id: &str) -> Result<ShowSystemResult, GraphError> {
    // Try name lookup first, then try as packed hex address
    let system = if let Some((_id, sys)) = model.system_by_name(name_or_id) {
        sys
    } else {
        let hex = name_or_id
            .strip_prefix("0x")
            .or_else(|| name_or_id.strip_prefix("0X"))
            .unwrap_or(name_or_id);
        let packed = u64::from_str_radix(hex, 16)
            .map_err(|_| GraphError::SystemNotFound(name_or_id.to_string()))?;
        // A bare portal address names no galaxy: try the player's, then every other
        // galaxy the model knows.
        let mut galaxies: Vec<u8> = model
            .player_position()
            .map(|p| p.reality_index)
            .into_iter()
            .collect();
        galaxies.extend(model.discovered_galaxies());
        galaxies
            .into_iter()
            .find_map(|galaxy| model.system(&SystemId::new(packed, galaxy)))
            .ok_or_else(|| GraphError::SystemNotFound(name_or_id.to_string()))?
    };

    let portal_hex = format!("{:012X}", system.address.packed());
    let galaxy = nms_core::galaxy::Galaxy::by_index(system.address.reality_index);

    // A distance across galaxies means nothing, so leave it out.
    let distance_from_player = model
        .player_position()
        .filter(|pos| pos.reality_index == system.address.reality_index)
        .map(|pos| pos.distance_ly(&system.address));

    let generated = model
        .generated_for(&SystemId::from_address(&system.address))
        .cloned();

    Ok(ShowSystemResult {
        system: system.clone(),
        portal_hex,
        galaxy_name: galaxy.name.to_string(),
        distance_from_player,
        generated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model() -> GalaxyModel {
        let json = r#"{
            "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
            "BaseContext": {"GameMode": 1, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 1, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": [{"BaseVersion": 8, "GalacticAddress": "0x00100000000064", "Position": [0.0,0.0,0.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 0, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"1","USN":"","PTK":"ST","TS":0}, "Name": "Alpha Base", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}]}},
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
                {"DD": {"UA": "0x00100000000064", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}}
            ]}}}
        }"#;
        nms_save::parse_save(json.as_bytes())
            .map(|save| GalaxyModel::from_save(&save))
            .unwrap()
    }

    #[test]
    fn test_show_system_by_hex_in_the_players_galaxy() {
        let model = test_model();
        // The portal form (PSSSYYZZZXXX) of the save address 0x00100000000064: system 1 at X=100.
        let result = show_system(&model, "0x001000000064").unwrap();
        assert_eq!(result.galaxy_name, "Euclid");
        assert_eq!(result.portal_hex.len(), 12);
        assert!(result.distance_from_player.is_some());
    }

    #[test]
    fn test_show_system_not_found() {
        let model = test_model();
        assert!(show_system(&model, "No System").is_err());
    }
}
