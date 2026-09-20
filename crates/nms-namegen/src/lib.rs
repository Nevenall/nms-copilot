//! Region and system names and the galaxy map's hover properties, generated from the
//! address by running the community `nms_namegen` tool.
//!
//! The tool is a Python script (Python 3.13 or later with numpy) kept as a git checkout
//! at a pinned commit; its batch mode reads addresses on stdin and writes one JSON object
//! per line. Its answer is a pure function of the address and galaxy, so every answer is
//! kept in a JSON cache and each address costs one run in the tool's lifetime. Anything
//! that goes wrong (no tool, no Python, a failed run) leaves the affected addresses out
//! of the answer and says why at debug level; callers show nothing rather than an error.
//!
//! Plan: `docs/plans/project03-save-tooling/0049-generated-galaxy.md`.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use nms_core::address::GalacticAddress;
use nms_core::generated::{AddressGenerator, Generated, SystemAttributes};
use serde::Deserialize;

/// The commit of `hadsh/nms_namegen` this crate was validated against.
pub const PINNED_COMMIT: &str = "52ad48affaa4089c8f487a470a888dc9b7a650aa";

/// Where the tool is expected when nothing says otherwise: `~/.nms-copilot/nms_namegen/namegen.py`.
pub fn default_script_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".nms-copilot")
            .join("nms_namegen")
            .join("namegen.py")
    })
}

/// Where answers are kept: `~/.nms-copilot/namegen-cache.json`.
pub fn default_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".nms-copilot").join("namegen-cache.json"))
}

/// How to find and run the tool. Every field is optional; see [`NameGen::discover`].
#[derive(Debug, Clone, Default)]
pub struct NameGenConfig {
    /// The `namegen.py` script.
    pub script: Option<PathBuf>,
    /// The Python interpreter to run it with.
    pub python: Option<String>,
    /// The answer cache.
    pub cache: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum NameGenError {
    #[error("nms_namegen script not found at {0}")]
    ScriptMissing(PathBuf),
    #[error("no Python interpreter found (tried {0})")]
    NoPython(String),
    #[error("nms_namegen failed: {0}")]
    Run(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// The subprocess client with its answer cache.
#[derive(Debug)]
pub struct NameGen {
    script: PathBuf,
    python: String,
    cache_path: Option<PathBuf>,
    cache: Mutex<HashMap<String, Generated>>,
}

impl NameGen {
    /// Find the tool: the configured script, else `NMS_NAMEGEN`, else the default path;
    /// the configured interpreter, else `python`, else `python3`. `None` when the script
    /// or an interpreter is missing, with the reason logged at debug level.
    pub fn discover(config: &NameGenConfig) -> Option<Self> {
        match Self::try_discover(config) {
            Ok(namegen) => Some(namegen),
            Err(e) => {
                log::debug!("generated names unavailable: {e}");
                None
            }
        }
    }

    /// [`discover`](Self::discover) with the reason it failed.
    pub fn try_discover(config: &NameGenConfig) -> Result<Self, NameGenError> {
        let script = config
            .script
            .clone()
            .or_else(|| std::env::var_os("NMS_NAMEGEN").map(PathBuf::from))
            .or_else(default_script_path)
            .ok_or_else(|| {
                NameGenError::ScriptMissing(PathBuf::from("~/.nms-copilot/nms_namegen/namegen.py"))
            })?;
        if !script.is_file() {
            return Err(NameGenError::ScriptMissing(script));
        }
        let candidates: Vec<String> = match &config.python {
            Some(p) => vec![p.clone()],
            None => vec!["python".to_string(), "python3".to_string()],
        };
        let python = candidates
            .iter()
            .find(|p| python_works(p))
            .cloned()
            .ok_or_else(|| NameGenError::NoPython(candidates.join(", ")))?;
        let cache_path = config.cache.clone().or_else(default_cache_path);
        let cache = cache_path.as_deref().map(load_cache).unwrap_or_default();
        Ok(Self {
            script,
            python,
            cache_path,
            cache: Mutex::new(cache),
        })
    }

    /// A client over a script and interpreter with no disk cache, for tests.
    pub fn uncached(script: PathBuf, python: String) -> Self {
        Self {
            script,
            python,
            cache_path: None,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn script(&self) -> &Path {
        &self.script
    }

    /// Answers held in memory, cached or generated this session.
    pub fn cached_count(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }

    /// Run one batch kind over addresses, each line `PSSSYYZZZXXX galaxy`, and return the
    /// JSON object for each address that came back without an `error`.
    fn run_batch(
        &self,
        kind: &str,
        addresses: &[GalacticAddress],
    ) -> Result<HashMap<GalacticAddress, serde_json::Value>, NameGenError> {
        let mut child = Command::new(&self.python)
            .arg(&self.script)
            .arg("batch")
            .arg("--batch-kind")
            .arg(kind)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let input: String = addresses
            .iter()
            .map(|a| format!("{:012X} {}\n", system_packed(a), a.reality_index))
            .collect();
        // The tool answers as it reads, so feed it from another thread or a long list
        // fills the output pipe before the input is written.
        let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
        let mut stderr = child.stderr.take().expect("stderr was piped");
        let errors = std::thread::spawn(move || {
            let mut err = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr, &mut err);
            err
        });
        let stdout = child.stdout.take().expect("stdout was piped");
        let mut answers = HashMap::new();
        for line in BufReader::new(stdout).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(&line)?;
            if let Some(err) = value.get("error") {
                log::debug!("nms_namegen {kind}: {err}");
                continue;
            }
            let Some(addr) = address_of(&value) else {
                continue;
            };
            answers.insert(addr, value);
        }
        let _ = writer.join();
        let status = child.wait()?;
        let err = errors.join().unwrap_or_default();
        if !status.success() {
            let last = err.lines().last().unwrap_or("").to_string();
            return Err(NameGenError::Run(format!(
                "{kind} exited with {status}: {last}"
            )));
        }
        Ok(answers)
    }

    /// Generate for addresses not yet in the cache and add them to it.
    fn generate_missing(
        &self,
        missing: &[GalacticAddress],
    ) -> Result<HashMap<GalacticAddress, Generated>, NameGenError> {
        let regions = self.run_batch("region", missing)?;
        let names = self.run_batch("system", missing)?;
        let raw = self.run_batch("system-attributes", missing)?;
        let rendered = self.run_batch("attributes", missing)?;
        let mut out = HashMap::new();
        for addr in missing {
            let (Some(region), Some(name)) = (regions.get(addr), names.get(addr)) else {
                continue;
            };
            let Some(region) = region.get("region_name").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(name) = name.get("system_name").and_then(|v| v.as_str()) else {
                continue;
            };
            let attributes = raw
                .get(addr)
                .and_then(|r| decode_attributes(r, rendered.get(addr)));
            out.insert(
                *addr,
                Generated::new(region.to_string(), name.to_string(), attributes),
            );
        }
        Ok(out)
    }

    fn save_cache(&self) {
        let Some(path) = &self.cache_path else {
            return;
        };
        let Ok(cache) = self.cache.lock() else {
            return;
        };
        if let Err(e) = write_cache(path, &cache) {
            log::debug!("could not write {}: {e}", path.display());
        }
    }
}

impl AddressGenerator for NameGen {
    fn generate(&self, addresses: &[GalacticAddress]) -> HashMap<GalacticAddress, Generated> {
        let wanted: Vec<GalacticAddress> = {
            let mut seen = HashSet::new();
            addresses
                .iter()
                .map(system_address)
                .filter(|a| seen.insert(*a))
                .collect()
        };
        let mut out = HashMap::new();
        let mut missing = Vec::new();
        {
            let Ok(cache) = self.cache.lock() else {
                return out;
            };
            for addr in &wanted {
                match cache.get(&cache_key(addr)) {
                    Some(g) => {
                        out.insert(*addr, g.clone());
                    }
                    None => missing.push(*addr),
                }
            }
        }
        if !missing.is_empty() {
            match self.generate_missing(&missing) {
                Ok(fresh) => {
                    if let Ok(mut cache) = self.cache.lock() {
                        for (addr, g) in &fresh {
                            cache.insert(cache_key(addr), g.clone());
                        }
                    }
                    let added = !fresh.is_empty();
                    out.extend(fresh);
                    if added {
                        self.save_cache();
                    }
                }
                Err(e) => log::warn!("generated names: {e}"),
            }
        }
        // Answer under the addresses asked for, planet index and all.
        addresses
            .iter()
            .filter_map(|a| out.get(&system_address(a)).map(|g| (*a, g.clone())))
            .collect()
    }
}

/// The address with its planet index cleared, which is what the tool generates for.
fn system_address(addr: &GalacticAddress) -> GalacticAddress {
    GalacticAddress::new(
        addr.voxel_x(),
        addr.voxel_y(),
        addr.voxel_z(),
        addr.solar_system_index(),
        0,
        addr.reality_index,
    )
}

/// The portal-layout value the tool takes, `PSSSYYZZZXXX` with planet 0.
fn system_packed(addr: &GalacticAddress) -> u64 {
    system_address(addr).packed()
}

fn cache_key(addr: &GalacticAddress) -> String {
    format!("{}:{:012X}", addr.reality_index, system_packed(addr))
}

/// The address a batch line answers for, from its `address` and `galaxy` fields.
fn address_of(value: &serde_json::Value) -> Option<GalacticAddress> {
    let hex = value.get("address")?.as_str()?;
    let packed = u64::from_str_radix(hex, 16).ok()?;
    let galaxy = u8::try_from(value.get("galaxy")?.as_i64()?).ok()?;
    Some(system_address(&GalacticAddress::from_packed(
        packed, galaxy,
    )))
}

/// Raw `system-attributes` output, plus the rendered counts from `attributes`.
#[derive(Debug, Deserialize)]
struct RawAttributes {
    star_type: i64,
    economy_type: i64,
    wealth: i64,
    conflict_level: i64,
    dominant_race: i64,
    uncharted: bool,
    abandoned: bool,
    pirate: bool,
    #[serde(default)]
    planet_count: i64,
    #[serde(default)]
    gas_giant: bool,
}

#[derive(Debug, Deserialize)]
struct RenderedCounts {
    rendered_planets: i64,
    rendered_moons: i64,
}

fn decode_attributes(
    raw: &serde_json::Value,
    rendered: Option<&serde_json::Value>,
) -> Option<SystemAttributes> {
    let r: RawAttributes = serde_json::from_value(raw.clone()).ok()?;
    let (planets, moons) =
        match rendered.and_then(|v| serde_json::from_value::<RenderedCounts>(v.clone()).ok()) {
            Some(c) => (c.rendered_planets, c.rendered_moons),
            None => (r.planet_count, 0),
        };
    SystemAttributes::from_codes(
        r.star_type,
        r.economy_type,
        r.wealth,
        r.conflict_level,
        r.dominant_race,
        r.uncharted,
        r.abandoned,
        r.pirate,
        planets,
        moons,
        r.gas_giant,
    )
}

fn python_works(python: &str) -> bool {
    Command::new(python)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn load_cache(path: &Path) -> HashMap<String, Generated> {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(cache) => cache,
            Err(e) => {
                log::debug!("ignoring {}: {e}", path.display());
                HashMap::new()
            }
        },
        Err(_) => HashMap::new(),
    }
}

fn write_cache(path: &Path, cache: &HashMap<String, Generated>) -> Result<(), NameGenError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(cache)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::generated::{Economy, Race, StarColour, Tier};

    fn lauderen() -> GalacticAddress {
        GalacticAddress::new(-532, -4, -1706, 0x43, 0, 0)
    }

    #[test]
    fn test_system_packed_clears_the_planet_and_matches_the_glyphs() {
        let planet = GalacticAddress::new(-532, -4, -1706, 0x43, 3, 0);
        assert_eq!(format!("{:012X}", system_packed(&planet)), "0043FC956DEC");
        assert_eq!(system_address(&planet), lauderen());
    }

    #[test]
    fn test_cache_key_carries_the_galaxy() {
        let hilbert = GalacticAddress::new(-532, -4, -1706, 0x43, 0, 1);
        assert_eq!(cache_key(&lauderen()), "0:0043FC956DEC");
        assert_eq!(cache_key(&hilbert), "1:0043FC956DEC");
    }

    #[test]
    fn test_address_of_reads_a_batch_line() {
        let line: serde_json::Value = serde_json::from_str(
            r#"{"system_name": "Lauderen", "address": "0043FC956DEC", "galaxy": 0}"#,
        )
        .unwrap();
        assert_eq!(address_of(&line), Some(lauderen()));
        let lower: serde_json::Value =
            serde_json::from_str(r#"{"address": "0043fc956dec", "galaxy": 1}"#).unwrap();
        assert_eq!(address_of(&lower).map(|a| a.reality_index), Some(1));
    }

    #[test]
    fn test_decode_attributes_uses_rendered_counts() {
        let raw: serde_json::Value = serde_json::from_str(r#"{"planet_count": 5, "prime_planet_count": 1, "safe_start_planet": 3, "gas_giant": false, "star_type": 0, "economy_type": 3, "wealth": 2, "conflict_level": 3, "dominant_race": 2, "uncharted": false, "abandoned": false, "pirate": false}"#).unwrap();
        let rendered: serde_json::Value =
            serde_json::from_str(r#"{"rendered_planets": 4, "rendered_moons": 1}"#).unwrap();
        let a = decode_attributes(&raw, Some(&rendered)).unwrap();
        assert_eq!(a.star, StarColour::Yellow);
        assert_eq!(a.economy, Economy::Scientific);
        assert_eq!(a.wealth, Tier::Medium);
        assert_eq!(a.conflict, Tier::High);
        assert_eq!(a.race, Some(Race::Korvax));
        assert_eq!((a.planets, a.moons), (4, 1));
        let without = decode_attributes(&raw, None).unwrap();
        assert_eq!((without.planets, without.moons), (5, 0));
    }

    #[test]
    fn test_cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.json");
        let mut cache = HashMap::new();
        cache.insert(
            cache_key(&lauderen()),
            Generated::new("Piponera Anomaly".into(), "Lauderen".into(), None),
        );
        write_cache(&path, &cache).unwrap();
        let back = load_cache(&path);
        assert_eq!(back, cache);
        assert!(load_cache(&dir.path().join("missing.json")).is_empty());
    }

    #[test]
    fn test_discover_without_script_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let config = NameGenConfig {
            script: Some(dir.path().join("namegen.py")),
            python: None,
            cache: None,
        };
        assert!(matches!(
            NameGen::try_discover(&config),
            Err(NameGenError::ScriptMissing(_))
        ));
    }

    /// Runs the real tool; needs `~/.nms-copilot/nms_namegen` and Python 3.13 with numpy.
    #[test]
    #[ignore]
    fn test_real_tool_generates_the_sample_names() {
        let namegen = NameGen::try_discover(&NameGenConfig {
            cache: Some(tempfile::tempdir().unwrap().path().join("c.json")),
            ..Default::default()
        })
        .unwrap();
        let hilbert = GalacticAddress::new(1433, -2, -929, 0x73, 0, 1);
        let out = namegen.generate(&[lauderen(), hilbert]);
        assert_eq!(out[&lauderen()].region, "Piponera Anomaly");
        assert_eq!(out[&lauderen()].name, "Lauderen");
        assert_eq!(out[&hilbert].name, "Soflokk-Sara");
        let a = out[&lauderen()].attributes.as_ref().unwrap();
        assert_eq!(a.economy, Economy::Scientific);
    }
}
