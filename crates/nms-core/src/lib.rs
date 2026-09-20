//! Core types, enums, address math, and portal glyph display for NMS Copilot.

pub mod address;
pub mod base;
pub mod biome;
pub mod delta;
pub mod discovery;
pub mod fleet;
pub mod galaxy;
pub mod generated;
pub mod glyph;
pub mod holdings;
pub mod items;
pub mod pipes;
pub mod player;
pub mod system;

pub use address::{AddressParseError, GalacticAddress, PortalAddress, PortalParseError};
pub use base::{
    BaseObjects, Battery, Crop, CropKind, Depot, Extractor, ExtractorKind, Generator,
    GeneratorKind, Network, RawBaseObject,
};
pub use biome::{Biome, BiomeParseError, BiomeSubType};
pub use delta::{PlayerMoved, SaveDelta};
pub use discovery::{Discovery, DiscoveryParseError, DiscoveryRecord};
pub use fleet::{
    DurationClass, Event, Expedition, ExpeditionCategory, ExpeditionState, Fleet, Frigate,
    FrigateClass, FrigateGrade, Grade,
};
pub use galaxy::{Galaxy, GalaxyType, GalaxyTypeParseError};
pub use generated::{
    AddressGenerator, Economy, Generated, Race, StarColour, SystemAttributes, Tier,
};
pub use glyph::{Glyph, GlyphParseError};
pub use holdings::{
    Container, ContainerKind, Holdings, ItemId, ItemKind, ItemStack, MultiToolSummary, ShipSummary,
    ShipType, VehicleSummary,
};
pub use items::ItemInfo;
pub use player::{BaseType, BaseTypeParseError, PlayerBase, PlayerState};
pub use system::{Planet, System, SystemId};
