use regex::Regex;
use std::borrow::Cow;

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

#[inline]
pub fn sum_match_lengths(text: &str, pattern: &Regex) -> usize {
    pattern.find_iter(text).map(|m| m.len()).sum()
}

pub fn chain_replacements<'a>(mut text: Cow<'a, str>, replacements: &[(&Regex, &str)]) -> Cow<'a, str> {
    for (pattern, replacement) in replacements {
        if pattern.is_match(&text) {
            text = Cow::Owned(pattern.replace_all(&text, *replacement).into_owned());
        }
    }
    text
}

pub mod quality_weights {
    pub const OCR_PENALTY_WEIGHT: f64 = 0.3;
    pub const SCRIPT_PENALTY_WEIGHT: f64 = 0.2;
    pub const NAV_PENALTY_WEIGHT: f64 = 0.1;
    pub const STRUCTURE_BONUS_WEIGHT: f64 = 0.2;
    pub const METADATA_BONUS_WEIGHT: f64 = 0.1;
}

pub mod text_thresholds {
    pub const MIN_TEXT_LENGTH: usize = 10;
    pub const LARGE_TEXT_LENGTH: usize = 1000;
    pub const MIN_SENTENCE_WORDS: f64 = 10.0;
    pub const MAX_SENTENCE_WORDS: f64 = 30.0;
    pub const MIN_PARAGRAPH_WORDS: f64 = 50.0;
    pub const MAX_PARAGRAPH_WORDS: f64 = 300.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    static TEST_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"test").unwrap());
    static NUMBER_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+").unwrap());
    static SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

    #[test]
    fn test_replace_with_if_matches_no_match() {
        let text = "hello world";
        let pattern = &TEST_PATTERN;
        let result = replace_with_if_matches(text, pattern, |_| "replaced".to_string());

        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_replace_with_if_matches_with_match() {
        let text = "this is a test string";
        let pattern = &TEST_PATTERN;
        let result = replace_with_if_matches(text, pattern, |_| "replaced".to_string());

        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "this is a replaced string");
    }

    #[test]
    fn test_replace_with_if_matches_multiple() {
        let text = "test one and test two";
        let pattern = &TEST_PATTERN;
        let result = replace_with_if_matches(text, pattern, |_| "exam".to_string());

        assert_eq!(result, "exam one and exam two");
    }

    #[test]
    fn test_replace_with_if_matches_capture_groups() {
        let text = "number 123 and 456";
        let pattern = &NUMBER_PATTERN;
        let result = replace_with_if_matches(text, pattern, |caps| format!("[{}]", &caps[0]));

        assert_eq!(result, "number [123] and [456]");
    }

    #[test]
    fn test_sum_match_lengths_no_matches() {
        let text = "hello world";
        let pattern = &TEST_PATTERN;
        assert_eq!(sum_match_lengths(text, pattern), 0);
    }

    #[test]
    fn test_sum_match_lengths_single_match() {
        let text = "this is a test";
        let pattern = &TEST_PATTERN;
        assert_eq!(sum_match_lengths(text, pattern), 4);
    }

    #[test]
    fn test_sum_match_lengths_multiple_matches() {
        let text = "test one test two";
        let pattern = &TEST_PATTERN;
        assert_eq!(sum_match_lengths(text, pattern), 8);
    }

    #[test]
    fn test_sum_match_lengths_variable_length() {
        let text = "1 22 333 4444";
        let pattern = &NUMBER_PATTERN;
        assert_eq!(sum_match_lengths(text, pattern), 10);
    }

    #[test]
    fn test_chain_replacements_empty_list() {
        let text = Cow::Borrowed("hello world");
        let replacements = &[];
        let result = chain_replacements(text, replacements);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_chain_replacements_no_matches() {
        let text = Cow::Borrowed("hello world");
        let replacements = &[(&*TEST_PATTERN, "replaced"), (&*NUMBER_PATTERN, "NUM")];
        let result = chain_replacements(text, replacements);

        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_chain_replacements_single_replacement() {
        let text = Cow::Borrowed("this is a test");
        let replacements = &[(&*TEST_PATTERN, "exam")];
        let result = chain_replacements(text, replacements);

        assert_eq!(result, "this is a exam");
    }

    #[test]
    fn test_chain_replacements_multiple_replacements() {
        let text = Cow::Borrowed("test 123 with spaces");
        let replacements = &[
            (&*TEST_PATTERN, "exam"),
            (&*NUMBER_PATTERN, "NUM"),
            (&*SPACE_PATTERN, "_"),
        ];
        let result = chain_replacements(text, replacements);

        assert_eq!(result, "exam_NUM_with_spaces");
    }

    #[test]
    fn test_chain_replacements_order_matters() {
        let text = Cow::Borrowed("test123");

        let replacements1 = &[(&*NUMBER_PATTERN, "_"), (&*TEST_PATTERN, "exam")];
        let result1 = chain_replacements(text.clone(), replacements1);
        assert_eq!(result1, "exam_");

        let replacements2 = &[(&*TEST_PATTERN, "exam"), (&*NUMBER_PATTERN, "_")];
        let result2 = chain_replacements(text, replacements2);
        assert_eq!(result2, "exam_");
    }

    #[test]
    fn test_chain_replacements_owned_input() {
        let text = Cow::Owned("test string".to_string());
        let replacements = &[(&*TEST_PATTERN, "exam")];
        let result = chain_replacements(text, replacements);

        assert_eq!(result, "exam string");
    }

    #[test]
    fn test_text_thresholds_values() {
        use text_thresholds::*;

        let _ = MIN_TEXT_LENGTH;
        let _ = LARGE_TEXT_LENGTH;
        let _ = MIN_SENTENCE_WORDS;
        let _ = MAX_SENTENCE_WORDS;
        let _ = MIN_PARAGRAPH_WORDS;
        let _ = MAX_PARAGRAPH_WORDS;
    }

    #[test]
    fn test_replace_with_if_matches_empty_text() {
        let text = "";
        let pattern = &TEST_PATTERN;
        let result = replace_with_if_matches(text, pattern, |_| "replaced".to_string());
        assert_eq!(result, "");
    }

    #[test]
    fn test_sum_match_lengths_empty_text() {
        let text = "";
        let pattern = &TEST_PATTERN;
        assert_eq!(sum_match_lengths(text, pattern), 0);
    }

    #[test]
    fn test_chain_replacements_empty_text() {
        let text = Cow::Borrowed("");
        let replacements = &[(&*TEST_PATTERN, "replaced")];
        let result = chain_replacements(text, replacements);
        assert_eq!(result, "");
    }
}
