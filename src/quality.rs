use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;

// OCR artifact patterns
static SCATTERED_CHARS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b").unwrap());
static REPEATED_PUNCT_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.]{3,}|[-]{3,}|[_]{3,}").unwrap());
static ISOLATED_PUNCT_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s[.,;:!?]\s").unwrap());
static MALFORMED_WORDS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b").unwrap());
static EXCESSIVE_WHITESPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{3,}").unwrap());

// Script patterns for detection and cleaning
static JS_FUNCTION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)function\s+\w+\s*\([^)]*\)\s*\{[^}]*\}").unwrap());
static CSS_RULES_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\.[a-zA-Z][\w-]*\s*\{[^}]*\}").unwrap());
static SCRIPT_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
static STYLE_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());

// Navigation patterns
static NAV_WORDS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:Skip to main content|Back to top|Main navigation|Site navigation)\b").unwrap());
static BREADCRUMB_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:Home\s*[>»]\s*|[>»]\s*){2,}").unwrap());
static PAGINATION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:Page \d+ of \d+|First page|Last page|Previous page|Next page|^\d+ of \d+$)\b").unwrap()
});

// Text structure detection
static SENTENCE_DETECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]\s+[A-Z]").unwrap());
static PUNCTUATION_DETECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?]").unwrap());

// Whitespace normalization
static WHITESPACE_NORMALIZE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[ \t\f\v\r\xa0\u{2000}-\u{200b}\u{2028}\u{2029}\u{3000}]+").unwrap());
static NEWLINE_NORMALIZE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n\s*\n\s*\n+").unwrap());
static NEWLINE_CLEANUP: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n+").unwrap());

/// Calculate quality score for extracted text
#[pyfunction]
#[pyo3(signature = (text, metadata=None))]
pub fn calculate_quality_score_rust(text: &str, metadata: Option<HashMap<String, String>>) -> f64 {
    if text.is_empty() || text.trim().is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;

    // Early return for very small texts
    if total_chars < 10.0 {
        return 0.1;
    }

    let mut score = 1.0;

    // Calculate penalties and bonuses in parallel if text is large enough
    if total_chars > 1000.0 {
        let ocr_penalty = calculate_ocr_penalty(text, total_chars);
        let script_penalty = calculate_script_penalty(text, total_chars);
        let nav_penalty = calculate_navigation_penalty(text, total_chars);
        let structure_bonus = calculate_structure_bonus(text);

        score -= ocr_penalty * 0.3;
        score -= script_penalty * 0.2;
        score -= nav_penalty * 0.1;
        score += structure_bonus * 0.2;
    } else {
        // For smaller texts, skip some expensive operations
        score -= calculate_ocr_penalty(text, total_chars) * 0.3;
        score += calculate_structure_bonus(text) * 0.2;
    }

    // Calculate metadata bonus if provided
    if let Some(metadata) = metadata {
        score += calculate_metadata_bonus(&metadata) * 0.1;
    }

    score.clamp(0.0, 1.0)
}

#[inline]
fn calculate_ocr_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    // Early exit if text looks clean (common case)
    if !text.contains("  ") && !text.contains("...") {
        return 0.0;
    }

    let artifact_chars = SCATTERED_CHARS_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + REPEATED_PUNCT_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + ISOLATED_PUNCT_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + MALFORMED_WORDS_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + EXCESSIVE_WHITESPACE_PATTERN
            .find_iter(text)
            .map(|m| m.len())
            .sum::<usize>();

    (artifact_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_script_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    // Early exit if no script indicators
    if !text.contains("function") && !text.contains("<script") && !text.contains("<style") {
        return 0.0;
    }

    let script_chars = JS_FUNCTION_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + CSS_RULES_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + SCRIPT_TAG_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + STYLE_TAG_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>();

    (script_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn calculate_navigation_penalty(text: &str, total_chars: f64) -> f64 {
    if total_chars == 0.0 {
        return 0.0;
    }

    let nav_chars = NAV_WORDS_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + BREADCRUMB_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>()
        + PAGINATION_PATTERN.find_iter(text).map(|m| m.len()).sum::<usize>();

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

    if (10.0..=30.0).contains(&avg_words_per_sentence) {
        structure_score += 0.3;
    }

    if (50.0..=300.0).contains(&avg_words_per_paragraph) {
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
fn calculate_metadata_bonus(metadata: &HashMap<String, String>) -> f64 {
    const IMPORTANT_FIELDS: &[&str] = &["title", "author", "subject", "description", "keywords"];

    let present_fields = IMPORTANT_FIELDS
        .iter()
        .filter(|&&field| metadata.contains_key(field))
        .count();

    present_fields as f64 / IMPORTANT_FIELDS.len() as f64
}

/// Clean extracted text by removing artifacts and unwanted content
#[pyfunction]
pub fn clean_extracted_text_rust(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Use Cow to avoid unnecessary allocations
    let mut result = Cow::Borrowed(text);

    // Only allocate if we find patterns to replace
    if SCRIPT_TAG_PATTERN.is_match(&result) {
        result = Cow::Owned(SCRIPT_TAG_PATTERN.replace_all(&result, " ").into_owned());
    }
    if STYLE_TAG_PATTERN.is_match(&result) {
        result = Cow::Owned(STYLE_TAG_PATTERN.replace_all(&result, " ").into_owned());
    }
    if JS_FUNCTION_PATTERN.is_match(&result) {
        result = Cow::Owned(JS_FUNCTION_PATTERN.replace_all(&result, " ").into_owned());
    }
    if CSS_RULES_PATTERN.is_match(&result) {
        result = Cow::Owned(CSS_RULES_PATTERN.replace_all(&result, " ").into_owned());
    }

    // Clean OCR artifacts
    let mut owned = result.into_owned();
    owned = clean_ocr_artifacts(&owned);

    // Clean navigation elements
    owned = clean_navigation_elements(&owned);

    // Normalize whitespace
    owned = WHITESPACE_NORMALIZE.replace_all(&owned, " ").into_owned();
    owned = NEWLINE_NORMALIZE.replace_all(&owned, "\n\n").into_owned();

    owned.trim().to_string()
}

#[inline]
fn clean_ocr_artifacts(text: &str) -> String {
    let mut result = Cow::Borrowed(text);

    // Only process if we find artifacts
    if SCATTERED_CHARS_PATTERN.is_match(&result) {
        result = Cow::Owned(
            SCATTERED_CHARS_PATTERN
                .replace_all(&result, |caps: &regex::Captures| {
                    caps[0].chars().filter(|c| !c.is_whitespace()).collect::<String>()
                })
                .into_owned(),
        );
    }

    if REPEATED_PUNCT_PATTERN.is_match(&result) {
        result = Cow::Owned(REPEATED_PUNCT_PATTERN.replace_all(&result, "...").into_owned());
    }

    if ISOLATED_PUNCT_PATTERN.is_match(&result) {
        result = Cow::Owned(ISOLATED_PUNCT_PATTERN.replace_all(&result, " ").into_owned());
    }

    if MALFORMED_WORDS_PATTERN.is_match(&result) {
        result = Cow::Owned(MALFORMED_WORDS_PATTERN.replace_all(&result, " ").into_owned());
    }

    if EXCESSIVE_WHITESPACE_PATTERN.is_match(&result) {
        result = Cow::Owned(EXCESSIVE_WHITESPACE_PATTERN.replace_all(&result, " ").into_owned());
    }

    result.into_owned()
}

#[inline]
fn clean_navigation_elements(text: &str) -> String {
    let mut result = Cow::Borrowed(text);

    if NAV_WORDS_PATTERN.is_match(&result) {
        result = Cow::Owned(NAV_WORDS_PATTERN.replace_all(&result, " ").into_owned());
    }

    if BREADCRUMB_PATTERN.is_match(&result) {
        result = Cow::Owned(BREADCRUMB_PATTERN.replace_all(&result, " ").into_owned());
    }

    if PAGINATION_PATTERN.is_match(&result) {
        result = Cow::Owned(PAGINATION_PATTERN.replace_all(&result, " ").into_owned());
    }

    result.into_owned()
}

/// Normalize spaces in text
#[pyfunction]
pub fn normalize_spaces_rust(text: &str) -> String {
    if text.is_empty() || text.trim().is_empty() {
        return String::new();
    }

    // Pre-allocate with reasonable capacity
    let mut result = String::with_capacity(text.len());

    // Process paragraphs
    let mut first = true;
    for paragraph in text.split("\n\n") {
        // Skip empty paragraphs
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !first {
            result.push_str("\n\n");
        }
        first = false;

        // Clean whitespace within paragraph more efficiently
        let cleaned = WHITESPACE_NORMALIZE.replace_all(paragraph, " ");
        let cleaned = NEWLINE_CLEANUP.replace_all(&cleaned, "\n");

        // Process lines
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
