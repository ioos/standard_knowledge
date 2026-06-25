use std::convert::From;

use pyo3::exceptions::{PyIndexError, PyKeyError};
use pyo3::prelude::*;

use crate::PyStandard;
use standard_knowledge::standards_filter::StandardsFilter;
use standard_knowledge::Standard;

#[pyclass]
pub struct PyStandardsFilterIter {
    standards: Vec<Standard>,
    index: usize,
}

#[pymethods]
impl PyStandardsFilterIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Py<PyStandard>>> {
        let index = slf.index;
        if index < slf.standards.len() {
            let standard = slf.standards[index].clone();
            slf.index += 1;
            let py = slf.py();
            Ok(Some(Py::new(py, PyStandard(standard))?))
        } else {
            Ok(None)
        }
    }
}

#[pyclass(name = "StandardsFilter")]
#[derive(Clone)]
pub struct PyStandardsFilter {
    inner: StandardsFilter,
}

impl From<StandardsFilter> for PyStandardsFilter {
    fn from(filter: StandardsFilter) -> Self {
        PyStandardsFilter { inner: filter }
    }
}

#[pymethods]
impl PyStandardsFilter {
    /// Return standards by common variable name
    fn by_variable_name(&self, py: Python, variable_name: &str) -> PyResult<Py<PyStandardsFilter>> {
        let filtered = self.inner.by_variable_name(variable_name);
        Py::new(py, PyStandardsFilter { inner: filtered })
    }

    /// Return standards by IOOS category
    fn by_ioos_category(&self, py: Python, category: &str) -> PyResult<Py<PyStandardsFilter>> {
        let filtered = self.inner.by_ioos_category(category);
        Py::new(py, PyStandardsFilter { inner: filtered })
    }

    /// Return standards for a given unit
    fn by_unit(&self, py: Python, unit: &str) -> PyResult<Py<PyStandardsFilter>> {
        let filtered = self.inner.by_unit(unit);
        Py::new(py, PyStandardsFilter { inner: filtered })
    }

    /// Return standards that have QARTOD tests
    fn has_qartod_tests(&self, py: Python) -> PyResult<Py<PyStandardsFilter>> {
        let filtered = self.inner.has_qartod_tests();
        Py::new(py, PyStandardsFilter { inner: filtered })
    }

    /// Return standards that match a search pattern
    fn search(&self, py: Python, search_str: &str) -> PyResult<Py<PyStandardsFilter>> {
        let filtered = self.inner.search(search_str);
        Py::new(py, PyStandardsFilter { inner: filtered })
    }

    /// Get a specific standard by name or alias
    fn get(&self, py: Python, standard_name_or_alias: &str) -> PyResult<Py<PyStandard>> {
        match self.inner.get(standard_name_or_alias) {
            Ok(standard) => {
                let py_standard = PyStandard(standard.clone());
                Py::new(py, py_standard)
            }
            Err(_) => Err(PyKeyError::new_err("Unknown Standard")),
        }
    }

    /// Return an iterator over the standards
    fn __iter__(&self, py: Python) -> PyResult<Py<PyStandardsFilterIter>> {
        Py::new(
            py,
            PyStandardsFilterIter {
                standards: self.inner.standards.clone(),
                index: 0,
            },
        )
    }

    /// Return the number of standards in the filter
    fn __len__(&self) -> usize {
        self.inner.standards.len()
    }

    /// Get a standard by index
    fn __getitem__(&self, py: Python, index: usize) -> PyResult<Py<PyStandard>> {
        if index < self.inner.standards.len() {
            let py_standard = PyStandard(self.inner.standards[index].clone());
            Py::new(py, py_standard)
        } else {
            Err(PyIndexError::new_err("Index out of range"))
        }
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "<StandardsFilter: {} standards>",
            self.inner.standards.len()
        ))
    }
}
