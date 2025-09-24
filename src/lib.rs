use pyo3::prelude::*;

mod quality;
mod string_utils;

use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};
use string_utils::{batch_process_texts, calculate_text_confidence, fix_mojibake, get_encoding_cache_key, safe_decode};

/// Internal Rust bindings for kreuzberg - not for direct use
#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(calculate_quality_score, m)?)?;
    m.add_function(wrap_pyfunction!(clean_extracted_text, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_spaces, m)?)?;

    m.add_function(wrap_pyfunction!(safe_decode, m)?)?;
    m.add_function(wrap_pyfunction!(batch_process_texts, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_text_confidence, m)?)?;
    m.add_function(wrap_pyfunction!(fix_mojibake, m)?)?;
    m.add_function(wrap_pyfunction!(get_encoding_cache_key, m)?)?;

    Ok(())
}
