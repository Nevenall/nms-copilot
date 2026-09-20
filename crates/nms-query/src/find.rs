//! Planet/system search queries.

use nms_core::address::GalacticAddress;
use nms_core::biome::{Biome, BiomeSubType};
use nms_core::generated::Generated;
use nms_core::system::{Planet, System};
use nms_graph::query::BiomeFilter;
use nms_graph::{GalaxyModel, GraphError};

/// How to determine the "from" point for distance calculations.
#[derive(Debug, Clone, Default)]
pub enum ReferencePoint {
    /// Use the player's current position from the save file.
    #[default]
    CurrentPosition,
    /// Use a named base's position.
    Base(String),
    /// Use an explicit galactic address.
    Address(GalacticAddress),
}

/// What orders the results.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FindSort {
    /// Nearest first.
    #[default]
    Distance,
    /// Most species recorded first, then nearest.
    Fauna,
    /// Most plants recorded first, then nearest.
    Flora,
    /// Most minerals recorded first, then nearest.
    Minerals,
}

impl FindSort {
    /// Parse the `--sort` word.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "distance" | "nearest" => Ok(Self::Distance),
            "fauna" | "animals" | "creatures" => Ok(Self::Fauna),
            "flora" | "plants" => Ok(Self::Flora),
            "minerals" | "mineral" => Ok(Self::Minerals),
            other => Err(format!(
                "unknown sort: {other} (expected distance, fauna, flora, or minerals)"
            )),
        }
    }

    /// The scanned count this sort orders by, if it is a scanned sort.
    fn scanned_count(self, planet: &Planet) -> Option<u16> {
        match self {
            Self::Distance => None,
            Self::Fauna => Some(planet.scanned.fauna),
            Self::Flora => Some(planet.scanned.flora),
            Self::Minerals => Some(planet.scanned.minerals),
        }
    }
}

/// Parameters for a planet/system search.
#[derive(Debug, Clone, Default)]
pub struct FindQuery {
    /// Filter by biome type.
    pub biome: Option<Biome>,
    /// Filter by biome subtype/variant.
    pub biome_subtype: Option<BiomeSubType>,
    /// Filter by infested flag.
    pub infested: Option<bool>,
    /// Only include results within this radius (light-years).
    pub within_ly: Option<f64>,
    /// Return at most this many results (nearest first).
    pub nearest: Option<usize>,
    /// Filter by planet/system name pattern (case-insensitive substring).
    pub name_pattern: Option<String>,
    /// Filter by discoverer username (case-insensitive substring).
    pub discoverer: Option<String>,
    /// Only include named planets/systems.
    pub named_only: bool,
    /// Reference point for distance calculations.
    pub from: ReferencePoint,
    /// What orders the results; with a scanned sort, `nearest` is a plain limit over the sorted list.
    pub sort: FindSort,
}

/// A single result from a find query.
#[derive(Debug, Clone)]
pub struct FindResult {
    /// The matching planet.
    pub planet: Planet,
    /// The system containing the planet.
    pub system: System,
    /// Distance from the reference point in light-years.
    pub distance_ly: f64,
    /// Portal address of the planet as 12 hex digits (planet index in the first digit).
    /// Caller renders as emoji.
    pub portal_hex: String,
    /// Portal address of the system as 12 hex digits (planet digit zero). Identical for
    /// every planet in the same system, so it doubles as the group key and as a label
    /// for systems without a name.
    pub system_hex: String,
    /// The region, generated name, and hover properties of the system, when a generator answered.
    pub generated: Option<Generated>,
}

/// Execute a find query against the galaxy model.
///
/// Returns results in the query's order: nearest first, or most recorded first for a scanned sort.
pub fn execute_find(model: &GalaxyModel, query: &FindQuery) -> Result<Vec<FindResult>, GraphError> {
    // Resolve the reference point
    let from = match &query.from {
        ReferencePoint::CurrentPosition => model
            .player_position()
            .copied()
            .ok_or(GraphError::NoPlayerPosition)?,
        ReferencePoint::Base(name) => model
            .base(name)
            .map(|b| b.address)
            .ok_or_else(|| GraphError::BaseNotFound(name.clone()))?,
        ReferencePoint::Address(addr) => *addr,
    };

    let biome_filter = BiomeFilter {
        biome: query.biome,
        biome_subtype: query.biome_subtype,
        infested: query.infested,
        named_only: query.named_only,
    };

    // Choose between nearest-N or within-radius. A scanned sort must see every candidate
    // before it limits, so `nearest` is applied after sorting in that case.
    let planet_matches = if let (Some(n), FindSort::Distance) = (query.nearest, query.sort) {
        model.nearest_planets(&from, n * 2, &biome_filter) // over-fetch for post-filtering
    } else if let Some(radius) = query.within_ly {
        model.planets_within_radius(&from, radius, &biome_filter)
    } else {
        // No spatial constraint: get all matching planets
        let mut all = Vec::new();
        if let Some(biome) = query.biome {
            for planet in model.planets_by_biome(biome) {
                for (&sys_id, system) in &model.systems {
                    if system.planets.iter().any(|p| p.index == planet.index) {
                        let dist = from.distance_ly(&system.address);
                        all.push(((sys_id, planet.index), planet, dist));
                        break;
                    }
                }
            }
        } else {
            for (&sys_id, system) in &model.systems {
                for planet in &system.planets {
                    let dist = from.distance_ly(&system.address);
                    all.push(((sys_id, planet.index), planet, dist));
                }
            }
        }
        all
    };

    let mut results: Vec<FindResult> = planet_matches
        .into_iter()
        .filter_map(|(key, planet, dist)| {
            let system = model.system(&key.0)?;

            // Apply name pattern filter
            if let Some(ref pattern) = query.name_pattern {
                let pattern_lower = pattern.to_lowercase();
                let name_matches = planet
                    .name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&pattern_lower))
                    .unwrap_or(false)
                    || system
                        .name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&pattern_lower))
                        .unwrap_or(false);
                if !name_matches {
                    return None;
                }
            }

            // Apply discoverer filter
            if let Some(ref disc) = query.discoverer {
                let disc_lower = disc.to_lowercase();
                let disc_matches = system
                    .discoverer
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&disc_lower))
                    .unwrap_or(false);
                if !disc_matches {
                    return None;
                }
            }

            // Apply within_ly if both nearest and within are specified
            if let Some(radius) = query.within_ly
                && dist > radius
            {
                return None;
            }

            let sys_addr = &system.address;
            let planet_addr = GalacticAddress::new(
                sys_addr.voxel_x(),
                sys_addr.voxel_y(),
                sys_addr.voxel_z(),
                sys_addr.solar_system_index(),
                planet.index,
                sys_addr.reality_index,
            );
            let portal_hex = format!("{:012X}", planet_addr.packed());
            let system_hex = format!("{:012X}", sys_addr.packed() & 0x0FFF_FFFF_FFFF);
            let generated = model.generated_for(&key.0).cloned();

            Some(FindResult {
                planet: planet.clone(),
                system: system.clone(),
                distance_ly: dist,
                portal_hex,
                system_hex,
                generated,
            })
        })
        .collect();

    // Sort by distance, keeping each system's planets contiguous and in index order
    // so the display can group them; a scanned sort puts the most recorded first.
    results.sort_by(|a, b| {
        let by_count = match (
            query.sort.scanned_count(&a.planet),
            query.sort.scanned_count(&b.planet),
        ) {
            (Some(x), Some(y)) => y.cmp(&x),
            _ => std::cmp::Ordering::Equal,
        };
        by_count
            .then_with(|| a.distance_ly.partial_cmp(&b.distance_ly).unwrap())
            .then_with(|| a.system_hex.cmp(&b.system_hex))
            .then_with(|| a.planet.index.cmp(&b.planet.index))
    });

    // Apply nearest limit
    if let Some(n) = query.nearest {
        results.truncate(n);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_model() -> GalaxyModel {
        let json = r#"{
            "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 1, "PlanetIndex": 0}},
                    "Units": 0, "Nanites": 0, "Specials": 0,
                    "PersistentPlayerBases": [{"BaseVersion": 8, "GalacticAddress": "0x00100000000064", "Position": [0.0,0.0,0.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 0, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"1","USN":"","PTK":"ST","TS":0}, "Name": "Alpha Base", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}]
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
                {"DD": {"UA": "0x00100000000064", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10100000000064", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x00200000000C80", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10200000000C80", "DT": "Planet", "VP": ["0xCD", 1]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10100000000064", "DT": "Animal", "VP": ["0x1", "0x2", "0x3", "0x4"]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10200000000C80", "DT": "Animal", "VP": ["0x5", "0x6", "0x7", "0x8"]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10200000000C80", "DT": "Animal", "VP": ["0x9", "0xA", "0xB", "0xC"]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10200000000C80", "DT": "Animal", "VP": ["0xD", "0xE", "0xF", "0x10"]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10200000000C80", "DT": "Flora", "VP": ["0x11", "0x12"]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}}
            ]}}}
        }"#;
        nms_save::parse_save(json.as_bytes())
            .map(|save| GalaxyModel::from_save(&save))
            .unwrap()
    }

    #[test]
    fn test_find_all_planets() {
        let model = test_model();
        let query = FindQuery::default();
        let results = execute_find(&model, &query).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_find_by_biome() {
        let model = test_model();
        let query = FindQuery {
            biome: Some(Biome::Lush),
            ..Default::default()
        };
        let results = execute_find(&model, &query).unwrap();
        for r in &results {
            assert_eq!(r.planet.biome, Some(Biome::Lush));
        }
    }

    #[test]
    fn test_find_nearest_limit() {
        let model = test_model();
        let query = FindQuery {
            nearest: Some(1),
            ..Default::default()
        };
        let results = execute_find(&model, &query).unwrap();
        assert!(results.len() <= 1);
    }

    #[test]
    fn test_find_from_base() {
        let model = test_model();
        let query = FindQuery {
            from: ReferencePoint::Base("Alpha Base".into()),
            ..Default::default()
        };
        let results = execute_find(&model, &query);
        assert!(results.is_ok());
    }

    #[test]
    fn test_find_from_nonexistent_base_errors() {
        let model = test_model();
        let query = FindQuery {
            from: ReferencePoint::Base("No Such Base".into()),
            ..Default::default()
        };
        assert!(execute_find(&model, &query).is_err());
    }

    #[test]
    fn test_find_results_sorted_by_distance() {
        let model = test_model();
        let query = FindQuery::default();
        let results = execute_find(&model, &query).unwrap();
        for i in 1..results.len() {
            assert!(results[i].distance_ly >= results[i - 1].distance_ly);
        }
    }

    #[test]
    fn test_find_sorted_by_fauna_puts_the_most_recorded_first() {
        let model = test_model();
        let nearest = execute_find(&model, &FindQuery::default()).unwrap();
        assert_eq!(
            nearest[0].planet.scanned.fauna, 1,
            "the near planet comes first by distance"
        );
        assert_eq!(nearest[1].planet.scanned, nms_core::Scanned::new(3, 1, 0));

        let query = FindQuery {
            sort: FindSort::Fauna,
            ..Default::default()
        };
        let by_fauna = execute_find(&model, &query).unwrap();
        assert_eq!(
            by_fauna[0].planet.scanned.fauna, 3,
            "the far planet comes first by fauna"
        );
        assert_eq!(by_fauna[1].planet.scanned.fauna, 1);

        // With a scanned sort, `nearest` limits the sorted list rather than pre-fetching by distance.
        let query = FindQuery {
            sort: FindSort::Fauna,
            nearest: Some(1),
            ..Default::default()
        };
        let top = execute_find(&model, &query).unwrap();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].planet.scanned.fauna, 3);

        assert_eq!(FindSort::parse("Flora"), Ok(FindSort::Flora));
        assert_eq!(FindSort::parse("minerals"), Ok(FindSort::Minerals));
        assert_eq!(FindSort::parse("nearest"), Ok(FindSort::Distance));
        assert!(FindSort::parse("size").is_err());
    }

    #[test]
    fn test_find_portal_hex_is_12_digits() {
        let model = test_model();
        let query = FindQuery::default();
        let results = execute_find(&model, &query).unwrap();
        for r in &results {
            assert_eq!(r.portal_hex.len(), 12);
            assert_eq!(r.system_hex.len(), 12);
            // The planet address carries the planet index in its first digit; the
            // system address has a zero there. The rest is identical.
            assert_eq!(&r.portal_hex[1..], &r.system_hex[1..]);
            assert_eq!(&r.system_hex[..1], "0");
            assert_eq!(
                u8::from_str_radix(&r.portal_hex[..1], 16).unwrap(),
                r.planet.index
            );
        }
    }

    #[test]
    fn test_find_by_discoverer() {
        let model = test_model();
        let query = FindQuery {
            discoverer: Some("Explorer".into()),
            ..Default::default()
        };
        let results = execute_find(&model, &query).unwrap();
        for r in &results {
            assert!(r.system.discoverer.as_ref().unwrap().contains("Explorer"));
        }
    }

    #[test]
    fn test_find_results_group_planets_by_system() {
        let model = test_model();
        let query = FindQuery {
            biome: None,
            biome_subtype: None,
            infested: None,
            within_ly: None,
            nearest: None,
            name_pattern: None,
            discoverer: None,
            named_only: false,
            from: ReferencePoint::CurrentPosition,
            sort: FindSort::Distance,
        };
        let results = execute_find(&model, &query).unwrap();
        assert!(results.len() >= 2);
        // Every system's planets appear contiguously: once a system_hex changes it
        // must never reappear later in the list.
        let mut seen: Vec<&str> = Vec::new();
        for r in &results {
            match seen.last() {
                Some(last) if *last == r.system_hex => {}
                _ => {
                    assert!(
                        !seen.contains(&r.system_hex.as_str()),
                        "system {} appears in two separate runs",
                        r.system_hex
                    );
                    seen.push(&r.system_hex);
                }
            }
        }
        // Distances never decrease.
        for w in results.windows(2) {
            assert!(w[0].distance_ly <= w[1].distance_ly);
        }
    }
}
