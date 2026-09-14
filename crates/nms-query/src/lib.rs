//! Shared query engine for NMS Copilot.
//!
//! Pure, stateless query layer consumed by all three interfaces (CLI, REPL, MCP).
//! Takes an immutable reference to the `GalaxyModel` and returns typed results.

pub mod base;
pub mod display;
pub mod find;
pub mod layout;
pub mod route;
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
    format_distance, format_duration, format_find_results, format_route, format_show_result,
    format_snapshot, format_stats, hex_to_emoji,
};
pub use find::{FindQuery, FindResult, ReferencePoint};
pub use layout::terminal_width;
pub use route::{RouteFrom, RouteQuery, RouteResult, TargetSelection};
pub use show::{ShowQuery, ShowResult};
pub use stats::{StatsQuery, StatsResult};
pub use theme::{Theme, should_use_colors};
