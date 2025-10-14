use pyo3::prelude::*;

use crate::error::to_py_err;
use crate::types::PyTextExtractionResult;

/// Extract text from plain text or markdown content.
///
/// Args:
///     text_bytes: The raw bytes of the text file
///     is_markdown: Whether to treat the content as markdown and extract metadata
///
/// Returns:
///     TextExtractionResult with content and optional metadata
///
/// Example:
///     >>> result = parse_text(b"# Hello\\nWorld", is_markdown=True)
///     >>> print(result.content)
///     # Hello
///     World
///     >>> print(result.headers)
///     ['Hello']
#[pyfunction]
pub fn parse_text(py: Python<'_>, text_bytes: &[u8], is_markdown: bool) -> PyResult<PyTextExtractionResult> {
    // Release GIL during heavy computation
    let result = py
        .detach(|| kreuzberg::extraction::parse_text(text_bytes, is_markdown))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyTextExtractionResult::from(result))
}
