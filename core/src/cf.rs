use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::standard::Standard;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CfYaml {
    aliases: HashMap<String, String>,
    standard_names: HashMap<String, CfStandard>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct CfStandard {
    description: String,
    unit: String,
}

/// Returns a HashMap of standard names: vector of aliases
fn aliases_by_standard_name(cf_yaml: &CfYaml) -> HashMap<String, Vec<String>> {
    let aliases = &cf_yaml.aliases;

    let mut standards = HashMap::new();

    for (alias, standard_name) in aliases {
        standards
            .entry(standard_name.clone())
            .or_insert_with(Vec::new)
            .push(alias.clone());
    }

    standards
}

fn cf_yaml_to_standards(cf_yaml: CfYaml) -> HashMap<String, Standard> {
    let alias_map = aliases_by_standard_name(&cf_yaml);
    let mut standards = HashMap::new();
    for (name, cf_standard) in &cf_yaml.standard_names {
        let empty_vec = Vec::new();
        let aliases = alias_map.get(name).unwrap_or(&empty_vec);
        standards.insert(
            name.to_string(),
            Standard {
                name: name.to_string(),
                unit: cf_standard.unit.clone(),
                description: cf_standard.description.clone(),
                aliases: aliases.to_vec(),
                ..Standard::default()
            },
        );
    }
    standards
}

#[cfg(feature = "embedded-data")]
fn load_cf_yaml() -> CfYaml {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let compressed_data = include_bytes!(concat!(env!("OUT_DIR"), "/cf_standards.yaml.gz"));

    // Decompress the data
    let mut decoder = GzDecoder::new(&compressed_data[..]);
    let mut yaml_data = String::new();
    decoder.read_to_string(&mut yaml_data).unwrap();

    // Deserialize from YAML
    serde_yaml_ng::from_str(&yaml_data).unwrap()
}

/// Returns a HashMap of standard names to Standard from the embedded compressed data.
#[cfg(feature = "embedded-data")]
pub fn cf_standards() -> HashMap<String, Standard> {
    let cf_yaml = load_cf_yaml();
    cf_yaml_to_standards(cf_yaml)
}

/// Returns CF standards by deserializing a YAML string.
pub fn cf_standards_from_yaml(yaml: &str) -> Result<HashMap<String, Standard>, String> {
    serde_yaml_ng::from_str::<CfYaml>(yaml)
        .map(cf_yaml_to_standards)
        .map_err(|e| e.to_string())
}

/// Returns CF standards by deserializing a JSON string.
pub fn cf_standards_from_json(json: &str) -> Result<HashMap<String, Standard>, String> {
    serde_json::from_str::<CfYaml>(json)
        .map(cf_yaml_to_standards)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "embedded-data")]
    #[test]
    fn load_cf_standards() {
        let standards = cf_standards();
        let pressure = standards["air_pressure_at_mean_sea_level"].clone();
        assert_eq!(pressure.name, "air_pressure_at_mean_sea_level");

        println!("Name is correct");

        assert!(
            pressure
                .aliases
                .contains(&"air_pressure_at_sea_level".to_string()),
            "The standard `air_pressure_at_mean_sea_level` should contain the alias `air_pressure_at_sea_level`"
        )
    }

    #[cfg(feature = "embedded-data")]
    #[test]
    fn test_compressed_loading() {
        use flate2::read::GzDecoder;
        use std::io::Read;

        // Load the compressed data directly to ensure compression is working
        let compressed_data = include_bytes!(concat!(env!("OUT_DIR"), "/cf_standards.yaml.gz"));

        // Decompress the data
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut yaml_data = String::new();
        decoder.read_to_string(&mut yaml_data).unwrap();

        // Deserialize from YAML
        let cf: CfYaml = serde_yaml_ng::from_str(&yaml_data).unwrap();

        // Basic validation that we can load the compressed data
        assert!(
            !cf.standard_names.is_empty(),
            "CF standards should not be empty"
        );
        assert!(!cf.aliases.is_empty(), "CF aliases should not be empty");

        // Test that compression achieved significant reduction
        // Original YAML is ~3.9MB, compressed should be much smaller
        println!("Compressed size: {} bytes", compressed_data.len());
        assert!(
            compressed_data.len() < 1_000_000,
            "Compressed data should be less than 1MB"
        );
    }

    #[test]
    fn cf_standards_from_yaml_roundtrip() {
        let yaml = r#"
aliases:
  air_pressure_at_sea_level: air_pressure_at_mean_sea_level
standard_names:
  air_pressure_at_mean_sea_level:
    description: Air pressure at mean sea level
    unit: Pa
"#;
        let standards = cf_standards_from_yaml(yaml).unwrap();
        let p = &standards["air_pressure_at_mean_sea_level"];
        assert_eq!(p.unit, "Pa");
        assert!(p.aliases.contains(&"air_pressure_at_sea_level".to_string()));
    }

    #[test]
    fn cf_standards_from_json_roundtrip() {
        let json = r#"{
  "aliases": {"air_pressure_at_sea_level": "air_pressure_at_mean_sea_level"},
  "standard_names": {
    "air_pressure_at_mean_sea_level": {"description": "Air pressure at mean sea level", "unit": "Pa"}
  }
}"#;
        let standards = cf_standards_from_json(json).unwrap();
        let p = &standards["air_pressure_at_mean_sea_level"];
        assert_eq!(p.unit, "Pa");
        assert!(p.aliases.contains(&"air_pressure_at_sea_level".to_string()));
    }
}
