//! Base objects decoded from the save: crops, supply depots, extractors, and power equipment.
//!
//! Every placed object at a base carries a 64-bit `UserData` field. For the object types decoded here the meaningful value sits in the high 32 bits (`UserData >> 32`); the low 32 bits are zero on all of them in the saves examined. `Timestamp` on every object is rewritten to the save time whenever the game saves, so it is the snapshot instant for the value.
//!
//! Crops store seconds of growth accumulated at the snapshot, capped at the crop's growth time. Supply depots and extractors store the units that object holds times [`INDUSTRY_SCALE`], and a network's total is the sum over its members. Every depot on a network carries one value and every extractor another, so a member's value says which kind it is, not which network it is on; which machines share a network comes from the pipes instead, in [`crate::pipes`]. Batteries store their charge directly. See `docs/reference/nms-save-notes.md`.
//!
//! Nothing here reads the clock: every time-dependent method takes `now` as Unix seconds so results are deterministic in tests.

use serde::{Deserialize, Serialize};

/// Stored-amount scale for depots and extractors: `UserData >> 32` divided by this is units.
pub const INDUSTRY_SCALE: u64 = 1440;

/// Units a supply depot holds.
pub const DEPOT_CAPACITY: u32 = 1000;

/// Units an extractor's own buffer holds.
pub const EXTRACTOR_CAPACITY: u32 = 250;

/// A full battery's `UserData >> 32` reading.
pub const BATTERY_CAPACITY: u32 = 45_000;

/// A farmable crop with a known object ID and growth time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum CropKind {
    StarBulb,
    GammaRoot,
    FungalMould,
    Coprite,
    FrostCrystal,
    CactusFlesh,
    Solanium,
    MorditeRoot,
}

impl CropKind {
    /// Every known crop, in display order.
    pub const ALL: [CropKind; 8] = [
        CropKind::StarBulb,
        CropKind::GammaRoot,
        CropKind::FungalMould,
        CropKind::Coprite,
        CropKind::FrostCrystal,
        CropKind::CactusFlesh,
        CropKind::Solanium,
        CropKind::MorditeRoot,
    ];

    /// Look up a crop by its save-file object ID (for example `^SNOWPLANT`).
    pub fn from_object_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.object_id() == id)
    }

    /// The save-file object ID for this crop.
    ///
    /// All but Mordite Root were read from a real save; `^CREATUREPLANT` is the game's known ID for Mordite Root but has not been seen in a save yet.
    pub fn object_id(self) -> &'static str {
        match self {
            CropKind::StarBulb => "^LUSHPLANT",
            CropKind::GammaRoot => "^RADIOPLANT",
            CropKind::FungalMould => "^TOXICPLANT",
            CropKind::Coprite => "^POOPPLANT",
            CropKind::FrostCrystal => "^SNOWPLANT",
            CropKind::CactusFlesh => "^BARRENPLANT",
            CropKind::Solanium => "^SCORCHEDPLANT",
            CropKind::MorditeRoot => "^CREATUREPLANT",
        }
    }

    /// Time from planting to harvest, in seconds.
    pub fn growth_secs(self) -> u32 {
        match self {
            CropKind::FrostCrystal => 3_600,
            CropKind::StarBulb
            | CropKind::GammaRoot
            | CropKind::FungalMould
            | CropKind::Coprite => 14_400,
            CropKind::MorditeRoot => 28_800,
            CropKind::CactusFlesh | CropKind::Solanium => 57_600,
        }
    }

    /// The harvested product's name.
    pub fn display_name(self) -> &'static str {
        match self {
            CropKind::StarBulb => "Star Bulb",
            CropKind::GammaRoot => "Gamma Root",
            CropKind::FungalMould => "Fungal Mould",
            CropKind::Coprite => "Coprite",
            CropKind::FrostCrystal => "Frost Crystal",
            CropKind::CactusFlesh => "Cactus Flesh",
            CropKind::Solanium => "Solanium",
            CropKind::MorditeRoot => "Mordite Root",
        }
    }
}

/// One planted crop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Crop {
    /// Known crop, or `None` for a plant object whose ID is not in the table.
    pub kind: Option<CropKind>,
    /// Raw object ID, kept so unknown plants can still be labelled.
    pub object_id: String,
    /// Seconds of growth accumulated at the snapshot.
    pub grown_at_snapshot: u32,
    /// Snapshot instant, Unix seconds.
    pub snapshot: i64,
}

impl Crop {
    /// Growth time in seconds, if the crop is known.
    pub fn growth_secs(&self) -> Option<u32> {
        self.kind.map(CropKind::growth_secs)
    }

    /// Seconds of growth accumulated by `now`, capped at the growth time when known.
    pub fn grown_secs(&self, now: i64) -> i64 {
        let grown = i64::from(self.grown_at_snapshot) + (now - self.snapshot).max(0);
        match self.growth_secs() {
            Some(total) => grown.min(i64::from(total)),
            None => grown,
        }
    }

    /// Seconds until harvest at `now` (zero when ready), or `None` for an unknown crop.
    pub fn remaining_secs(&self, now: i64) -> Option<i64> {
        self.growth_secs()
            .map(|total| (i64::from(total) - self.grown_secs(now)).max(0))
    }

    /// Whether the crop can be harvested at `now`, or `None` for an unknown crop.
    pub fn ready(&self, now: i64) -> Option<bool> {
        self.remaining_secs(now).map(|left| left == 0)
    }

    /// Growth progress in `0.0..=1.0`, or `None` for an unknown crop.
    pub fn progress(&self, now: i64) -> Option<f64> {
        self.growth_secs()
            .map(|total| (self.grown_secs(now) as f64 / f64::from(total)).clamp(0.0, 1.0))
    }

    /// Display label: the product name, or the raw ID without its `^` prefix.
    pub fn label(&self) -> String {
        match self.kind {
            Some(kind) => kind.display_name().to_string(),
            None => self.object_id.trim_start_matches('^').to_string(),
        }
    }
}

/// A supply depot (`^U_SILO_S`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Depot {
    /// Units this depot held at the snapshot.
    pub units: u32,
    /// Units this depot can hold.
    pub capacity: u32,
    /// 1-based index of the pipe network within the base, from the pipes that join it.
    pub network: u32,
    /// Snapshot instant, Unix seconds.
    pub snapshot: i64,
}

/// What an extractor pulls from its hotspot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum ExtractorKind {
    Mineral,
    Gas,
}

impl ExtractorKind {
    pub fn display_name(self) -> &'static str {
        match self {
            ExtractorKind::Mineral => "Mineral Extractor",
            ExtractorKind::Gas => "Gas Extractor",
        }
    }

    /// Short lowercase word for compact tables.
    pub fn short_name(self) -> &'static str {
        match self {
            ExtractorKind::Mineral => "mineral",
            ExtractorKind::Gas => "gas",
        }
    }
}

/// A mineral or gas extractor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Extractor {
    pub kind: ExtractorKind,
    /// Units in the buffer at the snapshot, capped at [`EXTRACTOR_CAPACITY`].
    pub units: u32,
    pub capacity: u32,
    /// 1-based pipe network index within the base, shared with the depots it feeds.
    pub network: u32,
    /// Snapshot instant, Unix seconds.
    pub snapshot: i64,
}

/// A battery (`^U_BATTERY_S`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Battery {
    /// Charge reading at the snapshot, in the save's own power units.
    pub charge: u32,
    pub capacity: u32,
}

impl Battery {
    pub fn is_full(&self) -> bool {
        self.charge >= self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.charge == 0
    }
}

/// A power source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub enum GeneratorKind {
    SolarPanel,
    BiofuelReactor,
    ElectromagneticGenerator,
}

impl GeneratorKind {
    pub fn from_object_id(id: &str) -> Option<Self> {
        match id {
            "^U_SOLAR_S" => Some(GeneratorKind::SolarPanel),
            "^U_BIOGENERATOR" => Some(GeneratorKind::BiofuelReactor),
            "^U_GENERATOR_S" => Some(GeneratorKind::ElectromagneticGenerator),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            GeneratorKind::SolarPanel => "Solar Panel",
            GeneratorKind::BiofuelReactor => "Biofuel Reactor",
            GeneratorKind::ElectromagneticGenerator => "Electromagnetic Generator",
        }
    }
}

/// A power source with its raw state, which is not yet decoded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Generator {
    pub kind: GeneratorKind,
    /// `UserData >> 32`, kept raw. Electromagnetic generators read 0; solar panels have not been seen in a save yet; the biofuel reactor value is not understood.
    pub raw: u32,
}

/// The decoded objects of one base.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "archive",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct BaseObjects {
    pub crops: Vec<Crop>,
    pub depots: Vec<Depot>,
    pub extractors: Vec<Extractor>,
    pub batteries: Vec<Battery>,
    pub generators: Vec<Generator>,
    /// Electrical wire segments (`^U_POWERLINE`).
    pub wires: u32,
    /// Pipe segments (`^U_PIPELINE`).
    pub pipes: u32,
    /// Objects of every other kind: structure, decoration, and anything not decoded.
    pub other: u32,
}

/// The fields of a save-file base object that decoding needs.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawBaseObject<'a> {
    pub object_id: &'a str,
    /// Unix seconds.
    pub timestamp: i64,
    pub user_data: u64,
    /// Where the object stands, relative to the base.
    pub position: [f32; 3],
    /// A unit orientation vector, except on a line part where it is the run itself. See [`crate::pipes`].
    pub at: [f32; 3],
}

impl BaseObjects {
    /// Decode a base's object list.
    ///
    /// Which depots and extractors share a pipe network is worked out from the pipes themselves, by [`crate::pipes::networks`]; nothing in the object list records it.
    pub fn decode<'a>(objects: impl IntoIterator<Item = RawBaseObject<'a>>) -> Self {
        let mut out = BaseObjects::default();
        // Machines in the order they are met, so networks come out numbered in save order, and the row each one filled while its network was still unknown.
        let mut attachments: Vec<crate::pipes::Attachment> = Vec::new();
        let mut slots: Vec<MachineSlot> = Vec::new();
        let mut segments: Vec<crate::pipes::Segment> = Vec::new();

        for obj in objects {
            let value = obj.user_data >> 32;
            let value32 = u32::try_from(value).unwrap_or(u32::MAX);
            match obj.object_id {
                "^U_SILO_S" => {
                    attachments.push(crate::pipes::Attachment {
                        position: obj.position,
                        connector: crate::pipes::DEPOT_CONNECTOR,
                    });
                    slots.push(MachineSlot::Depot(out.depots.len()));
                    out.depots.push(Depot {
                        units: scaled_units(value, DEPOT_CAPACITY),
                        capacity: DEPOT_CAPACITY,
                        network: 0,
                        snapshot: obj.timestamp,
                    });
                }
                "^U_EXTRACTOR_S" | "^U_GASEXTRACTOR" => {
                    let kind = if obj.object_id == "^U_GASEXTRACTOR" {
                        ExtractorKind::Gas
                    } else {
                        ExtractorKind::Mineral
                    };
                    attachments.push(crate::pipes::Attachment {
                        position: obj.position,
                        connector: crate::pipes::EXTRACTOR_CONNECTOR,
                    });
                    slots.push(MachineSlot::Extractor(out.extractors.len()));
                    out.extractors.push(Extractor {
                        kind,
                        units: scaled_units(value, EXTRACTOR_CAPACITY),
                        capacity: EXTRACTOR_CAPACITY,
                        network: 0,
                        snapshot: obj.timestamp,
                    });
                }
                "^U_BATTERY_S" => out.batteries.push(Battery {
                    charge: value32.min(BATTERY_CAPACITY),
                    capacity: BATTERY_CAPACITY,
                }),
                "^U_POWERLINE" => out.wires += 1,
                "^U_PIPELINE" => {
                    out.pipes += 1;
                    if let Some(segment) = crate::pipes::Segment::run(obj.position, obj.at) {
                        segments.push(segment);
                    }
                }
                id => {
                    if let Some(kind) = GeneratorKind::from_object_id(id) {
                        out.generators.push(Generator { kind, raw: value32 });
                    } else if let Some(kind) = CropKind::from_object_id(id) {
                        out.crops.push(Crop {
                            kind: Some(kind),
                            object_id: id.to_string(),
                            grown_at_snapshot: value32,
                            snapshot: obj.timestamp,
                        });
                    } else if is_unknown_plant(id) {
                        out.crops.push(Crop {
                            kind: None,
                            object_id: id.to_string(),
                            grown_at_snapshot: value32,
                            snapshot: obj.timestamp,
                        });
                    } else {
                        out.other += 1;
                    }
                }
            }
        }

        for (slot, network) in slots
            .iter()
            .zip(crate::pipes::networks(&segments, &attachments))
        {
            match *slot {
                MachineSlot::Depot(i) => out.depots[i].network = network,
                MachineSlot::Extractor(i) => out.extractors[i].network = network,
            }
        }
        out
    }

    /// Total decoded and undecoded objects.
    pub fn total(&self) -> usize {
        self.crops.len()
            + self.depots.len()
            + self.extractors.len()
            + self.batteries.len()
            + self.generators.len()
            + self.wires as usize
            + self.pipes as usize
            + self.other as usize
    }

    pub fn has_crops(&self) -> bool {
        !self.crops.is_empty()
    }

    pub fn has_industry(&self) -> bool {
        !self.depots.is_empty() || !self.extractors.is_empty()
    }

    pub fn has_power(&self) -> bool {
        !self.batteries.is_empty() || !self.generators.is_empty() || self.wires > 0
    }

    /// Number of distinct pipe networks.
    pub fn network_count(&self) -> u32 {
        self.depots
            .iter()
            .map(|d| d.network)
            .chain(self.extractors.iter().map(|e| e.network))
            .max()
            .unwrap_or(0)
    }

    /// Pipe networks in index order, with their stored and capacity totals.
    pub fn networks(&self) -> Vec<Network> {
        (1..=self.network_count())
            .map(|index| {
                let depots: Vec<&Depot> =
                    self.depots.iter().filter(|d| d.network == index).collect();
                let extractors: Vec<&Extractor> = self
                    .extractors
                    .iter()
                    .filter(|e| e.network == index)
                    .collect();
                let stored = depots.iter().map(|d| d.units).sum::<u32>()
                    + extractors.iter().map(|e| e.units).sum::<u32>();
                let capacity = depots.iter().map(|d| d.capacity).sum::<u32>()
                    + extractors.iter().map(|e| e.capacity).sum::<u32>();
                let snapshot = depots
                    .iter()
                    .map(|d| d.snapshot)
                    .chain(extractors.iter().map(|e| e.snapshot))
                    .max()
                    .unwrap_or(0);
                Network {
                    index,
                    depots: depots.len(),
                    mineral_extractors: extractors
                        .iter()
                        .filter(|e| e.kind == ExtractorKind::Mineral)
                        .count(),
                    gas_extractors: extractors
                        .iter()
                        .filter(|e| e.kind == ExtractorKind::Gas)
                        .count(),
                    stored,
                    capacity,
                    snapshot,
                }
            })
            .collect()
    }
}

/// One pipe network's totals, as the game shows them when a depot is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Network {
    /// 1-based index within the base.
    pub index: u32,
    pub depots: usize,
    pub mineral_extractors: usize,
    pub gas_extractors: usize,
    /// Units stored across all members at the snapshot.
    pub stored: u32,
    /// Units the whole network can hold.
    pub capacity: u32,
    /// Snapshot instant, Unix seconds.
    pub snapshot: i64,
}

impl Network {
    /// The network's letter, for telling a base's networks apart in a listing.
    pub fn label(&self) -> String {
        crate::pipes::label(self.index)
    }

    pub fn is_full(&self) -> bool {
        self.capacity > 0 && self.stored >= self.capacity
    }

    /// Fill fraction in `0.0..=1.0`.
    pub fn fill(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            (f64::from(self.stored) / f64::from(self.capacity)).clamp(0.0, 1.0)
        }
    }
}

/// Where a machine's row went while its network was still unknown.
#[derive(Debug, Clone, Copy)]
enum MachineSlot {
    Depot(usize),
    Extractor(usize),
}

/// A planter object not in the crop table. Freighter planter rooms are excluded because they use a different encoding.
fn is_unknown_plant(id: &str) -> bool {
    id.ends_with("PLANT") && !id.starts_with("^FRE_")
}

fn scaled_units(value: u64, capacity: u32) -> u32 {
    u32::try_from(value / INDUSTRY_SCALE)
        .unwrap_or(u32::MAX)
        .min(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNAPSHOT: i64 = 1_789_399_213;

    fn raw(object_id: &str, hi: u64) -> RawBaseObject<'_> {
        RawBaseObject {
            object_id,
            timestamp: SNAPSHOT,
            user_data: hi << 32,
            ..Default::default()
        }
    }

    #[test]
    fn crop_kind_round_trips_object_ids() {
        for kind in CropKind::ALL {
            assert_eq!(CropKind::from_object_id(kind.object_id()), Some(kind));
        }
        assert_eq!(CropKind::from_object_id("^U_SILO_S"), None);
    }

    #[test]
    fn decodes_mature_frost_crystal_from_plan_value() {
        let objects = BaseObjects::decode([RawBaseObject {
            object_id: "^SNOWPLANT",
            timestamp: SNAPSHOT,
            user_data: 15_461_882_265_600,
            ..Default::default()
        }]);
        let crop = &objects.crops[0];
        assert_eq!(crop.kind, Some(CropKind::FrostCrystal));
        assert_eq!(crop.grown_at_snapshot, 3_600);
        assert_eq!(crop.ready(SNAPSHOT), Some(true));
        assert_eq!(crop.progress(SNAPSHOT), Some(1.0));
    }

    #[test]
    fn crop_grows_with_elapsed_time_and_caps() {
        let objects = BaseObjects::decode([raw("^LUSHPLANT", 1004)]);
        let crop = &objects.crops[0];
        assert_eq!(crop.grown_secs(SNAPSHOT), 1004);
        assert_eq!(crop.remaining_secs(SNAPSHOT), Some(13_396));
        assert_eq!(crop.ready(SNAPSHOT), Some(false));
        assert_eq!(crop.remaining_secs(SNAPSHOT + 13_395), Some(1));
        assert_eq!(crop.ready(SNAPSHOT + 13_396), Some(true));
        assert_eq!(crop.grown_secs(SNAPSHOT + 100_000), 14_400);
        assert_eq!(crop.progress(SNAPSHOT + 100_000), Some(1.0));
        let half = crop.progress(SNAPSHOT + 7_200 - 1004).unwrap();
        assert!((half - 0.5).abs() < 1e-9);
    }

    #[test]
    fn crop_matches_second_save_48_minutes_later() {
        // Real save pair: LUSHPLANT read 1004 at 1789399213 and 3878 at 1789402087; SNOWPLANT read 989 then 3600.
        let objects = BaseObjects::decode([raw("^LUSHPLANT", 1004), raw("^SNOWPLANT", 989)]);
        let later = 1_789_402_087;
        assert_eq!(objects.crops[0].grown_secs(later), 3878);
        assert_eq!(objects.crops[1].grown_secs(later), 3600);
        assert_eq!(objects.crops[1].ready(later), Some(true));
    }

    #[test]
    fn crop_ignores_clock_before_snapshot() {
        let objects = BaseObjects::decode([raw("^LUSHPLANT", 1004)]);
        assert_eq!(objects.crops[0].grown_secs(SNAPSHOT - 5_000), 1004);
    }

    #[test]
    fn unknown_plant_is_a_crop_without_timing() {
        let objects = BaseObjects::decode([
            raw("^GRAVPLANT", 500),
            raw("^FRE_ROOM_PLANT1", 0),
            raw("^PLANTTUBE", 0),
        ]);
        assert_eq!(objects.crops.len(), 1);
        let crop = &objects.crops[0];
        assert_eq!(crop.kind, None);
        assert_eq!(crop.label(), "GRAVPLANT");
        assert_eq!(crop.grown_secs(SNAPSHOT + 10), 510);
        assert_eq!(crop.ready(SNAPSHOT), None);
        assert_eq!(crop.progress(SNAPSHOT), None);
        assert_eq!(objects.other, 2);
    }

    #[test]
    fn decodes_full_depot_extractor_and_battery_from_plan_values() {
        let objects = BaseObjects::decode([
            RawBaseObject {
                object_id: "^U_SILO_S",
                timestamp: SNAPSHOT,
                user_data: 6_184_752_906_240_000,
                ..Default::default()
            },
            RawBaseObject {
                object_id: "^U_EXTRACTOR_S",
                timestamp: SNAPSHOT,
                user_data: 1_546_188_226_560_000,
                ..Default::default()
            },
            RawBaseObject {
                object_id: "^U_BATTERY_S",
                timestamp: SNAPSHOT,
                user_data: 193_273_528_320_000,
                ..Default::default()
            },
        ]);
        assert_eq!(objects.depots[0].units, 1000);
        assert_eq!(objects.depots[0].capacity, DEPOT_CAPACITY);
        assert_eq!(objects.extractors[0].units, 250);
        assert_eq!(objects.extractors[0].kind, ExtractorKind::Mineral);
        assert!(objects.batteries[0].is_full());
    }

    /// One machine standing at `position`.
    fn machine(object_id: &str, hi: u64, position: [f32; 3]) -> RawBaseObject<'_> {
        RawBaseObject {
            object_id,
            timestamp: SNAPSHOT,
            user_data: hi << 32,
            position,
            at: [0.0, 0.0, 1.0],
        }
    }

    /// A pipe laid from `from` to `to`, recorded the way the game records it: `At` is the run plus a one-unit overshoot.
    fn pipe(from: [f32; 3], to: [f32; 3]) -> RawBaseObject<'static> {
        let run = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let len = (run[0] * run[0] + run[1] * run[1] + run[2] * run[2]).sqrt();
        RawBaseObject {
            object_id: "^U_PIPELINE",
            timestamp: SNAPSHOT,
            position: from,
            at: [
                run[0] + run[0] / len,
                run[1] + run[1] / len,
                run[2] + run[2] / len,
            ],
            ..Default::default()
        }
    }

    #[test]
    fn networks_come_from_the_pipes_not_from_equal_values() {
        use crate::pipes::{DEPOT_CONNECTOR, EXTRACTOR_CONNECTOR};
        // Two networks whose members all sit at their caps, so every depot carries one value and every extractor another. Grouping by value would give one network of depots and one of extractors; the pipes give the real pair.
        let objects = BaseObjects::decode([
            machine("^U_GASEXTRACTOR", 360_000, [0.0, 0.0, 0.0]),
            machine("^U_SILO_S", 1_440_000, [20.0, 0.0, 0.0]),
            machine("^U_EXTRACTOR_S", 360_000, [0.0, 0.0, 60.0]),
            machine("^U_SILO_S", 1_440_000, [20.0, 0.0, 60.0]),
            pipe(
                [EXTRACTOR_CONNECTOR, 0.0, 0.0],
                [20.0 - DEPOT_CONNECTOR, 0.0, 0.0],
            ),
            pipe(
                [EXTRACTOR_CONNECTOR, 0.0, 60.0],
                [20.0 - DEPOT_CONNECTOR, 0.0, 60.0],
            ),
        ]);
        assert_eq!(objects.network_count(), 2);
        assert_eq!(objects.extractors[0].kind, ExtractorKind::Gas);
        assert_eq!(objects.extractors[0].network, 1);
        assert_eq!(objects.depots[0].network, 1);
        assert_eq!(objects.extractors[1].network, 2);
        assert_eq!(objects.depots[1].network, 2);
        let networks = objects.networks();
        assert_eq!(networks[0].label(), "A");
        assert_eq!(networks[1].label(), "B");
        assert_eq!(networks[0].capacity, 1_250);
        assert!(networks.iter().all(Network::is_full));
    }

    #[test]
    fn depots_standing_together_share_a_network_without_a_pipe() {
        let objects = BaseObjects::decode([
            machine("^U_SILO_S", 720_000, [0.0, 0.0, 0.0]),
            machine("^U_SILO_S", 720_000, [1.5, 0.0, 0.0]),
            machine("^U_SILO_S", 720_000, [40.0, 0.0, 0.0]),
        ]);
        assert_eq!(objects.network_count(), 2);
        assert_eq!(objects.depots[0].network, 1);
        assert_eq!(objects.depots[1].network, 1, "touching its neighbour");
        assert_eq!(objects.depots[2].network, 2, "too far to touch anything");
    }

    #[test]
    fn a_depot_with_no_geometry_is_still_a_network() {
        let objects = BaseObjects::decode([raw("^U_SILO_S", 154_712)]);
        assert_eq!(objects.network_count(), 1);
        assert_eq!(objects.depots[0].network, 1);
    }

    #[test]
    fn network_totals_match_in_game_reading() {
        // Farm nitrogen network at 1789402087: 3 gas extractors + 4 depots all at 154,712; the game showed 750 of 4,750. Everything stands at the origin, so the machines chain by touching and form the one network.
        let mut raws = vec![raw("^U_GASEXTRACTOR", 154_712); 3];
        raws.extend(vec![raw("^U_SILO_S", 154_712); 4]);
        let objects = BaseObjects::decode(raws);
        let networks = objects.networks();
        assert_eq!(networks.len(), 1);
        let net = &networks[0];
        assert_eq!(net.index, 1);
        assert_eq!(net.depots, 4);
        assert_eq!(net.gas_extractors, 3);
        assert_eq!(net.mineral_extractors, 0);
        assert_eq!(net.capacity, 4_750);
        assert_eq!(net.stored, 749);
        assert!(!net.is_full());
        assert!((net.fill() - 749.0 / 4750.0).abs() < 1e-9);
    }

    #[test]
    fn full_network_reads_full_on_every_member() {
        let objects = BaseObjects::decode([
            raw("^U_EXTRACTOR_S", 360_000),
            raw("^U_SILO_S", 1_440_000),
            raw("^U_SILO_S", 1_440_000),
        ]);
        // Members at different caps land on different value groups; both groups are full.
        let networks = objects.networks();
        assert!(networks.iter().all(Network::is_full));
        assert_eq!(networks.iter().map(|n| n.stored).sum::<u32>(), 2_250);
    }

    #[test]
    fn decodes_power_equipment_and_counts_lines() {
        let objects = BaseObjects::decode([
            raw("^U_SOLAR_S", 0),
            raw("^U_BIOGENERATOR", 87_267),
            raw("^U_GENERATOR_S", 0),
            raw("^U_PARAGON", 1_000_000),
            raw("^U_BATTERY_S", 0),
            raw("^U_POWERLINE", 0),
            raw("^U_POWERLINE", 0),
            raw("^U_PIPELINE", 0),
            raw("^BUILD_REFINER1", 0),
        ]);
        assert_eq!(objects.generators.len(), 3);
        assert_eq!(objects.generators[0].kind, GeneratorKind::SolarPanel);
        assert_eq!(objects.generators[1].raw, 87_267);
        assert_eq!(
            objects.generators[2].kind,
            GeneratorKind::ElectromagneticGenerator
        );
        assert!(objects.batteries[0].is_empty());
        assert_eq!(objects.wires, 2);
        assert_eq!(objects.pipes, 1);
        assert_eq!(objects.other, 2, "refiner and the unknown ^U_PARAGON");
        assert_eq!(objects.total(), 9);
        assert!(objects.has_power());
        assert!(!objects.has_crops());
        assert!(!objects.has_industry());
    }

    #[test]
    fn empty_object_list_decodes_to_default() {
        assert_eq!(
            BaseObjects::decode(std::iter::empty()),
            BaseObjects::default()
        );
        assert!(BaseObjects::default().networks().is_empty());
    }
}
