//! Table detection and extraction from OCR output
//!
//! This module provides functionality to detect and extract tables from
//! Tesseract TSV output, reconstruct table structure, and generate markdown tables.

pub mod detection;
pub mod markdown;
pub mod reconstruction;
pub mod tsv_parser;

#[cfg(test)]
mod test_helpers;

pub use markdown::table_to_markdown;
pub use reconstruction::reconstruct_table;
pub use tsv_parser::extract_words;
