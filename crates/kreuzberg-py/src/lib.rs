//! Kreuzberg PyO3 Bindings v4
//!
//! This module exposes the Rust core extraction API to Python with both
//! synchronous and asynchronous variants.
//!
//! # Architecture
//!
//! - All extraction logic is in the Rust core (crates/kreuzberg)
//! - Python is a thin wrapper that adds language-specific features
//! - Zero duplication of core functionality
//! - Modern PyO3 0.26 patterns throughout

#![deny(unsafe_code)]

use pyo3::prelude::*;

mod config;
mod core;
mod error;
mod plugins;
mod types;

/// Internal bindings module for Kreuzberg
#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Configuration types (10 types)
    m.add_class::<config::ExtractionConfig>()?;
    m.add_class::<config::OcrConfig>()?;
    m.add_class::<config::PdfConfig>()?;
    m.add_class::<config::ChunkingConfig>()?;
    m.add_class::<config::LanguageDetectionConfig>()?;
    m.add_class::<config::TokenReductionConfig>()?;
    m.add_class::<config::ImageExtractionConfig>()?;
    m.add_class::<config::PostProcessorConfig>()?;
    m.add_class::<config::TesseractConfig>()?;
    m.add_class::<config::ImagePreprocessingConfig>()?;

    // Result types (2 types)
    m.add_class::<types::ExtractionResult>()?;
    m.add_class::<types::ExtractedTable>()?;

    // Extraction functions (8 functions: 4 sync + 4 async)
    m.add_function(wrap_pyfunction!(core::extract_file_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_bytes_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_files_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_bytes_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_file, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_files, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_bytes, m)?)?;

    // Plugin registration functions (2 functions)
    m.add_function(wrap_pyfunction!(plugins::register_ocr_backend, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::register_post_processor, m)?)?;

    Ok(())
}
