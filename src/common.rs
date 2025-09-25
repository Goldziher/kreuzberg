//! Common utilities shared across modules

use regex::Regex;
use std::borrow::Cow;

/// Apply a regex replacement with a closure only if the pattern matches
#[inline]
pub fn replace_with_if_matches<'a, F>(text: &'a str, pattern: &Regex, replacer: F) -> Cow<'a, str>
where
    F: FnMut(&regex::Captures) -> String,
{
    if pattern.is_match(text) {
        Cow::Owned(pattern.replace_all(text, replacer).into_owned())
    } else {
        Cow::Borrowed(text)
    }
}

/// Calculate the total length of all matches for a pattern
#[inline]
pub fn sum_match_lengths(text: &str, pattern: &Regex) -> usize {
    pattern.find_iter(text).map(|m| m.len()).sum()
}

/// Chain multiple regex replacements efficiently
pub fn chain_replacements<'a>(mut text: Cow<'a, str>, replacements: &[(&Regex, &str)]) -> Cow<'a, str> {
    for (pattern, replacement) in replacements {
        if pattern.is_match(&text) {
            text = Cow::Owned(pattern.replace_all(&text, *replacement).into_owned());
        }
    }
    text
}

/// Constants for quality scoring
pub mod quality_weights {
    pub const OCR_PENALTY_WEIGHT: f64 = 0.3;
    pub const SCRIPT_PENALTY_WEIGHT: f64 = 0.2;
    pub const NAV_PENALTY_WEIGHT: f64 = 0.1;
    pub const STRUCTURE_BONUS_WEIGHT: f64 = 0.2;
    pub const METADATA_BONUS_WEIGHT: f64 = 0.1;
}

/// Text structure thresholds
pub mod text_thresholds {
    pub const MIN_TEXT_LENGTH: usize = 10;
    pub const LARGE_TEXT_LENGTH: usize = 1000;
    pub const MIN_SENTENCE_WORDS: f64 = 10.0;
    pub const MAX_SENTENCE_WORDS: f64 = 30.0;
    pub const MIN_PARAGRAPH_WORDS: f64 = 50.0;
    pub const MAX_PARAGRAPH_WORDS: f64 = 300.0;
}
