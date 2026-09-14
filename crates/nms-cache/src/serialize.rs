//! Serialize and deserialize galaxy data to/from rkyv archives.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::TimeZone;
use rkyv::rancor::Error as RkyvError;

use nms_core::system::System;
use nms_graph::{EdgeStrategy, GalaxyModel};

use crate::data::{CacheData, CachedSystem};
use crate::error::CacheError;

/// Extract cache data from a GalaxyModel.
pub fn extract_cache_data(model: &GalaxyModel, save_version: u32) -> CacheData {
    let systems: Vec<CachedSystem> = model
        .systems
        .values()
        .map(|system| CachedSystem {
            address: system.address,
            name: system.name.clone(),
            discoverer: system.discoverer.clone(),
            timestamp_secs: system.timestamp.map(|ts| ts.timestamp()),
            planets: system.planets.clone(),
        })
        .collect();

    let bases = model.bases.values().cloned().collect();

    let cached_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    CacheData {
        systems,
        bases,
        player_state: model.player_state.clone(),
        save_version,
        cached_at,
    }
}

/// Magic bytes at the start of a cache file.
const CACHE_MAGIC: &[u8; 4] = b"NMSC";

/// Version of the archived data. Bump whenever an archived type changes shape, or whenever a decoding rule that feeds archived data changes (base objects are stored decoded), so caches written by an older binary are rebuilt instead of trusted.
pub const CACHE_FORMAT_VERSION: u32 = 3;

const HEADER_LEN: usize = CACHE_MAGIC.len() + 4;

/// Serialize cache data to bytes, prefixed with the format header.
pub fn serialize(data: &CacheData) -> Result<Vec<u8>, CacheError> {
    let archived =
        rkyv::to_bytes::<RkyvError>(data).map_err(|e| CacheError::Serialize(e.to_string()))?;
    let mut bytes = Vec::with_capacity(HEADER_LEN + archived.len());
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&CACHE_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&archived);
    Ok(bytes)
}

/// Deserialize cache data from bytes, rejecting files without the current format header.
pub fn deserialize(bytes: &[u8]) -> Result<CacheData, CacheError> {
    let (magic, rest) = bytes
        .split_at_checked(CACHE_MAGIC.len())
        .ok_or_else(|| CacheError::Deserialize("cache file too short".into()))?;
    if magic != CACHE_MAGIC {
        return Err(CacheError::Deserialize(
            "not a cache file (missing header)".into(),
        ));
    }
    let (version, payload) = rest
        .split_at_checked(4)
        .ok_or_else(|| CacheError::Deserialize("cache file too short".into()))?;
    let version = u32::from_le_bytes(version.try_into().expect("four bytes"));
    if version != CACHE_FORMAT_VERSION {
        return Err(CacheError::Deserialize(format!(
            "cache format version {version}, expected {CACHE_FORMAT_VERSION}"
        )));
    }
    // rkyv needs the archive aligned; copy the payload into an aligned buffer.
    let mut aligned = rkyv::util::AlignedVec::<16>::with_capacity(payload.len());
    aligned.extend_from_slice(payload);
    rkyv::from_bytes::<CacheData, RkyvError>(&aligned)
        .map_err(|e| CacheError::Deserialize(e.to_string()))
}

/// Write cache data to a file.
pub fn write_cache(data: &CacheData, path: &Path) -> Result<(), CacheError> {
    let bytes = serialize(data)?;

    // Ensure parent directories exist (e.g. per-save cache paths)
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(CacheError::Io)?;
    }

    // Write to a temp file first, then rename for atomicity
    let tmp_path = path.with_extension("rkyv.tmp");
    fs::write(&tmp_path, &bytes).map_err(CacheError::Io)?;
    fs::rename(&tmp_path, path).map_err(CacheError::Io)?;

    Ok(())
}

/// Read and deserialize cache data from a file.
pub fn read_cache(path: &Path) -> Result<CacheData, CacheError> {
    let bytes = fs::read(path).map_err(CacheError::Io)?;
    deserialize(&bytes)
}

/// Rebuild a GalaxyModel from cache data.
///
/// Reconstructs the graph, R-tree, and all HashMap indices.
pub fn rebuild_model(data: &CacheData) -> GalaxyModel {
    let mut model = GalaxyModel::new();

    for cached in &data.systems {
        let timestamp = cached
            .timestamp_secs
            .and_then(|secs| chrono::Utc.timestamp_opt(secs, 0).single());

        let system = System::new(
            cached.address,
            cached.name.clone(),
            cached.discoverer.clone(),
            timestamp,
            cached.planets.clone(),
        );
        model.insert_system(system);
    }

    for base in &data.bases {
        model.insert_base(base.clone());
    }

    model.player_state = data.player_state.clone();
    model.build_edges(EdgeStrategy::default());

    model
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
                    "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 100, "VoxelY": 50, "VoxelZ": -200, "SolarSystemIndex": 42, "PlanetIndex": 0}},
                    "Units": 5000000, "Nanites": 10000, "Specials": 500,
                    "PersistentPlayerBases": [
                        {"BaseVersion": 8, "GalacticAddress": "0x05000003AB8C07", "Position": [0.0,0.0,0.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 0, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"1","USN":"","PTK":"ST","TS":0}, "Name": "Test Base", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}
                    ]
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
                {"DD": {"UA": "0x05000003AB8C07", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}},
                {"DD": {"UA": "0x15000003AB8C07", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}},
                {"DD": {"UA": "0x25000003AB8C07", "DT": "Planet", "VP": ["0xCD", 1]}, "DM": {}, "OWS": {"LID":"","UID":"1","USN":"Explorer","PTK":"ST","TS":0}, "FL": {"U": 1}}
            ]}}}
        }"#;
        let save = nms_save::parse_save(json.as_bytes()).unwrap();
        GalaxyModel::from_save(&save)
    }

    #[test]
    fn test_extract_cache_data() {
        let model = test_model();
        let data = extract_cache_data(&model, 4720);
        assert_eq!(data.systems.len(), model.systems.len());
        assert_eq!(data.bases.len(), model.bases.len());
        assert_eq!(data.save_version, 4720);
        assert!(data.cached_at > 0);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let model = test_model();
        let data = extract_cache_data(&model, 4720);
        let bytes = serialize(&data).unwrap();
        let restored = deserialize(&bytes).unwrap();
        assert_eq!(restored.systems.len(), data.systems.len());
        assert_eq!(restored.bases.len(), data.bases.len());
    }

    #[test]
    fn test_rebuild_model_preserves_counts() {
        let model = test_model();
        let data = extract_cache_data(&model, 4720);
        let bytes = serialize(&data).unwrap();
        let restored_data = deserialize(&bytes).unwrap();
        let rebuilt = rebuild_model(&restored_data);

        assert_eq!(rebuilt.systems.len(), model.systems.len());
        assert_eq!(rebuilt.planets.len(), model.planets.len());
        assert_eq!(rebuilt.bases.len(), model.bases.len());
    }

    #[test]
    fn test_write_and_read_cache_file() {
        let model = test_model();
        let data = extract_cache_data(&model, 4720);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rkyv");

        write_cache(&data, &path).unwrap();
        assert!(path.exists());

        let restored = read_cache(&path).unwrap();
        assert_eq!(restored.systems.len(), data.systems.len());
    }

    #[test]
    fn test_read_nonexistent_cache_errors() {
        let result = read_cache(Path::new("/tmp/nonexistent_nms_cache.rkyv"));
        assert!(result.is_err());
    }

    #[test]
    fn test_rebuild_preserves_player_state() {
        let model = test_model();
        let data = extract_cache_data(&model, 4720);
        let rebuilt = rebuild_model(&data);
        assert!(rebuilt.player_state.is_some());
        assert_eq!(
            rebuilt.player_state.as_ref().unwrap().current_address,
            model.player_state.as_ref().unwrap().current_address
        );
    }

    #[test]
    fn test_rebuild_preserves_base_lookup() {
        let model = test_model();
        let data = extract_cache_data(&model, 4720);
        let rebuilt = rebuild_model(&data);
        assert!(rebuilt.base("Test Base").is_some());
    }

    #[test]
    fn deserialize_rejects_missing_header() {
        let data = extract_cache_data(&test_model(), 4720);
        let bytes = rkyv::to_bytes::<RkyvError>(&data).unwrap();
        let err = deserialize(&bytes).unwrap_err();
        assert!(err.to_string().contains("header"), "{err}");
    }

    #[test]
    fn deserialize_rejects_other_format_version() {
        let data = extract_cache_data(&test_model(), 4720);
        let mut bytes = serialize(&data).unwrap();
        bytes[4..8].copy_from_slice(&(CACHE_FORMAT_VERSION + 1).to_le_bytes());
        let err = deserialize(&bytes).unwrap_err();
        assert!(err.to_string().contains("version"), "{err}");
    }

    #[test]
    fn cache_round_trips_base_objects() {
        use nms_core::{BaseObjects, RawBaseObject};
        let mut model = test_model();
        let mut base = model.bases.values().next().unwrap().clone();
        base.objects = BaseObjects::decode([
            RawBaseObject {
                object_id: "^SNOWPLANT",
                timestamp: 1789279852,
                user_data: 15461882265600,
            },
            RawBaseObject {
                object_id: "^U_SILO_S",
                timestamp: 1789279852,
                user_data: 6184752906240000,
            },
        ]);
        model.insert_base(base.clone());
        let data = extract_cache_data(&model, 4720);
        let restored = deserialize(&serialize(&data).unwrap()).unwrap();
        let rebuilt = rebuild_model(&restored);
        assert_eq!(
            rebuilt.bases[&base.name.to_lowercase()].objects,
            base.objects
        );
    }
}
