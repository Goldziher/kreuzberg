use pyo3::prelude::*;

mod cache;
mod common;
mod error_utils;
mod excel;
mod image_preprocessing;
mod quality;
mod string_utils;
mod table_processing;
mod token_reduction;

use cache::{
    batch_cleanup_caches, batch_generate_cache_keys, cleanup_cache, clear_cache_directory, fast_hash,
    filter_old_cache_entries, generate_cache_key, get_available_disk_space, get_cache_metadata, is_cache_valid,
    smart_cleanup_cache, sort_cache_by_access_time, validate_cache_key, CacheStats,
};
use excel::{benchmark_excel_reading, excel_to_markdown, read_excel_bytes, read_excel_file, ExcelSheet, ExcelWorkbook};
use image_preprocessing::{batch_normalize_images, normalize_image_dpi, ExtractionConfig, ImagePreprocessingMetadata};
use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};
use string_utils::{batch_process_texts, calculate_text_confidence, fix_mojibake, get_encoding_cache_key, safe_decode};
use table_processing::table_from_arrow_to_markdown;
use token_reduction::{
    batch_reduce_tokens, get_reduction_statistics, reduce_tokens, ReductionLevel, TokenReductionConfig,
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

    m.add_function(wrap_pyfunction!(normalize_image_dpi, m)?)?;
    m.add_function(wrap_pyfunction!(batch_normalize_images, m)?)?;
    m.add_class::<ImagePreprocessingMetadata>()?;
    m.add_class::<ExtractionConfig>()?;

    m.add_function(wrap_pyfunction!(reduce_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(batch_reduce_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(get_reduction_statistics, m)?)?;
    m.add_class::<TokenReductionConfig>()?;
    m.add_class::<ReductionLevel>()?;

    // Cache functions
    m.add_function(wrap_pyfunction!(generate_cache_key, m)?)?;
    m.add_function(wrap_pyfunction!(batch_generate_cache_keys, m)?)?;
    m.add_function(wrap_pyfunction!(fast_hash, m)?)?;
    m.add_function(wrap_pyfunction!(validate_cache_key, m)?)?;
    m.add_function(wrap_pyfunction!(filter_old_cache_entries, m)?)?;
    m.add_function(wrap_pyfunction!(sort_cache_by_access_time, m)?)?;
    m.add_function(wrap_pyfunction!(get_available_disk_space, m)?)?;
    m.add_function(wrap_pyfunction!(get_cache_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(cleanup_cache, m)?)?;
    m.add_function(wrap_pyfunction!(smart_cleanup_cache, m)?)?;
    m.add_function(wrap_pyfunction!(is_cache_valid, m)?)?;
    m.add_function(wrap_pyfunction!(clear_cache_directory, m)?)?;
    m.add_function(wrap_pyfunction!(batch_cleanup_caches, m)?)?;
    m.add_class::<CacheStats>()?;

    // Table processing via Arrow IPC bridge
    m.add_function(wrap_pyfunction!(table_from_arrow_to_markdown, m)?)?;

    // Excel processing with Calamine
    m.add_function(wrap_pyfunction!(read_excel_file, m)?)?;
    m.add_function(wrap_pyfunction!(read_excel_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(excel_to_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark_excel_reading, m)?)?;
    m.add_class::<ExcelWorkbook>()?;
    m.add_class::<ExcelSheet>()?;

    Ok(())
}
