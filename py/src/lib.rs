pub mod standard;
pub mod standards_filter;
pub mod standards_library;
pub mod test_suite;

pub use standard::PyStandard;
pub use standards_filter::{PyStandardsFilter, PyStandardsFilterIter};
pub use standards_library::PyStandardsLibrary;
pub use test_suite::PyTestSuite;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn _standard_knowledge_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStandard>()?;
    m.add_class::<PyStandardsLibrary>()?;
    m.add_class::<PyStandardsFilter>()?;
    m.add_class::<PyStandardsFilterIter>()?;
    m.add_class::<PyTestSuite>()?;
    Ok(())
}
