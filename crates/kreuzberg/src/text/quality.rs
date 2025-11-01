use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::utils::quality::{collapse_scattered_ascii, normalize_whitespace_ascii};

// ============================================================================
// ============================================================================

const OCR_PENALTY_WEIGHT: f64 = 0.3;
const SCRIPT_PENALTY_WEIGHT: f64 = 0.2;
const NAV_PENALTY_WEIGHT: f64 = 0.1;
const STRUCTURE_BONUS_WEIGHT: f64 = 0.2;
const METADATA_BONUS_WEIGHT: f64 = 0.1;

const MIN_TEXT_LENGTH: usize = 10;
const LARGE_TEXT_LENGTH: usize = 1000;
const MIN_SENTENCE_WORDS: f64 = 10.0;
const MAX_SENTENCE_WORDS: f64 = 30.0;
const MIN_PARAGRAPH_WORDS: f64 = 50.0;
const MAX_PARAGRAPH_WORDS: f64 = 300.0;

static SCATTERED_CHARS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b")
        .expect("Scattered chars regex pattern is valid and should compile")
});
static REPEATED_PUNCT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[.]{3,}|[_]{3,}").expect("Repeated punctuation regex pattern is valid and should compile")
});
static DASH_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[-]{3,}").expect("Dash pattern regex is valid and should compile"));
static ISOLATED_PUNCT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s[.,;:!?]\s").expect("Isolated punctuation regex pattern is valid and should compile"));
static MALFORMED_WORDS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b")
        .expect("Malformed words regex pattern is valid and should compile")
});
static EXCESSIVE_WHITESPACE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s{3,}").expect("Excessive whitespace regex pattern is valid and should compile"));

static JS_FUNCTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)function\s+\w+\s*\([^)]*\)\s*\{[^}]*\}")
        .expect("JavaScript function regex pattern is valid and should compile")
});
static CSS_RULES_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\.[a-zA-Z][\w-]*\s*\{[^}]*\}").expect("CSS rules regex pattern is valid and should compile")
});
static SCRIPT_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("Script tag regex pattern is valid and should compile")
});
static STYLE_TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("Style tag regex pattern is valid and should compile")
});

static NAV_WORDS_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:Skip to main content|Back to top|Main navigation|Site navigation)\b")
        .expect("Navigation words regex pattern is valid and should compile")
});
static BREADCRUMB_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:Home\s*[>»]\s*|[>»]\s*){2,}").expect("Breadcrumb regex pattern is valid and should compile")
});
static PAGINATION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:Page \d+ of \d+|First page|Last page|Previous page|Next page|^\d+ of \d+$)\b")
        .expect("Pagination regex pattern is valid and should compile")
});

static SENTENCE_DETECT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[.!?]\s+[A-Z]").expect("Sentence detection regex pattern is valid and should compile"));
static PUNCTUATION_DETECT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[.!?]").expect("Punctuation detection regex pattern is valid and should compile"));

static WHITESPACE_NORMALIZE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[ \t\f\v\r\xa0\u{2000}-\u{200b}\u{2028}\u{2029}\u{3000}]+")
        .expect("Whitespace normalization regex pattern is valid and should compile")
});
static NEWLINE_NORMALIZE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\n\s*\n\s*\n+").expect("Newline normalization regex pattern is valid and should compile")
});
static NEWLINE_CLEANUP: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n+").expect("Newline cleanup regex pattern is valid and should compile"));

#[inline]
fn sum_match_lengths(text: &str, pattern: &Regex) -> usize {
    pattern.find_iter(text).map(|m| m.len()).sum()
}

fn chain_replacements<'a>(mut text: Cow<'a, str>, replacements: &[(&Regex, &str)]) -> Cow<'a, str> {
    for (pattern, replacement) in replacements {
        if pattern.is_match(&text) {
            text = Cow::Owned(pattern.replace_all(&text, *replacement).into_owned());
        }
    }
    text
}

#[inline]
fn replace_with_if_matches<'a, F>(text: &'a str, pattern: &Regex, replacer: F) -> Cow<'a, str>
where
    F: FnMut(&regex::Captures) -> String,
{
    if pattern.is_match(text) {
        Cow::Owned(pattern.replace_all(text, replacer).into_owned())
    } else {
        Cow::Borrowed(text)
    }
}

pub fn calculate_quality_score(text: &str, metadata: Option<&HashMap<String, String>>) -> f64 {
    if text.is_empty() || text.trim().is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;

    if text.len() < MIN_TEXT_LENGTH {
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
        score += calculate_metadata_bonus(metadata) * METADATA_BONUS_WEIGHT;
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
        + count_non_table_dash_artifacts(text)
        + sum_match_lengths(text, &ISOLATED_PUNCT_PATTERN)
        + sum_match_lengths(text, &MALFORMED_WORDS_PATTERN)
        + sum_match_lengths(text, &EXCESSIVE_WHITESPACE_PATTERN);

    (artifact_chars as f64 / total_chars).min(1.0)
}

#[inline]
fn count_non_table_dash_artifacts(text: &str) -> usize {
    let mut artifact_count = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        let is_table_separator = trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && trimmed
                .chars()
                .all(|c| c == '|' || c == '-' || c.is_whitespace() || c == ':');

        if !is_table_separator {
            for m in DASH_PATTERN.find_iter(line) {
                artifact_count += m.len();
            }
        }
    }

    artifact_count
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
fn calculate_metadata_bonus(metadata: &HashMap<String, String>) -> f64 {
    const IMPORTANT_FIELDS: &[&str] = &["title", "author", "subject", "description", "keywords"];

    let present_fields = IMPORTANT_FIELDS
        .iter()
        .filter(|&&field| metadata.contains_key(field))
        .count();

    present_fields as f64 / IMPORTANT_FIELDS.len() as f64
}

pub fn clean_extracted_text(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut working_text = Cow::Borrowed(text);

    working_text = clean_scripts(working_text);

    working_text = clean_ocr_artifacts_cow(working_text);

    working_text = clean_navigation_elements_cow(working_text);

    working_text = normalize_whitespace_cow(working_text);

    working_text.trim().to_string()
}

#[inline]
fn clean_scripts<'a>(text: Cow<'a, str>) -> Cow<'a, str> {
    let script_replacements = [
        (&*SCRIPT_TAG_PATTERN, " "),
        (&*STYLE_TAG_PATTERN, " "),
        (&*JS_FUNCTION_PATTERN, " "),
        (&*CSS_RULES_PATTERN, " "),
    ];
    chain_replacements(text, &script_replacements)
}

#[inline]
fn normalize_whitespace_cow<'a>(text: Cow<'a, str>) -> Cow<'a, str> {
    if let Some(fast) = normalize_whitespace_ascii(text.as_ref()) {
        return Cow::Owned(fast);
    }

    let mut result = text;

    if WHITESPACE_NORMALIZE.is_match(&result) {
        result = Cow::Owned(WHITESPACE_NORMALIZE.replace_all(&result, " ").into_owned());
    }

    if NEWLINE_NORMALIZE.is_match(&result) {
        result = Cow::Owned(NEWLINE_NORMALIZE.replace_all(&result, "\n\n").into_owned());
    }

    result
}

#[inline]
fn clean_ocr_artifacts_cow<'a>(text: Cow<'a, str>) -> Cow<'a, str> {
    let result = if let Some(fixed) = collapse_scattered_ascii(&text) {
        Cow::Owned(fixed)
    } else if SCATTERED_CHARS_PATTERN.is_match(&text) {
        Cow::Owned(
            replace_with_if_matches(&text, &SCATTERED_CHARS_PATTERN, |caps: &regex::Captures| {
                caps[0].chars().filter(|c| !c.is_whitespace()).collect::<String>()
            })
            .into_owned(),
        )
    } else {
        text
    };

    let result = clean_dashes_preserve_tables(result);

    let ocr_replacements = [
        (&*REPEATED_PUNCT_PATTERN, "..."),
        (&*ISOLATED_PUNCT_PATTERN, " "),
        (&*MALFORMED_WORDS_PATTERN, " "),
        (&*EXCESSIVE_WHITESPACE_PATTERN, " "),
    ];

    chain_replacements(result, &ocr_replacements)
}

#[inline]
fn clean_dashes_preserve_tables<'a>(text: Cow<'a, str>) -> Cow<'a, str> {
    if !DASH_PATTERN.is_match(&text) {
        return text;
    }

    let mut result = String::with_capacity(text.len());
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }

        let trimmed = line.trim();
        let is_table_separator = trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && trimmed
                .chars()
                .all(|c| c == '|' || c == '-' || c.is_whitespace() || c == ':');

        if is_table_separator {
            result.push_str(line);
        } else {
            let cleaned_line = DASH_PATTERN.replace_all(line, "...");
            result.push_str(&cleaned_line);
        }
    }

    Cow::Owned(result)
}

#[inline]
fn clean_navigation_elements_cow<'a>(text: Cow<'a, str>) -> Cow<'a, str> {
    let nav_replacements = [
        (&*NAV_WORDS_PATTERN, " "),
        (&*BREADCRUMB_PATTERN, " "),
        (&*PAGINATION_PATTERN, " "),
    ];

    chain_replacements(text, &nav_replacements)
}

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
    fn test_calculate_quality_score_with_metadata() {
        let text = "This is a normal text with proper structure.";
        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), "Test Title".to_string());
        metadata.insert("author".to_string(), "Test Author".to_string());

        let score = calculate_quality_score(text, Some(&metadata));
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_calculate_ocr_penalty_clean_text() {
        let text = "This is clean text without artifacts";
        let penalty = calculate_ocr_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_ocr_penalty_with_artifacts() {
        let text = "Text with  excessive   spaces and ....... dots";
        let penalty = calculate_ocr_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
        assert!(penalty <= 1.0);
    }

    #[test]
    fn test_calculate_script_penalty_clean_text() {
        let text = "This is clean text without scripts";
        let penalty = calculate_script_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_script_penalty_with_js() {
        let text = "function test() { return 42; }";
        let penalty = calculate_script_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_calculate_navigation_penalty_clean_text() {
        let text = "This is clean text without navigation";
        let penalty = calculate_navigation_penalty(text, text.len() as f64);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_calculate_navigation_penalty_with_nav() {
        let text = "Skip to main content and Back to top links everywhere";
        let penalty = calculate_navigation_penalty(text, text.len() as f64);
        assert!(penalty > 0.0);
    }

    #[test]
    fn test_calculate_structure_bonus_empty() {
        assert_eq!(calculate_structure_bonus(""), 0.0);
    }

    #[test]
    fn test_calculate_structure_bonus_well_structured() {
        let text = "This is a sentence. This is another sentence.\n\nNew paragraph here. More content.";
        let bonus = calculate_structure_bonus(text);
        assert!(bonus > 0.0);
        assert!(bonus <= 1.0);
    }

    #[test]
    fn test_calculate_metadata_bonus_empty() {
        let metadata = HashMap::new();
        let bonus = calculate_metadata_bonus(&metadata);
        assert_eq!(bonus, 0.0);
    }

    #[test]
    fn test_calculate_metadata_bonus_full() {
        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), "Title".to_string());
        metadata.insert("author".to_string(), "Author".to_string());
        metadata.insert("subject".to_string(), "Subject".to_string());
        metadata.insert("description".to_string(), "Description".to_string());
        metadata.insert("keywords".to_string(), "Keywords".to_string());

        let bonus = calculate_metadata_bonus(&metadata);
        assert_eq!(bonus, 1.0);
    }

    #[test]
    fn test_clean_extracted_text_removes_styles() {
        let text = "Before <style>.class { color: red; }</style> After";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.contains("<style"));
        assert!(cleaned.contains("Before"));
        assert!(cleaned.contains("After"));
    }

    #[test]
    fn test_clean_extracted_text_ocr_artifacts() {
        let text = "Text with   excessive    spaces";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.contains("   "));
    }

    #[test]
    fn test_clean_extracted_text_navigation() {
        let text = "Content Skip to main content more content";
        let cleaned = clean_extracted_text(text);
        assert!(cleaned.contains("Content"));
        assert!(cleaned.contains("more content"));
    }

    #[test]
    fn test_normalize_spaces_multiple_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let normalized = normalize_spaces(text);
        assert!(normalized.contains("\n\n"));
    }

    #[test]
    fn test_normalize_spaces_preserves_paragraphs() {
        let text = "Para 1\n\n\n\nPara 2";
        let normalized = normalize_spaces(text);
        assert_eq!(normalized, "Para 1\n\nPara 2");
    }

    #[test]
    fn test_count_non_table_dash_artifacts() {
        let text = "Some text --- with dashes";
        let count = count_non_table_dash_artifacts(text);
        assert!(count > 0);
    }

    #[test]
    fn test_count_non_table_dash_artifacts_preserves_tables() {
        let text = "| Header |\n|--------|\n| Data   |";
        let count = count_non_table_dash_artifacts(text);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_clean_dashes_preserve_tables_simple() {
        let text = Cow::Borrowed("| Col1 |\n|------|\n| Data |");
        let result = clean_dashes_preserve_tables(text);
        assert!(result.contains("|------"));
    }

    #[test]
    fn test_clean_dashes_preserve_tables_replaces_non_table() {
        let text = Cow::Borrowed("Text with --- dashes");
        let result = clean_dashes_preserve_tables(text);
        assert!(result.contains("..."));
        assert!(!result.contains("---"));
    }

    #[test]
    fn test_sum_match_lengths() {
        let text = "test ... test ... test";
        let count = sum_match_lengths(text, &REPEATED_PUNCT_PATTERN);
        assert!(count > 0);
    }

    #[test]
    fn test_quality_score_large_text_with_ocr_issues() {
        let text = "a".repeat(2000) + "   " + &"b".repeat(2000);
        let score = calculate_quality_score(&text, None);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_quality_score_clamped_to_range() {
        let perfect_text = "This is perfect text. ".repeat(100);
        let score = calculate_quality_score(&perfect_text, None);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_clean_extracted_text_scattered_chars() {
        let text = "a  b  c scattered";
        let cleaned = clean_extracted_text(text);
        assert!(!cleaned.is_empty());
    }

    #[test]
    fn test_normalize_whitespace_cow_no_changes() {
        let text = Cow::Borrowed("normaltext");
        let result = normalize_whitespace_cow(text);
        assert_eq!(result.as_ref(), "normaltext");
    }

    #[test]
    fn test_normalize_whitespace_cow_with_changes() {
        let text = Cow::Borrowed("text   with   spaces");
        let result = normalize_whitespace_cow(text);
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_clean_scripts_no_scripts() {
        let text = Cow::Borrowed("clean text");
        let result = clean_scripts(text);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_clean_scripts_with_script_tag() {
        let text = Cow::Borrowed("<script>code</script>");
        let result = clean_scripts(text);
        assert!(!result.contains("<script"));
    }

    #[test]
    fn test_quality_constants() {
        assert_eq!(MIN_TEXT_LENGTH, 10);
        assert_eq!(LARGE_TEXT_LENGTH, 1000);
        assert_eq!(OCR_PENALTY_WEIGHT, 0.3);
    }
}
