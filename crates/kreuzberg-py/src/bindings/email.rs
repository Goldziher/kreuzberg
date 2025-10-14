use pyo3::prelude::*;

use crate::error::to_py_err;
use crate::types::PyEmailExtractionResult;

#[pyfunction]
pub fn extract_email_content(py: Python<'_>, data: &[u8], mime_type: &str) -> PyResult<PyEmailExtractionResult> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::extract_email_content(data, mime_type))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyEmailExtractionResult::from(result))
}

#[pyfunction]
pub fn parse_eml_content(py: Python<'_>, data: &[u8]) -> PyResult<PyEmailExtractionResult> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_eml_content(data))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyEmailExtractionResult::from(result))
}

#[pyfunction]
pub fn parse_msg_content(py: Python<'_>, data: &[u8]) -> PyResult<PyEmailExtractionResult> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_msg_content(data))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyEmailExtractionResult::from(result))
}

#[pyfunction]
pub fn build_email_text_output(py: Python<'_>, result: &PyEmailExtractionResult) -> PyResult<String> {
    // Access the inner Rust type
    let core_result = &result.inner;

    // Release GIL during computation
    Ok(py.detach(|| kreuzberg::extraction::build_email_text_output(core_result)))
}
