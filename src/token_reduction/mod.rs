//! Modern token reduction implementation using 2025 state-of-the-art techniques.
//!
//! This module provides semantic-aware token reduction that goes far beyond traditional
//! stopword removal, utilizing SIMD optimization, parallel processing, and modern NLP
//! approaches for maximum performance and semantic preservation.

mod config;
mod core;
mod filters;
mod semantic;
mod simd_text;

pub use config::{ReductionLevel, TokenReductionConfig};
pub use core::TokenReducer;

use pyo3::prelude::*;

/// Python bindings for the modern token reduction system
#[pyfunction]
#[pyo3(signature = (text, config, language_hint=None))]
pub fn reduce_tokens_rust(text: &str, config: &TokenReductionConfig, language_hint: Option<&str>) -> PyResult<String> {
    let reducer = TokenReducer::new(config, language_hint)?;
    Ok(reducer.reduce(text))
}

/// Batch processing for multiple texts
#[pyfunction]
#[pyo3(signature = (texts, config, language_hint=None))]
pub fn batch_reduce_tokens_rust(
    texts: Vec<String>,
    config: &TokenReductionConfig,
    language_hint: Option<&str>,
) -> PyResult<Vec<String>> {
    let reducer = TokenReducer::new(config, language_hint)?;
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    Ok(reducer.batch_reduce(&text_refs))
}

/// Get detailed reduction statistics
#[pyfunction]
pub fn get_reduction_statistics_rust(
    original: &str,
    reduced: &str,
) -> PyResult<(f64, f64, usize, usize, usize, usize)> {
    let original_chars = original.chars().count();
    let reduced_chars = reduced.chars().count();
    let original_tokens = original.split_whitespace().count();
    let reduced_tokens = reduced.split_whitespace().count();

    let char_reduction = if original_chars > 0 {
        1.0 - (reduced_chars as f64 / original_chars as f64)
    } else {
        0.0
    };

    let token_reduction = if original_tokens > 0 {
        1.0 - (reduced_tokens as f64 / original_tokens as f64)
    } else {
        0.0
    };

    Ok((
        char_reduction,
        token_reduction,
        original_chars,
        reduced_chars,
        original_tokens,
        reduced_tokens,
    ))
}
