use pyo3::prelude::*;

use crate::error::to_py_err;
use crate::types::PyXmlExtractionResult;

#[pyfunction]
pub fn parse_xml(py: Python<'_>, xml_bytes: &[u8], preserve_whitespace: bool) -> PyResult<PyXmlExtractionResult> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_xml(xml_bytes, preserve_whitespace))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyXmlExtractionResult::from(result))
}
