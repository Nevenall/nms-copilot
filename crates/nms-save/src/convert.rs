//! Conversion from raw save model types to `nms-core` domain types.

use chrono::DateTime;

use crate::model::*;

impl RawDiscoveryRecord {
    /// Convert to an `nms_core::DiscoveryRecord`, or `None` if the discovery
    /// type is unrecognized.
    pub fn to_core_record(&self) -> Option<nms_core::DiscoveryRecord> {
        let discovery_type = match self.dd.dt.as_str() {
            "Planet" => nms_core::Discovery::Planet,
            "SolarSystem" => nms_core::Discovery::SolarSystem,
            "Sector" => nms_core::Discovery::Sector,
            "Animal" => nms_core::Discovery::Animal,
            "Flora" => nms_core::Discovery::Flora,
            "Mineral" => nms_core::Discovery::Mineral,
            _ => return None,
        };

        let timestamp = if self.ows.ts > 0 {
            DateTime::from_timestamp(self.ows.ts as i64, 0)
        } else {
            None
        };

        let discoverer = if self.ows.usn.is_empty() {
            None
        } else {
            Some(self.ows.usn.clone())
        };

        let is_uploaded = self.fl.uploaded.unwrap_or(0) > 0;

        Some(nms_core::DiscoveryRecord::new(
            discovery_type,
            self.dd.ua.to_galactic_address(0),
            timestamp,
            self.dm.name(),
            discoverer,
            is_uploaded,
        ))
    }
}

impl PersistentPlayerBase {
    /// Convert to an `nms_core::PlayerBase`.
    pub fn to_core_base(&self) -> nms_core::PlayerBase {
        let base_type = match self.base_type.persistent_base_types.as_str() {
            "HomePlanetBase" => nms_core::BaseType::HomePlanetBase,
            "FreighterBase" => nms_core::BaseType::FreighterBase,
            _ => nms_core::BaseType::ExternalPlanetBase,
        };

        nms_core::PlayerBase::new(
            self.name.clone(),
            base_type,
            self.galactic_address.to_galactic_address(0),
            self.position,
            if self.owner.uid.is_empty() {
                None
            } else {
                Some(self.owner.uid.clone())
            },
        )
        .with_objects(nms_core::BaseObjects::decode(
            self.objects.iter().map(BaseObject::as_raw),
        ))
    }
}

impl SaveRoot {
    /// Get `PlayerStateData` for the active context.
    pub fn active_player_state(&self) -> &PlayerStateData {
        match self.active_context.as_str() {
            "Expedition" => &self.expedition_context.player_state_data,
            _ => &self.base_context.player_state_data,
        }
    }

    /// Convert active player state to `nms_core::PlayerState`.
    pub fn to_core_player_state(&self) -> nms_core::PlayerState {
        let ps = self.active_player_state();
        let ua = &ps.universe_address;
        let current_address = ua.galactic_address.to_galactic_address(ua.reality_index);

        let prev_ua = &ps.previous_universe_address;
        let previous_address = if prev_ua.galactic_address.voxel_x == 0
            && prev_ua.galactic_address.voxel_y == 0
            && prev_ua.galactic_address.voxel_z == 0
            && prev_ua.galactic_address.solar_system_index == 0
        {
            None
        } else {
            Some(
                prev_ua
                    .galactic_address
                    .to_galactic_address(prev_ua.reality_index),
            )
        };

        nms_core::PlayerState::new(
            current_address,
            ua.reality_index,
            previous_address,
            None, // freighter_address not yet extracted
            ps.units as u64,
            ps.nanites as u64,
            ps.specials as u64,
        )
    }
}

impl crate::model::FleetEvent {
    /// Convert to the core event; the location is `None` while the save holds 0.
    pub fn to_core(&self, reality_index: u8) -> nms_core::fleet::Event {
        nms_core::fleet::Event {
            id: self.event_id.clone(),
            intervention_id: self.intervention_event_id.clone(),
            is_intervention: self.is_intervention_event,
            success: self.success,
            location: (self.ua.0 != 0)
                .then(|| nms_core::GalacticAddress::from_save_ua(self.ua.0, reality_index)),
            affected: self.affected_frigate_indices.clone(),
        }
    }
}

impl crate::model::FleetExpedition {
    /// Convert to the core expedition. The save's addresses carry no galaxy, so the player's is supplied.
    pub fn to_core(&self, reality_index: u8) -> nms_core::fleet::Expedition {
        nms_core::fleet::Expedition {
            seed: self.seed.value(),
            name: self.custom_name.clone(),
            category: nms_core::fleet::ExpeditionCategory::from_save_name(
                &self.expedition_category.value,
            ),
            category_raw: self.expedition_category.value.clone(),
            duration: nms_core::fleet::DurationClass::from_save_name(
                &self.expedition_duration.value,
            ),
            duration_raw: self.expedition_duration.value.clone(),
            start: self.start_time,
            pause: self.pause_time,
            speed_multiplier: self.speed_multiplier,
            location: (self.ua.0 != 0)
                .then(|| nms_core::GalacticAddress::from_save_ua(self.ua.0, reality_index)),
            last_move: self.time_of_last_ua_change,
            frigates: self.all_frigate_indices.clone(),
            active: self.active_frigate_indices.clone(),
            damaged: self.damaged_frigate_indices.clone(),
            destroyed: self.destroyed_frigate_indices.clone(),
            events: self
                .events
                .iter()
                .map(|e| e.to_core(reality_index))
                .collect(),
            next_event: self.next_event_to_trigger,
            intervention_pending: self.intervention_phone_call_activated,
            successes: self.number_of_successful_events_this_expedition,
            failures: self.number_of_failed_events_this_expedition,
        }
    }
}

impl crate::model::FleetFrigate {
    /// Convert to the core frigate at `index` in the fleet list.
    pub fn to_core(&self, index: u32, reality_index: u8) -> nms_core::fleet::Frigate {
        nms_core::fleet::Frigate {
            index,
            name: self.custom_name.clone(),
            class: nms_core::fleet::FrigateClass::from_save_name(&self.frigate_class.value),
            class_raw: self.frigate_class.value.clone(),
            race: self.race.value.clone(),
            grade: nms_core::fleet::FrigateGrade::from_save_name(&self.inventory_class.value),
            stats: self.stats.clone(),
            traits: self.trait_ids.clone(),
            damage_taken: self.damage_taken,
            times_damaged: self.number_of_times_damaged,
            repairs: self.repairs_made,
            expeditions: self.total_number_of_expeditions,
            successes: self.total_number_of_successful_events,
            failures: self.total_number_of_failed_events,
            home: (self.home_system_seed.value() != 0).then(|| {
                nms_core::GalacticAddress::from_save_ua(
                    self.home_system_seed.value(),
                    reality_index,
                )
            }),
        }
    }
}

/// Object ID of a Fleet Command Room on the freighter.
const FLEET_ROOM_ID: &str = "^FRE_ROOM_FLEET";

impl SaveRoot {
    /// The fleet of the active context: expeditions, frigates, the offer day, and the command rooms counted across the player's bases.
    pub fn to_core_fleet(&self) -> nms_core::fleet::Fleet {
        let ps = self.active_player_state();
        let reality_index = ps.universe_address.reality_index;
        let command_rooms = ps
            .persistent_player_bases
            .iter()
            .flat_map(|b| b.objects.iter())
            .filter(|o| o.object_id == FLEET_ROOM_ID)
            .count();
        nms_core::fleet::Fleet {
            expeditions: ps
                .fleet_expeditions
                .iter()
                .map(|e| e.to_core(reality_index))
                .collect(),
            frigates: ps
                .fleet_frigates
                .iter()
                .enumerate()
                .map(|(i, f)| f.to_core(u32::try_from(i).unwrap_or(u32::MAX), reality_index))
                .collect(),
            offer_day: ps.last_known_day,
            launched_today: ps
                .expedition_seeds_selected_today
                .iter()
                .map(|s| s.value())
                .collect(),
            command_rooms: u32::try_from(command_rooms).unwrap_or(u32::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_record_to_core() {
        let raw = RawDiscoveryRecord {
            dd: DiscoveryData {
                ua: PackedGalacticAddress(0x513300F79B1D82),
                dt: "Flora".into(),
                vp: vec![],
            },
            dm: DiscoveryMetadata {
                custom_name: Some("Named Flora".into()),
            },
            ows: OwnershipData {
                lid: String::new(),
                uid: "12345".into(),
                usn: "TestUser".into(),
                ptk: "ST".into(),
                ts: 1700000000,
            },
            fl: DiscoveryFlags {
                created: Some(1),
                uploaded: Some(1),
            },
            rid: Some("abc".into()),
        };

        let core = raw.to_core_record().unwrap();
        assert_eq!(core.discovery_type, nms_core::Discovery::Flora);
        assert_eq!(core.discoverer.as_deref(), Some("TestUser"));
        assert_eq!(core.name.as_deref(), Some("Named Flora"));
        assert!(core.is_uploaded);
        assert!(core.timestamp.is_some());
    }

    #[test]
    fn unknown_discovery_type_returns_none() {
        let raw = RawDiscoveryRecord {
            dd: DiscoveryData {
                ua: PackedGalacticAddress(0),
                dt: "UnknownType".into(),
                vp: vec![],
            },
            dm: DiscoveryMetadata::default(),
            ows: OwnershipData::default(),
            fl: DiscoveryFlags::default(),
            rid: None,
        };
        assert!(raw.to_core_record().is_none());
    }

    #[test]
    fn base_to_core() {
        let base = PersistentPlayerBase {
            base_version: 8,
            galactic_address: PackedGalacticAddress(0x40050003AB8C07),
            position: [100.0, 200.0, 300.0],
            forward: [1.0, 0.0, 0.0],
            last_update_timestamp: 1700000000,
            objects: vec![],
            rid: String::new(),
            owner: OwnershipData {
                lid: String::new(),
                uid: "76561198025707979".into(),
                usn: String::new(),
                ptk: "ST".into(),
                ts: 0,
            },
            name: "My Base".into(),
            base_type: BaseTypeWrapper {
                persistent_base_types: "HomePlanetBase".into(),
            },
            last_edited_by_id: String::new(),
            last_edited_by_username: String::new(),
            game_mode: None,
        };

        let core = base.to_core_base();
        assert_eq!(core.name, "My Base");
        assert_eq!(core.base_type, nms_core::BaseType::HomePlanetBase);
        assert_eq!(core.owner_uid.as_deref(), Some("76561198025707979"));
    }

    #[test]
    fn galactic_address_object_to_core() {
        let obj = GalacticAddressObject {
            voxel_x: 1699,
            voxel_y: -2,
            voxel_z: 165,
            solar_system_index: 369,
            planet_index: 0,
        };
        let addr = obj.to_galactic_address(0);
        assert_eq!(addr.voxel_x(), 1699);
        assert_eq!(addr.voxel_y(), -2);
        assert_eq!(addr.voxel_z(), 165);
        assert_eq!(addr.solar_system_index(), 369);
        assert_eq!(addr.planet_index(), 0);
        assert_eq!(addr.reality_index, 0);
    }

    #[test]
    fn active_context_main() {
        let json = r#"{
            "Version": 4720,
            "Platform": "Mac|Final",
            "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "test"},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {"Units": 999}
            },
            "ExpeditionContext": {
                "GameMode": 6,
                "PlayerStateData": {"Units": 111}
            },
            "DiscoveryManagerData": {"DiscoveryData-v1": {"Store": {"Record": []}}}
        }"#;
        let save: SaveRoot = serde_json::from_str(json).unwrap();
        assert_eq!(save.active_player_state().units, 999);
    }

    #[test]
    fn active_context_expedition() {
        let json = r#"{
            "Version": 4720,
            "Platform": "Mac|Final",
            "ActiveContext": "Expedition",
            "CommonStateData": {"SaveName": "test"},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {"Units": 999}
            },
            "ExpeditionContext": {
                "GameMode": 6,
                "PlayerStateData": {"Units": 111}
            },
            "DiscoveryManagerData": {"DiscoveryData-v1": {"Store": {"Record": []}}}
        }"#;
        let save: SaveRoot = serde_json::from_str(json).unwrap();
        assert_eq!(save.active_player_state().units, 111);
    }

    #[test]
    fn to_core_player_state_with_previous() {
        let json = r#"{
            "Version": 4720,
            "Platform": "Mac|Final",
            "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "test"},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {
                        "RealityIndex": 0,
                        "GalacticAddress": {"VoxelX": 100, "VoxelY": 10, "VoxelZ": 200, "SolarSystemIndex": 369, "PlanetIndex": 2}
                    },
                    "PreviousUniverseAddress": {
                        "RealityIndex": 0,
                        "GalacticAddress": {"VoxelX": 50, "VoxelY": 5, "VoxelZ": 100, "SolarSystemIndex": 505, "PlanetIndex": 0}
                    },
                    "Units": 1000000,
                    "Nanites": 5000,
                    "Specials": 200
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"Store": {"Record": []}}}
        }"#;
        let save: SaveRoot = serde_json::from_str(json).unwrap();
        let state = save.to_core_player_state();
        assert_eq!(state.units, 1000000);
        assert_eq!(state.nanites, 5000);
        assert_eq!(state.quicksilver, 200);
        assert_eq!(state.current_address.voxel_x(), 100);
        assert!(state.previous_address.is_some());
        assert_eq!(state.previous_address.unwrap().voxel_x(), 50);
    }

    #[test]
    fn to_core_player_state_zero_previous_is_none() {
        let json = r#"{
            "Version": 4720,
            "Platform": "Mac|Final",
            "ActiveContext": "Main",
            "CommonStateData": {},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {
                        "RealityIndex": 0,
                        "GalacticAddress": {"VoxelX": 100, "VoxelY": 10, "VoxelZ": 200, "SolarSystemIndex": 369, "PlanetIndex": 0}
                    },
                    "PreviousUniverseAddress": {
                        "RealityIndex": 0,
                        "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}
                    },
                    "Units": 500
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"Store": {"Record": []}}}
        }"#;
        let save: SaveRoot = serde_json::from_str(json).unwrap();
        let state = save.to_core_player_state();
        assert!(state.previous_address.is_none());
    }

    #[test]
    fn to_core_fleet_converts_expeditions_frigates_and_rooms() {
        let json = r#"{
            "Version": 4720,
            "Platform": "Mac|Final",
            "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "test"},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 186, "VoxelY": 2, "VoxelZ": -1760, "SolarSystemIndex": 122, "PlanetIndex": 0}},
                    "Units": 0, "Nanites": 0, "Specials": 0,
                    "PersistentPlayerBases": [
                        {"GalacticAddress": "0x00100000000064", "Name": "Home", "BaseType": {"PersistentBaseTypes": "FreighterBase"},
                         "Objects": [
                            {"ObjectID": "^FRE_ROOM_FLEET", "Timestamp": 0, "UserData": 0},
                            {"ObjectID": "^FRE_ROOM_FLEET", "Timestamp": 0, "UserData": 0},
                            {"ObjectID": "^FRE_ROOM_STORE0", "Timestamp": 0, "UserData": 0}
                         ]}
                    ],
                    "FleetExpeditions": [{
                        "Seed": [true, "0x5F98B405C7B30D18"],
                        "ExpeditionCategory": {"ExpeditionCategory": "Diplomacy"},
                        "ExpeditionDuration": {"ExpeditionDuration": "VeryLong"},
                        "StartTime": 1789344506, "PauseTime": 1789396923, "SpeedMultiplier": 1.0,
                        "UA": "0x2E00FC956DEC", "TimeOfLastUAChange": 1789398099,
                        "AllFrigateIndices": [0], "ActiveFrigateIndices": [0], "DamagedFrigateIndices": [], "DestroyedFrigateIndices": [],
                        "Events": [
                            {"EventID": "^DIPLOMATIC_2", "IsInterventionEvent": false, "InterventionEventID": "^", "Success": true, "UA": "0x2E00FC956DEC"},
                            {"EventID": "^DIPLOMATIC_2", "IsInterventionEvent": true, "InterventionEventID": "^INT_TRADING_CHOOSE_FUND", "Success": false, "UA": 0}
                        ],
                        "NextEventToTrigger": 1,
                        "NumberOfSuccessfulEventsThisExpedition": 1, "NumberOfFailedEventsThisExpedition": 0,
                        "InterventionPhoneCallActivated": true
                    }],
                    "FleetFrigates": [{
                        "CustomName": "SV-8 Zuhotoh",
                        "FrigateClass": {"FrigateClass": "Combat"},
                        "Race": {"AlienRace": "Traders"},
                        "InventoryClass": {"InventoryClass": "S"},
                        "Stats": [33, 14, 8, 10, 10, 0, 0, 0, 0, 0, 0],
                        "TraitIDs": ["^COMBAT_PRI", "^", "^", "^", "^"],
                        "NumberOfTimesDamaged": 3,
                        "TotalNumberOfExpeditions": 34, "TotalNumberOfSuccessfulEvents": 286, "TotalNumberOfFailedEvents": 9,
                        "HomeSystemSeed": [true, "0x2E00FC956DEC"]
                    }],
                    "ExpeditionSeedsSelectedToday": ["0x5F98B405C7B30D18"],
                    "LastKnownDay": 20710
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": []}}}
        }"#;
        let save = crate::parse_save(json.as_bytes()).unwrap();
        let fleet = save.to_core_fleet();
        assert_eq!(fleet.command_rooms, 2);
        assert_eq!(fleet.offer_day, 20710);
        assert_eq!(fleet.launched_today, vec![0x5F98_B405_C7B3_0D18]);
        assert_eq!(fleet.expeditions.len(), 1);
        let e = &fleet.expeditions[0];
        assert_eq!(e.seed, 0x5F98_B405_C7B3_0D18);
        assert_eq!(
            e.category,
            Some(nms_core::fleet::ExpeditionCategory::Diplomacy)
        );
        assert_eq!(e.duration, Some(nms_core::fleet::DurationClass::VeryLong));
        assert_eq!(e.start, 1789344506);
        assert_eq!(e.pause, 1789396923);
        assert_eq!(e.last_move, 1789398099);
        let loc = e.location.expect("fleet location");
        assert_eq!(
            (
                loc.voxel_x(),
                loc.voxel_y(),
                loc.voxel_z(),
                loc.solar_system_index()
            ),
            (-532, -4, -1706, 46)
        );
        assert_eq!(e.frigates, vec![0]);
        assert_eq!(e.events.len(), 2);
        assert!(e.events[0].location.is_some());
        assert!(e.events[1].location.is_none());
        assert!(e.events[1].is_intervention);
        assert!(e.is_waiting());
        let f = &fleet.frigates[0];
        assert_eq!(f.index, 0);
        assert_eq!(f.name, "SV-8 Zuhotoh");
        assert_eq!(f.class, Some(nms_core::fleet::FrigateClass::Combat));
        assert_eq!(f.grade, Some(nms_core::fleet::FrigateGrade::S));
        assert_eq!(f.race, "Traders");
        assert_eq!(f.combat(), 33);
        assert_eq!(f.times_damaged, 3);
        assert_eq!(f.successes, 286);
        assert_eq!(f.home.map(|h| h.solar_system_index()), Some(46));
    }

    #[test]
    fn to_core_fleet_without_fleet_fields_is_empty() {
        let json = r#"{
            "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "test"},
            "BaseContext": {"GameMode": 1, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 1, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": []}}}
        }"#;
        let save = crate::parse_save(json.as_bytes()).unwrap();
        assert!(save.to_core_fleet().is_empty());
    }

    #[test]
    fn to_core_base_decodes_objects() {
        let json = r#"{
            "GalacticAddress": "0x00100000000064",
            "Objects": [
                {"ObjectID": "^SNOWPLANT", "Timestamp": 1789279852, "UserData": 15461882265600},
                {"ObjectID": "^U_SILO_S", "Timestamp": 1789279852, "UserData": 6184752906240000},
                {"ObjectID": "^U_BATTERY_S", "Timestamp": 1789279852, "UserData": 193273528320000},
                {"ObjectID": "^BUILDLANDINGPAD", "Timestamp": 1789279852, "UserData": 0}
            ],
            "Name": "Farm",
            "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}
        }"#;
        let base: PersistentPlayerBase = serde_json::from_str(json).unwrap();
        let core = base.to_core_base();
        assert_eq!(core.objects.crops.len(), 1);
        assert_eq!(
            core.objects.crops[0].kind,
            Some(nms_core::CropKind::FrostCrystal)
        );
        assert_eq!(core.objects.depots[0].units, 1000);
        assert!(core.objects.batteries[0].is_full());
        assert_eq!(core.objects.other, 1);
    }
}
