//! MIME type detection and validation utilities

use crate::error::to_py_err;
use pyo3::prelude::*;

/// Detect MIME type from file path.
///
/// Args:
///     path: Path to the file
///
/// Returns:
///     Detected MIME type string
///
/// Raises:
///     ValueError: If MIME type cannot be determined
///
/// Example:
///     >>> from kreuzberg import detect_mime_type
///     >>> mime = detect_mime_type("document.pdf")
///     >>> assert mime == "application/pdf"
#[pyfunction]
pub fn detect_mime_type(path: String) -> PyResult<String> {
    kreuzberg::detect_mime_type(&path, true).map_err(to_py_err)
}

/// Validate MIME type string.
///
/// Checks if a MIME type string is valid and supported by Kreuzberg.
///
/// Args:
///     mime_type: MIME type string to validate
///
/// Returns:
///     Validated MIME type string (normalized)
///
/// Raises:
///     ValueError: If MIME type is invalid or unsupported
///
/// Example:
///     >>> from kreuzberg import validate_mime_type
///     >>> mime = validate_mime_type("application/pdf")
///     >>> assert mime == "application/pdf"
#[pyfunction]
pub fn validate_mime_type(mime_type: String) -> PyResult<String> {
    kreuzberg::validate_mime_type(&mime_type).map_err(to_py_err)
}

/// Register MIME type constants in the Python module.
///
/// Adds all MIME type constants from the Rust core as module-level constants.
pub fn register_constants(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Document formats
    m.add("PDF_MIME_TYPE", kreuzberg::PDF_MIME_TYPE)?;
    m.add("DOCX_MIME_TYPE", kreuzberg::DOCX_MIME_TYPE)?;
    m.add("EXCEL_MIME_TYPE", kreuzberg::EXCEL_MIME_TYPE)?;
    m.add("POWER_POINT_MIME_TYPE", kreuzberg::POWER_POINT_MIME_TYPE)?;

    // Text formats
    m.add("PLAIN_TEXT_MIME_TYPE", kreuzberg::PLAIN_TEXT_MIME_TYPE)?;
    m.add("MARKDOWN_MIME_TYPE", kreuzberg::MARKDOWN_MIME_TYPE)?;
    m.add("HTML_MIME_TYPE", kreuzberg::HTML_MIME_TYPE)?;
    m.add("XML_MIME_TYPE", kreuzberg::XML_MIME_TYPE)?;

    // Data formats
    m.add("JSON_MIME_TYPE", kreuzberg::JSON_MIME_TYPE)?;

    Ok(())
}
