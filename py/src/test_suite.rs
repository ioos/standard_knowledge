use pyo3::prelude::*;
use pyo3::types::PyDict;
use standard_knowledge::qartod::{types::ArgumentValue, QcTestSuite};
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
        // TODO: Add arguments and test_types if needed
        Ok(dict.into())
    }

    /// Generate a configuration for the test suite
    fn scaffold(
        &self,
        py: Python<'_>,
        arguments: HashMap<String, Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        // Convert Python arguments to ArgumentValue
        let mut rust_args = HashMap::new();
        for (key, value) in arguments {
            // For now, we'll handle basic types. This could be expanded.
            if let Ok(s) = value.extract::<String>(py) {
                rust_args.insert(key, ArgumentValue::String(s));
            } else if let Ok(f) = value.extract::<f64>(py) {
                rust_args.insert(key, ArgumentValue::Float(f));
            } else if let Ok(i) = value.extract::<i64>(py) {
                rust_args.insert(key, ArgumentValue::Int(i));
            }
            // Add more types as needed
        }

        match self.test_suite.scaffold(rust_args) {
            Ok(config) => {
                // Convert ConfigStream to Python dict
                let result = PyDict::new(py);

                // Convert the qartod config to a Python dict
                let qartod_config = &config.qartod;
                let qartod_dict = PyDict::new(py);

                if let Some(ref gross_range) = qartod_config.gross_range_test {
                    let test_dict = PyDict::new(py);
                    test_dict.set_item(
                        "fail_span",
                        vec![gross_range.fail_span.0, gross_range.fail_span.1],
                    )?;
                    test_dict.set_item(
                        "suspect_span",
                        vec![gross_range.suspect_span.0, gross_range.suspect_span.1],
                    )?;
                    qartod_dict.set_item("gross_range_test", test_dict)?;
                }

                if let Some(ref flat_line) = qartod_config.flat_line_test {
                    let test_dict = PyDict::new(py);
                    test_dict.set_item("tolerance", flat_line.tolerance)?;
                    test_dict.set_item("suspect_threshold", flat_line.suspect_threshold)?;
                    test_dict.set_item("fail_threshold", flat_line.fail_threshold)?;
                    qartod_dict.set_item("flat_line_test", test_dict)?;
                }

                if let Some(ref spike) = qartod_config.spike_test {
                    let test_dict = PyDict::new(py);
                    test_dict.set_item("suspect_threshold", spike.suspect_threshold)?;
                    test_dict.set_item("fail_threshold", spike.fail_threshold)?;
                    qartod_dict.set_item("spike_test", test_dict)?;
                }

                if let Some(ref rate_of_change) = qartod_config.rate_of_change_test {
                    let test_dict = PyDict::new(py);
                    test_dict.set_item("threshold", rate_of_change.threshold)?;
                    qartod_dict.set_item("rate_of_change_test", test_dict)?;
                }

                result.set_item("qartod", qartod_dict)?;

                Ok(result.into())
            }
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e)),
        }
    }
}
