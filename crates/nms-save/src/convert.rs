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
            "PlayerShipBase" => nms_core::BaseType::PlayerShipBase,
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

impl crate::model::InventoryGrid {
    /// Convert to a core container of the given kind.
    pub fn to_core(
        &self,
        kind: nms_core::ContainerKind,
        access: Vec<String>,
    ) -> nms_core::Container {
        nms_core::Container {
            kind,
            class: nms_core::Grade::from_save_name(&self.class.value),
            width: u8::try_from(self.width).unwrap_or(u8::MAX),
            height: u8::try_from(self.height).unwrap_or(u8::MAX),
            unlocked_slots: u16::try_from(self.valid_slot_indices.len()).unwrap_or(u16::MAX),
            stacks: self.slots.iter().map(|slot| slot.to_core()).collect(),
            access,
        }
    }

    fn unlocked(&self) -> u16 {
        u16::try_from(self.valid_slot_indices.len()).unwrap_or(u16::MAX)
    }
}

impl crate::model::InventorySlot {
    /// Convert to a core stack; negative amounts, which the game has written, clamp to zero.
    pub fn to_core(&self) -> nms_core::ItemStack {
        nms_core::ItemStack {
            id: nms_core::ItemId::new(self.id.clone()),
            kind: nms_core::ItemKind::from_save_name(&self.slot_type.value),
            amount: u32::try_from(self.amount.max(0)).unwrap_or(u32::MAX),
            max: u32::try_from(self.max_amount.max(0)).unwrap_or(u32::MAX),
            slot: (
                u8::try_from(self.index.x.max(0)).unwrap_or(u8::MAX),
                u8::try_from(self.index.y.max(0)).unwrap_or(u8::MAX),
            ),
        }
    }
}

/// The name a base is known by in access lists: its name, or its type when it has none.
fn base_label(base: &PersistentPlayerBase) -> String {
    if !base.name.is_empty() {
        return base.name.clone();
    }
    match base.base_type.persistent_base_types.as_str() {
        "FreighterBase" => "Freighter".to_string(),
        "PlayerShipBase" => "Corvette".to_string(),
        _ => "Unnamed base".to_string(),
    }
}

/// Whether a base stands on a planet, where an exocraft can be parked; the freighter and the corvette share the system's address but hold no exocraft.
fn is_planet_base(base: &PersistentPlayerBase) -> bool {
    !matches!(
        base.base_type.persistent_base_types.as_str(),
        "FreighterBase" | "PlayerShipBase"
    )
}

/// The storage container number, 1 to 10, that a placed object opens: `^CONTAINER0` in a planet base and `^FRE_ROOM_STORE0` on the freighter both open container 1.
fn storage_number(object_id: &str) -> Option<u8> {
    let digit = object_id
        .strip_prefix("^CONTAINER")
        .or_else(|| object_id.strip_prefix("^FRE_ROOM_STORE"))?;
    let n: u8 = digit.parse().ok()?;
    (n < 10).then_some(n + 1)
}

impl SaveRoot {
    /// Everything the player owns in the active context: every grid with where it can be opened, the ships, the exocraft, and the multi-tools.
    pub fn to_core_holdings(&self) -> nms_core::Holdings {
        use nms_core::ContainerKind as K;
        let ps = self.active_player_state();
        let reality_index = ps.universe_address.reality_index;
        let bases = &ps.persistent_player_bases;

        // Which bases place each storage container.
        let mut storage_access: Vec<Vec<String>> = vec![Vec::new(); 10];
        for base in bases {
            let label = base_label(base);
            for n in base
                .objects
                .iter()
                .filter_map(|o| storage_number(&o.object_id))
            {
                let list = &mut storage_access[usize::from(n - 1)];
                if !list.contains(&label) {
                    list.push(label.clone());
                }
            }
        }

        let with_you = vec!["with you".to_string()];
        let mut containers = vec![
            ps.inventory.to_core(K::Exosuit, with_you.clone()),
            ps.inventory_cargo
                .to_core(K::ExosuitCargo, with_you.clone()),
            ps.inventory_tech_only
                .to_core(K::ExosuitTech, with_you.clone()),
            ps.freighter_inventory.to_core(K::Freighter, vec![]),
            ps.freighter_inventory_cargo
                .to_core(K::FreighterCargo, vec![]),
            ps.freighter_inventory_tech_only
                .to_core(K::FreighterTech, vec![]),
        ];
        let chests = [
            &ps.chest1_inventory,
            &ps.chest2_inventory,
            &ps.chest3_inventory,
            &ps.chest4_inventory,
            &ps.chest5_inventory,
            &ps.chest6_inventory,
            &ps.chest7_inventory,
            &ps.chest8_inventory,
            &ps.chest9_inventory,
            &ps.chest10_inventory,
        ];
        for (i, chest) in chests.iter().enumerate() {
            let n = u8::try_from(i + 1).unwrap_or(u8::MAX);
            containers.push(chest.to_core(K::Storage(n), storage_access[i].clone()));
        }

        let mut ships = Vec::new();
        for (i, ship) in ps
            .ship_ownership
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_real())
        {
            let index = u8::try_from(i).unwrap_or(u8::MAX);
            let primary = i == ps.primary_ship as usize;
            let access = vec![if primary {
                "with you".to_string()
            } else {
                "in the collection".to_string()
            }];
            containers.push(
                ship.inventory
                    .to_core(K::Ship { index, primary }, access.clone()),
            );
            containers.push(
                ship.inventory_cargo
                    .to_core(K::ShipCargo { index }, access.clone()),
            );
            containers.push(
                ship.inventory_tech_only
                    .to_core(K::ShipTech { index }, access),
            );
            let folder = ship
                .resource
                .filename
                .rsplit('/')
                .nth(1)
                .unwrap_or("")
                .to_string();
            ships.push(nms_core::ShipSummary {
                index,
                name: ship.name.clone(),
                ship_type: nms_core::ShipType::from_filename(&ship.resource.filename),
                type_raw: folder,
                class: nms_core::Grade::from_save_name(&ship.inventory.class.value),
                primary,
                general_slots: ship.inventory.unlocked(),
                cargo_slots: ship.inventory_cargo.unlocked(),
                tech_slots: ship.inventory_tech_only.unlocked(),
                tech_installed: u16::try_from(
                    ship.inventory.tech_count() + ship.inventory_tech_only.tech_count(),
                )
                .unwrap_or(u16::MAX),
                damage: ship.inventory.stat("^SHIP_DAMAGE"),
                shield: ship.inventory.stat("^SHIP_SHIELD"),
                hyperdrive: ship.inventory.stat("^SHIP_HYPERDRIVE"),
                agility: ship.inventory.stat("^SHIP_AGILE"),
            });
        }

        let mut exocraft = Vec::new();
        for (i, vehicle) in ps.vehicle_ownership.iter().enumerate() {
            let index = u8::try_from(i).unwrap_or(u8::MAX);
            let parked_at = (vehicle.location.0 != 0).then(|| {
                nms_core::GalacticAddress::from_save_ua(vehicle.location.0, reality_index)
            });
            let access: Vec<String> = parked_at
                .map(|addr| {
                    bases
                        .iter()
                        .filter(|b| {
                            is_planet_base(b)
                                && b.galactic_address.to_galactic_address(reality_index) == addr
                        })
                        .map(base_label)
                        .fold(Vec::new(), |mut acc, label| {
                            if !acc.contains(&label) {
                                acc.push(label);
                            }
                            acc
                        })
                })
                .unwrap_or_default();
            containers.push(vehicle.inventory.to_core(K::Exocraft { index }, access));
            exocraft.push(nms_core::VehicleSummary {
                index,
                name: vehicle.name.clone(),
                parked_at,
                slots: vehicle.inventory.unlocked(),
                tech_installed: u16::try_from(
                    vehicle.inventory.tech_count() + vehicle.inventory_tech_only.tech_count(),
                )
                .unwrap_or(u16::MAX),
            });
        }

        let mut multitools = Vec::new();
        for (i, tool) in ps
            .multitools
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_real())
        {
            let index = u8::try_from(i).unwrap_or(u8::MAX);
            let active = i == ps.active_multitool_index as usize;
            containers.push(tool.store.to_core(K::MultiTool { index, active }, vec![]));
            multitools.push(nms_core::MultiToolSummary {
                index,
                name: tool.name.clone(),
                class: nms_core::Grade::from_save_name(&tool.store.class.value),
                active,
                slots: tool.store.unlocked(),
                tech_installed: u16::try_from(tool.store.tech_count()).unwrap_or(u16::MAX),
                damage: tool.store.stat("^WEAPON_DAMAGE"),
                mining: tool.store.stat("^WEAPON_MINING"),
                scan: tool.store.stat("^WEAPON_SCAN"),
            });
        }

        // Machine buffers list `^MAINT_*` placeholders at amount 0 for their empty cells; only what is actually in them counts.
        for (i, machine) in ps.refiner_buffer_data.iter().enumerate() {
            let mut buffer = machine.inventory_container.to_core(
                K::MachineBuffer {
                    index: u8::try_from(i).unwrap_or(u8::MAX),
                },
                vec![],
            );
            buffer
                .stacks
                .retain(|s| s.amount > 0 && !s.id.bare().starts_with("MAINT_"));
            if !buffer.stacks.is_empty() {
                containers.push(buffer);
            }
        }

        containers.push(
            ps.corvette_storage_inventory
                .to_core(K::CorvetteParts, vec![]),
        );
        // `ChestMagicInventory` and `ChestMagic2Inventory` are not read: nobody knows what they are, so their contents stay out of every total.
        for (key, grid) in [
            (
                "CookingIngredientsInventory",
                &ps.cooking_ingredients_inventory,
            ),
            ("FishBaitBoxInventory", &ps.fish_bait_box_inventory),
            ("FoodUnitInventory", &ps.food_unit_inventory),
            ("RocketLockerInventory", &ps.rocket_locker_inventory),
            ("GraveInventory", &ps.grave_inventory),
        ] {
            containers.push(grid.to_core(K::Other(key.to_string()), vec![]));
        }

        // Grids the player does not have (nothing unlocked, nothing in them) are left out.
        containers.retain(|c| c.exists());

        let mut holdings = nms_core::Holdings {
            containers,
            ships,
            exocraft,
            multitools,
        };
        holdings.sort();
        holdings
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

    fn fixture() -> SaveRoot {
        let json = include_str!("../../../data/test/multi_system_save.json");
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn holdings_from_fixture_containers_and_access() {
        use nms_core::ContainerKind as K;
        let h = fixture().to_core_holdings();
        let kinds: Vec<&K> = h.containers.iter().map(|c| &c.kind).collect();
        assert!(kinds.contains(&&K::Exosuit));
        assert!(
            !kinds.contains(&&K::ExosuitCargo),
            "nothing unlocked, so the grid is left out"
        );
        assert!(kinds.contains(&&K::CorvetteParts));
        assert_eq!(
            kinds.len(),
            17,
            "no container for the unidentified ChestMagic grids the fixture carries"
        );
        assert!(kinds.contains(&&K::Other("CookingIngredientsInventory".into())));
        assert_eq!(
            kinds
                .iter()
                .filter(|k| matches!(k, K::MultiTool { .. }))
                .count(),
            1,
            "WeaponInventory is not a second container"
        );
        assert_eq!(
            kinds.iter().filter(|k| matches!(k, K::Ship { .. })).count(),
            2,
            "the empty ship slot is skipped"
        );

        let suit = h.exosuit().unwrap();
        assert_eq!((suit.width, suit.height, suit.unlocked_slots), (10, 12, 93));
        assert_eq!(suit.stacks.len(), 4);
        assert_eq!(
            suit.countable().count(),
            3,
            "the installed jetpack is not counted"
        );
        assert_eq!(suit.access, vec!["with you".to_string()]);

        let storage1 = h.container(&K::Storage(1)).unwrap();
        assert_eq!(
            storage1.access,
            vec![
                "Lush Haven".to_string(),
                "Frost Outpost".to_string(),
                "Home Freighter".to_string()
            ]
        );
        let storage2 = h.container(&K::Storage(2)).unwrap();
        assert_eq!(
            storage2.access,
            vec!["Home Freighter".to_string()],
            "only the freighter's room 1 opens container 2"
        );

        assert_eq!(
            h.total(&nms_core::ItemId::new("^ASTEROID2")),
            11_297,
            "the 15 Gold in ChestMagicInventory are not counted"
        );
        assert_eq!(h.total(&nms_core::ItemId::new("^ASTEROID1")), 2_959);
        assert_eq!(h.total(&nms_core::ItemId::new("^ASTEROID3")), 2_994);

        let machine = h.container(&K::MachineBuffer { index: 0 }).unwrap();
        assert_eq!(
            machine.stacks.len(),
            1,
            "the ^MAINT_ placeholder at amount 0 is dropped"
        );
        assert_eq!(machine.stacks[0].amount, 180);
    }

    #[test]
    fn holdings_from_fixture_ships_exocraft_and_tools() {
        let h = fixture().to_core_holdings();
        assert_eq!(h.ships.len(), 2);
        let primary = h.primary_ship().unwrap();
        assert_eq!(primary.index, 1);
        assert_eq!(primary.name, "Starbird");
        assert_eq!(primary.ship_type, Some(nms_core::ShipType::Exotic));
        assert_eq!(primary.class, Some(nms_core::Grade::S));
        assert_eq!(
            (
                primary.general_slots,
                primary.cargo_slots,
                primary.tech_slots
            ),
            (31, 0, 30)
        );
        assert_eq!(primary.tech_installed, 2);
        assert_eq!(primary.hyperdrive, 90.0);
        assert_eq!(h.ships[0].ship_type, Some(nms_core::ShipType::Fighter));
        assert!(!h.ships[0].primary);

        assert_eq!(h.exocraft.len(), 2);
        assert!(h.exocraft[0].parked_at.is_some());
        assert_eq!(h.exocraft[0].tech_installed, 2);
        assert!(h.exocraft[1].parked_at.is_none());
        let parked = h
            .container(&nms_core::ContainerKind::Exocraft { index: 0 })
            .unwrap();
        assert_eq!(
            parked.access,
            vec!["Lush Haven".to_string()],
            "the freighter at the same address is not a parking place"
        );

        assert_eq!(h.multitools.len(), 1);
        assert!(h.multitools[0].active);
        assert_eq!(h.multitools[0].slots, 24);
        assert_eq!(h.multitools[0].tech_installed, 3);
        assert_eq!(h.multitools[0].scan, 100.0);
    }

    #[test]
    fn holdings_from_empty_player_state_is_empty() {
        let save: SaveRoot = serde_json::from_str(r#"{"Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main", "CommonStateData": {"SaveName": "t"}, "BaseContext": {"GameMode": 1, "PlayerStateData": {}}, "DiscoveryManagerData": {"DiscoveryData-v1": {"Store": {"Record": []}}}}"#).unwrap();
        let h = save.to_core_holdings();
        assert!(h.containers.is_empty());
        assert!(h.ships.is_empty());
        assert!(h.exocraft.is_empty());
        assert!(h.multitools.is_empty());
    }

    #[test]
    fn negative_amounts_clamp_to_zero() {
        let slot: InventorySlot = serde_json::from_str(r#"{"Id": "^ASTEROID2", "Amount": -5, "MaxAmount": 9999, "Index": {"X": 1, "Y": 2}, "Type": {"InventoryType": "Substance"}}"#).unwrap();
        let stack = slot.to_core();
        assert_eq!(stack.amount, 0);
        assert_eq!(stack.slot, (1, 2));
        assert_eq!(stack.kind, Some(nms_core::ItemKind::Substance));
    }

    #[test]
    fn storage_numbers_from_object_ids() {
        assert_eq!(storage_number("^CONTAINER0"), Some(1));
        assert_eq!(storage_number("^CONTAINER9"), Some(10));
        assert_eq!(storage_number("^FRE_ROOM_STORE3"), Some(4));
        assert_eq!(storage_number("^CONTAINER"), None);
        assert_eq!(storage_number("^SNOWPLANT"), None);
    }

    #[test]
    fn player_ship_base_type_is_kept() {
        let json = r#"{"Name": "Default", "BaseType": {"PersistentBaseTypes": "PlayerShipBase"}, "GalacticAddress": "0x00100000000064", "Position": [0.0, 0.0, 0.0], "Owner": {"UID": ""}, "Objects": []}"#;
        let base: PersistentPlayerBase = serde_json::from_str(json).unwrap();
        assert_eq!(
            base.to_core_base().base_type,
            nms_core::BaseType::PlayerShipBase
        );
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
