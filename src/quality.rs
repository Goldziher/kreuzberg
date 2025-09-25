use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use regex::Regex;
use std::borrow::Cow;

use crate::common::quality_weights::*;
use crate::common::text_thresholds::*;
use crate::common::{chain_replacements, replace_with_if_matches, sum_match_lengths};

static SCATTERED_CHARS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b").unwrap());
static REPEATED_PUNCT_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.]{3,}|[-]{3,}|[_]{3,}").unwrap());
static ISOLATED_PUNCT_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s[.,;:!?]\s").unwrap());
static MALFORMED_WORDS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b").unwrap());
static EXCESSIVE_WHITESPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{3,}").unwrap());

static JS_FUNCTION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)function\s+\w+\s*\([^)]*\)\s*\{[^}]*\}").unwrap());
static CSS_RULES_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\.[a-zA-Z][\w-]*\s*\{[^}]*\}").unwrap());
static SCRIPT_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
static STYLE_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());

static NAV_WORDS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:Skip to main content|Back to top|Main navigation|Site navigation)\b").unwrap());
static BREADCRUMB_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:Home\s*[>»]\s*|[>»]\s*){2,}").unwrap());
static PAGINATION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:Page \d+ of \d+|First page|Last page|Previous page|Next page|^\d+ of \d+$)\b").unwrap()
});

static SENTENCE_DETECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]\s+[A-Z]").unwrap());
static PUNCTUATION_DETECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]").unwrap());

static WHITESPACE_NORMALIZE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[ \t\f\v\r\xa0\u{2000}-\u{200b}\u{2028}\u{2029}\u{3000}]+").unwrap());
static NEWLINE_NORMALIZE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n\s*\n\s*\n+").unwrap());
static NEWLINE_CLEANUP: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n+").unwrap());

/// Calculate quality score for extracted text
#[pyfunction]
#[pyo3(signature = (text, metadata=None))]
pub fn calculate_quality_score(text: &str, metadata: Option<Bound<PyDict>>) -> f64 {
    if text.is_empty() || text.trim().is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;

    if (text.len()) < MIN_TEXT_LENGTH {
        return 0.1;
    }

    let mut score = 1.0;

    if text.len() > LARGE_TEXT_LENGTH {
        let ocr_penalty = calculate_ocr_penalty(text, total_chars);
        let script_penalty = calculate_script_penalty(text, total_chars);
        let nav_penalty = calculate_navigation_penalty(text, total_chars);
        let structure_bonus = calculate_structure_bonus(text);

        score -= ocr_penalty * OCR_PENALTY_WEIGHT;
        score -= script_penalty * SCRIPT_PENALTY_WEIGHT;
        score -= nav_penalty * NAV_PENALTY_WEIGHT;
        score += structure_bonus * STRUCTURE_BONUS_WEIGHT;
    } else {
        score -= calculate_ocr_penalty(text, total_chars) * OCR_PENALTY_WEIGHT;
        score += calculate_structure_bonus(text) * STRUCTURE_BONUS_WEIGHT;
    }

    if let Some(metadata) = metadata {
        score += calculate_metadata_bonus(&metadata) * METADATA_BONUS_WEIGHT;
    }

    score.clamp(0.0, 1.0)
}

#[inline]
fn calculate_ocr_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    if !text.contains("  ") && !text.contains("...") {
        return 0.0;
    }

    let artifact_chars = sum_match_lengths(text, &SCATTERED_CHARS_PATTERN)
        + sum_match_lengths(text, &REPEATED_PUNCT_PATTERN)
        + sum_match_lengths(text, &ISOLATED_PUNCT_PATTERN)
        + sum_match_lengths(text, &MALFORMED_WORDS_PATTERN)
        + sum_match_lengths(text, &EXCESSIVE_WHITESPACE_PATTERN);

    (artifact_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_script_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    if !text.contains("function") && !text.contains("<script") && !text.contains("<style") {
        return 0.0;
    }

    let script_chars = sum_match_lengths(text, &JS_FUNCTION_PATTERN)
        + sum_match_lengths(text, &CSS_RULES_PATTERN)
        + sum_match_lengths(text, &SCRIPT_TAG_PATTERN)
        + sum_match_lengths(text, &STYLE_TAG_PATTERN);

    (script_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_navigation_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    let nav_chars = sum_match_lengths(text, &NAV_WORDS_PATTERN)
        + sum_match_lengths(text, &BREADCRUMB_PATTERN)
        + sum_match_lengths(text, &PAGINATION_PATTERN);

    (nav_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_structure_bonus(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let sentence_count = SENTENCE_DETECT.find_iter(text).count() as f64;
    let paragraph_count = text.matches("\n\n").count() as f64 + 1.0;
    let words = text.split_whitespace().count() as f64;

    if words == 0.0 {
        return 0.0;
    }

    let avg_words_per_sentence = words / sentence_count.max(1.0);
    let avg_words_per_paragraph = words / paragraph_count;

    let mut structure_score: f64 = 0.0;

    if (MIN_SENTENCE_WORDS..=MAX_SENTENCE_WORDS).contains(&avg_words_per_sentence) {
        structure_score += 0.3;
    }

    if (MIN_PARAGRAPH_WORDS..=MAX_PARAGRAPH_WORDS).contains(&avg_words_per_paragraph) {
        structure_score += 0.3;
    }

    if paragraph_count > 1.0 {
        structure_score += 0.2;
    }

    if PUNCTUATION_DETECT.is_match(text) {
        structure_score += 0.2;
    }

    structure_score.min(1.0)
}

#[inline]
fn calculate_metadata_bonus(metadata: &Bound<PyDict>) -> f64 {
    const IMPORTANT_FIELDS: &[&str] = &["title", "author", "subject", "description", "keywords"];

    let present_fields = IMPORTANT_FIELDS
        .iter()
        .filter(|&&field| metadata.contains(field).unwrap_or(false))
        .count();

    present_fields as f64 / IMPORTANT_FIELDS.len() as f64
}

/// Clean extracted text by removing artifacts and unwanted content
#[pyfunction]
pub fn clean_extracted_text(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let script_replacements = [
        (&*SCRIPT_TAG_PATTERN, " "),
        (&*STYLE_TAG_PATTERN, " "),
        (&*JS_FUNCTION_PATTERN, " "),
        (&*CSS_RULES_PATTERN, " "),
    ];

    let result = chain_replacements(Cow::Borrowed(text), &script_replacements);

    let mut owned = result.into_owned();
    owned = clean_ocr_artifacts(&owned);

    owned = clean_navigation_elements(&owned);

    owned = WHITESPACE_NORMALIZE.replace_all(&owned, " ").into_owned();
    owned = NEWLINE_NORMALIZE.replace_all(&owned, "\n\n").into_owned();

    owned.trim().to_string()
}

#[inline]
fn clean_ocr_artifacts(text: &str) -> String {
    let result = replace_with_if_matches(text, &SCATTERED_CHARS_PATTERN, |caps: &regex::Captures| {
        caps[0].chars().filter(|c| !c.is_whitespace()).collect::<String>()
    });

    let ocr_replacements = [
        (&*REPEATED_PUNCT_PATTERN, "..."),
        (&*ISOLATED_PUNCT_PATTERN, " "),
        (&*MALFORMED_WORDS_PATTERN, " "),
        (&*EXCESSIVE_WHITESPACE_PATTERN, " "),
    ];

    chain_replacements(result, &ocr_replacements).into_owned()
}

#[inline]
fn clean_navigation_elements(text: &str) -> String {
    let nav_replacements = [
        (&*NAV_WORDS_PATTERN, " "),
        (&*BREADCRUMB_PATTERN, " "),
        (&*PAGINATION_PATTERN, " "),
    ];

    chain_replacements(Cow::Borrowed(text), &nav_replacements).into_owned()
}

/// Normalize spaces in text
#[pyfunction]
pub fn normalize_spaces(text: &str) -> String {
    if text.is_empty() || text.trim().is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(text.len());

    let mut first = true;
    for paragraph in text.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !first {
            result.push_str("\n\n");
        }
        first = false;

        let cleaned = WHITESPACE_NORMALIZE.replace_all(paragraph, " ");
        let cleaned = NEWLINE_CLEANUP.replace_all(&cleaned, "\n");

        let mut first_line = true;
        for line in cleaned.split('\n') {
            let line = line.trim();
            if !line.is_empty() {
                if !first_line {
                    result.push('\n');
                }
                result.push_str(line);
                first_line = false;
            }
        }
    }

    result
}
