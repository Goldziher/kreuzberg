//! Text utility functions for quality processing and string manipulation.
//!
//! This module provides:
//! - Quality processing: clean OCR artifacts, calculate quality scores
//! - String utilities: safe decoding, mojibake fixing, encoding detection

#[cfg(feature = "quality")]
pub mod quality;

#[cfg(feature = "quality")]
pub mod string_utils;

#[cfg(feature = "quality")]
pub use quality::{calculate_quality_score, clean_extracted_text, normalize_spaces};

#[cfg(feature = "quality")]
pub use string_utils::{calculate_text_confidence, fix_mojibake, safe_decode};
