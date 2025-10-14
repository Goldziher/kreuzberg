use pyo3::PyErr;
use pyo3::exceptions::PyRuntimeError;

/// Convert a Kreuzberg error to a Python exception
pub fn to_py_err(error: kreuzberg::error::KreuzbergError) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}
