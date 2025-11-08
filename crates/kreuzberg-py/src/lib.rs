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

use once_cell::sync::OnceCell;
use pyo3::prelude::*;
use pyo3_async_runtimes::TaskLocals;

mod config;
mod core;
mod error;
mod plugins;
mod types;

/// Global Python event loop task locals for async handlers
/// Initialized once at startup to avoid ~55µs overhead per call
static TASK_LOCALS: OnceCell<TaskLocals> = OnceCell::new();

/// Initialize Python event loop for async plugin callbacks
///
/// This should be called once after Python initialization to set up
/// the event loop that will be used for all async Python plugin calls.
/// Avoids ~55µs overhead of creating event loops per call.
///
/// Based on spikard's high-performance async patterns.
#[pyfunction]
fn init_async_runtime() -> PyResult<()> {
    Python::attach(|py| {
        let asyncio = py.import("asyncio")?;
        let event_loop = asyncio.call_method0("new_event_loop")?;
        asyncio.call_method1("set_event_loop", (event_loop.clone(),))?;

        TASK_LOCALS.get_or_init(|| TaskLocals::new(event_loop));

        Ok(())
    })
}

/// Internal bindings module for Kreuzberg
#[pymodule]
fn _internal_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ValidationError", m.py().get_type::<error::ValidationError>())?;
    m.add("ParsingError", m.py().get_type::<error::ParsingError>())?;
    m.add("OCRError", m.py().get_type::<error::OCRError>())?;
    m.add(
        "MissingDependencyError",
        m.py().get_type::<error::MissingDependencyError>(),
    )?;

    m.add_class::<config::ExtractionConfig>()?;
    m.add_class::<config::OcrConfig>()?;
    m.add_class::<config::PdfConfig>()?;
    m.add_class::<config::ChunkingConfig>()?;
    m.add_class::<config::EmbeddingConfig>()?;
    m.add_class::<config::EmbeddingModelType>()?;
    m.add_class::<config::LanguageDetectionConfig>()?;
    m.add_class::<config::TokenReductionConfig>()?;
    m.add_class::<config::ImageExtractionConfig>()?;
    m.add_class::<config::PostProcessorConfig>()?;
    m.add_class::<config::TesseractConfig>()?;
    m.add_class::<config::ImagePreprocessingConfig>()?;

    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    {
        m.add_class::<config::KeywordAlgorithm>()?;
        m.add_class::<config::KeywordConfig>()?;
    }
    #[cfg(feature = "keywords-yake")]
    m.add_class::<config::YakeParams>()?;
    #[cfg(feature = "keywords-rake")]
    m.add_class::<config::RakeParams>()?;

    m.add_class::<types::ExtractionResult>()?;
    m.add_class::<types::ExtractedTable>()?;

    m.add_function(wrap_pyfunction!(core::extract_file_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_bytes_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_files_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_bytes_sync, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_file, m)?)?;
    m.add_function(wrap_pyfunction!(core::extract_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_files, m)?)?;
    m.add_function(wrap_pyfunction!(core::batch_extract_bytes, m)?)?;

    m.add_function(wrap_pyfunction!(plugins::register_ocr_backend, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::register_post_processor, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::unregister_post_processor, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::clear_post_processors, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::register_validator, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::unregister_validator, m)?)?;
    m.add_function(wrap_pyfunction!(plugins::clear_validators, m)?)?;

    m.add_function(wrap_pyfunction!(init_async_runtime, m)?)?;

    Ok(())
}
