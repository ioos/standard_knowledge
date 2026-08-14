use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::{write::GzEncoder, Compression};
// use serde::{Deserialize, Serialize};

include!("./src/qartod/config.rs");
include!("./src/qartod/static_qc_include.rs");
include!("./src/knowledge_include.rs");

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

pub fn write_cf_standards_from_yaml() {
    let standard_path = Path::new("standards/_cf_standards.yaml");
    let contents = fs::read_to_string(standard_path).expect("Unable to read standards");

    let cf: CfYaml = serde_yaml_ng::from_str(&contents).unwrap();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("cf_standards.yaml.gz");

    // Serialize to YAML
    let yaml_string = serde_yaml_ng::to_string(&cf).unwrap();
    let yaml_bytes = yaml_string.as_bytes();

    // Then compress with gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(yaml_bytes).unwrap();
    let compressed_data = encoder.finish().unwrap();

    println!(
        "CF standards: {} bytes → {} bytes (YAML) → {} bytes (compressed), ratio: {:.1}%",
        contents.len(),
        yaml_bytes.len(),
        compressed_data.len(),
        (compressed_data.len() as f64 / contents.len() as f64) * 100.0
    );

    fs::write(&dest_path, compressed_data).unwrap()
}

fn find_knowledge() -> Vec<PathBuf> {
    use std::path::Path;

    let mut knowledge_files = Vec::new();

    let path = Path::new("standards");

    for entry in path.read_dir().expect("read_dir call failed").flatten() {
        let file_path = entry.path();

        if let Some(ext) = file_path.extension() {
            if ext == "yaml" && file_path.file_stem().unwrap() != "_cf_standards" {
                knowledge_files.push(file_path);
            }
        }
    }

    knowledge_files
}

fn load_knowledge(path: &PathBuf) -> Knowledge {
    let name = &path.file_stem().unwrap();
    let contents = fs::read_to_string(path).expect("Unable to read knowledge");

    let partial_knowledge: YamlKnowledge = serde_yaml_ng::from_str(&contents)
        .expect(format!("Failed to parse knowledge from {}", path.display()).as_str());
    Knowledge {
        name: name.to_str().unwrap().to_string(),
        long_name: partial_knowledge.long_name,
        ioos_category: partial_knowledge.ioos_category,
        common_variable_names: partial_knowledge.common_variable_names.unwrap_or_default(),
        related_standards: partial_knowledge.related_standards.unwrap_or_default(),
        sibling_standards: partial_knowledge.sibling_standards.unwrap_or_default(),
        extra_attrs: partial_knowledge.extra_attrs.unwrap_or_default(),
        other_units: partial_knowledge.other_units.unwrap_or_default(),
        comments: partial_knowledge.comments,
        qc: partial_knowledge.qc,
    }
}

fn write_knowledge() {
    let knowledge_paths = find_knowledge();
    let mut loaded_knowledge = Vec::new();

    for path in knowledge_paths {
        let knowledge = load_knowledge(&path);
        loaded_knowledge.push(knowledge);
    }

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("knowledge.yaml.gz");

    // Serialize to YAML
    let yaml_string = serde_yaml_ng::to_string(&loaded_knowledge).unwrap();
    let yaml_bytes = yaml_string.as_bytes();

    // Then compress with gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(yaml_bytes).unwrap();
    let compressed_data = encoder.finish().unwrap();

    println!(
        "Knowledge: {} bytes (YAML) → {} bytes (compressed), ratio: {:.1}%",
        yaml_bytes.len(),
        compressed_data.len(),
        (compressed_data.len() as f64 / yaml_bytes.len() as f64) * 100.0
    );

    fs::write(&dest_path, compressed_data).unwrap()
}

fn main() {
    if std::env::var("CARGO_FEATURE_EMBEDDED_DATA").is_ok() {
        write_cf_standards_from_yaml();
        write_knowledge();
    }

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=standards/")
}
