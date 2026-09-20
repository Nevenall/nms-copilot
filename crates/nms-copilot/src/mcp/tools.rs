//! NMS Copilot MCP tools.
//!
//! Each tool wraps a function from `nms-query`, translating between
//! JSON tool arguments and typed query structs.

use std::sync::Arc;

use fabryk_mcp::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp::{ToolRegistry, ToolResult, empty_input_schema};
use serde_json::{Value, json};
use tokio::sync::RwLock;

use nms_core::address::GalacticAddress;
use nms_core::biome::Biome;
use nms_core::fleet::ExpeditionState;
use nms_core::galaxy::Galaxy;
use nms_graph::BiomeFilter;
use nms_graph::GalaxyModel;
use nms_graph::RoutingAlgorithm;
use nms_query::base::{BaseQuery, alerts_from, execute_base};
use nms_query::display::{
    expedition_status_cell, format_distance, format_navigator_line, hex_to_emoji,
};
use nms_query::find::{FindQuery, ReferencePoint, execute_find};
use nms_query::fleet::{execute_fleet, fleet_alerts};
use nms_query::inventory::{
    HaveQuery, InventoryQuery, InventoryResult, execute_exocraft, execute_have, execute_inventory,
    holdings,
};
use nms_query::route::{RouteFrom, RouteQuery, TargetSelection, execute_route};
use nms_query::show::{ShowQuery, ShowResult, execute_show};
use nms_query::stats::{StatsQuery, execute_stats};

/// All NMS tools backed by a shared GalaxyModel.
///
/// Uses `RwLock` to support live updates from the file watcher.
/// Tool handlers acquire a read lock; the watcher takes a write lock
/// to apply deltas.
pub struct NmsTools {
    model: Arc<RwLock<GalaxyModel>>,
}

impl NmsTools {
    pub fn new(model: Arc<RwLock<GalaxyModel>>) -> Self {
        Self { model }
    }
}

impl ToolRegistry for NmsTools {
    fn tools(&self) -> Vec<Tool> {
        vec![
            search_planets_tool(),
            plan_route_tool(),
            where_am_i_tool(),
            whats_nearby_tool(),
            show_system_tool(),
            show_base_tool(),
            base_status_tool(),
            fleet_status_tool(),
            have_item_tool(),
            inventory_summary_tool(),
            list_ships_tool(),
            convert_coordinates_tool(),
            galaxy_stats_tool(),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let model = Arc::clone(&self.model);
        match name {
            "search_planets" => Some(Box::pin(handle_search_planets(model, args))),
            "plan_route" => Some(Box::pin(handle_plan_route(model, args))),
            "where_am_i" => Some(Box::pin(handle_where_am_i(model, args))),
            "whats_nearby" => Some(Box::pin(handle_whats_nearby(model, args))),
            "show_system" => Some(Box::pin(handle_show_system(model, args))),
            "show_base" => Some(Box::pin(handle_show_base(model, args))),
            "base_status" => Some(Box::pin(handle_base_status(model, args))),
            "fleet_status" => Some(Box::pin(handle_fleet_status(model, args))),
            "have_item" => Some(Box::pin(handle_have_item(model, args))),
            "inventory_summary" => Some(Box::pin(handle_inventory_summary(model, args))),
            "list_ships" => Some(Box::pin(handle_list_ships(model, args))),
            "convert_coordinates" => Some(Box::pin(handle_convert(model, args))),
            "galaxy_stats" => Some(Box::pin(handle_galaxy_stats(model, args))),
            _ => None,
        }
    }
}

// ── Tool Definitions ────────────────────────────────────────────

fn schema(json: Value) -> Arc<serde_json::Map<String, Value>> {
    match json {
        Value::Object(map) => Arc::new(map),
        _ => unreachable!("schema must be a JSON object"),
    }
}

fn search_planets_tool() -> Tool {
    Tool::new(
        "search_planets",
        "Search planets by biome, distance, discoverer, or name.",
        schema(json!({
            "type": "object",
            "properties": {
                "biome": {
                    "type": "string",
                    "description": "Biome type (Lush, Toxic, Scorched, Radioactive, Frozen, Barren, Dead, Weird, Swamp, Lava, etc.)"
                },
                "within_ly": {
                    "type": "number",
                    "description": "Maximum distance in light-years from reference point"
                },
                "nearest": {
                    "type": "integer",
                    "description": "Return only the N nearest results"
                },
                "discoverer": {
                    "type": "string",
                    "description": "Filter by discoverer username (substring match)"
                },
                "named_only": {
                    "type": "boolean",
                    "description": "Only include named planets/systems"
                },
                "from_base": {
                    "type": "string",
                    "description": "Measure distance from this base name (default: player position)"
                },
                "infested": {
                    "type": "boolean",
                    "description": "Only include infested planets"
                }
            }
        })),
    )
}

fn plan_route_tool() -> Tool {
    Tool::new(
        "plan_route",
        "Plan an optimal route through target systems.",
        schema(json!({
            "type": "object",
            "properties": {
                "biome": {
                    "type": "string",
                    "description": "Visit all systems with this biome type"
                },
                "targets": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Specific system or base names to visit"
                },
                "from_base": {
                    "type": "string",
                    "description": "Start from this base (default: player position)"
                },
                "warp_range": {
                    "type": "number",
                    "description": "Maximum warp range per hop in light-years"
                },
                "within_ly": {
                    "type": "number",
                    "description": "Only include targets within this radius"
                },
                "max_targets": {
                    "type": "integer",
                    "description": "Maximum number of targets to include"
                },
                "algorithm": {
                    "type": "string",
                    "enum": ["2opt", "nearest-neighbor"],
                    "description": "Routing algorithm (default: 2opt)"
                },
                "round_trip": {
                    "type": "boolean",
                    "description": "Return to starting system after visiting all targets"
                }
            }
        })),
    )
}

fn where_am_i_tool() -> Tool {
    Tool::new(
        "where_am_i",
        "Get the player's current location.",
        Arc::new(empty_input_schema()),
    )
}

fn whats_nearby_tool() -> Tool {
    Tool::new(
        "whats_nearby",
        "Find systems and planets near the player's current position.",
        schema(json!({
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "description": "Number of nearby results to return (default: 10)"
                },
                "biome": {
                    "type": "string",
                    "description": "Filter by biome type"
                }
            }
        })),
    )
}

fn show_system_tool() -> Tool {
    Tool::new(
        "show_system",
        "Get detailed information about a star system.",
        schema(json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "System name or hex address"
                }
            },
            "required": ["name"]
        })),
    )
}

fn show_base_tool() -> Tool {
    Tool::new(
        "show_base",
        "Get detailed information about a player base.",
        schema(json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Base name (case-insensitive)"
                }
            },
            "required": ["name"]
        })),
    )
}

fn base_status_tool() -> Tool {
    Tool::new(
        "base_status",
        "Crops ready to harvest, supply depot fill by pipe network, and power equipment at the player's bases. Depot contents are as of the last save; crop timing is computed from the current clock.",
        schema(json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Base name (exact, then substring match). Omit for every base."
                }
            }
        })),
    )
}

fn fleet_status_tool() -> Tool {
    Tool::new(
        "fleet_status",
        "Frigate expeditions: which are running, waiting for the player's decision, or back and awaiting debrief, with elapsed time and a rough estimate of time left; the Navigator's remaining daily offers and next refresh; free Fleet Command Rooms; and every frigate with its stats and whether it is out. Timing is computed from the current clock against the last save.",
        schema(json!({ "type": "object", "properties": {} })),
    )
}

fn have_item_tool() -> Tool {
    Tool::new(
        "have_item",
        "Does the player have an item, how much, and where? Matches item names and internal IDs (substring, case-insensitive) across every container: exosuit, freighter, the ten storage containers, ships, exocraft, multi-tools, machine buffers, and the corvette parts store. Returns one entry per matched item with its total and one row per stack, each saying which container and where that container can be opened. Technology slots are never counted. Contents are as of the last save.",
        schema(json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Item name or ID to look for, e.g. \"gold\" or \"ASTEROID2\""
                },
                "kind": {
                    "type": "string",
                    "description": "Only items of this type: substance, product, or technology"
                }
            },
            "required": ["pattern"]
        })),
    )
}

fn inventory_summary_tool() -> Tool {
    Tool::new(
        "inventory_summary",
        "Every container the player owns with its class, used and unlocked slots, free slots, and where it can be opened; or, given a container name, that container's contents slot by slot. Free space comes from the unlocked slots, not the grid's shape.",
        schema(json!({
            "type": "object",
            "properties": {
                "container": {
                    "type": "string",
                    "description": "A container to list in full (\"storage 3\", \"exosuit\", \"freighter\", \"ship 1\") or a word its label contains (\"ship\", \"storage\"). Omit for the overview."
                },
                "free_only": {
                    "type": "boolean",
                    "description": "Overview only: order by free slots, most first, and leave out full containers"
                }
            }
        })),
    )
}

fn list_ships_tool() -> Tool {
    Tool::new(
        "list_ships",
        "The player's ships, exocraft, and multi-tools: type, class, unlocked slots, installed technology count, class bonuses, which ship is primary and which tool is equipped, and where each exocraft is parked.",
        schema(json!({ "type": "object", "properties": {} })),
    )
}

fn convert_coordinates_tool() -> Tool {
    Tool::new(
        "convert_coordinates",
        "Convert between portal glyphs, signal booster coordinates, and galactic addresses.",
        schema(json!({
            "type": "object",
            "properties": {
                "glyphs": {
                    "type": "string",
                    "description": "Portal glyphs as 12 hex digits (e.g., 01717D8A4EA2)"
                },
                "coords": {
                    "type": "string",
                    "description": "Signal booster coordinates (XXXX:YYYY:ZZZZ:SSSS)"
                },
                "galactic_address": {
                    "type": "string",
                    "description": "Galactic address as hex (0x...)"
                }
            }
        })),
    )
}

fn galaxy_stats_tool() -> Tool {
    Tool::new(
        "galaxy_stats",
        "Get aggregate statistics about the explored galaxy.",
        Arc::new(empty_input_schema()),
    )
}

// ── Helpers ─────────────────────────────────────────────────────

fn text_result(json: Value) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string()),
    )]))
}

fn tool_error(msg: &str) -> ErrorData {
    ErrorData::invalid_params(msg.to_string(), None)
}

// ── Shared JSON builders (used by both tools and resources) ──────

/// Build JSON for the player's current location.
///
/// Returns `Err` if the player position is not available.
pub(crate) fn build_where_am_i_json(model: &GalaxyModel) -> Result<serde_json::Value, String> {
    let addr = model
        .player_position()
        .ok_or_else(|| "Player position not available".to_string())?;

    let portal_hex = format!("{:012X}", addr.packed());
    let galaxy = Galaxy::by_index(addr.reality_index);

    let nearest = model.nearest_systems(addr, 1);
    let (system_name, system_planets) = nearest
        .first()
        .and_then(|(id, _)| model.system(id))
        .map(|s| (s.name.as_deref().unwrap_or("-"), s.planets.len()))
        .unwrap_or(("(unknown)", 0));

    Ok(json!({
        "system": system_name,
        "planets_in_system": system_planets,
        "galaxy": galaxy.name,
        "voxel_x": addr.voxel_x(),
        "voxel_y": addr.voxel_y(),
        "voxel_z": addr.voxel_z(),
        "solar_system_index": addr.solar_system_index(),
        "portal_glyphs_hex": portal_hex,
        "portal_glyphs_emoji": hex_to_emoji(&portal_hex),
        "signal_booster": addr.to_signal_booster(),
    }))
}

/// Build JSON for galaxy statistics.
pub(crate) fn build_galaxy_stats_json(model: &GalaxyModel) -> serde_json::Value {
    let result = execute_stats(
        model,
        &StatsQuery {
            biomes: true,
            discoveries: true,
        },
    );

    let biome_breakdown: Vec<Value> = {
        let mut biomes: Vec<_> = result.biome_counts.iter().collect();
        biomes.sort_by_key(|item| std::cmp::Reverse(*item.1));
        biomes
            .iter()
            .map(|(biome, count)| json!({ "biome": biome.to_string(), "count": count }))
            .collect()
    };

    json!({
        "systems": result.system_count,
        "planets": result.planet_count,
        "bases": result.base_count,
        "named_systems": result.named_system_count,
        "named_planets": result.named_planet_count,
        "infested_planets": result.infested_count,
        "biome_distribution": biome_breakdown,
        "unknown_biome_count": result.unknown_biome_count,
    })
}

/// Build JSON for all player bases.
pub(crate) fn build_bases_json(model: &GalaxyModel) -> serde_json::Value {
    let bases: Vec<Value> = model
        .bases
        .values()
        .map(|b| {
            let portal_hex = format!("{:012X}", b.address.packed());
            json!({
                "name": b.name,
                "type": format!("{}", b.base_type),
                "portal_glyphs_hex": portal_hex,
                "portal_glyphs_emoji": hex_to_emoji(&portal_hex),
            })
        })
        .collect();

    json!({
        "count": bases.len(),
        "bases": bases,
    })
}

// ── Tool Handlers ───────────────────────────────────────────────

async fn handle_search_planets(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let biome = parse_biome_arg(&args, "biome")?;

    let reference = match args.get("from_base").and_then(|v| v.as_str()) {
        Some(name) => ReferencePoint::Base(name.into()),
        None => ReferencePoint::CurrentPosition,
    };

    let infested = args
        .get("infested")
        .and_then(|v| v.as_bool())
        .and_then(|b| b.then_some(true));

    let query = FindQuery {
        biome,
        biome_subtype: None,
        infested,
        within_ly: args.get("within_ly").and_then(|v| v.as_f64()),
        nearest: args
            .get("nearest")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        discoverer: args
            .get("discoverer")
            .and_then(|v| v.as_str())
            .map(String::from),
        named_only: args
            .get("named_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        name_pattern: None,
        from: reference,
    };

    let results = execute_find(&model, &query).map_err(|e| tool_error(&e.to_string()))?;

    let planets: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "planet": r.planet.name.as_deref().unwrap_or("-"),
                "biome": r.planet.biome.map(|b| b.to_string()),
                "infested": r.planet.infested,
                "system": r.system.name.as_deref().unwrap_or("-"),
                "distance": format_distance(r.distance_ly),
                "distance_ly": r.distance_ly,
                "portal_glyphs_hex": &r.portal_hex,
                "portal_glyphs_emoji": hex_to_emoji(&r.portal_hex),
                "discoverer": r.system.discoverer.as_deref().unwrap_or("unknown"),
            })
        })
        .collect();

    text_result(json!({
        "count": planets.len(),
        "results": planets,
    }))
}

async fn handle_plan_route(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let targets_arg = args.get("targets").and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>()
    });

    let biome_arg = parse_biome_arg(&args, "biome")?;

    let targets = if let Some(names) = targets_arg {
        if names.is_empty() {
            return Err(tool_error("targets array is empty"));
        }
        TargetSelection::Named(names)
    } else if let Some(biome) = biome_arg {
        TargetSelection::Biome(BiomeFilter {
            biome: Some(biome),
            ..Default::default()
        })
    } else {
        return Err(tool_error("Specify either 'biome' or 'targets'"));
    };

    let from = match args.get("from_base").and_then(|v| v.as_str()) {
        Some(name) => RouteFrom::Base(name.into()),
        None => RouteFrom::CurrentPosition,
    };

    let algorithm = match args.get("algorithm").and_then(|v| v.as_str()) {
        Some("nearest-neighbor") | Some("nn") => RoutingAlgorithm::NearestNeighbor,
        _ => RoutingAlgorithm::TwoOpt,
    };

    let query = RouteQuery {
        targets,
        from,
        warp_range: args.get("warp_range").and_then(|v| v.as_f64()),
        within_ly: args.get("within_ly").and_then(|v| v.as_f64()),
        max_targets: args
            .get("max_targets")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        algorithm,
        return_to_start: args
            .get("round_trip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    let result = execute_route(&model, &query).map_err(|e| tool_error(&e.to_string()))?;

    let hops: Vec<Value> = result
        .route
        .hops
        .iter()
        .enumerate()
        .map(|(i, hop)| {
            let sys = model.system(&hop.system_id);
            let sys_name = sys.and_then(|s| s.name.as_deref()).unwrap_or("-");
            let portal_hex = sys
                .map(|s| format!("{:012X}", s.address.packed()))
                .unwrap_or_default();

            json!({
                "hop": i + 1,
                "system": sys_name,
                "is_waypoint": hop.is_waypoint,
                "leg_distance": format_distance(hop.leg_distance_ly),
                "leg_distance_ly": hop.leg_distance_ly,
                "cumulative": format_distance(hop.cumulative_ly),
                "cumulative_ly": hop.cumulative_ly,
                "portal_glyphs_hex": portal_hex,
                "portal_glyphs_emoji": hex_to_emoji(&portal_hex),
            })
        })
        .collect();

    let algo_name = match result.algorithm {
        RoutingAlgorithm::NearestNeighbor => "nearest-neighbor",
        RoutingAlgorithm::TwoOpt => "2-opt",
    };

    text_result(json!({
        "hops": hops,
        "total_distance": format_distance(result.route.total_distance_ly),
        "total_distance_ly": result.route.total_distance_ly,
        "targets_visited": result.targets_visited,
        "algorithm": algo_name,
        "warp_range": result.warp_range,
        "warp_jumps": result.warp_jumps,
    }))
}

async fn handle_where_am_i(
    model: Arc<RwLock<GalaxyModel>>,
    _args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let json = build_where_am_i_json(&model).map_err(|e| tool_error(&e))?;
    text_result(json)
}

async fn handle_whats_nearby(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let count = args
        .get("count")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(10);

    let biome = parse_biome_arg(&args, "biome")?;

    let query = FindQuery {
        biome,
        nearest: Some(count),
        from: ReferencePoint::CurrentPosition,
        ..Default::default()
    };

    let results = execute_find(&model, &query).map_err(|e| tool_error(&e.to_string()))?;

    let nearby: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "planet": r.planet.name.as_deref().unwrap_or("-"),
                "biome": r.planet.biome.map(|b| b.to_string()),
                "system": r.system.name.as_deref().unwrap_or("-"),
                "distance": format_distance(r.distance_ly),
                "distance_ly": r.distance_ly,
                "portal_glyphs_emoji": hex_to_emoji(&r.portal_hex),
            })
        })
        .collect();

    text_result(json!({
        "count": nearby.len(),
        "from": "player position",
        "results": nearby,
    }))
}

async fn handle_show_system(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tool_error("'name' is required"))?;

    let result = execute_show(&model, &ShowQuery::System(name.into()))
        .map_err(|e| tool_error(&e.to_string()))?;

    match result {
        ShowResult::System(s) => {
            let planets: Vec<Value> = s
                .system
                .planets
                .iter()
                .map(|p| {
                    json!({
                        "index": p.index,
                        "name": p.name.as_deref().unwrap_or("-"),
                        "biome": p.biome.map(|b| b.to_string()),
                        "infested": p.infested,
                    })
                })
                .collect();

            let generated = s.generated.as_ref();
            let attributes = generated.and_then(|g| g.attributes.as_ref()).map(|a| {
                json!({
                    "star": a.star.to_string(),
                    "economy": a.economy.to_string(),
                    "wealth": a.wealth.to_string(),
                    "conflict": a.conflict.to_string(),
                    "lifeform": a.race.map(|r| r.to_string()),
                    "uncharted": a.uncharted,
                    "abandoned": a.abandoned,
                    "pirate": a.pirate,
                    "planets": a.planets,
                    "moons": a.moons,
                })
            });
            text_result(json!({
                "name": s.system.name.as_deref().or(generated.map(|g| g.name.as_str())).unwrap_or("-"),
                "name_generated": s.system.name.is_none() && generated.is_some(),
                "region": generated.map(|g| g.region.as_str()),
                "attributes": attributes,
                "galaxy": s.galaxy_name,
                "discoverer": s.system.discoverer.as_deref().unwrap_or("unknown"),
                "portal_glyphs_hex": s.portal_hex,
                "portal_glyphs_emoji": hex_to_emoji(&s.portal_hex),
                "distance_from_player": s.distance_from_player.map(format_distance),
                "voxel_x": s.system.address.voxel_x(),
                "voxel_y": s.system.address.voxel_y(),
                "voxel_z": s.system.address.voxel_z(),
                "planets": planets,
            }))
        }
        ShowResult::Base(_) => Err(tool_error("unexpected result type")),
    }
}

async fn handle_show_base(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tool_error("'name' is required"))?;

    let result = execute_show(&model, &ShowQuery::Base(name.into()))
        .map_err(|e| tool_error(&e.to_string()))?;

    match result {
        ShowResult::Base(b) => text_result(json!({
            "name": b.base.name,
            "type": format!("{}", b.base.base_type),
            "galaxy": b.galaxy_name,
            "portal_glyphs_hex": b.portal_hex,
            "portal_glyphs_emoji": hex_to_emoji(&b.portal_hex),
            "distance_from_player": b.distance_from_player.map(format_distance),
            "system": b.system.as_ref().and_then(|s| s.name.as_deref()),
            "system_planet_count": b.system.as_ref().map(|s| s.planets.len()),
        })),
        ShowResult::System(_) => Err(tool_error("unexpected result type")),
    }
}

/// Current Unix time in seconds.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build JSON for base statuses.
pub(crate) fn build_base_status_json(
    model: &GalaxyModel,
    name: Option<&str>,
    now: i64,
) -> Result<Value, nms_graph::GraphError> {
    let statuses = execute_base(
        model,
        &BaseQuery {
            name: name.map(str::to_string),
        },
        now,
    )?;
    let bases: Vec<Value> = statuses.iter().map(|s| {
        let crops: Vec<Value> = s.crops.iter().map(|row| json!({
            "crop": row.label,
            "count": row.count,
            "ready": row.ready,
            "next_ready_secs": row.next_ready_secs(),
            "progress": row.progress,
            "batches": row.batches.iter().map(|b| json!({ "count": b.count, "remaining_secs": b.remaining_secs })).collect::<Vec<_>>(),
        })).collect();
        let networks: Vec<Value> = s.networks.iter().map(|n| json!({
            "network": n.index,
            "depots": n.depots,
            "gas_extractors": n.gas_extractors,
            "mineral_extractors": n.mineral_extractors,
            "stored": n.stored,
            "capacity": n.capacity,
            "full": n.is_full(),
        })).collect();
        let generators: Vec<Value> = s.power.generators.iter().map(|g| json!({ "kind": g.kind.display_name(), "count": g.count, "raw": g.raw })).collect();
        json!({
            "name": s.base.name,
            "type": format!("{}", s.base.base_type),
            "galaxy": s.galaxy_name,
            "portal_glyphs_hex": s.portal_hex,
            "distance_from_player": s.distance_from_player.map(format_distance),
            "snapshot_unix": s.snapshot,
            "snapshot_age_secs": s.snapshot.map(|t| (now - t).max(0)),
            "crops_ready": s.crops_ready,
            "crops_total": s.crops_total,
            "crops": crops,
            "extraction": { "stored": s.extraction_stored(), "capacity": s.extraction_capacity(), "full_networks": s.full_networks(), "networks": networks },
            "power": {
                "batteries": s.power.batteries,
                "batteries_full": s.power.batteries_full,
                "batteries_empty": s.power.batteries_empty,
                "battery_charge": s.power.battery_charge,
                "battery_capacity": s.power.battery_capacity,
                "generators": generators,
                "wires": s.power.wires,
            },
        })
    }).collect();
    let alerts: Vec<String> = alerts_from(&statuses).iter().map(|a| a.text()).collect();
    Ok(json!({ "now_unix": now, "count": bases.len(), "bases": bases, "alerts": alerts }))
}

async fn handle_base_status(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let name = args.get("name").and_then(|v| v.as_str());
    let json =
        build_base_status_json(&model, name, unix_now()).map_err(|e| tool_error(&e.to_string()))?;
    text_result(json)
}

/// Build JSON for the fleet status.
pub(crate) fn build_fleet_status_json(
    model: &GalaxyModel,
    now: i64,
) -> Result<Value, nms_graph::GraphError> {
    let status = execute_fleet(model, now)?;
    let expeditions: Vec<Value> = status
        .expeditions
        .iter()
        .map(|row| {
            let e = &row.expedition;
            let state = match row.state {
                ExpeditionState::Waiting => "waiting_for_player",
                ExpeditionState::Complete => "returned",
                ExpeditionState::Running => "running",
            };
            let location = e.location.map(|addr| {
                json!({
                    "system": row.system.as_ref().and_then(|s| s.name.clone()),
                    "hex": format!("{:012X}", addr.packed()),
                    "distance_from_player": row.distance_from_player.map(format_distance),
                })
            });
            let frigates: Vec<Value> = row
                .frigates
                .iter()
                .map(|f| {
                    json!({
                        "index": f.frigate.index + 1,
                        "name": f.frigate.label(),
                        "class": f.frigate.class_label(),
                        "grade": f.frigate.grade_label(),
                        "damaged": e.damaged.contains(&f.frigate.index),
                        "destroyed": e.destroyed.contains(&f.frigate.index),
                    })
                })
                .collect();
            let events: Vec<Value> = e
                .events
                .iter()
                .enumerate()
                .map(|(i, ev)| {
                    json!({
                        "number": i + 1,
                        "event": ev.label(),
                        "decision": ev.intervention_label(),
                        "resolved": i < e.resolved(),
                        "success": (i < e.resolved()).then_some(ev.success),
                        "where": row.event_places.get(i),
                    })
                })
                .collect();
            json!({
                "number": row.number,
                "seed": format!("{:#x}", e.seed),
                "name": e.name,
                "type": row.category_label(),
                "length": row.duration_label(),
                "state": state,
                "status": expedition_status_cell(row),
                "started_unix": e.start,
                "elapsed_secs": row.elapsed_secs,
                "waiting_since_unix": e.waiting_since(),
                "waiting_secs": row.waiting_secs,
                "estimate_remaining_secs": row.estimate_remaining_secs,
                "events_resolved": e.resolved(),
                "events_total": e.total(),
                "successes": e.successes,
                "failures": e.failures,
                "speed_multiplier": e.speed_multiplier,
                "location": location,
                "frigates": frigates,
                "events": events,
            })
        })
        .collect();
    let frigates: Vec<Value> = status.frigates.iter().map(|f| {
        let fr = &f.frigate;
        json!({
            "index": fr.index + 1,
            "name": fr.label(),
            "class": fr.class_label(),
            "race": fr.race_label(),
            "grade": fr.grade_label(),
            "stats": { "combat": fr.combat(), "exploration": fr.exploration(), "industrial": fr.industrial(), "trade": fr.trade(), "stat_5": fr.stat_five(), "support": fr.support() },
            "perks": fr.perks(),
            "modules": fr.module_summary(),
            "module_ids": fr.trait_labels(),
            "damage_taken": fr.damage_taken,
            "times_damaged": fr.times_damaged,
            "expeditions": fr.expeditions,
            "successes": fr.successes,
            "failures": fr.failures,
            "out_on": f.out_on,
        })
    }).collect();
    let alerts: Vec<String> = fleet_alerts(&status).iter().map(|a| a.text()).collect();
    Ok(json!({
        "now_unix": now,
        "expeditions": expeditions,
        "navigator": {
            "offer_day": status.offers.day,
            "next_refresh_unix": status.offers.next_refresh,
            "secs_until_refresh": status.offers.secs_until_refresh,
            "refreshed": status.offers.refreshed,
            "launched_today": status.offers.launched_today,
            "offers_left": status.offers.left,
            "offers_per_day": status.offers.per_day,
        },
        "command_rooms": status.command_rooms,
        "rooms_free": status.rooms_free,
        "frigates_total": status.frigates.len(),
        "frigates_home": status.frigates_home,
        "frigates": frigates,
        "summary": format_navigator_line(&status),
        "alerts": alerts,
    }))
}

async fn handle_fleet_status(
    model: Arc<RwLock<GalaxyModel>>,
    _args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let json =
        build_fleet_status_json(&model, unix_now()).map_err(|e| tool_error(&e.to_string()))?;
    text_result(json)
}

fn container_json(c: &nms_core::Container) -> Value {
    json!({
        "container": c.label(),
        "class": c.class_label(),
        "used": c.occupied(),
        "unlocked": c.kind.has_capacity().then_some(c.unlocked_slots),
        "free": (c.kind.has_capacity() && !c.kind.is_technology()).then_some(c.free()),
        "technology_only": c.kind.is_technology(),
        "reachable_from": c.access,
    })
}

async fn handle_have_item(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| tool_error("pattern is required"))?;
    let kind = match args.get("kind").and_then(|v| v.as_str()) {
        None => None,
        Some(word) => Some(nms_core::ItemKind::parse(word).ok_or_else(|| {
            tool_error(&format!(
                "unknown item type \"{word}\": use substance, product, or technology"
            ))
        })?),
    };
    let model = model.read().await;
    let results = execute_have(
        &model,
        &HaveQuery {
            pattern: pattern.to_string(),
            kind,
        },
    )
    .map_err(|e| tool_error(&e.to_string()))?;
    let items: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "id": r.id.display_id(),
                "name": r.name,
                "type": r.kind.map(|k| k.display_name()),
                "total": r.total,
                "stacks": r.locations.iter().map(|l| json!({
                    "container": l.label,
                    "amount": l.amount,
                    "stack_max": l.max,
                    "reachable_from": l.access,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    text_result(json!({ "pattern": pattern, "matches": items }))
}

async fn handle_inventory_summary(
    model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let container = args
        .get("container")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let free_only = args
        .get("free_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let model = model.read().await;
    let result = execute_inventory(
        &model,
        &InventoryQuery {
            container,
            free_only,
        },
    )
    .map_err(|e| tool_error(&e.to_string()))?;
    match result {
        InventoryResult::Overview(containers) => text_result(json!({
            "containers": containers.iter().map(container_json).collect::<Vec<_>>(),
        })),
        InventoryResult::Contents(containers) => text_result(json!({
            "containers": containers.iter().map(|c| {
                let mut v = container_json(c);
                v["slots"] = c.stacks.iter().map(|s| json!({
                    "slot": [s.slot.0 + 1, s.slot.1 + 1],
                    "id": s.id.display_id(),
                    "name": s.name(),
                    "type": s.kind.map(|k| k.display_name()),
                    "amount": s.amount,
                    "stack_max": s.max,
                })).collect::<Vec<_>>().into();
                v
            }).collect::<Vec<_>>(),
        })),
    }
}

async fn handle_list_ships(
    model: Arc<RwLock<GalaxyModel>>,
    _args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    let h = holdings(&model).map_err(|e| tool_error(&e.to_string()))?;
    let exocraft = execute_exocraft(&model).map_err(|e| tool_error(&e.to_string()))?;
    text_result(json!({
        "ships": h.ships.iter().map(|s| json!({
            "index": s.index + 1,
            "name": s.label(),
            "type": s.type_label(),
            "class": s.class_label(),
            "primary": s.primary,
            "slots": { "general": s.general_slots, "cargo": s.cargo_slots, "technology": s.tech_slots },
            "technology_installed": s.tech_installed,
            "bonuses": { "damage": s.damage, "shield": s.shield, "hyperdrive": s.hyperdrive, "agility": s.agility },
            "where": s.location(),
        })).collect::<Vec<_>>(),
        "exocraft": exocraft.iter().map(|row| json!({
            "index": row.vehicle.index + 1,
            "name": row.vehicle.label(),
            "slots": row.vehicle.slots,
            "technology_installed": row.vehicle.tech_installed,
            "parked_at_base": row.base_name,
            "parked_at_system": row.system.as_ref().and_then(|s| s.name.clone()),
            "parked_at_address": row.vehicle.parked_at.map(|a| format!("{:012X}", a.packed())),
        })).collect::<Vec<_>>(),
        "multitools": h.multitools.iter().map(|t| json!({
            "index": t.index + 1,
            "name": t.label(),
            "class": t.class_label(),
            "equipped": t.active,
            "slots": t.slots,
            "technology_installed": t.tech_installed,
            "bonuses": { "damage": t.damage, "mining": t.mining, "scan": t.scan },
        })).collect::<Vec<_>>(),
    }))
}

async fn handle_convert(
    _model: Arc<RwLock<GalaxyModel>>,
    args: Value,
) -> Result<CallToolResult, ErrorData> {
    let addr = if let Some(glyphs) = args.get("glyphs").and_then(|v| v.as_str()) {
        let hex = glyphs
            .strip_prefix("0x")
            .or_else(|| glyphs.strip_prefix("0X"))
            .unwrap_or(glyphs);
        if hex.len() != 12 {
            return Err(tool_error(&format!(
                "Portal glyphs must be 12 hex digits, got {}",
                hex.len()
            )));
        }
        let packed =
            u64::from_str_radix(hex, 16).map_err(|_| tool_error(&format!("Invalid hex: {hex}")))?;
        GalacticAddress::from_packed(packed, 0)
    } else if let Some(coords) = args.get("coords").and_then(|v| v.as_str()) {
        GalacticAddress::from_signal_booster(coords, 0, 0)
            .map_err(|e| tool_error(&format!("Invalid coordinates: {e}")))?
    } else if let Some(ga) = args.get("galactic_address").and_then(|v| v.as_str()) {
        let hex = ga
            .strip_prefix("0x")
            .or_else(|| ga.strip_prefix("0X"))
            .unwrap_or(ga);
        let ua = u64::from_str_radix(hex, 16)
            .map_err(|_| tool_error(&format!("Invalid galactic address: {ga}")))?;
        // Save-file universe address layout (system and planet index in the upper bits).
        GalacticAddress::from_save_ua(ua, 0)
    } else {
        return Err(tool_error(
            "Specify 'glyphs', 'coords', or 'galactic_address'",
        ));
    };

    let portal_hex = format!("{:012X}", addr.packed());
    let galaxy = Galaxy::by_index(addr.reality_index);

    text_result(json!({
        "portal_glyphs_hex": portal_hex,
        "portal_glyphs_emoji": hex_to_emoji(&portal_hex),
        "signal_booster": addr.to_signal_booster(),
        "galactic_address": format!("0x{:X}", addr.to_save_ua()),
        "voxel_x": addr.voxel_x(),
        "voxel_y": addr.voxel_y(),
        "voxel_z": addr.voxel_z(),
        "solar_system_index": addr.solar_system_index(),
        "planet_index": addr.planet_index(),
        "galaxy": galaxy.name,
    }))
}

async fn handle_galaxy_stats(
    model: Arc<RwLock<GalaxyModel>>,
    _args: Value,
) -> Result<CallToolResult, ErrorData> {
    let model = model.read().await;
    text_result(build_galaxy_stats_json(&model))
}

fn parse_biome_arg(args: &Value, key: &str) -> Result<Option<Biome>, ErrorData> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| {
            s.parse::<Biome>()
                .map_err(|e| tool_error(&format!("Invalid biome: {e}")))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_graph::GalaxyModel;
    use tokio::sync::RwLock;

    fn test_model() -> Arc<RwLock<GalaxyModel>> {
        let json = r#"{
            "Version": 4720, "Platform": "Mac|Final", "ActiveContext": "Main",
            "CommonStateData": {"SaveName": "Test", "TotalPlayTime": 100},
            "BaseContext": {
                "GameMode": 1,
                "PlayerStateData": {
                    "UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 1, "PlanetIndex": 0}},
                    "Units": 0, "Nanites": 0, "Specials": 0,
                    "PersistentPlayerBases": [{"BaseVersion": 8, "GalacticAddress": "0x00100000000064", "Position": [0.0,0.0,0.0], "Forward": [1.0,0.0,0.0], "LastUpdateTimestamp": 0, "Objects": [], "RID": "", "Owner": {"LID":"","UID":"1","USN":"","PTK":"ST","TS":0}, "Name": "Alpha Base", "BaseType": {"PersistentBaseTypes": "HomePlanetBase"}, "LastEditedById": "", "LastEditedByUsername": ""}]
                }
            },
            "ExpeditionContext": {"GameMode": 6, "PlayerStateData": {"UniverseAddress": {"RealityIndex": 0, "GalacticAddress": {"VoxelX": 0, "VoxelY": 0, "VoxelZ": 0, "SolarSystemIndex": 0, "PlanetIndex": 0}}, "Units": 0, "Nanites": 0, "Specials": 0, "PersistentPlayerBases": []}},
            "DiscoveryManagerData": {"DiscoveryData-v1": {"ReserveStore": 0, "ReserveManaged": 0, "Store": {"Record": [
                {"DD": {"UA": "0x00100000000064", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10100000000064", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Explorer", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x00200000000C80", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10200000000C80", "DT": "Planet", "VP": ["0xCD", 1]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x00300000001900", "DT": "SolarSystem", "VP": []}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}},
                {"DD": {"UA": "0x10300000001900", "DT": "Planet", "VP": ["0xAB", 0]}, "DM": {}, "OWS": {"LID": "", "UID": "1", "USN": "Traveler", "PTK": "ST", "TS": 1700000000}, "FL": {"U": 1}}
            ]}}}
        }"#;
        Arc::new(RwLock::new(
            nms_save::parse_save(json.as_bytes())
                .map(|save| GalaxyModel::from_save(&save))
                .expect("test model JSON is valid"),
        ))
    }

    #[test]
    fn test_tools_has_all_ten() {
        let tools = NmsTools::new(test_model());
        let tool_list = tools.tools();
        let names: Vec<&str> = tool_list.iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(names.len(), 13);
        assert!(names.contains(&"base_status"));
        assert!(names.contains(&"fleet_status"));
        assert!(names.contains(&"have_item"));
        assert!(names.contains(&"inventory_summary"));
        assert!(names.contains(&"list_ships"));
        assert!(names.contains(&"search_planets"));
        assert!(names.contains(&"plan_route"));
        assert!(names.contains(&"where_am_i"));
        assert!(names.contains(&"whats_nearby"));
        assert!(names.contains(&"show_system"));
        assert!(names.contains(&"show_base"));
        assert!(names.contains(&"convert_coordinates"));
        assert!(names.contains(&"galaxy_stats"));
    }

    #[test]
    fn test_tools_unknown_returns_none() {
        let tools = NmsTools::new(test_model());
        assert!(tools.call("nonexistent", json!({})).is_none());
    }

    #[test]
    fn test_tools_tool_count() {
        let tools = NmsTools::new(test_model());
        assert_eq!(tools.tool_count(), 13);
    }

    fn fixture_model() -> Arc<RwLock<GalaxyModel>> {
        let json = include_str!("../../../../data/test/multi_system_save.json");
        let save = nms_save::parse_save(json.as_bytes()).unwrap();
        Arc::new(RwLock::new(GalaxyModel::from_save(&save)))
    }

    async fn call_json(tools: &NmsTools, name: &str, args: Value) -> Value {
        let result = tools.call(name, args).unwrap().await.unwrap();
        let text = result.content[0]
            .as_text()
            .map(|t| t.text.clone())
            .expect("text content");
        serde_json::from_str(&text).unwrap()
    }

    #[tokio::test]
    async fn test_have_item_tool_finds_gold() {
        let tools = NmsTools::new(fixture_model());
        let json = call_json(&tools, "have_item", json!({"pattern": "gold"})).await;
        let matches = json["matches"].as_array().unwrap();
        assert_eq!(matches[0]["name"], "Gold");
        assert_eq!(matches[0]["id"], "ASTEROID2");
        assert_eq!(matches[0]["total"], 11_297);
        let stacks = matches[0]["stacks"].as_array().unwrap();
        assert_eq!(stacks.len(), 3);
        assert_eq!(stacks[1]["container"], "Storage 1");
        assert_eq!(stacks[1]["reachable_from"][0], "Lush Haven");
        let none = call_json(
            &tools,
            "have_item",
            json!({"pattern": "gold", "kind": "product"}),
        )
        .await;
        assert!(none["matches"].as_array().unwrap().is_empty());
        assert!(
            tools.call("have_item", json!({})).unwrap().await.is_err(),
            "pattern is required"
        );
    }

    #[tokio::test]
    async fn test_inventory_summary_tool_overview_and_contents() {
        let tools = NmsTools::new(fixture_model());
        let json = call_json(&tools, "inventory_summary", json!({})).await;
        let containers = json["containers"].as_array().unwrap();
        let suit = containers
            .iter()
            .find(|c| c["container"] == "Exosuit")
            .unwrap();
        assert_eq!(suit["used"], 4);
        assert_eq!(suit["unlocked"], 93);
        assert_eq!(suit["free"], 89);
        let machine = containers
            .iter()
            .find(|c| c["container"] == "Machine 1")
            .unwrap();
        assert!(
            machine["unlocked"].is_null(),
            "no capacity is recorded for a machine buffer"
        );
        let json = call_json(
            &tools,
            "inventory_summary",
            json!({"container": "storage 1"}),
        )
        .await;
        let slots = json["containers"][0]["slots"].as_array().unwrap();
        assert_eq!(slots.len(), 4);
        assert_eq!(slots[0]["name"], "Gold");
        assert!(
            tools
                .call("inventory_summary", json!({"container": "locker"}))
                .unwrap()
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_list_ships_tool_lists_everything_owned() {
        let tools = NmsTools::new(fixture_model());
        let json = call_json(&tools, "list_ships", json!({})).await;
        let ships = json["ships"].as_array().unwrap();
        assert_eq!(ships.len(), 2);
        assert_eq!(ships[1]["name"], "Starbird");
        assert_eq!(ships[1]["primary"], true);
        assert_eq!(ships[1]["type"], "Exotic");
        assert_eq!(ships[1]["slots"]["general"], 31);
        let exocraft = json["exocraft"].as_array().unwrap();
        assert_eq!(exocraft[0]["parked_at_base"], "Lush Haven");
        assert!(exocraft[1]["parked_at_base"].is_null());
        let tools_list = json["multitools"].as_array().unwrap();
        assert_eq!(tools_list[0]["equipped"], true);
        assert_eq!(tools_list[0]["class"], "S");
    }

    #[tokio::test]
    async fn test_fleet_status_tool_returns_navigator_and_lists() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("fleet_status", json!({}))
            .unwrap()
            .await
            .unwrap();
        let text = result.content[0]
            .as_text()
            .map(|t| t.text.clone())
            .expect("text content");
        let json: Value = serde_json::from_str(&text).unwrap();
        assert!(json["expeditions"].as_array().unwrap().is_empty());
        assert!(json["frigates"].as_array().unwrap().is_empty());
        assert_eq!(json["navigator"]["offers_per_day"], 5);
        assert_eq!(json["navigator"]["offer_day"], 0);
        assert_eq!(json["command_rooms"], 0);
        assert!(
            json["summary"]
                .as_str()
                .unwrap()
                .starts_with("Navigator: no offers recorded yet")
        );
        assert!(json["alerts"].as_array().unwrap().is_empty());
        assert!(json["alerts"].is_array());
    }

    #[tokio::test]
    async fn test_base_status_tool_returns_bases_and_alerts() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("base_status", json!({})).unwrap().await.unwrap();
        let text = result.content[0]
            .as_text()
            .map(|t| t.text.clone())
            .expect("text content");
        let json: Value = serde_json::from_str(&text).unwrap();
        assert!(json["count"].as_u64().unwrap() >= 1);
        assert!(json["bases"][0]["extraction"]["networks"].is_array());
        assert!(json["alerts"].is_array());
        assert!(json["now_unix"].as_i64().unwrap() > 1_700_000_000);

        let missing = tools
            .call("base_status", json!({"name": "no such base"}))
            .unwrap()
            .await;
        assert!(missing.is_err());
    }

    #[test]
    fn test_tools_schemas_valid() {
        let tools = NmsTools::new(test_model());
        fabryk_mcp::assert_tools_valid(&tools);
    }

    #[tokio::test]
    async fn test_where_am_i_returns_position() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("where_am_i", json!({})).unwrap().await;
        assert!(result.is_ok());
        let ctr = result.unwrap();
        let text = extract_text(&ctr);
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert!(v.get("system").is_some());
        assert!(v.get("portal_glyphs_hex").is_some());
        assert!(v.get("galaxy").is_some());
    }

    #[tokio::test]
    async fn test_galaxy_stats_returns_counts() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("galaxy_stats", json!({})).unwrap().await;
        assert!(result.is_ok());
        let ctr = result.unwrap();
        let text = extract_text(&ctr);
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert!(v["systems"].as_u64().unwrap() >= 3);
        assert!(v["planets"].as_u64().unwrap() >= 3);
    }

    #[tokio::test]
    async fn test_search_planets_all() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("search_planets", json!({})).unwrap().await;
        assert!(result.is_ok());
        let ctr = result.unwrap();
        let text = extract_text(&ctr);
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert!(v["count"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_search_planets_invalid_biome() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("search_planets", json!({"biome": "NotABiome"}))
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_whats_nearby_default() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("whats_nearby", json!({})).unwrap().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_whats_nearby_with_count() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("whats_nearby", json!({"count": 1}))
            .unwrap()
            .await;
        assert!(result.is_ok());
        let ctr = result.unwrap();
        let text = extract_text(&ctr);
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert!(v["count"].as_u64().unwrap() <= 1);
    }

    #[tokio::test]
    async fn test_show_base_existing() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("show_base", json!({"name": "Alpha Base"}))
            .unwrap()
            .await;
        assert!(result.is_ok());
        let ctr = result.unwrap();
        let text = extract_text(&ctr);
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(v["name"], "Alpha Base");
    }

    #[tokio::test]
    async fn test_show_base_not_found() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("show_base", json!({"name": "No Such Base"}))
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_show_base_missing_name() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("show_base", json!({})).unwrap().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_show_system_missing_name() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("show_system", json!({})).unwrap().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_glyphs() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("convert_coordinates", json!({"glyphs": "01717D8A4EA2"}))
            .unwrap()
            .await;
        assert!(result.is_ok());
        let ctr = result.unwrap();
        let text = extract_text(&ctr);
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(v["portal_glyphs_hex"], "01717D8A4EA2");
        assert!(v.get("signal_booster").is_some());
    }

    #[tokio::test]
    async fn test_convert_galactic_address() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call(
                "convert_coordinates",
                json!({"galactic_address": "0x01717D8A4EA2"}),
            )
            .unwrap()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_no_input_errors() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("convert_coordinates", json!({})).unwrap().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_bad_glyphs_length() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("convert_coordinates", json!({"glyphs": "ABC"}))
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_bad_hex() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("convert_coordinates", json!({"glyphs": "ZZZZZZZZZZZZ"}))
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plan_route_requires_targets_or_biome() {
        let tools = NmsTools::new(test_model());
        let result = tools.call("plan_route", json!({})).unwrap().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plan_route_empty_targets_errors() {
        let tools = NmsTools::new(test_model());
        let result = tools
            .call("plan_route", json!({"targets": []}))
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_biome_arg_valid() {
        let args = json!({"biome": "Lush"});
        assert_eq!(parse_biome_arg(&args, "biome").unwrap(), Some(Biome::Lush));
    }

    #[tokio::test]
    async fn test_parse_biome_arg_invalid() {
        let args = json!({"biome": "NotReal"});
        assert!(parse_biome_arg(&args, "biome").is_err());
    }

    #[tokio::test]
    async fn test_parse_biome_arg_missing() {
        let args = json!({});
        assert_eq!(parse_biome_arg(&args, "biome").unwrap(), None);
    }

    #[tokio::test]
    async fn test_model_updates_after_delta() {
        let model = test_model();
        let count_before = model.read().await.system_count();

        let new_sys = nms_core::System::new(
            GalacticAddress::new(500, 10, -300, 0x999, 0, 0),
            Some("New System".into()),
            None,
            None,
            vec![],
        );
        let delta = nms_core::SaveDelta {
            new_systems: vec![new_sys],
            new_planets: vec![],
            player_moved: None,
            new_bases: vec![],
            modified_bases: vec![],
            fleet: None,
            holdings: None,
        };

        {
            let mut m = model.write().await;
            m.apply_delta(&delta);
        }

        assert_eq!(model.read().await.system_count(), count_before + 1);
    }

    #[tokio::test]
    async fn test_tools_see_updated_model() {
        let model = test_model();
        let tools = NmsTools::new(Arc::clone(&model));

        let result1 = tools
            .call("galaxy_stats", json!({}))
            .unwrap()
            .await
            .unwrap();
        let text1 = extract_text(&result1);
        let v1: Value = serde_json::from_str(&text1).expect("valid JSON");
        let initial_count = v1["systems"].as_u64().unwrap();

        // Apply delta
        {
            let mut m = model.write().await;
            let new_sys = nms_core::System::new(
                GalacticAddress::new(600, 20, -400, 0xAAA, 0, 0),
                Some("Delta System".into()),
                None,
                None,
                vec![],
            );
            m.apply_delta(&nms_core::SaveDelta {
                new_systems: vec![new_sys],
                new_planets: vec![],
                player_moved: None,
                new_bases: vec![],
                modified_bases: vec![],
                fleet: None,
                holdings: None,
            });
        }

        // Stats should reflect new system
        let result2 = tools
            .call("galaxy_stats", json!({}))
            .unwrap()
            .await
            .unwrap();
        let text2 = extract_text(&result2);
        let v2: Value = serde_json::from_str(&text2).expect("valid JSON");
        assert_eq!(v2["systems"].as_u64().unwrap(), initial_count + 1);
    }

    #[tokio::test]
    async fn test_concurrent_read_locks() {
        let model = test_model();
        let tools1 = NmsTools::new(Arc::clone(&model));
        let tools2 = NmsTools::new(Arc::clone(&model));

        // Two concurrent tool calls should not deadlock
        let (r1, r2) = tokio::join!(
            tools1.call("where_am_i", json!({})).unwrap(),
            tools2.call("galaxy_stats", json!({})).unwrap(),
        );
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    fn extract_text(ctr: &CallToolResult) -> String {
        ctr.content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
            .collect::<Vec<_>>()
            .join("")
    }
}
