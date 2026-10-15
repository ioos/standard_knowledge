use std::collections::BTreeMap;
use std::convert::From;

use pyo3::prelude::*;
use standard_knowledge::Standard;

use crate::test_suite::PyTestSuite;

#[pyclass(name = "Standard")]
#[derive(Clone)]
pub struct PyStandard(pub Standard);

/// A CF compatible standard
#[pymethods]
impl PyStandard {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("<Standard: {}>", self.0.name))
    }

    #[getter]
    fn name(&self) -> PyResult<String> {
        Ok(self.0.name.clone())
    }

    #[getter]
    fn long_name(&self) -> PyResult<Option<String>> {
        Ok(self.0.long_name.clone())
    }

    #[getter]
    fn unit(&self) -> PyResult<String> {
        Ok(self.0.unit.clone())
    }

    #[getter]
    fn description(&self) -> PyResult<String> {
        Ok(self.0.description.clone())
    }

    #[getter]
    fn aliases(&self) -> PyResult<Vec<String>> {
        Ok(self.0.aliases.clone())
    }

    #[getter]
    fn ioos_category(&self) -> PyResult<Option<String>> {
        Ok(self.0.ioos_category.clone())
    }

    #[getter]
    fn common_variable_names(&self) -> PyResult<Vec<String>> {
        Ok(self.0.common_variable_names.clone())
    }

    #[getter]
    fn related_standards(&self) -> PyResult<Vec<String>> {
        Ok(self.0.related_standards.clone())
    }

    #[getter]
    fn other_units(&self) -> PyResult<Vec<String>> {
        Ok(self.0.other_units.clone())
    }

    #[getter]
    fn comments(&self) -> PyResult<Option<String>> {
        Ok(self.0.comments.clone())
    }

    #[getter]
    fn qc(&self, py: Python) -> PyResult<Option<Vec<Py<PyTestSuite>>>> {
        if self.0.qartod.is_empty() {
            Ok(None)
        } else {
            let test_suites: Result<Vec<_>, _> = self
                .0
                .qartod
                .iter()
                .map(|test_suite| {
                    let py_test_suite = PyTestSuite::new(test_suite.clone());
                    Py::new(py, py_test_suite)
                })
                .collect();
            Ok(Some(test_suites?))
        }
    }

    /// Return a dictionary of Xarray attributes
    fn attrs(&self) -> PyResult<BTreeMap<&str, &str>> {
        let map = self.0.xarray_attrs();

        Ok(map)
    }
}

impl From<Standard> for PyStandard {
    fn from(standard: Standard) -> Self {
        PyStandard(standard)
    }
}
