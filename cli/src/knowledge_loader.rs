use standard_knowledge::{Knowledge, StandardsLibrary, YamlKnowledge};
use std::fs;
use std::path::Path;

/// Load knowledge from a file path (single file or directory)
pub fn load_knowledge_from_path(
    library: &mut StandardsLibrary,
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();

    if path.is_file() {
        // Load single file
        let knowledge = load_single_knowledge_file(path)?;
        library.apply_knowledge(knowledge);
    } else if path.is_dir() {
        // Load all YAML files from directory
        let knowledge = load_knowledge_from_directory(path)?;
        library.apply_knowledge(knowledge);
    } else {
        return Err(format!("Path does not exist: {}", path.display()).into());
    }

    Ok(())
}

/// Load knowledge from a URL
pub fn load_knowledge_from_url(
    library: &mut StandardsLibrary,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Download the content from the URL
    let response =
        reqwest::blocking::get(url).map_err(|e| format!("Failed to fetch URL {url}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error {} when fetching {}", response.status(), url).into());
    }

    let contents = response
        .text()
        .map_err(|e| format!("Failed to read response from {url}: {e}"))?;

    // Extract filename from URL for standard name
    let filename = url
        .split('/')
        .next_back()
        .and_then(|f| f.strip_suffix(".yaml").or_else(|| f.strip_suffix(".yml")))
        .unwrap_or("unknown_standard");

    let knowledge = parse_knowledge(filename, contents);

    match knowledge {
        Err(e) => return Err(format!("Failed to parse knowledge from {url}: {e}").into()),
        Ok(knowledge) => library.apply_knowledge(knowledge),
    }
    Ok(())
}

/// Load a single knowledge file
fn load_single_knowledge_file(path: &Path) -> Result<Vec<Knowledge>, String> {
    let filename = path
        .file_stem()
        .ok_or_else(|| format!("Invalid filename: {}", path.display()))?
        .to_str()
        .ok_or_else(|| format!("Invalid UTF-8 in filename: {}", path.display()))?;

    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Unable to read knowledge file {}: {}", path.display(), e))?;

    parse_knowledge(filename, contents)
}

/// Parse knowledge from YAML contents
fn parse_knowledge(filename: &str, contents: String) -> Result<Vec<Knowledge>, String> {
    let partial_knowledge: Result<YamlKnowledge, serde_yaml_ng::Error> =
        serde_yaml_ng::from_str(&contents);

    // Report the error for a single knowledge unless the file is a list
    let is_list = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&contents)
        .is_ok_and(|value| value.is_sequence());
    if !is_list {
        let partial_knowledge =
            partial_knowledge.map_err(|e| format!("Failed to deserialize YAML from {e}"))?;
        let knowledge = Knowledge {
            name: partial_knowledge
                .name
                .unwrap_or_else(|| filename.to_string()),
            long_name: partial_knowledge.long_name,
            ioos_category: partial_knowledge.ioos_category,
            common_variable_names: partial_knowledge.common_variable_names.unwrap_or_default(),
            related_standards: partial_knowledge.related_standards.unwrap_or_default(),
            sibling_standards: partial_knowledge.sibling_standards.unwrap_or_default(),
            extra_attrs: partial_knowledge.extra_attrs.unwrap_or_default(),
            other_units: partial_knowledge.other_units.unwrap_or_default(),
            comments: partial_knowledge.comments,
            qc: partial_knowledge.qc,
        };
        return Ok(vec![knowledge]);
    }

    let knowledge: Result<Vec<YamlKnowledge>, serde_yaml_ng::Error> =
        serde_yaml_ng::from_str(&contents);
    match knowledge {
        Ok(knowledge) => Ok(knowledge
            .iter()
            .map(|partial_knowledge| Knowledge {
                name: partial_knowledge.name.clone().unwrap_or_default(),
                long_name: partial_knowledge.long_name.clone(),
                ioos_category: partial_knowledge.ioos_category.clone(),
                common_variable_names: partial_knowledge
                    .common_variable_names
                    .clone()
                    .unwrap_or_default(),
                related_standards: partial_knowledge
                    .related_standards
                    .clone()
                    .unwrap_or_default(),
                sibling_standards: partial_knowledge
                    .sibling_standards
                    .clone()
                    .unwrap_or_default(),
                extra_attrs: partial_knowledge.extra_attrs.clone().unwrap_or_default(),
                other_units: partial_knowledge.other_units.clone().unwrap_or_default(),
                comments: partial_knowledge.comments.clone(),
                qc: partial_knowledge.qc.clone(),
            })
            .collect()),
        Err(e) => Err(format!("Failed to deserialize YAML from {e}")),
    }
}

/// Load all knowledge files from a directory
fn load_knowledge_from_directory(
    path: &Path,
) -> Result<Vec<Knowledge>, Box<dyn std::error::Error>> {
    let mut knowledge_list: Vec<Knowledge> = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();

        if let Some(ext) = file_path.extension() {
            if ext == "yaml" || ext == "yml" {
                // Skip CF standards file if it exists
                if let Some(stem) = file_path.file_stem() {
                    if stem == "_cf_standards" {
                        continue;
                    }
                }

                match load_single_knowledge_file(&file_path) {
                    Ok(mut knowledge) => knowledge_list.append(&mut knowledge),
                    Err(e) => eprintln!("Warning: Failed to load {}: {}", file_path.display(), e),
                }
            }
        }
    }

    Ok(knowledge_list)
}
