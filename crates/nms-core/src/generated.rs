//! What the game generates from an address alone: the region and system names and the
//! properties the galaxy map shows on hover.
//!
//! None of this is in the save. An [`AddressGenerator`] computes it, the model keeps it beside
//! the systems it describes, and the display shows it where the save has nothing to say.
//! The category numbers decoded here are the `nms_namegen` library's convention (see
//! `docs/plans/project03-save-tooling/0049-generated-galaxy.md`).

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::address::GalacticAddress;

/// A star's colour class, which fixes the drive needed to reach it and the metals on its planets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StarColour {
    Yellow,
    Green,
    Blue,
    Red,
    Purple,
}

impl StarColour {
    /// From the library's `star_type` (0 yellow, 1 green, 2 blue, 3 red, 4 purple).
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            0 => Self::Yellow,
            1 => Self::Green,
            2 => Self::Blue,
            3 => Self::Red,
            4 => Self::Purple,
            _ => return None,
        })
    }
}

impl fmt::Display for StarColour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Red => "Red",
            Self::Purple => "Purple",
        })
    }
}

/// A system's economy category, as the galaxy map names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Economy {
    Trading,
    AdvancedMaterials,
    Scientific,
    Mining,
    Manufacturing,
    Technology,
    PowerGeneration,
}

impl Economy {
    /// From the library's `economy_type` (1 to 7 in the order of the variants).
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            1 => Self::Trading,
            2 => Self::AdvancedMaterials,
            3 => Self::Scientific,
            4 => Self::Mining,
            5 => Self::Manufacturing,
            6 => Self::Technology,
            7 => Self::PowerGeneration,
            _ => return None,
        })
    }
}

impl fmt::Display for Economy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Trading => "Trading",
            Self::AdvancedMaterials => "Advanced Materials",
            Self::Scientific => "Scientific",
            Self::Mining => "Mining",
            Self::Manufacturing => "Manufacturing",
            Self::Technology => "Technology",
            Self::PowerGeneration => "Power Generation",
        })
    }
}

/// A three-step tier, used for both wealth and conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Tier {
    Low,
    Medium,
    High,
}

impl Tier {
    /// From the library's 1 low, 2 medium, 3 high.
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            1 => Self::Low,
            2 => Self::Medium,
            3 => Self::High,
            _ => return None,
        })
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        })
    }
}

/// The race that holds a system's station.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Race {
    Gek,
    Korvax,
    Vykeen,
}

impl Race {
    /// From the library's 1 Gek, 2 Korvax, 3 Vy'keen; 0 is an uncharted system with no race.
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            1 => Self::Gek,
            2 => Self::Korvax,
            3 => Self::Vykeen,
            _ => return None,
        })
    }
}

impl fmt::Display for Race {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Gek => "Gek",
            Self::Korvax => "Korvax",
            Self::Vykeen => "Vy'keen",
        })
    }
}

/// The properties the galaxy map shows for a system on hover, generated from its address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SystemAttributes {
    pub star: StarColour,
    pub economy: Economy,
    pub wealth: Tier,
    pub conflict: Tier,
    /// `None` for an uncharted system.
    pub race: Option<Race>,
    pub uncharted: bool,
    pub abandoned: bool,
    /// Unvalidated upstream: the community corpus carries no pirate field.
    pub pirate: bool,
    /// Bodies the map draws as planets and as moons.
    pub planets: u8,
    pub moons: u8,
    pub gas_giant: bool,
}

impl SystemAttributes {
    /// Build from the library's category numbers; `None` when any is outside its range,
    /// so a stale mapping never shows as a wrong label.
    #[allow(clippy::too_many_arguments)]
    pub fn from_codes(
        star: i64,
        economy: i64,
        wealth: i64,
        conflict: i64,
        race: i64,
        uncharted: bool,
        abandoned: bool,
        pirate: bool,
        planets: i64,
        moons: i64,
        gas_giant: bool,
    ) -> Option<Self> {
        let race = if uncharted && race == 0 {
            None
        } else {
            Some(Race::from_code(race)?)
        };
        Some(Self {
            star: StarColour::from_code(star)?,
            economy: Economy::from_code(economy)?,
            wealth: Tier::from_code(wealth)?,
            conflict: Tier::from_code(conflict)?,
            race,
            uncharted,
            abandoned,
            pirate,
            planets: u8::try_from(planets).ok()?,
            moons: u8::try_from(moons).ok()?,
            gas_giant,
        })
    }
}

/// Everything generated for one system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Generated {
    /// The region's name as the galaxy map shows it.
    pub region: String,
    /// The system's generated name, which the game shows unless the player renamed it.
    pub name: String,
    /// `None` when the generator gave no attributes or gave ones this crate cannot decode.
    pub attributes: Option<SystemAttributes>,
}

impl Generated {
    pub fn new(region: String, name: String, attributes: Option<SystemAttributes>) -> Self {
        Self {
            region,
            name,
            attributes,
        }
    }
}

/// A source of generated data for addresses.
///
/// The answer is a pure function of the address and its galaxy, so an implementation
/// may cache without limit. An address the implementation cannot answer is left out.
pub trait AddressGenerator {
    fn generate(&self, addresses: &[GalacticAddress]) -> HashMap<GalacticAddress, Generated>;
}

/// A generator over a fixed table, for tests and fixtures.
#[derive(Debug, Default, Clone)]
pub struct TableGenerator {
    pub table: HashMap<GalacticAddress, Generated>,
}

impl AddressGenerator for TableGenerator {
    fn generate(&self, addresses: &[GalacticAddress]) -> HashMap<GalacticAddress, Generated> {
        addresses
            .iter()
            .filter_map(|a| self.table.get(a).map(|g| (*a, g.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_codes_valid_maps_every_category() {
        let a =
            SystemAttributes::from_codes(2, 6, 3, 1, 2, false, false, false, 4, 1, false).unwrap();
        assert_eq!(a.star, StarColour::Blue);
        assert_eq!(a.economy, Economy::Technology);
        assert_eq!(a.wealth, Tier::High);
        assert_eq!(a.conflict, Tier::Low);
        assert_eq!(a.race, Some(Race::Korvax));
        assert_eq!((a.planets, a.moons), (4, 1));
    }

    #[test]
    fn test_from_codes_uncharted_has_no_race() {
        let a =
            SystemAttributes::from_codes(0, 1, 1, 1, 0, true, false, false, 2, 0, false).unwrap();
        assert!(a.uncharted);
        assert_eq!(a.race, None);
    }

    #[test]
    fn test_from_codes_out_of_range_is_none() {
        assert!(
            SystemAttributes::from_codes(5, 1, 1, 1, 1, false, false, false, 1, 0, false).is_none()
        );
        assert!(
            SystemAttributes::from_codes(0, 8, 1, 1, 1, false, false, false, 1, 0, false).is_none()
        );
        assert!(
            SystemAttributes::from_codes(0, 1, 1, 1, 0, false, false, false, 1, 0, false).is_none()
        );
    }

    #[test]
    fn test_table_generator_leaves_unknown_out() {
        let addr = GalacticAddress::new(-532, -4, -1706, 0x43, 0, 0);
        let other = GalacticAddress::new(-532, -4, -1706, 0x44, 0, 0);
        let mut table = HashMap::new();
        table.insert(
            addr,
            Generated::new("Piponera Anomaly".into(), "Lauderen".into(), None),
        );
        let generator = TableGenerator { table };
        let out = generator.generate(&[addr, other]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[&addr].name, "Lauderen");
    }

    #[test]
    fn test_display_names_match_the_map() {
        assert_eq!(Economy::AdvancedMaterials.to_string(), "Advanced Materials");
        assert_eq!(Race::Vykeen.to_string(), "Vy'keen");
        assert_eq!(Tier::Medium.to_string(), "Medium");
    }
}
