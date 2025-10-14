use pyo3::prelude::*;

use crate::error::to_py_err;
use crate::types::PyHtmlExtractionResult;

#[pyfunction]
pub fn convert_html_to_markdown(py: Python<'_>, html: &str, config_json: Option<&str>) -> PyResult<String> {
    // Deserialize config if provided
    let config = if let Some(cfg_json) = config_json {
        Some(
            serde_json::from_str::<kreuzberg::extraction::html::HtmlConversionConfig>(cfg_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?,
        )
    } else {
        None
    };

    // Release GIL during computation
    py.detach(|| kreuzberg::extraction::convert_html_to_markdown(html, config))
        .map_err(to_py_err)
}

#[pyfunction]
pub fn process_html(
    py: Python<'_>,
    html: &str,
    config_json: Option<&str>,
    extract_images: bool,
    max_image_size: u64,
) -> PyResult<PyHtmlExtractionResult> {
    // Deserialize config if provided
    let config = if let Some(cfg_json) = config_json {
        Some(
            serde_json::from_str::<kreuzberg::extraction::html::HtmlConversionConfig>(cfg_json)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?,
        )
    } else {
        None
    };

    // Release GIL during computation
    let result = py
        .detach(|| kreuzberg::extraction::process_html(html, config, extract_images, max_image_size))
        .map_err(to_py_err)?;

    // Convert Rust type directly to Python type (zero serialization overhead)
    Ok(PyHtmlExtractionResult::from(result))
}
