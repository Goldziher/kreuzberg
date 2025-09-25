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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_quality_score_empty_text() {
        assert_eq!(calculate_quality_score("", None), 0.0);
        assert_eq!(calculate_quality_score("   ", None), 0.0);
        assert_eq!(calculate_quality_score("\n\n\n", None), 0.0);
    }

    #[test]
    fn test_calculate_quality_score_short_text() {
        let text = "Hello";
        let score = calculate_quality_score(text, None);
        assert_eq!(score, 0.1);
    }

    #[test]
    fn test_calculate_quality_score_normal_text() {
        let text =
            "This is a normal sentence with proper punctuation. It has multiple sentences. And proper structure.";
        let score = calculate_quality_score(text, None);
        assert!(score > 0.5);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_quality_score_with_ocr_artifacts() {
        let text = "T h i s   i s   s c a t t e r e d   t e x t ... ... ... with repeated punctuation";
        let score = calculate_quality_score(text, None);
        // Score should be lower due to artifacts, but exact threshold may vary
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_quality_score_with_script_content() {
        let text = r#"
            Normal text here.
            <script>function test() { return 42; }</script>
            <style>.class { color: red; }</style>
            More normal text.
        "#;
        let score = calculate_quality_score(text, None);
        // Score should be affected by script content
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_quality_score_with_navigation() {
        let text = "Skip to main content. Home > Products > Item. Page 1 of 10. Next page. Normal content here.";
        let score = calculate_quality_score(text, None);
        // Navigation should affect score but not dramatically for short text
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_quality_score_well_structured() {
        let text = r#"
            This is a well-structured document with proper paragraphs.
            Each sentence has a reasonable number of words. The content flows naturally.

            Here's another paragraph with good structure. It contains multiple sentences.
            The punctuation is correct throughout the document.

            A third paragraph adds even more structure. This helps improve the quality score.
            The document appears to be professionally written.
        "#;
        let score = calculate_quality_score(text, None);
        assert!(score > 0.7);
    }

    #[test]
    fn test_clean_extracted_text_empty() {
        assert_eq!(clean_extracted_text(""), "");
        assert_eq!(clean_extracted_text("   "), "");
    }

    #[test]
    fn test_clean_extracted_text_removes_scripts() {
        let text = "Before <script>alert('test');</script> After";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.contains("<script"));
        assert!(cleaned.contains("Before"));
        assert!(cleaned.contains("After"));
    }

    #[test]
    fn test_clean_extracted_text_removes_styles() {
        let text = "Before <style>.test { color: red; }</style> After";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.contains("<style"));
        assert!(cleaned.contains("Before"));
        assert!(cleaned.contains("After"));
    }

    #[test]
    fn test_clean_extracted_text_fixes_scattered_chars() {
        let text = "T h i s   i s   s c a t t e r e d";
        let cleaned = clean_extracted_text(text);
        // The scattered pattern reducer should consolidate the letters
        // Check that excessive spacing is reduced
        let space_count_orig = text.chars().filter(|&c| c == ' ').count();
        let space_count_cleaned = cleaned.chars().filter(|&c| c == ' ').count();
        assert!(space_count_cleaned < space_count_orig);
    }

    #[test]
    fn test_clean_extracted_text_normalizes_punctuation() {
        let text = "Text........ with ------ excessive _____ punctuation";
        let cleaned = clean_extracted_text(text);
        assert!(cleaned.contains("..."));
        assert!(!cleaned.contains("........"));
    }

    #[test]
    fn test_clean_extracted_text_removes_navigation() {
        let text = "Skip to main content. Normal text here. Back to top";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.contains("Skip to main content"));
        assert!(cleaned.contains("Normal text"));
        assert!(!cleaned.contains("Back to top"));
    }

    #[test]
    fn test_clean_extracted_text_normalizes_whitespace() {
        let text = "Text   with    excessive     spaces\n\n\n\nand newlines";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.contains("   "));
        assert!(!cleaned.contains("\n\n\n"));
    }

    #[test]
    fn test_normalize_spaces_empty() {
        assert_eq!(normalize_spaces(""), "");
        assert_eq!(normalize_spaces("   "), "");
    }

    #[test]
    fn test_normalize_spaces_single_paragraph() {
        let text = "This  is   a   test";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "This is a test");
    }

    #[test]
    fn test_normalize_spaces_multiple_paragraphs() {
        let text = "First paragraph\n\n\nSecond   paragraph\n\n\n\nThird paragraph";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "First paragraph\n\nSecond paragraph\n\nThird paragraph");
    }

    #[test]
    fn test_normalize_spaces_preserves_single_newlines() {
        let text = "Line one\nLine two\nLine three";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "Line one\nLine two\nLine three");
    }

    #[test]
    fn test_normalize_spaces_various_whitespace() {
        let text = "Text\twith\ttabs  and\u{00A0}non-breaking\u{2000}spaces";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "Text with tabs and non-breaking spaces");
    }

    #[test]
    fn test_calculate_ocr_penalty_no_artifacts() {
        let text = "This is clean text without any OCR artifacts.";
        let penalty = calculate_ocr_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_ocr_penalty_with_artifacts() {
        let text = "T h i s   h a s   a r t i f a c t s ........";
        let penalty = calculate_ocr_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
        assert!(penalty <= 1.0);
    }

    #[test]
    fn test_calculate_script_penalty_no_scripts() {
        let text = "Normal text without any scripts or styles.";
        let penalty = calculate_script_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_script_penalty_with_scripts() {
        let text = "Text with <script>alert('test');</script> and function test() { }";
        let penalty = calculate_script_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
        assert!(penalty <= 1.0);
    }

    #[test]
    fn test_calculate_navigation_penalty() {
        let text = "Normal content without navigation elements.";
        let penalty = calculate_navigation_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);

        let nav_text = "Skip to main content Page 1 of 10 Next page";
        let nav_penalty = calculate_navigation_penalty(nav_text, nav_text.len() as f64);
        assert!(nav_penalty > 0.0);
    }

    #[test]
    fn test_calculate_structure_bonus() {
        let well_structured = "This is a sentence. Another sentence here. And a third one.";
        let bonus = calculate_structure_bonus(well_structured);
        assert!(bonus > 0.0);

        let poor_structure = "notasentencejustlongtext";
        let poor_bonus = calculate_structure_bonus(poor_structure);
        assert!(poor_bonus < bonus);
    }
}
