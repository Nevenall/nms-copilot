//! Holdings queries: do I have an item, what is in each container, and what do I own.
//!
//! Everything reads `GalaxyModel::holdings`; technology slots are never totalled.

use std::collections::BTreeMap;

use nms_core::holdings::{Container, ContainerKind, Holdings, ItemId, ItemKind};
use nms_core::player::BaseType;
use nms_core::system::System;
use nms_graph::spatial::SystemId;
use nms_graph::{GalaxyModel, GraphError};

/// `have <pattern>`: everything matching by name or ID, with where it sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaveQuery {
    pub pattern: String,
    pub kind: Option<ItemKind>,
}

/// One item the pattern matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaveResult {
    pub id: ItemId,
    pub name: String,
    pub kind: Option<ItemKind>,
    /// Countable total across every grid.
    pub total: u64,
    /// One row per stack, in container order.
    pub locations: Vec<HaveLocation>,
}

/// One stack of a found item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaveLocation {
    pub container: ContainerKind,
    pub label: String,
    pub amount: u32,
    pub max: u32,
    /// Where the container can be opened.
    pub access: Vec<String>,
}

/// `inventory [container] [--free]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryQuery {
    /// A container label to show in full (`storage 3`, `exosuit`), or a word every matching container's label contains (`ship`).
    pub container: Option<String>,
    /// Order the overview by free slots, most first, and drop containers with none.
    pub free_only: bool,
}

/// What `inventory` shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryResult {
    /// One row per container.
    Overview(Vec<Container>),
    /// The stacks of one or more named containers.
    Contents(Vec<Container>),
}

/// `list items`: totals per item.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListItemsQuery {
    pub kind: Option<ItemKind>,
    pub min_amount: u32,
}

/// One item's total across every grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTotal {
    pub id: ItemId,
    pub name: String,
    pub kind: Option<ItemKind>,
    pub total: u64,
    pub stacks: usize,
    /// Distinct containers holding it.
    pub containers: usize,
}

/// One exocraft with its parking place named.
#[derive(Debug, Clone, PartialEq)]
pub struct ExocraftRow {
    pub vehicle: nms_core::VehicleSummary,
    /// The base it is parked at, when the atlas knows one at that address.
    pub base_name: Option<String>,
    pub system: Option<System>,
}

/// The model's holdings, or an error for a model built without a save.
pub fn holdings(model: &GalaxyModel) -> Result<&Holdings, GraphError> {
    model.holdings.as_ref().ok_or(GraphError::NoHoldings)
}

/// Find every item matching the pattern and where each stack sits, sorted by total.
pub fn execute_have(model: &GalaxyModel, query: &HaveQuery) -> Result<Vec<HaveResult>, GraphError> {
    let holdings = holdings(model)?;
    let pattern = query.pattern.trim();
    if pattern.is_empty() {
        return Ok(Vec::new());
    }
    let mut found: BTreeMap<ItemId, HaveResult> = BTreeMap::new();
    for (container, stack) in holdings.stacks() {
        if !stack.id.matches(pattern) {
            continue;
        }
        if query.kind.is_some() && stack.kind != query.kind {
            continue;
        }
        let entry = found.entry(stack.id.clone()).or_insert_with(|| HaveResult {
            id: stack.id.clone(),
            name: stack.id.name(),
            kind: stack.kind,
            total: 0,
            locations: Vec::new(),
        });
        entry.total += u64::from(stack.amount);
        entry.locations.push(HaveLocation {
            container: container.kind.clone(),
            label: container.label(),
            amount: stack.amount,
            max: stack.max,
            access: container.access.clone(),
        });
    }
    let mut results: Vec<HaveResult> = found.into_values().collect();
    results.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.name.cmp(&b.name)));
    Ok(results)
}

/// The container overview, or the contents of the containers a label picks out.
pub fn execute_inventory(
    model: &GalaxyModel,
    query: &InventoryQuery,
) -> Result<InventoryResult, GraphError> {
    let holdings = holdings(model)?;
    if let Some(filter) = query
        .container
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let matched = select_containers(holdings, filter);
        if matched.is_empty() {
            return Err(GraphError::ContainerNotFound(filter.to_string()));
        }
        return Ok(InventoryResult::Contents(matched));
    }
    let mut containers: Vec<Container> = holdings.containers.clone();
    if query.free_only {
        containers.retain(|c| c.free() > 0 && !c.kind.is_technology() && c.kind.has_capacity());
        containers.sort_by_key(|c| std::cmp::Reverse(c.free()));
    }
    Ok(InventoryResult::Overview(containers))
}

/// Containers whose label is `filter` exactly, or, failing that, every container whose label contains it; case-insensitive.
fn select_containers(holdings: &Holdings, filter: &str) -> Vec<Container> {
    let wanted = filter.to_lowercase();
    let exact: Vec<Container> = holdings
        .containers
        .iter()
        .filter(|c| c.label().to_lowercase() == wanted)
        .cloned()
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    holdings
        .containers
        .iter()
        .filter(|c| c.label().to_lowercase().contains(&wanted))
        .cloned()
        .collect()
}

/// Totals per item across every grid, largest first.
pub fn execute_list_items(
    model: &GalaxyModel,
    query: &ListItemsQuery,
) -> Result<Vec<ItemTotal>, GraphError> {
    let holdings = holdings(model)?;
    let mut totals: BTreeMap<ItemId, (ItemTotal, Vec<ContainerKind>)> = BTreeMap::new();
    for (container, stack) in holdings.stacks() {
        if query.kind.is_some() && stack.kind != query.kind {
            continue;
        }
        let (entry, kinds) = totals.entry(stack.id.clone()).or_insert_with(|| {
            (
                ItemTotal {
                    id: stack.id.clone(),
                    name: stack.id.name(),
                    kind: stack.kind,
                    total: 0,
                    stacks: 0,
                    containers: 0,
                },
                Vec::new(),
            )
        });
        entry.total += u64::from(stack.amount);
        entry.stacks += 1;
        if !kinds.contains(&container.kind) {
            kinds.push(container.kind.clone());
        }
    }
    let mut items: Vec<ItemTotal> = totals
        .into_values()
        .map(|(mut item, kinds)| {
            item.containers = kinds.len();
            item
        })
        .filter(|item| item.total >= u64::from(query.min_amount))
        .collect();
    items.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.name.cmp(&b.name)));
    Ok(items)
}

/// Every exocraft with its parking base named from the atlas.
pub fn execute_exocraft(model: &GalaxyModel) -> Result<Vec<ExocraftRow>, GraphError> {
    let holdings = holdings(model)?;
    Ok(holdings
        .exocraft
        .iter()
        .map(|vehicle| {
            let base_name = vehicle.parked_at.and_then(|addr| {
                model
                    .bases
                    .values()
                    .find(|b| {
                        b.address == addr
                            && !matches!(
                                b.base_type,
                                BaseType::FreighterBase | BaseType::PlayerShipBase
                            )
                    })
                    .map(|b| b.name.clone())
            });
            let system = vehicle
                .parked_at
                .and_then(|addr| model.system(&SystemId::from_address(&addr)))
                .cloned();
            ExocraftRow {
                vehicle: vehicle.clone(),
                base_name,
                system,
            }
        })
        .collect())
}

/// A one-line summary of the exosuit and the storage containers for the dashboard: `Exosuit 30/93 · Storage 1 full`.
pub fn holdings_summary(holdings: &Holdings) -> String {
    let mut parts = Vec::new();
    if let Some(suit) = holdings.exosuit() {
        parts.push(format!(
            "Exosuit {}/{}",
            suit.occupied(),
            suit.unlocked_slots
        ));
    }
    let full: Vec<String> = holdings.full_storage().iter().map(|c| c.label()).collect();
    if !full.is_empty() {
        parts.push(format!("{} full", full.join(", ")));
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" \u{00B7} ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::Grade;
    use nms_core::holdings::{ItemStack, ShipSummary, ShipType};

    fn stack(id: &str, kind: ItemKind, amount: u32) -> ItemStack {
        ItemStack {
            id: ItemId::new(id),
            kind: Some(kind),
            amount,
            max: 9999,
            slot: (0, 0),
        }
    }

    fn container(
        kind: ContainerKind,
        unlocked: u16,
        stacks: Vec<ItemStack>,
        access: &[&str],
    ) -> Container {
        Container {
            kind,
            class: Some(Grade::C),
            width: 10,
            height: 6,
            unlocked_slots: unlocked,
            stacks,
            access: access.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn model() -> GalaxyModel {
        let mut model = GalaxyModel::new();
        let mut holdings = Holdings::default();
        holdings.containers.push(container(
            ContainerKind::Exosuit,
            93,
            vec![
                stack("^ASTEROID2", ItemKind::Substance, 371),
                stack("^GAS1", ItemKind::Substance, 40),
            ],
            &["with you"],
        ));
        holdings.containers.push(container(
            ContainerKind::ExosuitTech,
            60,
            vec![stack("^ASTEROID2", ItemKind::Technology, 100)],
            &["with you"],
        ));
        holdings.containers.push(container(
            ContainerKind::ExosuitCargo,
            0,
            vec![],
            &["with you"],
        ));
        holdings.containers.push(container(
            ContainerKind::Storage(1),
            2,
            vec![
                stack("^ASTEROID2", ItemKind::Substance, 9999),
                stack("^ASTEROID2", ItemKind::Substance, 927),
            ],
            &["Radioactive Base", "Freighter"],
        ));
        holdings.containers.push(container(
            ContainerKind::Storage(2),
            50,
            vec![stack("^ASTEROID1", ItemKind::Substance, 2959)],
            &["Freighter"],
        ));
        holdings.containers.push(container(
            ContainerKind::Storage(10),
            50,
            vec![stack("^GAS2", ItemKind::Substance, 5)],
            &["Freighter"],
        ));
        holdings.ships.push(ShipSummary {
            index: 0,
            name: String::new(),
            ship_type: Some(ShipType::Fighter),
            type_raw: "FIGHTERS".into(),
            class: Some(Grade::S),
            primary: true,
            general_slots: 32,
            cargo_slots: 0,
            tech_slots: 35,
            tech_installed: 20,
            damage: 79.6,
            shield: 26.5,
            hyperdrive: 0.0,
            agility: 37.8,
        });
        model.holdings = Some(holdings);
        model
    }

    #[test]
    fn have_totals_across_containers_and_skips_technology() {
        let results = execute_have(
            &model(),
            &HaveQuery {
                pattern: "gold".into(),
                kind: None,
            },
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        let gold = &results[0];
        assert_eq!(gold.name, "Gold");
        assert_eq!(gold.total, 11_297);
        assert_eq!(gold.locations.len(), 3);
        assert_eq!(gold.locations[0].label, "Exosuit");
        assert_eq!(gold.locations[1].amount, 9999);
        assert_eq!(
            gold.locations[1].access,
            vec!["Radioactive Base".to_string(), "Freighter".to_string()]
        );
    }

    #[test]
    fn have_matches_id_and_several_items() {
        let results = execute_have(
            &model(),
            &HaveQuery {
                pattern: "gas".into(),
                kind: None,
            },
        )
        .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Sulphurine");
        assert_eq!(results[1].name, "Radon");
        let by_id = execute_have(
            &model(),
            &HaveQuery {
                pattern: "asteroid1".into(),
                kind: None,
            },
        )
        .unwrap();
        assert_eq!(by_id[0].name, "Silver");
        assert!(
            execute_have(
                &model(),
                &HaveQuery {
                    pattern: "".into(),
                    kind: None
                }
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn have_kind_filter() {
        let results = execute_have(
            &model(),
            &HaveQuery {
                pattern: "gold".into(),
                kind: Some(ItemKind::Product),
            },
        )
        .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn inventory_overview_and_free_ordering() {
        match execute_inventory(&model(), &InventoryQuery::default()).unwrap() {
            InventoryResult::Overview(rows) => assert_eq!(rows.len(), 6),
            other => panic!("{other:?}"),
        }
        match execute_inventory(
            &model(),
            &InventoryQuery {
                container: None,
                free_only: true,
            },
        )
        .unwrap()
        {
            InventoryResult::Overview(rows) => {
                assert_eq!(rows[0].kind, ContainerKind::Exosuit);
                assert_eq!(rows[0].free(), 91);
                assert_eq!(rows[1].kind, ContainerKind::Storage(2));
                assert_eq!(rows[1].free(), 49);
                assert!(
                    rows.iter().all(|c| c.kind != ContainerKind::ExosuitCargo),
                    "nothing unlocked is not free space"
                );
                assert!(
                    rows.iter().all(|c| c.kind != ContainerKind::Storage(1)),
                    "a full container has no free slots"
                );
                assert!(rows.iter().all(|c| c.kind != ContainerKind::ExosuitTech));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn inventory_selects_exact_label_before_substring() {
        match execute_inventory(
            &model(),
            &InventoryQuery {
                container: Some("storage 1".into()),
                free_only: false,
            },
        )
        .unwrap()
        {
            InventoryResult::Contents(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].kind, ContainerKind::Storage(1));
            }
            other => panic!("{other:?}"),
        }
        match execute_inventory(
            &model(),
            &InventoryQuery {
                container: Some("Storage".into()),
                free_only: false,
            },
        )
        .unwrap()
        {
            InventoryResult::Contents(rows) => assert_eq!(rows.len(), 3),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            execute_inventory(
                &model(),
                &InventoryQuery {
                    container: Some("locker".into()),
                    free_only: false
                }
            ),
            Err(GraphError::ContainerNotFound(_))
        ));
    }

    #[test]
    fn list_items_totals_and_filters() {
        let items = execute_list_items(&model(), &ListItemsQuery::default()).unwrap();
        assert_eq!(items[0].name, "Gold");
        assert_eq!(items[0].total, 11_297);
        assert_eq!(items[0].stacks, 3);
        assert_eq!(items[0].containers, 2);
        assert_eq!(items.len(), 4);
        let big = execute_list_items(
            &model(),
            &ListItemsQuery {
                kind: None,
                min_amount: 100,
            },
        )
        .unwrap();
        assert_eq!(big.len(), 2);
        let tech = execute_list_items(
            &model(),
            &ListItemsQuery {
                kind: Some(ItemKind::Technology),
                min_amount: 0,
            },
        )
        .unwrap();
        assert!(tech.is_empty(), "technology is never totalled");
    }

    #[test]
    fn summary_line() {
        let model = model();
        assert_eq!(
            holdings_summary(model.holdings.as_ref().unwrap()),
            "Exosuit 2/93 \u{00B7} Storage 1 full"
        );
        assert_eq!(holdings_summary(&Holdings::default()), "-");
    }

    #[test]
    fn no_holdings_is_an_error() {
        let model = GalaxyModel::new();
        assert!(matches!(
            execute_have(
                &model,
                &HaveQuery {
                    pattern: "gold".into(),
                    kind: None
                }
            ),
            Err(GraphError::NoHoldings)
        ));
    }
}
