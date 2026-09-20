//! `raw`: print any part of the decoded save as JSON.
//!
//! A probe for save-file research: walk to a path inside the decoded, deobfuscated save and print what is there, list the keys at that point, or search key names below it. Large structures are pruned by depth and array length so the output stays readable; pruned parts are replaced by short summary strings, so the printed JSON is for reading, not for feeding back into a parser. The caller supplies the decoded save (`nms_save::read_save_json`); this module only walks and prints it.

use serde_json::{Map, Value};

/// What to print from the decoded save.
#[derive(Debug, Clone)]
pub struct RawQuery {
    /// Dotted path from the save root, such as `BaseContext.PlayerStateData.FleetExpeditions[0].Events`. Empty means the root.
    pub path: Option<String>,
    /// Levels of nesting to print below the target; 0 means unlimited.
    pub depth: usize,
    /// Array items to print per array; 0 means unlimited.
    pub limit: usize,
    /// List the keys at the target instead of printing it.
    pub keys: bool,
    /// Search key names below the target for this text (case-insensitive).
    pub find: Option<String>,
}

/// The requested view of `root`, ready to print.
pub fn format_raw(root: &Value, query: &RawQuery) -> Result<String, String> {
    let json_path = query.path.as_deref().unwrap_or("").trim();
    let segments = parse_path(json_path)?;
    let target = navigate(root, &segments)?;
    if let Some(needle) = query.find.as_deref() {
        Ok(format_find(target, json_path, needle))
    } else if query.keys {
        Ok(format_keys(target, query.limit))
    } else {
        serde_json::to_string_pretty(&prune(target, query.depth, query.limit))
            .map_err(|e| e.to_string())
    }
}

/// One step of a JSON path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Key(String),
    Index(usize),
}

impl std::fmt::Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Segment::Key(k) => write!(f, "{k}"),
            Segment::Index(i) => write!(f, "[{i}]"),
        }
    }
}

/// Parse a dotted path. Array indexes are written `Name[3]` or as a bare number segment `Name.3`.
pub fn parse_path(path: &str) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    for piece in path.split('.').filter(|p| !p.is_empty()) {
        let (name, mut rest) = match piece.find('[') {
            Some(i) => (&piece[..i], &piece[i..]),
            None => (piece, ""),
        };
        if !name.is_empty() {
            if name.chars().all(|c| c.is_ascii_digit()) {
                segments.push(Segment::Index(
                    name.parse().map_err(|_| format!("bad index `{name}`"))?,
                ));
            } else {
                segments.push(Segment::Key(name.to_string()));
            }
        }
        while !rest.is_empty() {
            let close = rest
                .find(']')
                .ok_or_else(|| format!("unclosed `[` in `{piece}`"))?;
            let index = &rest[1..close];
            segments.push(Segment::Index(
                index
                    .parse()
                    .map_err(|_| format!("bad index `{index}` in `{piece}`"))?,
            ));
            rest = &rest[close + 1..];
            if !rest.is_empty() && !rest.starts_with('[') {
                return Err(format!("unexpected text after `]` in `{piece}`"));
            }
        }
    }
    Ok(segments)
}

fn join_path(segments: &[Segment]) -> String {
    let mut out = String::new();
    for seg in segments {
        match seg {
            Segment::Key(k) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(k);
            }
            Segment::Index(i) => out.push_str(&format!("[{i}]")),
        }
    }
    if out.is_empty() {
        "(root)".to_string()
    } else {
        out
    }
}

/// Walk from `root` along `segments`, describing what is at the failing point when a step cannot be taken.
pub fn navigate<'a>(root: &'a Value, segments: &[Segment]) -> Result<&'a Value, String> {
    let mut current = root;
    for (i, seg) in segments.iter().enumerate() {
        let here = join_path(&segments[..i]);
        current = match (seg, current) {
            (Segment::Key(k), Value::Object(map)) => map.get(k).ok_or_else(|| {
                let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                keys.sort_unstable();
                let shown: Vec<&str> = keys.iter().copied().take(30).collect();
                let more = if keys.len() > shown.len() {
                    format!(", … {} more", keys.len() - shown.len())
                } else {
                    String::new()
                };
                format!(
                    "no key `{k}` at {here}; keys here: {}{more}",
                    shown.join(", ")
                )
            })?,
            (Segment::Index(n), Value::Array(items)) => items.get(*n).ok_or_else(|| {
                format!(
                    "index {n} out of range at {here} (array of {})",
                    items.len()
                )
            })?,
            (Segment::Key(k), Value::Array(items)) => {
                return Err(format!(
                    "{here} is an array of {}; use an index, not `{k}`",
                    items.len()
                ));
            }
            (Segment::Index(n), Value::Object(map)) => {
                return Err(format!(
                    "{here} is an object with {} keys; use a key, not [{n}]",
                    map.len()
                ));
            }
            (_, other) => {
                return Err(format!(
                    "{here} is a {}; cannot go into it",
                    kind_name(other)
                ));
            }
        };
    }
    Ok(current)
}

fn kind_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// One-line description of a value.
fn summary(v: &Value) -> String {
    match v {
        Value::Object(m) => format!("object ({} keys)", m.len()),
        Value::Array(a) => format!("array ({} items)", a.len()),
        Value::String(s) => {
            let shown: String = s.chars().take(60).collect();
            if shown.len() < s.len() {
                format!("string \"{shown}…\"")
            } else {
                format!("string \"{shown}\"")
            }
        }
        Value::Number(n) => format!("number {n}"),
        Value::Bool(b) => format!("bool {b}"),
        Value::Null => "null".to_string(),
    }
}

/// Copy `value` with nesting cut off below `depth` levels (0 = unlimited) and arrays cut to `limit` items (0 = unlimited). Cut parts become summary strings.
pub fn prune(value: &Value, depth: usize, limit: usize) -> Value {
    fn go(value: &Value, remaining: Option<usize>, limit: usize) -> Value {
        match value {
            Value::Object(map) => {
                if remaining == Some(0) {
                    return Value::String(format!("{{… {} keys}}", map.len()));
                }
                let next = remaining.map(|r| r - 1);
                let mut out = Map::new();
                for (k, v) in map {
                    out.insert(k.clone(), go(v, next, limit));
                }
                Value::Object(out)
            }
            Value::Array(items) => {
                if remaining == Some(0) {
                    return Value::String(format!("[… {} items]", items.len()));
                }
                let next = remaining.map(|r| r - 1);
                let take = if limit == 0 {
                    items.len()
                } else {
                    limit.min(items.len())
                };
                let mut out: Vec<Value> = items
                    .iter()
                    .take(take)
                    .map(|v| go(v, next, limit))
                    .collect();
                if take < items.len() {
                    out.push(Value::String(format!("… {} more", items.len() - take)));
                }
                Value::Array(out)
            }
            other => other.clone(),
        }
    }
    go(value, if depth == 0 { None } else { Some(depth) }, limit)
}

/// List what sits directly under the target, one line each.
pub fn format_keys(target: &Value, limit: usize) -> String {
    let rows: Vec<(String, String)> = match target {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            keys.into_iter()
                .map(|k| (k.clone(), summary(&map[k])))
                .collect()
        }
        Value::Array(items) => {
            let take = if limit == 0 {
                items.len()
            } else {
                limit.min(items.len())
            };
            let mut rows: Vec<(String, String)> = items
                .iter()
                .take(take)
                .enumerate()
                .map(|(i, v)| (format!("[{i}]"), summary(v)))
                .collect();
            if take < items.len() {
                rows.push(("…".to_string(), format!("{} more", items.len() - take)));
            }
            rows
        }
        other => vec![("(value)".to_string(), summary(other))],
    };
    let width = rows
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    rows.iter()
        .map(|(k, s)| format!("{k:<width$}  {s}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Search key names below the target. Every array element is searched, and paths are reported with `[*]` in place of indexes so a key that repeats across elements appears once, with a count.
pub fn format_find(target: &Value, base: &str, needle: &str) -> String {
    let needle = needle.to_lowercase();
    let mut found: Vec<(String, String, usize)> = Vec::new();
    fn walk(v: &Value, path: &str, needle: &str, found: &mut Vec<(String, String, usize)>) {
        match v {
            Value::Object(map) => {
                for (k, child) in map {
                    let p = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    if k.to_lowercase().contains(needle) {
                        match found.iter_mut().find(|(fp, _, _)| *fp == p) {
                            Some(entry) => entry.2 += 1,
                            None => found.push((p.clone(), summary(child), 1)),
                        }
                    }
                    walk(child, &p, needle, found);
                }
            }
            Value::Array(items) => {
                let p = format!("{path}[*]");
                for item in items {
                    walk(item, &p, needle, found);
                }
            }
            _ => {}
        }
    }
    let base = base.trim_matches('.');
    walk(target, base, &needle, &mut found);
    if found.is_empty() {
        return format!(
            "no key containing `{needle}` under {}",
            if base.is_empty() { "(root)" } else { base }
        );
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    let width = found
        .iter()
        .map(|(p, _, _)| p.chars().count())
        .max()
        .unwrap_or(0);
    found
        .iter()
        .map(|(p, s, n)| {
            if *n > 1 {
                format!("{p:<width$}  {s}  ×{n}")
            } else {
                format!("{p:<width$}  {s}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "Version": 4720,
            "BaseContext": {
                "PlayerStateData": {
                    "Units": 500,
                    "PersistentPlayerBases": [
                        {"Name": "Lush Haven", "Objects": [{"ObjectID": "^SNOWPLANT"}, {"ObjectID": "^U_SILO_S"}]},
                        {"Name": "Frost Outpost", "Objects": []}
                    ]
                }
            }
        })
    }

    #[test]
    fn parse_path_accepts_brackets_and_bare_numbers() {
        assert_eq!(
            parse_path("A.B[2].C").unwrap(),
            vec![
                Segment::Key("A".into()),
                Segment::Key("B".into()),
                Segment::Index(2),
                Segment::Key("C".into())
            ]
        );
        assert_eq!(
            parse_path("A.0.C").unwrap(),
            vec![
                Segment::Key("A".into()),
                Segment::Index(0),
                Segment::Key("C".into())
            ]
        );
        assert_eq!(
            parse_path("A[1][2]").unwrap(),
            vec![
                Segment::Key("A".into()),
                Segment::Index(1),
                Segment::Index(2)
            ]
        );
        assert!(parse_path("").unwrap().is_empty());
        assert!(parse_path("A[x]").is_err());
        assert!(parse_path("A[1").is_err());
    }

    #[test]
    fn navigate_reaches_values_and_explains_misses() {
        let root = sample();
        let name = navigate(
            &root,
            &parse_path("BaseContext.PlayerStateData.PersistentPlayerBases[0].Name").unwrap(),
        )
        .unwrap();
        assert_eq!(name, "Lush Haven");
        assert_eq!(navigate(&root, &[]).unwrap(), &root);

        let err = navigate(&root, &parse_path("BaseContext.Nope").unwrap()).unwrap_err();
        assert!(err.contains("no key `Nope` at BaseContext"), "{err}");
        assert!(err.contains("PlayerStateData"), "{err}");

        let err = navigate(
            &root,
            &parse_path("BaseContext.PlayerStateData.PersistentPlayerBases[5]").unwrap(),
        )
        .unwrap_err();
        assert!(err.contains("out of range"), "{err}");

        let err = navigate(
            &root,
            &parse_path("BaseContext.PlayerStateData.PersistentPlayerBases.Name").unwrap(),
        )
        .unwrap_err();
        assert!(err.contains("use an index"), "{err}");

        let err = navigate(&root, &parse_path("Version.x").unwrap()).unwrap_err();
        assert!(err.contains("is a number"), "{err}");
    }

    #[test]
    fn prune_cuts_depth_and_arrays() {
        let root = sample();
        let shallow = prune(&root, 1, 0);
        assert_eq!(shallow["Version"], 4720);
        assert_eq!(shallow["BaseContext"], "{… 1 keys}");

        let bases = &root["BaseContext"]["PlayerStateData"]["PersistentPlayerBases"];
        let cut = prune(bases, 0, 1);
        let items = cut.as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["Name"], "Lush Haven");
        assert_eq!(items[1], "… 1 more");

        assert_eq!(prune(&root, 0, 0), root);
    }

    #[test]
    fn keys_listing_describes_children() {
        let root = sample();
        let out = format_keys(&root["BaseContext"]["PlayerStateData"], 0);
        assert!(
            out.contains("PersistentPlayerBases  array (2 items)"),
            "{out}"
        );
        assert!(out.contains("Units                  number 500"), "{out}");

        let out = format_keys(
            &root["BaseContext"]["PlayerStateData"]["PersistentPlayerBases"],
            1,
        );
        assert!(out.contains("[0]  object (2 keys)"), "{out}");
        assert!(out.contains("1 more"), "{out}");

        assert_eq!(
            format_keys(&root["Version"], 0),
            "(value)  number 500".replace("500", "4720")
        );
    }

    #[test]
    fn find_reports_repeated_keys_once_with_counts() {
        let root = sample();
        let out = format_find(&root, "", "objectid");
        assert_eq!(
            out.trim(),
            "BaseContext.PlayerStateData.PersistentPlayerBases[*].Objects[*].ObjectID  string \"^SNOWPLANT\"  ×2"
        );

        let out = format_find(&root["BaseContext"], "BaseContext", "name");
        assert!(
            out.starts_with("BaseContext.PlayerStateData.PersistentPlayerBases[*].Name"),
            "{out}"
        );

        let out = format_find(&root, "", "zzz");
        assert!(out.contains("no key containing"), "{out}");
    }
}
