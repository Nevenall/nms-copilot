//! Tab completion for the NMS Copilot REPL.
//!
//! Provides context-aware completions:
//! - Command names (find, show, stats, convert, info, help, exit, quit)
//! - Subcommand names (show system, list saves, backup prune)
//! - Flag names (--biome, --nearest, etc.)
//! - Biome names from the Biome enum
//! - Base names from the loaded model
//! - System names from the loaded model

use reedline::{Completer, Span, Suggestion};

/// Completions that depend on the loaded galaxy model.
#[derive(Clone)]
pub struct ModelCompletions {
    /// Known base names (original casing).
    pub base_names: Vec<String>,
    /// Known system names (original casing).
    pub system_names: Vec<String>,
    /// Display names of every item held, for `have`.
    pub item_names: Vec<String>,
    /// Container labels, for `inventory`.
    pub container_names: Vec<String>,
}

/// REPL tab completer with static command knowledge and dynamic model data.
pub struct CopilotCompleter {
    model_data: ModelCompletions,
}

impl CopilotCompleter {
    pub fn new(model_data: ModelCompletions) -> Self {
        Self { model_data }
    }
}

const COMMANDS: &[&str] = &[
    "backup",
    "base",
    "convert",
    "dashboard",
    "exit",
    "export",
    "find",
    "fleet",
    "have",
    "help",
    "info",
    "inventory",
    "list",
    "map",
    "quit",
    "raw",
    "reset",
    "route",
    "set",
    "show",
    "stats",
];

const SHOW_SUBCOMMANDS: &[&str] = &["system"];

const LIST_SUBCOMMANDS: &[&str] = &[
    "bases",
    "biomes",
    "exocraft",
    "expeditions",
    "frigates",
    "galaxies",
    "glyphs",
    "items",
    "multitools",
    "saves",
    "ships",
    "systems",
];

const HAVE_FLAGS: &[&str] = &["--type"];

const INVENTORY_FLAGS: &[&str] = &["--free"];

const ITEM_TYPES: &[&str] = &["substance", "product", "technology"];

const FIND_FLAGS: &[&str] = &[
    "--biome",
    "--infested",
    "--within",
    "--nearest",
    "--named",
    "--discoverer",
    "--from",
    "--sort",
];

const SORT_WORDS: &[&str] = &["distance", "fauna", "flora", "minerals"];

const STATS_FLAGS: &[&str] = &["--biomes", "--discoveries"];

const BASE_FLAGS: &[&str] = &["--width"];

const BACKUP_SUBCOMMANDS: &[&str] = &["list", "off", "on", "prune"];

const EXPORT_FLAGS: &[&str] = &[
    "--biome",
    "--infested",
    "--within",
    "--nearest",
    "--named",
    "--discoverer",
    "--from",
    "--sort",
    "--format",
    "--to",
];

const RAW_FLAGS: &[&str] = &["--depth", "--limit", "--keys", "--find"];

const CONVERT_FLAGS: &[&str] = &[
    "--glyphs", "--coords", "--ga", "--voxel", "--ssi", "--planet", "--galaxy",
];

const ROUTE_FLAGS: &[&str] = &[
    "--algo",
    "--biome",
    "--from",
    "--max-targets",
    "--round-trip",
    "--target",
    "--warp-range",
    "--within",
];

const SET_SUBCOMMANDS: &[&str] = &["position", "biome", "warp-range"];

const RESET_TARGETS: &[&str] = &["position", "biome", "warp-range", "all"];

const BIOME_NAMES: &[&str] = &[
    "Lush",
    "Toxic",
    "Scorched",
    "Radioactive",
    "Frozen",
    "Barren",
    "Dead",
    "Weird",
    "Red",
    "Green",
    "Blue",
    "Swamp",
    "Lava",
    "Waterworld",
];

impl Completer for CopilotCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let line_to_pos = &line[..pos];
        let words: Vec<&str> = line_to_pos.split_whitespace().collect();
        // Lowercase words for case-insensitive context matching
        let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
        let lower_refs: Vec<&str> = lower.iter().map(|s| s.as_str()).collect();
        let trailing_space = line_to_pos.ends_with(' ');

        let (partial, candidates) = match lower_refs.as_slice() {
            [] => ("", COMMANDS.to_vec()),
            [_] if !trailing_space => (words[0], COMMANDS.to_vec()),

            ["list"] if trailing_space => ("", LIST_SUBCOMMANDS.to_vec()),
            ["list", _] if !trailing_space => (words[1], LIST_SUBCOMMANDS.to_vec()),

            ["have", .., "--type"] if trailing_space => {
                return self.filter_suggestions("", ITEM_TYPES, pos);
            }
            ["have", .., "--type", _] if !trailing_space => {
                return self.filter_suggestions(words[words.len() - 1], ITEM_TYPES, pos);
            }
            ["have", ..] if !trailing_space && words.last().is_some_and(|w| w.starts_with('-')) => {
                return self.filter_suggestions(words[words.len() - 1], HAVE_FLAGS, pos);
            }
            ["have"] if trailing_space => {
                return self.complete_names("", &self.model_data.item_names, pos);
            }
            ["have", _] if !trailing_space => {
                return self.complete_names(words[1], &self.model_data.item_names, pos);
            }
            ["have", _, ..] if trailing_space => {
                return self.filter_suggestions("", HAVE_FLAGS, pos);
            }

            ["inventory", ..]
                if !trailing_space && words.last().is_some_and(|w| w.starts_with('-')) =>
            {
                return self.filter_suggestions(words[words.len() - 1], INVENTORY_FLAGS, pos);
            }
            ["inventory"] if trailing_space => {
                return self.complete_names("", &self.model_data.container_names, pos);
            }
            ["inventory", _] if !trailing_space => {
                return self.complete_names(words[1], &self.model_data.container_names, pos);
            }
            ["inventory", _, ..] if trailing_space => {
                return self.filter_suggestions("", INVENTORY_FLAGS, pos);
            }

            ["backup"] if trailing_space => ("", BACKUP_SUBCOMMANDS.to_vec()),
            ["backup", _] if !trailing_space => (words[1], BACKUP_SUBCOMMANDS.to_vec()),

            ["show"] if trailing_space => ("", SHOW_SUBCOMMANDS.to_vec()),
            ["show", _] if !trailing_space => (words[1], SHOW_SUBCOMMANDS.to_vec()),

            ["base", ..] if !trailing_space && words.last().is_some_and(|w| w.starts_with('-')) => {
                return self.filter_suggestions(words[words.len() - 1], BASE_FLAGS, pos);
            }
            ["base"] if trailing_space => {
                return self.complete_names("", &self.model_data.base_names, pos);
            }
            ["base", _] if !trailing_space => {
                return self.complete_names(words[1], &self.model_data.base_names, pos);
            }
            ["base", _, ..] if trailing_space => {
                return self.filter_suggestions("", BASE_FLAGS, pos);
            }

            ["show", "system"] if trailing_space => {
                return self.complete_names("", &self.model_data.system_names, pos);
            }
            ["show", "system", _] if !trailing_space => {
                return self.complete_names(words[2], &self.model_data.system_names, pos);
            }

            ["set"] if trailing_space => ("", SET_SUBCOMMANDS.to_vec()),
            ["set", _] if !trailing_space => (words[1], SET_SUBCOMMANDS.to_vec()),

            ["set", "biome"] if trailing_space => {
                return self.filter_suggestions("", BIOME_NAMES, pos);
            }
            ["set", "biome", _] if !trailing_space => {
                return self.filter_suggestions(words[2], BIOME_NAMES, pos);
            }

            ["set", "position"] if trailing_space => {
                return self.complete_names("", &self.model_data.base_names, pos);
            }
            ["set", "position", _] if !trailing_space => {
                return self.complete_names(words[2], &self.model_data.base_names, pos);
            }

            ["reset"] if trailing_space => ("", RESET_TARGETS.to_vec()),
            ["reset", _] if !trailing_space => (words[1], RESET_TARGETS.to_vec()),

            [cmd, ..] if *cmd == "find" => {
                return self.complete_find_context(line_to_pos, &words, pos, FIND_FLAGS);
            }

            [cmd, ..] if *cmd == "export" => {
                return self.complete_find_context(line_to_pos, &words, pos, EXPORT_FLAGS);
            }

            [cmd, ..] if *cmd == "raw" => {
                let partial = if trailing_space {
                    ""
                } else {
                    words.last().copied().unwrap_or("")
                };
                if !partial.starts_with('-') && !(trailing_space && words.len() >= 2) {
                    return vec![];
                }
                (partial, RAW_FLAGS.to_vec())
            }

            [cmd, ..] if *cmd == "route" => {
                return self.complete_route_context(line_to_pos, &words, pos);
            }

            [cmd, ..] if *cmd == "stats" => {
                let partial = if trailing_space {
                    ""
                } else {
                    words.last().copied().unwrap_or("")
                };
                (partial, STATS_FLAGS.to_vec())
            }

            [cmd, ..] if *cmd == "convert" => {
                let partial = if trailing_space {
                    ""
                } else {
                    words.last().copied().unwrap_or("")
                };
                (partial, CONVERT_FLAGS.to_vec())
            }

            _ => return vec![],
        };

        self.filter_suggestions(partial, &candidates, pos)
    }
}

impl CopilotCompleter {
    /// `find` and `export` share their filters; `flags` is the command's own flag list.
    fn complete_find_context(
        &self,
        line_to_pos: &str,
        words: &[&str],
        pos: usize,
        flags: &[&str],
    ) -> Vec<Suggestion> {
        let last = if line_to_pos.ends_with(' ') {
            ""
        } else {
            words.last().copied().unwrap_or("")
        };

        let prev = if line_to_pos.ends_with(' ') {
            words.last().copied()
        } else if words.len() >= 2 {
            Some(words[words.len() - 2])
        } else {
            None
        };

        if prev == Some("--biome") {
            return self.filter_suggestions(last, BIOME_NAMES, pos);
        }

        if prev == Some("--from") {
            return self.complete_names(last, &self.model_data.base_names, pos);
        }

        if prev == Some("--format") {
            return self.filter_suggestions(last, &["csv", "json"], pos);
        }

        if prev == Some("--sort") {
            return self.filter_suggestions(last, SORT_WORDS, pos);
        }

        self.filter_suggestions(last, flags, pos)
    }

    fn complete_route_context(
        &self,
        line_to_pos: &str,
        words: &[&str],
        pos: usize,
    ) -> Vec<Suggestion> {
        let last = if line_to_pos.ends_with(' ') {
            ""
        } else {
            words.last().copied().unwrap_or("")
        };

        let prev = if line_to_pos.ends_with(' ') {
            words.last().copied()
        } else if words.len() >= 2 {
            Some(words[words.len() - 2])
        } else {
            None
        };

        if prev == Some("--biome") {
            return self.filter_suggestions(last, BIOME_NAMES, pos);
        }

        if prev == Some("--from") {
            return self.complete_names(last, &self.model_data.base_names, pos);
        }

        self.filter_suggestions(last, ROUTE_FLAGS, pos)
    }

    fn complete_names(&self, partial: &str, names: &[String], pos: usize) -> Vec<Suggestion> {
        let lower = partial.to_lowercase();
        names
            .iter()
            .filter(|n| n.to_lowercase().starts_with(&lower))
            .take(20)
            .map(|n| {
                let value = if n.contains(' ') {
                    format!("\"{n}\"")
                } else {
                    n.clone()
                };
                Suggestion {
                    value,
                    display_override: None,
                    description: None,
                    style: None,
                    extra: None,
                    span: Span::new(pos - partial.len(), pos),
                    append_whitespace: true,
                    match_indices: None,
                }
            })
            .collect()
    }

    fn filter_suggestions(
        &self,
        partial: &str,
        candidates: &[&str],
        pos: usize,
    ) -> Vec<Suggestion> {
        let lower = partial.to_lowercase();
        candidates
            .iter()
            .filter(|c| c.to_lowercase().starts_with(&lower))
            .map(|c| Suggestion {
                value: c.to_string(),
                display_override: None,
                description: None,
                style: None,
                extra: None,
                span: Span::new(pos - partial.len(), pos),
                append_whitespace: true,
                match_indices: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_completer() -> CopilotCompleter {
        CopilotCompleter::new(ModelCompletions {
            base_names: vec![
                "Acadia National Park".into(),
                "Alpha Base".into(),
                "Beta Station".into(),
            ],
            system_names: vec!["Gugestor Colony".into(), "Esurad".into()],
            item_names: vec!["Gold".into(), "Gold Ore".into(), "Silver".into()],
            container_names: vec!["Exosuit".into(), "Storage 1".into(), "Storage 10".into()],
        })
    }

    #[test]
    fn test_complete_empty_line_shows_commands() {
        let mut c = test_completer();
        let results = c.complete("", 0);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"find"));
        assert!(values.contains(&"show"));
        assert!(values.contains(&"exit"));
    }

    #[test]
    fn test_complete_partial_command() {
        let mut c = test_completer();
        let results = c.complete("fi", 2);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "find");
    }

    #[test]
    fn test_complete_show_subcommands() {
        let mut c = test_completer();
        let results = c.complete("show ", 5);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["system"], "a base in full is `base <name>`");
    }

    #[test]
    fn test_complete_base_name_with_spaces_is_quoted() {
        let mut c = test_completer();
        let results = c.complete("base Aca", 8);
        assert!(!results.is_empty());
        assert!(results[0].value.starts_with('"'));
    }

    #[test]
    fn test_complete_find_flags() {
        let mut c = test_completer();
        let results = c.complete("find --b", 8);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"--biome"));
    }

    #[test]
    fn test_complete_biome_after_flag() {
        let mut c = test_completer();
        let results = c.complete("find --biome L", 14);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"Lush"));
        assert!(values.contains(&"Lava"));
    }

    #[test]
    fn test_complete_from_base_names() {
        let mut c = test_completer();
        let results = c.complete("find --from B", 13);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.iter().any(|v| v.contains("Beta")));
    }

    #[test]
    fn test_complete_show_system_names() {
        let mut c = test_completer();
        let results = c.complete("show system G", 13);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.iter().any(|v| v.contains("Gugestor")));
    }

    #[test]
    fn test_complete_stats_flags() {
        let mut c = test_completer();
        let results = c.complete("stats --b", 9);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"--biomes"));
    }

    #[test]
    fn test_complete_convert_flags() {
        let mut c = test_completer();
        let results = c.complete("convert --g", 11);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"--glyphs"));
        assert!(values.contains(&"--ga"));
        assert!(values.contains(&"--galaxy"));
    }

    #[test]
    fn test_complete_case_insensitive_command() {
        let mut c = test_completer();
        // Typing "FI" should still match "find"
        let results = c.complete("FI", 2);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "find");
    }

    #[test]
    fn test_complete_case_insensitive_show_subcommand() {
        let mut c = test_completer();
        let results = c.complete("SHOW ", 5);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["system"]);
    }

    #[test]
    fn test_complete_case_insensitive_base() {
        let mut c = test_completer();
        let results = c.complete("Base a", 6);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.iter().any(|v| v.contains("Acadia")));
    }

    #[test]
    fn test_complete_route_flags() {
        let mut c = test_completer();
        let results = c.complete("route --b", 9);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"--biome"));
    }

    #[test]
    fn test_complete_route_biome_after_flag() {
        let mut c = test_completer();
        let results = c.complete("route --biome L", 15);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"Lush"));
        assert!(values.contains(&"Lava"));
    }

    #[test]
    fn test_complete_route_from_base_names() {
        let mut c = test_completer();
        let results = c.complete("route --from A", 14);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.iter().any(|v| v.contains("Alpha")));
    }

    #[test]
    fn test_complete_route_all_flags() {
        let mut c = test_completer();
        let results = c.complete("route ", 6);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"--algo"));
        assert!(values.contains(&"--biome"));
        assert!(values.contains(&"--from"));
        assert!(values.contains(&"--target"));
        assert!(values.contains(&"--warp-range"));
        assert!(values.contains(&"--within"));
        assert!(values.contains(&"--round-trip"));
        assert!(values.contains(&"--max-targets"));
    }

    #[test]
    fn test_complete_route_in_command_list() {
        let mut c = test_completer();
        let results = c.complete("r", 1);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"route"));
        assert!(values.contains(&"reset"));
    }

    #[test]
    fn test_complete_list_subcommands() {
        let mut c = test_completer();
        let results = c.complete("list ", 5);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"galaxies"));
        assert!(values.contains(&"biomes"));
        assert!(values.contains(&"glyphs"));
        assert!(values.contains(&"bases"));
        assert!(values.contains(&"systems"));
    }

    #[test]
    fn test_complete_list_partial() {
        let mut c = test_completer();
        let results = c.complete("list g", 6);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"galaxies"));
        assert!(values.contains(&"glyphs"));
        assert!(!values.contains(&"biomes"));
    }

    #[test]
    fn test_complete_case_insensitive_find_flags() {
        let mut c = test_completer();
        let results = c.complete("FIND --b", 8);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"--biome"));
    }

    #[test]
    fn test_complete_have_offers_item_names_and_types() {
        let mut c = test_completer();
        let results = c.complete("have go", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, vec!["Gold", "\"Gold Ore\""]);
        let results = c.complete("have gold --type ", 17);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"substance"));
        assert!(values.contains(&"technology"));
        let results = c.complete("have gold --", 12);
        assert_eq!(results[0].value, "--type");
    }

    #[test]
    fn test_complete_inventory_offers_container_labels() {
        let mut c = test_completer();
        let results = c.complete("inventory sto", 13);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, vec!["\"Storage 1\"", "\"Storage 10\""]);
        let results = c.complete("inventory --", 12);
        assert_eq!(results[0].value, "--free");
        let results = c.complete("list it", 7);
        assert_eq!(results[0].value, "items");
    }

    #[test]
    fn test_complete_list_offers_fleet_and_saves() {
        let mut c = test_completer();
        let results = c.complete("list fr", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["frigates"]);
        let results = c.complete("list ex", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["exocraft", "expeditions"]);
        let results = c.complete("list sa", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["saves"]);
        assert!(
            c.complete("fleet ", 6).is_empty(),
            "fleet takes an expedition number, which nothing completes"
        );
    }

    #[test]
    fn test_complete_export_raw_backup_and_dashboard() {
        let mut c = test_completer();
        let results = c.complete("export --f", 10);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["--from", "--format"]);
        let results = c.complete("export --biome L", 16);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"Lush"));
        let results = c.complete("raw --k", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["--keys"]);
        assert!(
            c.complete("raw Base", 8).is_empty(),
            "save paths are not completed"
        );
        let results = c.complete("backup pr", 9);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["prune"]);
        let results = c.complete("dash", 4);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["dashboard"]);
        assert!(
            c.complete("stat", 4).iter().all(|s| s.value == "stats"),
            "status is gone"
        );
    }

    #[test]
    fn test_complete_base_command_offers_base_names() {
        let mut c = test_completer();
        let results = c.complete("base A", 6);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.iter().any(|v| v.contains("Acadia")));
        assert!(values.iter().any(|v| v.contains("Alpha")));
        let results = c.complete("base ", 5);
        assert_eq!(results.len(), 3);
        let results = c.complete("base farm ", 10);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["--width"]);
        let results = c.complete("base --w", 8);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, ["--width"]);
    }
}
