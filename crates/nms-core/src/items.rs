//! The item ID to display name table.
//!
//! The save keys every slot by an internal ID such as `^ASTEROID2` and carries no display names. `data/items.json` maps the bare ID to the English name and group as the AssistantNMS API serves them from the game's data; `scripts/gen-items.py` regenerates it. An ID the table does not know is shown as itself so the gap is visible rather than hidden.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const ITEMS_JSON: &str = include_str!("../data/items.json");

/// What the table knows about one item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ItemInfo {
    pub name: String,
    /// The game's short category line ("Valuable Asteroid Mineral").
    #[serde(default)]
    pub group: String,
}

fn table() -> &'static HashMap<String, ItemInfo> {
    static TABLE: OnceLock<HashMap<String, ItemInfo>> = OnceLock::new();
    TABLE.get_or_init(|| serde_json::from_str(ITEMS_JSON).expect("bundled items.json is valid"))
}

/// The table entry for a bare ID (no caret).
pub fn lookup(bare_id: &str) -> Option<&'static ItemInfo> {
    table().get(bare_id)
}

/// The display name for a bare ID, or the ID itself when unknown.
pub fn display_name(bare_id: &str) -> String {
    lookup(bare_id)
        .map(|i| i.name.clone())
        .unwrap_or_else(|| bare_id.to_string())
}

/// Every (bare ID, info) pair whose name or ID contains `pattern`, case-insensitively.
pub fn search(pattern: &str) -> Vec<(&'static str, &'static ItemInfo)> {
    let pattern = pattern.trim().trim_start_matches('^').to_lowercase();
    if pattern.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<_> = table()
        .iter()
        .filter(|(id, info)| {
            id.to_lowercase().contains(&pattern) || info.name.to_lowercase().contains(&pattern)
        })
        .map(|(id, info)| (id.as_str(), info))
        .collect();
    out.sort_by(|a, b| a.1.name.cmp(&b.1.name).then(a.0.cmp(b.0)));
    out
}

/// How many IDs the table knows.
pub fn len() -> usize {
    table().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ids_resolve() {
        assert_eq!(display_name("ASTEROID2"), "Gold");
        assert_eq!(display_name("ASTEROID1"), "Silver");
        assert_eq!(display_name("STELLAR2"), "Chromatic Metal");
        assert_eq!(display_name("U_GENERATOR_S"), "Electromagnetic Generator");
    }

    #[test]
    fn unknown_id_is_itself() {
        assert_eq!(display_name("NOT_AN_ITEM_XYZ"), "NOT_AN_ITEM_XYZ");
        assert!(lookup("NOT_AN_ITEM_XYZ").is_none());
    }

    #[test]
    fn search_finds_by_name_and_id() {
        let gold = search("gold");
        assert!(gold.iter().any(|(id, _)| *id == "ASTEROID2"));
        let by_id = search("asteroid");
        assert!(by_id.iter().any(|(id, _)| *id == "ASTEROID1"));
        assert!(search("").is_empty());
    }

    #[test]
    fn table_is_large() {
        assert!(len() > 3000);
    }
}
