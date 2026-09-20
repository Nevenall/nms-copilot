//! `export`: the planets a `find` query matched, as JSON or CSV.
//!
//! One flat record per planet, so the output loads into a spreadsheet or a script without unpacking anything. Both binaries render through [`render_export`]; the CLI writes to stdout or a file, the REPL to a file or the terminal.

use std::io::Write;

use nms_core::galaxy::Galaxy;

use crate::display::hex_to_emoji;
use crate::find::FindResult;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            other => Err(format!("unknown format: {other} (expected json or csv)")),
        }
    }
}

/// A flat record for serialization (no nested objects).
#[derive(Debug, serde::Serialize)]
pub struct ExportRecord {
    pub planet_name: String,
    pub biome: String,
    pub system_name: String,
    /// The region's generated name, empty when no generator answered.
    pub region: String,
    pub distance_ly: f64,
    pub portal_glyphs: String,
    pub portal_emoji: String,
    pub coords_x: i16,
    pub coords_y: i8,
    pub coords_z: i16,
    pub galaxy: String,
    pub discoverer: String,
    pub infested: bool,
    /// Species, plants, and minerals the player has recorded on the planet.
    pub fauna_scanned: u16,
    pub flora_scanned: u16,
    pub minerals_scanned: u16,
}

impl From<&FindResult> for ExportRecord {
    fn from(r: &FindResult) -> Self {
        Self {
            planet_name: r.planet.name.clone().unwrap_or_default(),
            biome: r.planet.biome.map(|b| b.to_string()).unwrap_or_default(),
            system_name: r
                .system
                .name
                .clone()
                .or_else(|| r.generated.as_ref().map(|g| g.name.clone()))
                .unwrap_or_default(),
            region: r
                .generated
                .as_ref()
                .map(|g| g.region.clone())
                .unwrap_or_default(),
            distance_ly: r.distance_ly,
            portal_glyphs: r.portal_hex.clone(),
            portal_emoji: hex_to_emoji(&r.portal_hex),
            coords_x: r.system.address.voxel_x(),
            coords_y: r.system.address.voxel_y(),
            coords_z: r.system.address.voxel_z(),
            galaxy: Galaxy::by_index(r.system.address.reality_index)
                .name
                .to_string(),
            discoverer: r.system.discoverer.clone().unwrap_or_default(),
            infested: r.planet.infested,
            fauna_scanned: r.planet.scanned.fauna,
            flora_scanned: r.planet.scanned.flora,
            minerals_scanned: r.planet.scanned.minerals,
        }
    }
}

/// The records rendered in `format`, as text.
pub fn render_export(
    results: &[FindResult],
    format: ExportFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    match format {
        ExportFormat::Json => write_json(&mut buf, results)?,
        ExportFormat::Csv => write_csv(&mut buf, results)?,
    }
    Ok(String::from_utf8(buf)?)
}

pub fn write_json(
    out: &mut impl Write,
    results: &[FindResult],
) -> Result<(), Box<dyn std::error::Error>> {
    let records: Vec<ExportRecord> = results.iter().map(ExportRecord::from).collect();
    serde_json::to_writer_pretty(&mut *out, &records)?;
    writeln!(out)?;
    Ok(())
}

pub fn write_csv(
    out: &mut impl Write,
    results: &[FindResult],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_writer(&mut *out);
    for result in results {
        wtr.serialize(ExportRecord::from(result))?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nms_core::address::GalacticAddress;
    use nms_core::biome::Biome;
    use nms_core::system::{Planet, System};

    fn test_find_result() -> FindResult {
        let addr = GalacticAddress::new(100, 50, -200, 0x123, 0, 0);
        FindResult {
            planet: Planet::new(0, Some(Biome::Lush), None, false, Some("Eden".into()), None),
            system: System::new(
                addr,
                Some("Sol".into()),
                Some("Explorer".into()),
                None,
                vec![],
            ),
            distance_ly: 42_000.0,
            portal_hex: format!("{:012X}", addr.packed()),
            system_hex: format!("{:012X}", addr.packed()),
            generated: None,
        }
    }

    #[test]
    fn test_export_format_parse_json() {
        assert_eq!(ExportFormat::parse("json").unwrap(), ExportFormat::Json);
        assert_eq!(ExportFormat::parse("JSON").unwrap(), ExportFormat::Json);
    }

    #[test]
    fn test_export_format_parse_csv() {
        assert_eq!(ExportFormat::parse("csv").unwrap(), ExportFormat::Csv);
        assert_eq!(ExportFormat::parse("CSV").unwrap(), ExportFormat::Csv);
    }

    #[test]
    fn test_export_format_parse_unknown() {
        assert!(ExportFormat::parse("xml").is_err());
    }

    #[test]
    fn test_write_json_empty() {
        let mut buf = Vec::new();
        write_json(&mut buf, &[]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.trim(), "[]");
    }

    #[test]
    fn test_write_json_with_data() {
        let results = vec![test_find_result()];
        let mut buf = Vec::new();
        write_json(&mut buf, &results).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Eden"));
        assert!(s.contains("Lush"));
        assert!(s.contains("Sol"));
        assert!(s.contains("Explorer"));
        assert!(s.contains("Euclid"));
    }

    #[test]
    fn test_write_csv_empty() {
        let mut buf = Vec::new();
        write_csv(&mut buf, &[]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn test_write_csv_with_data() {
        let results = vec![test_find_result()];
        let mut buf = Vec::new();
        write_csv(&mut buf, &results).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("planet_name"));
        assert!(s.contains("Eden"));
        assert!(s.contains("Lush"));
        assert!(s.contains("Sol"));
    }

    #[test]
    fn test_export_record_from_find_result() {
        let result = test_find_result();
        let record = ExportRecord::from(&result);
        assert_eq!(record.planet_name, "Eden");
        assert_eq!(record.biome, "Lush");
        assert_eq!(record.system_name, "Sol");
        assert_eq!(record.distance_ly, 42_000.0);
        assert!(!record.portal_glyphs.is_empty());
        assert!(!record.portal_emoji.is_empty());
        assert_eq!(record.galaxy, "Euclid");
        assert_eq!(record.discoverer, "Explorer");
        assert!(!record.infested);
    }

    #[test]
    fn test_export_record_missing_names() {
        let addr = GalacticAddress::new(0, 0, 0, 1, 0, 0);
        let result = FindResult {
            planet: Planet::new(0, None, None, true, None, None),
            system: System::new(addr, None, None, None, vec![]),
            distance_ly: 0.0,
            portal_hex: "000000000001".into(),
            system_hex: "000000000001".into(),
            generated: None,
        };
        let record = ExportRecord::from(&result);
        assert!(record.planet_name.is_empty());
        assert!(record.system_name.is_empty());
        assert!(record.discoverer.is_empty());
        assert!(record.infested);
    }

    #[test]
    fn test_json_output_is_valid_json() {
        let results = vec![test_find_result()];
        let mut buf = Vec::new();
        write_json(&mut buf, &results).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_csv_output_has_header_and_row() {
        let results = vec![test_find_result()];
        let mut buf = Vec::new();
        write_csv(&mut buf, &results).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 row
        assert!(lines[0].starts_with("planet_name,"));
    }
}
