use pyo3::prelude::*;

mod quality;
mod string_utils;

use quality::{calculate_quality_score_rust, clean_extracted_text_rust, normalize_spaces_rust};
use string_utils::{batch_process_texts_rust, safe_decode_rust};

/// Fast text processing utilities for kreuzberg
#[pymodule]
fn kreuzberg_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Quality utilities
    m.add_function(wrap_pyfunction!(calculate_quality_score_rust, m)?)?;
    m.add_function(wrap_pyfunction!(clean_extracted_text_rust, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_spaces_rust, m)?)?;

    // String utilities
    m.add_function(wrap_pyfunction!(safe_decode_rust, m)?)?;
    m.add_function(wrap_pyfunction!(batch_process_texts_rust, m)?)?;

    Ok(())
}
