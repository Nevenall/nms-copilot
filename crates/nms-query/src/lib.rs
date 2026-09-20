//! Shared query engine for NMS Copilot.
//!
//! Pure, stateless query layer consumed by all three interfaces (CLI, REPL, MCP).
//! Takes an immutable reference to the `GalaxyModel` and returns typed results.

pub mod base;
pub mod display;
pub mod export;
pub mod find;
pub mod fleet;
pub mod inventory;
pub mod layout;
pub mod raw;
pub mod route;
pub mod saves;
pub mod show;
pub mod stats;
pub mod table;
pub mod theme;

pub use base::{
    Alert, AlertKind, AlertSummary, BaseQuery, BaseStatus, CropBatch, CropRow, GeneratorRow,
    PowerSummary, execute_base,
};
pub use display::{
    format_alert_indicator, format_alert_line, format_base_detail, format_base_overview,
    format_clock, format_distance, format_duration, format_expedition_detail, format_expeditions,
    format_find_results, format_fleet, format_fleet_overview, format_frigates,
    format_navigator_line, format_route, format_show_system, format_snapshot, format_stats,
    hex_to_emoji,
};
pub use display::{
    format_exocraft, format_have, format_inventory, format_items, format_multitools, format_ships,
};
pub use export::{ExportFormat, ExportRecord, render_export};
pub use find::{FindQuery, FindResult, FindSort, ReferencePoint};
pub use fleet::{
    ExpeditionRow, FleetStatus, FleetTarget, FrigateRow, OfferStatus, execute_fleet, fleet_alerts,
};
pub use inventory::{
    ExocraftRow, HaveLocation, HaveQuery, HaveResult, InventoryQuery, InventoryResult, ItemTotal,
    ListItemsQuery, execute_exocraft, execute_have, execute_inventory, execute_list_items,
    holdings_summary,
};
pub use layout::terminal_width;
pub use raw::{RawQuery, format_raw};
pub use route::{RouteFrom, RouteQuery, RouteResult, TargetSelection};
pub use saves::format_save_slots;
pub use show::{ShowSystemResult, show_system};
pub use stats::{StatsQuery, StatsResult};
pub use theme::{Theme, should_use_colors};
