use pyo3::prelude::*;

mod image_preprocessing;
mod quality;
mod string_utils;
mod token_reduction;

use image_preprocessing::{
    batch_normalize_images_rust, normalize_image_dpi_rust, ExtractionConfig, ImagePreprocessingMetadata,
};
use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};
use string_utils::{batch_process_texts, calculate_text_confidence, fix_mojibake, get_encoding_cache_key, safe_decode};
use token_reduction::{
    batch_reduce_tokens_rust, get_reduction_statistics_rust, reduce_tokens_rust, ReductionLevel, TokenReductionConfig,
};

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

    m.add_function(wrap_pyfunction!(normalize_image_dpi_rust, m)?)?;
    m.add_function(wrap_pyfunction!(batch_normalize_images_rust, m)?)?;
    m.add_class::<ImagePreprocessingMetadata>()?;
    m.add_class::<ExtractionConfig>()?;

    m.add_function(wrap_pyfunction!(reduce_tokens_rust, m)?)?;
    m.add_function(wrap_pyfunction!(batch_reduce_tokens_rust, m)?)?;
    m.add_function(wrap_pyfunction!(get_reduction_statistics_rust, m)?)?;
    m.add_class::<TokenReductionConfig>()?;
    m.add_class::<ReductionLevel>()?;

    Ok(())
}
