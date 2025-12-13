//! Type conversions and utilities for WASM bindings
//!
//! This module provides type conversions between Rust and JavaScript/TypeScript types
//! for seamless interoperability. Includes helpers for configuration and result handling.

use kreuzberg::{ExtractionConfig, ExtractionResult};
use wasm_bindgen::prelude::*;

/// Parse extraction configuration from JsValue using serde-wasm-bindgen.
///
/// Converts a JavaScript object to a Rust ExtractionConfig structure.
/// If config is None, returns the default ExtractionConfig.
///
/// # Arguments
///
/// * `config` - JavaScript object with extraction configuration (optional)
///
/// # Returns
///
/// Result containing the parsed ExtractionConfig or a JsValue error
pub fn parse_config(config: Option<JsValue>) -> Result<ExtractionConfig, JsValue> {
    match config {
        Some(js_config) => serde_wasm_bindgen::from_value(js_config)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e))),
        None => Ok(ExtractionConfig::default()),
    }
}

/// Convert extraction result to JsValue for JavaScript consumption.
///
/// Serializes the Rust ExtractionResult to a JavaScript object.
///
/// # Arguments
///
/// * `result` - The ExtractionResult to convert
///
/// # Returns
///
/// Result containing the JsValue or a JsValue error
pub fn result_to_js_value(result: &ExtractionResult) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(result).map_err(|e| JsValue::from_str(&format!("Failed to convert result: {}", e)))
}

/// Convert a vector of results to JsValue.
///
/// Serializes multiple ExtractionResults to a JavaScript array.
///
/// # Arguments
///
/// * `results` - Vector of ExtractionResults
///
/// # Returns
///
/// Result containing the JsValue array or an error
pub fn results_to_js_value(results: &[ExtractionResult]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(results).map_err(|e| JsValue::from_str(&format!("Failed to convert results: {}", e)))
}
