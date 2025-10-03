//! Tesseract OCR Rust implementation
//!
//! This module provides a high-performance Rust implementation of Tesseract OCR
//! functionality, replacing the Python implementation for improved speed and
//! reduced memory usage.
//!
//! ## Performance Targets
//! - 2x faster than Python implementation
//! - ≤70% memory usage of Python
//! - Match Python quality (±2%)

pub mod cache;
pub mod error;
pub mod hocr;
pub mod processor;
pub mod table;
pub mod types;
pub mod utils;
pub mod validation;

pub use cache::OCRCacheStats;
pub use processor::OCRProcessor;
pub use types::{BatchItemResult, ExtractionResultDTO, PSMMode, TableDTO, TesseractConfigDTO};
pub use validation::{validate_language_code, validate_tesseract_version};
