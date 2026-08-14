use crate::knowledge::Knowledge;

#[cfg(feature = "embedded-data")]
pub fn load_knowledge() -> Vec<Knowledge> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let compressed_data = include_bytes!(concat!(env!("OUT_DIR"), "/knowledge.yaml.gz"));

    // Decompress the data
    let mut decoder = GzDecoder::new(&compressed_data[..]);
    let mut yaml_data = String::new();
    decoder.read_to_string(&mut yaml_data).unwrap();

    // Deserialize from YAML
    let knowledge: Vec<Knowledge> = serde_yaml_ng::from_str(&yaml_data).unwrap();

    knowledge
}

/// Deserializes knowledge from a YAML string.
pub fn load_knowledge_from_yaml(yaml: &str) -> Result<Vec<Knowledge>, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| e.to_string())
}

/// Deserializes knowledge from a JSON string.
pub fn load_knowledge_from_json(json: &str) -> Result<Vec<Knowledge>, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}
