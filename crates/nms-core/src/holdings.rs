//! What the player owns: every inventory grid, the ships, the exocraft, and the multi-tools, decoded from the save.
//!
//! The save lists each grid's occupied slots with the item's internal ID, amount, and stack limit, and the unlocked cells in `ValidSlotIndices`; a grid's width and height are its shape, not its capacity. Technology slots carry charge rather than a count and are never totalled. The unidentified `ChestMagicInventory` grids are left out altogether until someone learns what they are. The decode rules and their evidence are in `docs/reference/nms-save-notes.md`, section 8; item names come from [`crate::items`].

use serde::{Deserialize, Serialize};

use crate::address::GalacticAddress;
use crate::fleet::Grade;
use crate::items::display_name;

/// An item's internal ID as the save writes it, with the leading caret (`^ASTEROID2`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ItemId(pub String);

impl ItemId {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        if id.starts_with('^') {
            Self(id)
        } else {
            Self(format!("^{id}"))
        }
    }

    /// The ID without its caret, as the name table and the API key it.
    pub fn bare(&self) -> &str {
        self.0.strip_prefix('^').unwrap_or(&self.0)
    }

    /// Whether this is a procedurally generated item: a binary key the reader could not decode as text, followed by `#` and the seed.
    pub fn is_procedural(&self) -> bool {
        self.0.contains('\u{FFFD}') || self.0.contains('#')
    }

    /// The ID as tables show it: the bare ID, or `#<seed>` for a procedural item whose key is binary.
    pub fn display_id(&self) -> String {
        if self.is_procedural() {
            format!("#{}", self.0.rsplit('#').next().unwrap_or(""))
        } else {
            self.bare().to_string()
        }
    }

    /// The display name; a procedural item is `Procedural item #<seed>`, and an ID the table has no name for is shown bare.
    pub fn name(&self) -> String {
        if self.is_procedural() {
            let seed = self.0.rsplit('#').next().unwrap_or("");
            return format!("Procedural item #{seed}");
        }
        display_name(self.bare())
    }

    /// Whether the ID or its name contains `pattern`, case-insensitively.
    pub fn matches(&self, pattern: &str) -> bool {
        let pattern = pattern.trim().trim_start_matches('^').to_lowercase();
        if pattern.is_empty() {
            return false;
        }
        self.bare().to_lowercase().contains(&pattern)
            || self.name().to_lowercase().contains(&pattern)
    }
}

/// What a slot holds: a raw substance, a crafted product, or an installed technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum ItemKind {
    Substance,
    Product,
    Technology,
}

impl ItemKind {
    pub fn from_save_name(name: &str) -> Option<Self> {
        match name {
            "Substance" => Some(Self::Substance),
            "Product" => Some(Self::Product),
            "Technology" => Some(Self::Technology),
            _ => None,
        }
    }

    /// Parse a user's filter word: `substance`, `product`, `technology`, or their first letters.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "substance" | "substances" | "s" | "raw" => Some(Self::Substance),
            "product" | "products" | "p" => Some(Self::Product),
            "technology" | "tech" | "t" => Some(Self::Technology),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Substance => "Substance",
            Self::Product => "Product",
            Self::Technology => "Technology",
        }
    }
}

/// One occupied slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ItemStack {
    pub id: ItemId,
    pub kind: Option<ItemKind>,
    /// A count for substances and products; charge or condition for technology.
    pub amount: u32,
    /// The stack limit in this grid.
    pub max: u32,
    /// Column and row.
    pub slot: (u8, u8),
}

impl ItemStack {
    /// Whether the stack counts toward holdings: anything that is not technology.
    pub fn is_countable(&self) -> bool {
        self.kind != Some(ItemKind::Technology)
    }

    pub fn name(&self) -> String {
        self.id.name()
    }
}

/// A ship's archetype, from the folder in its `Resource.Filename`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum ShipType {
    Fighter,
    Hauler,
    Explorer,
    Solar,
    Exotic,
    Living,
}

impl ShipType {
    /// The type from a scene path such as `MODELS/COMMON/SPACECRAFT/FIGHTERS/FIGHTER_PROC.SCENE.MBIN`.
    pub fn from_filename(path: &str) -> Option<Self> {
        let mut segments = path.rsplit('/');
        segments.next()?;
        match segments.next()? {
            "FIGHTERS" => Some(Self::Fighter),
            "DROPSHIPS" => Some(Self::Hauler),
            "SCIENTIFIC" => Some(Self::Explorer),
            "SAILSHIP" => Some(Self::Solar),
            "S-CLASS" => Some(Self::Exotic),
            "BIGGS" => Some(Self::Living),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Fighter => "Fighter",
            Self::Hauler => "Hauler",
            Self::Explorer => "Explorer",
            Self::Solar => "Solar",
            Self::Exotic => "Exotic",
            Self::Living => "Living ship",
        }
    }
}

/// Which grid a container is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum ContainerKind {
    Exosuit,
    ExosuitCargo,
    ExosuitTech,
    Freighter,
    FreighterCargo,
    FreighterTech,
    /// One of the ten numbered storage containers, 1 to 10.
    Storage(u8),
    /// A ship's general grid; `index` is its 0-based position in the save's ship list.
    Ship {
        index: u8,
        primary: bool,
    },
    ShipCargo {
        index: u8,
    },
    ShipTech {
        index: u8,
    },
    Exocraft {
        index: u8,
    },
    /// A multi-tool's grid; `WeaponInventory` is a copy of the active one and is not read.
    MultiTool {
        index: u8,
        active: bool,
    },
    /// One entry of `RefinerBufferData`: the buffer of a placed machine with a maintenance grid (a refiner, a harvester), holding whatever sits in it. Its capacity is not recorded.
    MachineBuffer {
        index: u8,
    },
    /// `CorvetteStorageInventory`, the corvette parts.
    CorvetteParts,
    /// A minor grid named by its save key (`CookingIngredientsInventory`, ...).
    Other(String),
}

impl ContainerKind {
    /// The name the tables and the `inventory` filter use.
    pub fn label(&self) -> String {
        match self {
            Self::Exosuit => "Exosuit".to_string(),
            Self::ExosuitCargo => "Exosuit cargo".to_string(),
            Self::ExosuitTech => "Exosuit technology".to_string(),
            Self::Freighter => "Freighter".to_string(),
            Self::FreighterCargo => "Freighter cargo".to_string(),
            Self::FreighterTech => "Freighter technology".to_string(),
            Self::Storage(n) => format!("Storage {n}"),
            Self::Ship { index, .. } => format!("Ship {}", index + 1),
            Self::ShipCargo { index } => format!("Ship {} cargo", index + 1),
            Self::ShipTech { index } => format!("Ship {} technology", index + 1),
            Self::Exocraft { index } => format!("Exocraft {}", index + 1),
            Self::MultiTool { index, .. } => format!("Multi-tool {}", index + 1),
            Self::MachineBuffer { index } => format!("Machine {}", index + 1),
            Self::CorvetteParts => "Corvette parts".to_string(),
            Self::Other(key) => other_label(key),
        }
    }

    /// Whether the grid's capacity is unknown: machine buffers carry no `ValidSlotIndices`.
    pub fn has_capacity(&self) -> bool {
        !matches!(self, Self::MachineBuffer { .. })
    }

    /// Whether the grid holds only installed technology.
    pub fn is_technology(&self) -> bool {
        matches!(
            self,
            Self::ExosuitTech
                | Self::FreighterTech
                | Self::ShipTech { .. }
                | Self::MultiTool { .. }
        )
    }

    /// Sort key: the player's own gear first, then storage, ships, vehicles, tools, and the rest.
    fn order(&self) -> (u8, u8) {
        match self {
            Self::Exosuit => (0, 0),
            Self::ExosuitCargo => (0, 1),
            Self::ExosuitTech => (0, 2),
            Self::Freighter => (1, 0),
            Self::FreighterCargo => (1, 1),
            Self::FreighterTech => (1, 2),
            Self::Storage(n) => (2, *n),
            Self::Ship { index, .. } => (3, index * 3),
            Self::ShipCargo { index } => (3, index * 3 + 1),
            Self::ShipTech { index } => (3, index * 3 + 2),
            Self::Exocraft { index } => (4, *index),
            Self::MultiTool { index, .. } => (5, *index),
            Self::CorvetteParts => (6, 0),
            Self::MachineBuffer { index } => (7, *index),
            Self::Other(_) => (9, 0),
        }
    }
}

/// A readable name for a minor grid's save key: `CookingIngredientsInventory` becomes `Cooking ingredients`.
fn other_label(key: &str) -> String {
    let stem = key.strip_suffix("Inventory").unwrap_or(key);
    let mut out = String::new();
    for (i, c) in stem.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// One inventory grid and what is in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Container {
    pub kind: ContainerKind,
    pub class: Option<Grade>,
    pub width: u8,
    pub height: u8,
    /// Cells the player has unlocked, the length of `ValidSlotIndices`; zero is a real answer.
    pub unlocked_slots: u16,
    /// Occupied slots in save order.
    pub stacks: Vec<ItemStack>,
    /// Where the grid can be opened: the bases that place a storage container, the base an exocraft is parked at, "with you" for the primary ship.
    pub access: Vec<String>,
}

impl Container {
    pub fn label(&self) -> String {
        self.kind.label()
    }

    pub fn occupied(&self) -> u16 {
        u16::try_from(self.stacks.len()).unwrap_or(u16::MAX)
    }

    pub fn free(&self) -> u16 {
        self.unlocked_slots.saturating_sub(self.occupied())
    }

    /// Whether every unlocked slot is taken; a grid with nothing unlocked is not full, it is absent.
    pub fn is_full(&self) -> bool {
        self.unlocked_slots > 0 && self.free() == 0
    }

    /// Whether the player has this grid at all.
    pub fn exists(&self) -> bool {
        self.unlocked_slots > 0 || !self.stacks.is_empty()
    }

    /// `occupied / unlocked`, or the count alone when the capacity is unknown.
    pub fn used_label(&self) -> String {
        if self.kind.has_capacity() {
            format!("{} / {}", self.occupied(), self.unlocked_slots)
        } else {
            self.occupied().to_string()
        }
    }

    /// Free slots as a cell: `-` for technology grids and grids with no known capacity.
    pub fn free_label(&self) -> String {
        if self.kind.is_technology() || !self.kind.has_capacity() {
            "-".to_string()
        } else {
            self.free().to_string()
        }
    }

    /// Stacks that count toward holdings.
    pub fn countable(&self) -> impl Iterator<Item = &ItemStack> {
        self.stacks.iter().filter(|s| s.is_countable())
    }

    pub fn class_label(&self) -> &str {
        self.class.map(Grade::display_name).unwrap_or("-")
    }
}

/// One owned starship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ShipSummary {
    /// 0-based position in the save's ship list.
    pub index: u8,
    /// Player-given name; empty unless renamed.
    pub name: String,
    pub ship_type: Option<ShipType>,
    /// The scene folder when the type is not recognised.
    pub type_raw: String,
    pub class: Option<Grade>,
    pub primary: bool,
    pub general_slots: u16,
    pub cargo_slots: u16,
    pub tech_slots: u16,
    /// Installed technologies across the general and technology grids.
    pub tech_installed: u16,
    /// Class bonuses as the save's percentages: damage, shield, hyperdrive, agility.
    pub damage: f32,
    pub shield: f32,
    pub hyperdrive: f32,
    pub agility: f32,
}

impl ShipSummary {
    /// The player's name for the ship, or `Ship N`.
    pub fn label(&self) -> String {
        if self.name.is_empty() {
            format!("Ship {}", self.index + 1)
        } else {
            self.name.clone()
        }
    }

    pub fn type_label(&self) -> &str {
        match self.ship_type {
            Some(t) => t.display_name(),
            None if self.type_raw.is_empty() => "?",
            None => &self.type_raw,
        }
    }

    pub fn class_label(&self) -> &str {
        self.class.map(Grade::display_name).unwrap_or("-")
    }

    /// Where the ship is, as far as the save proves.
    pub fn location(&self) -> &'static str {
        if self.primary {
            "with you"
        } else {
            "in the collection"
        }
    }
}

/// One owned exocraft. The save carries no type field; the entry's position in the list is presumed to fix the kind, which is unverified, so it is labelled by index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct VehicleSummary {
    pub index: u8,
    pub name: String,
    /// The base the exocraft is parked at; `None` when the save holds 0.
    pub parked_at: Option<GalacticAddress>,
    pub slots: u16,
    pub tech_installed: u16,
}

impl VehicleSummary {
    pub fn label(&self) -> String {
        if self.name.is_empty() {
            format!("Exocraft {}", self.index + 1)
        } else {
            self.name.clone()
        }
    }
}

/// One owned multi-tool. Its type (pistol, rifle, ...) is not in the live record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct MultiToolSummary {
    pub index: u8,
    pub name: String,
    pub class: Option<Grade>,
    pub active: bool,
    pub slots: u16,
    pub tech_installed: u16,
    /// Class bonuses as the save's percentages.
    pub damage: f32,
    pub mining: f32,
    pub scan: f32,
}

impl MultiToolSummary {
    pub fn label(&self) -> String {
        if self.name.is_empty() {
            format!("Multi-tool {}", self.index + 1)
        } else {
            self.name.clone()
        }
    }

    pub fn class_label(&self) -> &str {
        self.class.map(Grade::display_name).unwrap_or("-")
    }
}

/// Everything the player owns, from one save.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Holdings {
    /// Every grid, in display order.
    pub containers: Vec<Container>,
    pub ships: Vec<ShipSummary>,
    pub exocraft: Vec<VehicleSummary>,
    pub multitools: Vec<MultiToolSummary>,
}

impl Holdings {
    /// Sort the containers into display order; the save conversion calls this once.
    pub fn sort(&mut self) {
        self.containers.sort_by_key(|c| c.kind.order());
    }

    pub fn container(&self, kind: &ContainerKind) -> Option<&Container> {
        self.containers.iter().find(|c| &c.kind == kind)
    }

    /// Every countable stack with the container it sits in.
    pub fn stacks(&self) -> impl Iterator<Item = (&Container, &ItemStack)> {
        self.containers
            .iter()
            .flat_map(|c| c.countable().map(move |s| (c, s)))
    }

    /// The countable total of one item across every grid.
    pub fn total(&self, id: &ItemId) -> u64 {
        self.stacks()
            .filter(|(_, s)| &s.id == id)
            .map(|(_, s)| u64::from(s.amount))
            .sum()
    }

    /// Storage containers with no free slot.
    pub fn full_storage(&self) -> Vec<&Container> {
        self.containers
            .iter()
            .filter(|c| matches!(c.kind, ContainerKind::Storage(_)) && c.is_full())
            .collect()
    }

    /// The exosuit general grid.
    pub fn exosuit(&self) -> Option<&Container> {
        self.container(&ContainerKind::Exosuit)
    }

    pub fn primary_ship(&self) -> Option<&ShipSummary> {
        self.ships.iter().find(|s| s.primary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str, kind: ItemKind, amount: u32) -> ItemStack {
        ItemStack {
            id: ItemId::new(id),
            kind: Some(kind),
            amount,
            max: 9999,
            slot: (0, 0),
        }
    }

    #[test]
    fn item_id_keeps_one_caret() {
        assert_eq!(ItemId::new("ASTEROID2").0, "^ASTEROID2");
        assert_eq!(ItemId::new("^ASTEROID2").0, "^ASTEROID2");
        assert_eq!(ItemId::new("^ASTEROID2").bare(), "ASTEROID2");
    }

    #[test]
    fn item_id_matches_name_or_id() {
        let gold = ItemId::new("^ASTEROID2");
        assert!(gold.matches("gold"));
        assert!(gold.matches("GOLD"));
        assert!(gold.matches("asteroid2"));
        assert!(gold.matches("^ASTEROID"));
        assert!(!gold.matches("silver"));
        assert!(!gold.matches(""));
    }

    #[test]
    fn unknown_id_falls_back_to_bare_id() {
        assert_eq!(ItemId::new("^NOT_AN_ITEM_XYZ").name(), "NOT_AN_ITEM_XYZ");
    }

    #[test]
    fn procedural_ids_are_labelled() {
        let id = ItemId::new("^\u{FFFD}\u{FFFD}GLX#62054");
        assert!(id.is_procedural());
        assert_eq!(id.name(), "Procedural item #62054");
        assert!(id.matches("procedural"));
        assert!(!ItemId::new("^ASTEROID2").is_procedural());
    }

    #[test]
    fn ship_type_from_filename() {
        assert_eq!(
            ShipType::from_filename("MODELS/COMMON/SPACECRAFT/FIGHTERS/FIGHTER_PROC.SCENE.MBIN"),
            Some(ShipType::Fighter)
        );
        assert_eq!(
            ShipType::from_filename("MODELS/COMMON/SPACECRAFT/BIGGS/BIGGS.SCENE.MBIN"),
            Some(ShipType::Living)
        );
        assert_eq!(
            ShipType::from_filename("MODELS/COMMON/SPACECRAFT/S-CLASS/S-CLASS_PROC.SCENE.MBIN"),
            Some(ShipType::Exotic)
        );
        assert_eq!(ShipType::from_filename(""), None);
        assert_eq!(ShipType::from_filename("FIGHTER.MBIN"), None);
    }

    #[test]
    fn free_slots_come_from_unlocked_not_shape() {
        let cargo = Container {
            kind: ContainerKind::ExosuitCargo,
            class: None,
            width: 7,
            height: 5,
            unlocked_slots: 0,
            stacks: vec![],
            access: vec![],
        };
        assert_eq!(cargo.free(), 0);
        assert!(!cargo.is_full());
        assert!(!cargo.exists());
        let full = Container {
            kind: ContainerKind::Storage(1),
            class: Some(Grade::C),
            width: 10,
            height: 6,
            unlocked_slots: 2,
            stacks: vec![
                stack("^ASTEROID2", ItemKind::Substance, 9999),
                stack("^ASTEROID2", ItemKind::Substance, 927),
            ],
            access: vec![],
        };
        assert_eq!(full.free(), 0);
        assert!(full.is_full());
    }

    #[test]
    fn totals_skip_technology() {
        let mut holdings = Holdings::default();
        holdings.containers.push(Container {
            kind: ContainerKind::ExosuitTech,
            class: None,
            width: 10,
            height: 6,
            unlocked_slots: 60,
            stacks: vec![stack("^ASTEROID2", ItemKind::Technology, 100)],
            access: vec![],
        });
        holdings.containers.push(Container {
            kind: ContainerKind::Exosuit,
            class: None,
            width: 10,
            height: 12,
            unlocked_slots: 93,
            stacks: vec![stack("^ASTEROID2", ItemKind::Substance, 371)],
            access: vec![],
        });
        holdings.containers.push(Container {
            kind: ContainerKind::Storage(1),
            class: None,
            width: 10,
            height: 6,
            unlocked_slots: 50,
            stacks: vec![
                stack("^ASTEROID2", ItemKind::Substance, 9999),
                stack("^ASTEROID2", ItemKind::Substance, 927),
            ],
            access: vec![],
        });
        assert_eq!(holdings.total(&ItemId::new("^ASTEROID2")), 11_297);
        holdings.sort();
        assert_eq!(holdings.containers[0].kind, ContainerKind::Exosuit);
        assert_eq!(holdings.containers[1].kind, ContainerKind::ExosuitTech);
        assert_eq!(holdings.containers[2].kind, ContainerKind::Storage(1));
    }

    #[test]
    fn labels() {
        assert_eq!(ContainerKind::Storage(10).label(), "Storage 10");
        assert_eq!(
            ContainerKind::Ship {
                index: 2,
                primary: false
            }
            .label(),
            "Ship 3"
        );
        assert_eq!(
            ContainerKind::Other("CookingIngredientsInventory".into()).label(),
            "Cooking ingredients"
        );
        assert_eq!(
            ContainerKind::Other("FishBaitBoxInventory".into()).label(),
            "Fish bait box"
        );
    }

    #[test]
    fn item_kind_parses_short_forms() {
        assert_eq!(ItemKind::parse("tech"), Some(ItemKind::Technology));
        assert_eq!(ItemKind::parse("Products"), Some(ItemKind::Product));
        assert_eq!(ItemKind::parse("s"), Some(ItemKind::Substance));
        assert_eq!(ItemKind::parse("ore"), None);
    }
}
