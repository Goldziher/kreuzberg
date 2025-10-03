//! Shared test utilities for table processing tests

#![cfg(test)]

use super::tsv_parser::TSVWord;

/// Create a test TSVWord with specified position and text
///
/// All other fields use sensible defaults:
/// - level: 5 (word level)
/// - page_num: 1
/// - conf: 95.0
pub fn create_test_word(left: u32, top: u32, width: u32, height: u32, text: &str) -> TSVWord {
    TSVWord {
        level: 5,
        page_num: 1,
        block_num: 0,
        par_num: 0,
        line_num: 0,
        word_num: 0,
        left,
        top,
        width,
        height,
        conf: 95.0,
        text: text.to_string(),
    }
}
