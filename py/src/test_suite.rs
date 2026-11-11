use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::pythonize;
use standard_knowledge::qartod::QcTestSuite;
use std::collections::HashMap;

#[pyclass(name = "TestSuite")]
#[derive(Clone)]
pub struct PyTestSuite {
    test_suite: QcTestSuite,
}

impl PyTestSuite {
    pub fn new(test_suite: QcTestSuite) -> Self {
        Self { test_suite }
    }
}

/// A QARTOD test suite
#[pymethods]
impl PyTestSuite {
    fn __repr__(&self) -> PyResult<String> {
        let info = self.test_suite.info();
        Ok(format!("<TestSuite: {} ({})>", info.name, info.slug))
    }

    /// Get test suite information
    fn info(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let info = self.test_suite.info();
        let dict = PyDict::new(py);
        dict.set_item("name", info.name)?;
        dict.set_item("slug", info.slug)?;
        dict.set_item("summary", info.summary)?;
        dict.set_item("description", info.description)?;

        let arguments = PyDict::new(py);
        for (name, argument) in &info.arguments {
            let item = PyDict::new(py);
            item.set_item("type", argument.argument_type.to_string())?;
            item.set_item("description", &argument.description)?;
            item.set_item("required", argument.is_required())?;
            if let Some(default) = argument.default {
                item.set_item("default", default)?;
            }
            arguments.set_item(name, item)?;
        }
        dict.set_item("arguments", arguments)?;

        dict.set_item("tests", self.test_suite.test_names())?;
        let test_types: Vec<String> = info.test_types.iter().map(|t| t.to_string()).collect();
        dict.set_item("test_types", test_types)?;
        Ok(dict.into())
    }

    /// Generate an ioos_qc configuration for the test suite
    fn scaffold(&self, py: Python<'_>, arguments: HashMap<String, f64>) -> PyResult<Py<PyAny>> {
        let config = self
            .test_suite
            .scaffold(&arguments)
            .map_err(PyRuntimeError::new_err)?;
        // Through a YAML value, so that spans are lists as in the YAML output
        let value =
            serde_yaml_ng::to_value(&config).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(pythonize(py, &value)?.unbind())
    }
}
