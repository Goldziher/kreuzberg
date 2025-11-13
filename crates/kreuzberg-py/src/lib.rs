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

    m.add_class::<EmbeddingPreset>()?;
    m.add_function(wrap_pyfunction!(list_embedding_presets, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedding_preset, m)?)?;

    Ok(())
}

/// Embedding preset configuration.
///
/// Contains all settings for a specific embedding model preset including chunk size,
/// overlap, model name, embedding dimensions, and description.
///
/// Attributes:
///     name (str): Name of the preset (e.g., "fast", "balanced", "quality", "multilingual")
///     chunk_size (int): Recommended chunk size in characters
///     overlap (int): Recommended overlap in characters
///     model_name (str): Model identifier
///     dimensions (int): Embedding vector dimensions
///     description (str): Human-readable description of the preset
///
/// Example:
///     >>> from kreuzberg import get_embedding_preset
///     >>> preset = get_embedding_preset("balanced")
///     >>> print(f"Model: {preset.model_name}, Dims: {preset.dimensions}")
///     Model: BGEBaseENV15, Dims: 768
#[pyclass(name = "EmbeddingPreset", module = "kreuzberg")]
#[derive(Clone)]
pub struct EmbeddingPreset {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub chunk_size: usize,
    #[pyo3(get)]
    pub overlap: usize,
    #[pyo3(get)]
    pub model_name: String,
    #[pyo3(get)]
    pub dimensions: usize,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl EmbeddingPreset {
    fn __repr__(&self) -> String {
        format!(
            "EmbeddingPreset(name='{}', chunk_size={}, overlap={}, model_name='{}', dimensions={}, description='{}')",
            self.name, self.chunk_size, self.overlap, self.model_name, self.dimensions, self.description
        )
    }
}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with get_embedding_preset.
/// Available presets: "fast", "balanced", "quality", "multilingual".
///
/// Returns:
///     list[str]: List of 4 preset names
///
/// Example:
///     >>> from kreuzberg import list_embedding_presets
///     >>> presets = list_embedding_presets()
///     >>> print(presets)
///     ['fast', 'balanced', 'quality', 'multilingual']
#[pyfunction]
fn list_embedding_presets() -> Vec<String> {
    kreuzberg::embeddings::list_presets()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration object, or None if the preset name is not found.
///
/// Args:
///     name (str): The preset name (case-sensitive)
///
/// Returns:
///     EmbeddingPreset | None: Preset configuration or None if not found
///
/// Available presets:
///     - "fast": AllMiniLML6V2Q (384 dimensions) - Quick prototyping, low-latency
///     - "balanced": BGEBaseENV15 (768 dimensions) - General-purpose RAG
///     - "quality": BGELargeENV15 (1024 dimensions) - High-quality embeddings
///     - "multilingual": MultilingualE5Base (768 dimensions) - Multi-language support
///
/// Example:
///     >>> from kreuzberg import get_embedding_preset
///     >>> preset = get_embedding_preset("balanced")
///     >>> if preset:
///     ...     print(f"Model: {preset.model_name}, Dims: {preset.dimensions}")
///     ...     # Model: BGEBaseENV15, Dims: 768
#[pyfunction]
fn get_embedding_preset(name: String) -> Option<EmbeddingPreset> {
    let preset = kreuzberg::embeddings::get_preset(&name)?;

    // Note: When embeddings feature is enabled in kreuzberg, the model field is EmbeddingModel
    // When not enabled, it's model_name: &'static str. Since kreuzberg-py typically builds
    // with all features, we use the model field and format it.
    let model_name = format!("{:?}", preset.model);

    Some(EmbeddingPreset {
        name: preset.name.to_string(),
        chunk_size: preset.chunk_size,
        overlap: preset.overlap,
        model_name,
        dimensions: preset.dimensions,
        description: preset.description.to_string(),
    })
}
