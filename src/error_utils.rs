/// Minimal error handling utilities for live code only
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::{PyErr, PyResult};
use std::fmt::Display;

/// Trait for converting Result types to PyResults with IO error context
pub trait IntoKreuzbergError<T> {
    fn into_io_error(self, context: &str) -> PyResult<T>;
}

impl<T, E: Display> IntoKreuzbergError<T> for Result<T, E> {
    fn into_io_error(self, context: &str) -> PyResult<T> {
        self.map_err(|e| PyIOError::new_err(format!("{}: {}", context, e)))
    }
}

/// Common error creation helpers for live code
pub mod errors {
    use super::*;

    /// Create value error with formatted context
    #[inline]
    pub fn value_error(context: &str, details: &str) -> PyErr {
        PyValueError::new_err(format!("{}: {}", context, details))
    }

    /// Create value error for out-of-range values
    #[inline]
    pub fn out_of_range(param_name: &str, value: &dyn Display, min: &dyn Display, max: &dyn Display) -> PyErr {
        PyValueError::new_err(format!(
            "{} value '{}' is out of range [{}, {}]",
            param_name, value, min, max
        ))
    }
}
