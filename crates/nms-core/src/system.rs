use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::address::GalacticAddress;
use crate::biome::{Biome, BiomeSubType};

/// Unique identifier for a star system.
///
/// The low 48 bits are the packed galactic address with the planet index zeroed
/// out (bits 47-44 cleared); bits 55-48 are the galaxy. Two systems at the same
/// voxel coordinates but different SSI values, or the same address in two
/// galaxies, get different IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SystemId(pub u64);

impl SystemId {
    /// Create from a `GalacticAddress` by zeroing the planet index bits and adding the galaxy.
    pub fn from_address(addr: &GalacticAddress) -> Self {
        Self::new(addr.packed(), addr.reality_index)
    }

    /// Create from a packed 48-bit address (planet bits are ignored) and a galaxy.
    pub fn new(packed: u64, reality_index: u8) -> Self {
        SystemId((packed & 0x0FFF_FFFF_FFFF) | ((reality_index as u64) << 48))
    }
}

/// A star system containing one or more planets.
///
/// The galaxy (reality index) is encoded in the `address` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct System {
    pub address: GalacticAddress,
    pub name: Option<String>,
    pub discoverer: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub planets: Vec<Planet>,
}

impl System {
    pub fn new(
        address: GalacticAddress,
        name: Option<String>,
        discoverer: Option<String>,
        timestamp: Option<DateTime<Utc>>,
        planets: Vec<Planet>,
    ) -> Self {
        Self {
            address,
            name,
            discoverer,
            timestamp,
            planets,
        }
    }

    /// Galaxy index (convenience accessor for `address.reality_index`).
    pub fn reality_index(&self) -> u8 {
        self.address.reality_index
    }
}

/// What the player has scanned on a planet: one discovery record per species, plant, or mineral. The planet's totals are not in the save, so these say how much is recorded, not how much is left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Scanned {
    pub fauna: u16,
    pub flora: u16,
    pub minerals: u16,
}

impl Scanned {
    pub fn new(fauna: u16, flora: u16, minerals: u16) -> Self {
        Self {
            fauna,
            flora,
            minerals,
        }
    }

    /// Nothing recorded at all.
    pub fn is_empty(&self) -> bool {
        self.fauna == 0 && self.flora == 0 && self.minerals == 0
    }
}

/// A planet within a star system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct Planet {
    /// Planet index within the system (0-15).
    pub index: u8,
    pub biome: Option<Biome>,
    pub biome_subtype: Option<BiomeSubType>,
    /// Whether the planet has infested (biological horror) variant.
    pub infested: bool,
    pub name: Option<String>,
    /// Procedural generation seed.
    pub seed_hash: Option<u64>,
    /// Species, plants, and minerals the player has recorded here.
    pub scanned: Scanned,
}

impl Planet {
    pub fn new(
        index: u8,
        biome: Option<Biome>,
        biome_subtype: Option<BiomeSubType>,
        infested: bool,
        name: Option<String>,
        seed_hash: Option<u64>,
    ) -> Self {
        Self {
            index,
            biome,
            biome_subtype,
            infested,
            name,
            seed_hash,
            scanned: Scanned::default(),
        }
    }

    /// The same planet with its scanned counts set.
    pub fn with_scanned(mut self, scanned: Scanned) -> Self {
        self.scanned = scanned;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_reality_index_from_address() {
        let addr = GalacticAddress::new(0, 0, 0, 0x123, 0, 42);
        let sys = System::new(addr, None, None, None, vec![]);
        assert_eq!(sys.reality_index(), 42);
    }

    #[test]
    fn planet_constructor() {
        let p = Planet::new(3, Some(Biome::Lush), None, false, Some("Eden".into()), None);
        assert_eq!(p.index, 3);
        assert_eq!(p.biome, Some(Biome::Lush));
        assert!(!p.infested);
    }
}
