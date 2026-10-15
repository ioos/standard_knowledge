use std::collections::HashMap;

use pyo3::exceptions::{
    PyKeyError, PyOverflowError, PyTypeError, PyValueError, PyZeroDivisionError,
};
use pyo3::prelude::*;
use standard_knowledge::fx::{self, FxError};

fn to_py_err(error: FxError) -> PyErr {
    let message = error.to_string();
    match error {
        FxError::MissingStat(name) => PyKeyError::new_err(name),
        FxError::Type(_) => PyTypeError::new_err(message),
        FxError::ZeroDivision(_) => PyZeroDivisionError::new_err(message),
        FxError::Overflow(_) => PyOverflowError::new_err(message),
        FxError::Parse { .. }
        | FxError::InvalidIdentifier(_)
        | FxError::Domain(_)
        | FxError::NonReal(_) => PyValueError::new_err(message),
    }
}

/// Evaluate an ioos_qc `config_creator` threshold expression
///
/// `stats` supplies `min`, `max`, `mean` and `std`, for example
/// `eval_fx("mean + 3 * std", {"mean": 10.0, "std": 2.0})` returns 16.0.
/// Follows `ioos_qc.config_creator.fx_parser.eval_fx`.
#[pyfunction]
pub fn eval_fx(fx: &str, stats: HashMap<String, f64>) -> PyResult<f64> {
    fx::eval_fx(fx, &stats).map_err(to_py_err)
}
