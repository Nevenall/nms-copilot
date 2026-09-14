//! Base status queries: crops, extraction networks, and power for one base or all of them.
//!
//! Time-dependent values take `now` as Unix seconds; nothing here reads the clock.

use nms_core::base::{Crop, GeneratorKind, Network};
use nms_core::player::PlayerBase;
use nms_core::system::System;
use nms_graph::spatial::SystemId;
use nms_graph::{GalaxyModel, GraphError};

/// Crops planted within this many seconds of each other count as one batch.
const BATCH_TOLERANCE_SECS: i64 = 60;

/// Which bases to report on.
#[derive(Debug, Clone, Default)]
pub struct BaseQuery {
    /// A base name: exact match first, then case-insensitive substring. `None` means every base.
    pub name: Option<String>,
}

/// Everything the base view shows for one base.
#[derive(Debug, Clone)]
pub struct BaseStatus {
    pub base: PlayerBase,
    pub portal_hex: String,
    pub galaxy_name: String,
    pub system: Option<System>,
    pub distance_from_player: Option<f64>,
    /// One row per crop type, nearest harvest first.
    pub crops: Vec<CropRow>,
    pub crops_total: usize,
    pub crops_ready: usize,
    /// Pipe networks in index order.
    pub networks: Vec<Network>,
    pub power: PowerSummary,
    /// Most recent object snapshot at this base, Unix seconds, if it has any decoded objects.
    pub snapshot: Option<i64>,
}

impl BaseStatus {
    pub fn extraction_stored(&self) -> u32 {
        self.networks.iter().map(|n| n.stored).sum()
    }

    pub fn extraction_capacity(&self) -> u32 {
        self.networks.iter().map(|n| n.capacity).sum()
    }

    pub fn full_networks(&self) -> usize {
        self.networks.iter().filter(|n| n.is_full()).count()
    }

    /// Something at this base wants a visit: ready crops or a full network.
    pub fn has_alert(&self) -> bool {
        self.crops_ready > 0 || self.full_networks() > 0
    }
}

/// One crop type at a base.
#[derive(Debug, Clone, PartialEq)]
pub struct CropRow {
    pub label: String,
    pub count: usize,
    pub ready: usize,
    /// Planting batches, soonest first. Ready plants form the first batch with zero remaining.
    pub batches: Vec<CropBatch>,
    /// Progress of the soonest batch, or `None` for a crop with unknown growth time.
    pub progress: Option<f64>,
    /// Seconds grown by the soonest batch; the only timing available for unknown crops.
    pub grown_secs: i64,
}

impl CropRow {
    /// Seconds until the soonest batch is ready, or `None` for an unknown crop.
    pub fn next_ready_secs(&self) -> Option<i64> {
        self.batches.first().and_then(|b| b.remaining_secs)
    }
}

/// Plants of one type that were planted together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CropBatch {
    pub count: usize,
    /// Seconds until ready (zero when ready), or `None` for an unknown crop.
    pub remaining_secs: Option<i64>,
}

/// Power equipment at a base.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PowerSummary {
    /// Generator kinds present with their counts and raw readings, in first-seen order.
    pub generators: Vec<GeneratorRow>,
    pub batteries: usize,
    pub batteries_full: usize,
    pub batteries_empty: usize,
    pub battery_charge: u32,
    pub battery_capacity: u32,
    pub wires: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorRow {
    pub kind: GeneratorKind,
    pub count: usize,
    /// Raw readings, kept for the kinds that are not decoded yet.
    pub raw: Vec<u32>,
}

/// Report on the bases selected by `query`, alert-worthy bases first, then by name.
pub fn execute_base(
    model: &GalaxyModel,
    query: &BaseQuery,
    now: i64,
) -> Result<Vec<BaseStatus>, GraphError> {
    let selected: Vec<&PlayerBase> = match &query.name {
        Some(name) => select_bases(model, name)?,
        None => model.bases.values().collect(),
    };
    let mut statuses: Vec<BaseStatus> = selected
        .into_iter()
        .map(|base| base_status(model, base, now))
        .collect();
    statuses.sort_by(|a, b| {
        b.has_alert()
            .cmp(&a.has_alert())
            .then_with(|| a.base.name.to_lowercase().cmp(&b.base.name.to_lowercase()))
    });
    Ok(statuses)
}

/// Bases matching a name: the exact (case-insensitive) match if there is one, else every base whose name contains it.
fn select_bases<'m>(model: &'m GalaxyModel, name: &str) -> Result<Vec<&'m PlayerBase>, GraphError> {
    if let Some(base) = model.base(name) {
        return Ok(vec![base]);
    }
    let needle = name.to_lowercase();
    let matches: Vec<&PlayerBase> = model
        .bases
        .values()
        .filter(|b| b.name.to_lowercase().contains(&needle))
        .collect();
    if matches.is_empty() {
        Err(GraphError::BaseNotFound(name.to_string()))
    } else {
        Ok(matches)
    }
}

/// Build the status of one base.
pub fn base_status(model: &GalaxyModel, base: &PlayerBase, now: i64) -> BaseStatus {
    let portal_hex = format!("{:012X}", base.address.packed());
    let galaxy = nms_core::galaxy::Galaxy::by_index(base.address.reality_index);
    let system = model
        .system(&SystemId::from_address(&base.address))
        .cloned();
    let distance_from_player = model
        .player_position()
        .map(|pos| pos.distance_ly(&base.address));

    let objects = &base.objects;
    let crops = crop_rows(&objects.crops, now);
    let crops_ready = crops.iter().map(|r| r.ready).sum();
    let networks = objects.networks();
    let snapshot = objects
        .crops
        .iter()
        .map(|c| c.snapshot)
        .chain(networks.iter().map(|n| n.snapshot))
        .max();

    BaseStatus {
        base: base.clone(),
        portal_hex,
        galaxy_name: galaxy.name.to_string(),
        system,
        distance_from_player,
        crops,
        crops_total: objects.crops.len(),
        crops_ready,
        networks,
        power: power_summary(objects),
        snapshot,
    }
}

/// Group crops by type and planting batch, nearest harvest first.
pub fn crop_rows(crops: &[Crop], now: i64) -> Vec<CropRow> {
    let mut labels: Vec<String> = Vec::new();
    for crop in crops {
        let label = crop.label();
        if !labels.contains(&label) {
            labels.push(label);
        }
    }

    let mut rows: Vec<CropRow> = labels
        .into_iter()
        .map(|label| {
            let mut plants: Vec<&Crop> = crops.iter().filter(|c| c.label() == label).collect();
            // Soonest first; unknown timing sorts last.
            plants.sort_by_key(|c| {
                (
                    c.remaining_secs(now).is_none(),
                    c.remaining_secs(now).unwrap_or(0),
                )
            });
            let ready = plants.iter().filter(|c| c.ready(now) == Some(true)).count();
            let batches = batch(&plants, now);
            let soonest = plants[0];
            CropRow {
                label,
                count: plants.len(),
                ready,
                batches,
                progress: soonest.progress(now),
                grown_secs: soonest.grown_secs(now),
            }
        })
        .collect();

    rows.sort_by_key(|r| {
        (
            r.next_ready_secs().is_none(),
            r.next_ready_secs().unwrap_or(0),
        )
    });
    rows
}

/// Cluster plants (already sorted soonest first) whose remaining time is within [`BATCH_TOLERANCE_SECS`] of the batch's first plant.
fn batch(plants: &[&Crop], now: i64) -> Vec<CropBatch> {
    let mut batches: Vec<CropBatch> = Vec::new();
    let mut batch_start: Option<Option<i64>> = None;
    for plant in plants {
        let remaining = plant.remaining_secs(now);
        let same_batch = match (batch_start, remaining) {
            (Some(Some(start)), Some(r)) => r - start <= BATCH_TOLERANCE_SECS,
            (Some(None), None) => true,
            _ => false,
        };
        if same_batch {
            batches.last_mut().expect("batch exists").count += 1;
        } else {
            batches.push(CropBatch {
                count: 1,
                remaining_secs: remaining,
            });
            batch_start = Some(remaining);
        }
    }
    batches
}

fn power_summary(objects: &nms_core::BaseObjects) -> PowerSummary {
    let mut generators: Vec<GeneratorRow> = Vec::new();
    for g in &objects.generators {
        match generators.iter_mut().find(|row| row.kind == g.kind) {
            Some(row) => {
                row.count += 1;
                row.raw.push(g.raw);
            }
            None => generators.push(GeneratorRow {
                kind: g.kind,
                count: 1,
                raw: vec![g.raw],
            }),
        }
    }
    PowerSummary {
        generators,
        batteries: objects.batteries.len(),
        batteries_full: objects.batteries.iter().filter(|b| b.is_full()).count(),
        batteries_empty: objects.batteries.iter().filter(|b| b.is_empty()).count(),
        battery_charge: objects.batteries.iter().map(|b| b.charge).sum(),
        battery_capacity: objects.batteries.iter().map(|b| b.capacity).sum(),
        wires: objects.wires,
    }
}

// ── Alerts ──────────────────────────────────────────────────────

/// Something at a base that wants a visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub base: String,
    pub kind: AlertKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertKind {
    /// `count` plants of `crop` can be harvested.
    CropsReady { crop: String, count: usize },
    /// Pipe network `network` is at capacity.
    DepotsFull { network: u32, capacity: u32 },
}

impl Alert {
    /// Stable identity for de-duplication across refreshes. A crop alert keeps its key while any plants of that type are ready; a depots alert keeps its key while the network is full.
    pub fn key(&self) -> String {
        match &self.kind {
            AlertKind::CropsReady { crop, .. } => {
                format!("crops:{}:{crop}", self.base.to_lowercase())
            }
            AlertKind::DepotsFull { network, .. } => {
                format!("full:{}:{network}", self.base.to_lowercase())
            }
        }
    }

    /// One-line notice text.
    pub fn text(&self) -> String {
        match &self.kind {
            AlertKind::CropsReady { crop, count } => format!("{}: {count} {crop} ready", self.base),
            AlertKind::DepotsFull { network, capacity } => format!(
                "{}: depots full on extraction network {network} ({} units)",
                self.base,
                crate::display::thousands(*capacity)
            ),
        }
    }
}

/// Every current alert across all bases, in the order [`execute_base`] lists bases.
pub fn current_alerts(model: &GalaxyModel, now: i64) -> Vec<Alert> {
    let statuses = execute_base(model, &BaseQuery::default(), now).unwrap_or_default();
    alerts_from(&statuses)
}

/// Alerts for already-computed statuses.
pub fn alerts_from(statuses: &[BaseStatus]) -> Vec<Alert> {
    let mut alerts = Vec::new();
    for status in statuses {
        for row in status.crops.iter().filter(|r| r.ready > 0) {
            alerts.push(Alert {
                base: status.base.name.clone(),
                kind: AlertKind::CropsReady {
                    crop: row.label.clone(),
                    count: row.ready,
                },
            });
        }
        for net in status.networks.iter().filter(|n| n.is_full()) {
            alerts.push(Alert {
                base: status.base.name.clone(),
                kind: AlertKind::DepotsFull {
                    network: net.index,
                    capacity: net.capacity,
                },
            });
        }
    }
    alerts
}

/// Counts for a compact indicator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AlertSummary {
    /// Plants ready to harvest.
    pub crops_ready: usize,
    /// Extraction networks at capacity.
    pub networks_full: usize,
}

impl AlertSummary {
    pub fn from_alerts(alerts: &[Alert]) -> Self {
        let mut summary = AlertSummary::default();
        for alert in alerts {
            match alert.kind {
                AlertKind::CropsReady { count, .. } => summary.crops_ready += count,
                AlertKind::DepotsFull { .. } => summary.networks_full += 1,
            }
        }
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.crops_ready == 0 && self.networks_full == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::address::GalacticAddress;
    use nms_core::player::BaseType;
    use nms_core::{BaseObjects, RawBaseObject};

    const SNAPSHOT: i64 = 1_789_402_087;

    fn raw(object_id: &str, hi: u64) -> RawBaseObject<'_> {
        RawBaseObject {
            object_id,
            timestamp: SNAPSHOT,
            user_data: hi << 32,
        }
    }

    fn base(name: &str, objects: BaseObjects) -> PlayerBase {
        PlayerBase::new(
            name.into(),
            BaseType::HomePlanetBase,
            GalacticAddress::new(0, 0, 0, 1, 0, 0),
            [0.0; 3],
            None,
        )
        .with_objects(objects)
    }

    fn farm() -> PlayerBase {
        let mut raws = Vec::new();
        raws.extend(vec![raw("^SNOWPLANT", 3600); 16]);
        raws.extend(vec![raw("^RADIOPLANT", 3868); 16]);
        raws.extend(vec![raw("^RADIOPLANT", 3872); 16]);
        raws.extend(vec![raw("^BARRENPLANT", 3927); 13]);
        raws.extend(vec![raw("^BARRENPLANT", 30_000); 3]);
        raws.extend(vec![raw("^U_GASEXTRACTOR", 154_712); 3]);
        raws.extend(vec![raw("^U_SILO_S", 154_712); 4]);
        raws.extend(vec![raw("^U_SILO_S", 1_440_000); 1]);
        raws.push(raw("^U_BATTERY_S", 45_000));
        raws.extend(vec![raw("^U_GENERATOR_S", 0); 4]);
        raws.extend(vec![raw("^U_POWERLINE", 0); 30]);
        raws.push(raw("^BUILDLANDINGPAD", 0));
        base("Farm", BaseObjects::decode(raws))
    }

    fn model_with(bases: Vec<PlayerBase>) -> GalaxyModel {
        let mut model = GalaxyModel::new();
        for b in bases {
            model.insert_base(b);
        }
        model
    }

    #[test]
    fn crop_rows_group_batches_and_sort_soonest_first() {
        let status = base_status(&model_with(vec![]), &farm(), SNAPSHOT);
        assert_eq!(status.crops_total, 64);
        assert_eq!(status.crops_ready, 16);
        let labels: Vec<&str> = status.crops.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Frost Crystal", "Gamma Root", "Cactus Flesh"]);

        let frost = &status.crops[0];
        assert_eq!(frost.ready, 16);
        assert_eq!(
            frost.batches,
            vec![CropBatch {
                count: 16,
                remaining_secs: Some(0)
            }]
        );
        assert_eq!(frost.progress, Some(1.0));

        let gamma = &status.crops[1];
        assert_eq!(gamma.count, 32);
        assert_eq!(gamma.batches.len(), 1, "4 seconds apart is one batch");
        assert_eq!(gamma.next_ready_secs(), Some(14_400 - 3872));

        let cactus = &status.crops[2];
        assert_eq!(
            cactus.batches,
            vec![
                CropBatch {
                    count: 3,
                    remaining_secs: Some(27_600)
                },
                CropBatch {
                    count: 13,
                    remaining_secs: Some(57_600 - 3927)
                }
            ]
        );
    }

    #[test]
    fn crop_rows_advance_with_now() {
        let status = base_status(&model_with(vec![]), &farm(), SNAPSHOT + 14_400);
        assert_eq!(status.crops_ready, 48);
        assert_eq!(status.crops[1].label, "Gamma Root");
        assert_eq!(status.crops[1].ready, 32);
    }

    #[test]
    fn unknown_crop_sorts_last_without_timing() {
        let objects = BaseObjects::decode([raw("^GRAVPLANT", 100), raw("^SNOWPLANT", 0)]);
        let rows = crop_rows(&objects.crops, SNAPSHOT + 50);
        assert_eq!(rows[0].label, "Frost Crystal");
        assert_eq!(rows[1].label, "GRAVPLANT");
        assert_eq!(rows[1].next_ready_secs(), None);
        assert_eq!(rows[1].progress, None);
        assert_eq!(rows[1].grown_secs, 150);
        assert_eq!(
            rows[1].batches,
            vec![CropBatch {
                count: 1,
                remaining_secs: None
            }]
        );
    }

    #[test]
    fn extraction_and_power_summaries() {
        let status = base_status(&model_with(vec![]), &farm(), SNAPSHOT);
        assert_eq!(status.networks.len(), 2);
        assert_eq!(status.networks[0].capacity, 4_750);
        assert!(!status.networks[0].is_full());
        assert!(status.networks[1].is_full());
        assert_eq!(status.extraction_capacity(), 5_750);
        assert_eq!(status.full_networks(), 1);
        assert_eq!(status.power.batteries_full, 1);
        assert_eq!(
            status.power.generators,
            vec![GeneratorRow {
                kind: GeneratorKind::ElectromagneticGenerator,
                count: 4,
                raw: vec![0; 4]
            }]
        );
        assert_eq!(status.power.wires, 30);
        assert_eq!(status.snapshot, Some(SNAPSHOT));
        assert!(status.has_alert());
    }

    #[test]
    fn execute_base_matches_exact_then_substring() {
        let model = model_with(vec![
            farm(),
            base("Gold and Silver", BaseObjects::default()),
            base("Goldfish", BaseObjects::default()),
        ]);
        let one = execute_base(
            &model,
            &BaseQuery {
                name: Some("FARM".into()),
            },
            SNAPSHOT,
        )
        .unwrap();
        assert_eq!(one.len(), 1);
        let two = execute_base(
            &model,
            &BaseQuery {
                name: Some("gold".into()),
            },
            SNAPSHOT,
        )
        .unwrap();
        assert_eq!(two.len(), 2);
        assert!(matches!(
            execute_base(
                &model,
                &BaseQuery {
                    name: Some("nothing".into())
                },
                SNAPSHOT
            ),
            Err(GraphError::BaseNotFound(_))
        ));
    }

    #[test]
    fn execute_base_lists_alerting_bases_first() {
        let model = model_with(vec![
            base("Aardvark", BaseObjects::default()),
            farm(),
            base("Zebra", BaseObjects::default()),
        ]);
        let all = execute_base(&model, &BaseQuery::default(), SNAPSHOT).unwrap();
        let names: Vec<&str> = all.iter().map(|s| s.base.name.as_str()).collect();
        assert_eq!(names, ["Farm", "Aardvark", "Zebra"]);
    }

    #[test]
    fn alerts_and_summary() {
        let model = model_with(vec![farm()]);
        let alerts = current_alerts(&model, SNAPSHOT);
        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].text(), "Farm: 16 Frost Crystal ready");
        assert_eq!(alerts[0].key(), "crops:farm:Frost Crystal");
        assert_eq!(
            alerts[1].text(),
            "Farm: depots full on extraction network 2 (1,000 units)"
        );
        assert_eq!(alerts[1].key(), "full:farm:2");
        let summary = AlertSummary::from_alerts(&alerts);
        assert_eq!(
            summary,
            AlertSummary {
                crops_ready: 16,
                networks_full: 1
            }
        );
        assert!(AlertSummary::default().is_empty());
    }
}
