use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;

// ============================================================================
// Constants
// ============================================================================

// Quality weights
const OCR_PENALTY_WEIGHT: f64 = 0.3;
const SCRIPT_PENALTY_WEIGHT: f64 = 0.2;
const NAV_PENALTY_WEIGHT: f64 = 0.1;
const STRUCTURE_BONUS_WEIGHT: f64 = 0.2;
const METADATA_BONUS_WEIGHT: f64 = 0.1;

// Text thresholds
const MIN_TEXT_LENGTH: usize = 10;
const LARGE_TEXT_LENGTH: usize = 1000;
const MIN_SENTENCE_WORDS: f64 = 10.0;
const MAX_SENTENCE_WORDS: f64 = 30.0;
const MIN_PARAGRAPH_WORDS: f64 = 50.0;
const MAX_PARAGRAPH_WORDS: f64 = 300.0;

// ============================================================================
// Regex Patterns
// ============================================================================

static SCATTERED_CHARS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b").unwrap());
static REPEATED_PUNCT_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.]{3,}|[_]{3,}").unwrap());
static DASH_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"[-]{3,}").unwrap());
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

// ============================================================================
// Helper Functions
// ============================================================================

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

// ============================================================================
// Quality Scoring
// ============================================================================

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

// ============================================================================
// Text Cleaning
// ============================================================================

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
    let result = if SCATTERED_CHARS_PATTERN.is_match(&text) {
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

// ============================================================================
// Space Normalization
// ============================================================================

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

// ============================================================================
// Tests
// ============================================================================

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
}
