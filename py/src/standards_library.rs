use std::collections::{BTreeMap, HashMap};

use pyo3::{
    exceptions::{PyKeyError, PyValueError},
    prelude::*,
    types::{PyBool, PyDict},
};

use crate::standard::PyStandard;
use standard_knowledge::qartod::static_qc::StaticQc;
use standard_knowledge::{Knowledge, StandardsLibrary};

#[pyclass(name = "StandardsLibrary")]
#[derive(Clone)]
pub struct PyStandardsLibrary(pub StandardsLibrary);

/// A collection of CF compatible standards with methods for searching through them
#[pymethods]
impl PyStandardsLibrary {
    #[new]
    fn new() -> Self {
        Self(StandardsLibrary {
            standards: HashMap::new(),
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "<StandardsLibrary: {} standards>",
            self.0.standards.len()
        ))
    }

    /// Get the standards dictionary
    #[getter]
    fn standards(&self) -> PyResult<HashMap<String, crate::standard::PyStandard>> {
        let mut py_standards = HashMap::new();
        for (key, standard) in &self.0.standards {
            py_standards.insert(key.clone(), crate::standard::PyStandard(standard.clone()));
        }
        Ok(py_standards)
    }

    /// Load CF standards into library
    fn load_cf_standards(&mut self) {
        self.0.load_cf_standards();
    }

    /// Get a standard by standard name or aliases
    fn get(&self, py: Python, name_or_alias: &str) -> PyResult<Py<PyStandard>> {
        match self.0.get(name_or_alias) {
            Ok(standard) => {
                let py_standard = PyStandard(standard);
                Py::new(py, py_standard)
            }
            Err(e) => Err(PyKeyError::new_err(e.to_string())),
        }
    }

    /// Apply knowledge to loaded standards
    fn apply_knowledge(
        &mut self,
        knowledge: Vec<HashMap<String, KnowledgeValues>>,
    ) -> PyResult<()> {
        let mut cleaned_knowledge = Vec::new();

        for know in knowledge {
            let name;
            if let Some(value) = know.get("name") {
                if let KnowledgeValues::String(str_value) = value {
                    name = str_value.clone();
                } else {
                    return Err(PyKeyError::new_err("`name` is not a string in knowledge"));
                }
            } else {
                return Err(PyKeyError::new_err(
                    "The knowledge needs a name of the standard to be applied to",
                ));
            }

            let long_name = get_string_field(&know, "long_name")?;
            let ioos_category = get_string_field(&know, "ioos_category")?;
            let comments = get_string_field(&know, "comments")?;

            let common_variable_names = get_list_field(&know, "common_variable_names")?;
            let related_standards = get_list_field(&know, "related_standards")?;
            let sibling_standards = get_list_field(&know, "sibling_standards")?;
            let extra_attrs = get_dict_field(&know, "extra_attrs")?;
            let other_units = get_list_field(&know, "other_units")?;
            let qc = get_static_qc_field(&know, "qc")?;

            let cleaned = Knowledge {
                name,
                long_name,
                ioos_category,
                common_variable_names,
                related_standards,
                sibling_standards,
                extra_attrs,
                other_units,
                comments,
                qc: Some(qc),
            };
            cleaned_knowledge.push(cleaned);
        }

        self.0.apply_knowledge(cleaned_knowledge);

        Ok(())
    }

    /// Load community knowledge baked into the library
    fn load_knowledge(&mut self) {
        self.0.load_knowledge();
    }

    /// Return a standards filter for chaining operations
    fn filter(&self, py: Python) -> PyResult<Py<crate::PyStandardsFilter>> {
        let filter = self.0.filter();
        let py_filter = crate::PyStandardsFilter::from(filter);
        Py::new(py, py_filter)
    }

    /// Return known IOOS Categories
    fn known_ioos_categories(&self) -> Vec<String> {
        self.0.known_ioos_categories().into_iter().collect()
    }
}

#[derive(Debug)]
enum KnowledgeValues {
    String(String),
    List(Vec<String>),
    Dict(BTreeMap<String, String>),
    QC(BTreeMap<String, StaticQc>),
}

impl<'a, 'py> FromPyObject<'a, 'py> for KnowledgeValues {
    type Error = PyErr;

    fn extract(ob: pyo3::Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        // Try to extract as string first
        if let Ok(s) = ob.extract::<String>() {
            return Ok(KnowledgeValues::String(s));
        }

        // Try to extract as list of strings
        if let Ok(list) = ob.extract::<Vec<String>>() {
            return Ok(KnowledgeValues::List(list));
        }

        // Try to extract as dict of strings
        if let Ok(dict) = ob.extract::<BTreeMap<String, String>>() {
            return Ok(KnowledgeValues::Dict(dict));
        }

        // Try to extract as QC dict (BTreeMap<String, StaticQc>)
        if let Ok(dict) = ob.cast::<PyDict>() {
            let mut qc_map = BTreeMap::new();

            for (key, value) in dict.iter() {
                let key_str: String = key.extract()?;
                let static_qc: StaticQc = serde_yaml_ng::from_value(to_yaml_value(&value)?)
                    .map_err(|e| PyKeyError::new_err(format!("Invalid QC for {key_str}: {e}")))?;
                // Catch mistakes when the knowledge is applied rather than
                // when the suite is first used
                static_qc
                    .check()
                    .map_err(|e| PyValueError::new_err(format!("Invalid QC for {key_str}: {e}")))?;
                qc_map.insert(key_str, static_qc);
            }

            return Ok(KnowledgeValues::QC(qc_map));
        }

        Err(PyKeyError::new_err(
            "Could not extract KnowledgeValues from Python object",
        ))
    }
}

fn get_static_qc_field(
    knowledge: &HashMap<String, KnowledgeValues>,
    key: &str,
) -> PyResult<BTreeMap<String, StaticQc>> {
    match knowledge.get(key) {
        Some(KnowledgeValues::QC(qc_value)) => Ok(qc_value.clone()),
        Some(_) => Err(PyKeyError::new_err(format!(
            "`{key}` must be a QARTOD static QC field"
        ))),
        None => Ok(BTreeMap::new()),
    }
}

fn get_string_field(
    knowledge: &HashMap<String, KnowledgeValues>,
    key: &str,
) -> PyResult<Option<String>> {
    match knowledge.get(key) {
        Some(KnowledgeValues::String(str_value)) => Ok(Some(str_value.clone())),
        Some(_) => Err(PyKeyError::new_err(format!(
            "`{key}` must be a string field"
        ))),
        None => Ok(None),
    }
}

fn get_list_field(
    knowledge: &HashMap<String, KnowledgeValues>,
    key: &str,
) -> PyResult<Vec<String>> {
    match knowledge.get(key) {
        Some(KnowledgeValues::List(list_value)) => Ok(list_value.clone()),
        Some(_) => Err(PyKeyError::new_err(format!(
            "`{key}` must be a list of strings"
        ))),
        None => Ok(Vec::new()),
    }
}

fn get_dict_field(
    knowledge: &HashMap<String, KnowledgeValues>,
    key: &str,
) -> PyResult<BTreeMap<String, String>> {
    match knowledge.get(key) {
        Some(KnowledgeValues::Dict(dict_value)) => Ok(dict_value.clone()),
        Some(_) => Err(PyKeyError::new_err(format!(
            "`{key}` must be a dictionary of strings"
        ))),
        None => Ok(BTreeMap::new()),
    }
}

/// Convert plain Python data (dicts, lists, strings, numbers, bools and
/// None) to a YAML value
fn to_yaml_value(value: &Bound<'_, PyAny>) -> PyResult<serde_yaml_ng::Value> {
    use serde_yaml_ng::{Mapping, Value};

    if value.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = value.cast::<PyBool>() {
        return Ok(Value::Bool(b.is_true()));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(Value::from(i));
    }
    if let Ok(f) = value.extract::<f64>() {
        return Ok(Value::from(f));
    }
    if let Ok(s) = value.extract::<String>() {
        return Ok(Value::String(s));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut mapping = Mapping::new();
        for (key, item) in dict.iter() {
            mapping.insert(to_yaml_value(&key)?, to_yaml_value(&item)?);
        }
        return Ok(Value::Mapping(mapping));
    }
    if let Ok(items) = value.extract::<Vec<Bound<'_, PyAny>>>() {
        return items
            .iter()
            .map(to_yaml_value)
            .collect::<PyResult<Vec<_>>>()
            .map(Value::Sequence);
    }
    Err(PyValueError::new_err(format!(
        "Can't convert {} to QC configuration",
        value.get_type().name()?
    )))
}
