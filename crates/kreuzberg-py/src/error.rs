//! Error conversion from Rust to Python exceptions
//!
//! Converts `KreuzbergError` from the Rust core into appropriate Python exceptions.

use pyo3::{exceptions::*, prelude::*};

/// Convert Rust KreuzbergError to Python exception.
///
/// Maps error variants to appropriate Python exception types:
/// - `Validation` → `ValueError`
/// - `UnsupportedFormat` → `ValueError`
/// - `Parsing` → `RuntimeError`
/// - `Io` → `IOError`
/// - `Ocr` → `RuntimeError`
/// - `Plugin` → `RuntimeError`
/// - `Config` → `ValueError`
/// - `Other` → `RuntimeError`
pub fn to_py_err(error: kreuzberg::KreuzbergError) -> PyErr {
    use kreuzberg::KreuzbergError;

    match error {
        KreuzbergError::Validation { message, source } => {
            let full_message = if let Some(src) = source {
                format!("{}: {}", message, src)
            } else {
                message
            };
            PyValueError::new_err(full_message)
        }
        KreuzbergError::UnsupportedFormat(msg) => PyValueError::new_err(msg),
        KreuzbergError::Parsing { message, .. } => PyRuntimeError::new_err(message),
        KreuzbergError::Io(e) => PyIOError::new_err(e.to_string()),
        KreuzbergError::Ocr { message, .. } => PyRuntimeError::new_err(message),
        KreuzbergError::Plugin { message, .. } => PyRuntimeError::new_err(message),
        KreuzbergError::Cache { message, .. } => PyRuntimeError::new_err(message),
        KreuzbergError::ImageProcessing { message, .. } => PyRuntimeError::new_err(message),
        KreuzbergError::Serialization { message, .. } => PyRuntimeError::new_err(message),
        KreuzbergError::MissingDependency(msg) => PyRuntimeError::new_err(msg),
        KreuzbergError::Other(msg) => PyRuntimeError::new_err(msg),
    }
}
