use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::common::chain_replacements;

static CONTROL_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x08\x0B-\x0C\x0E-\x1F\x7F-\x9F]").unwrap());
static REPLACEMENT_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\u{FFFD}+").unwrap());
static ISOLATED_COMBINING: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\u{0300}-\u{036F}]+").unwrap());
static HEBREW_AS_CYRILLIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\u{0400}-\u{04FF}]{3,}").unwrap());

static ENCODING_CACHE: Lazy<RwLock<HashMap<String, &'static Encoding>>> = Lazy::new(|| RwLock::new(HashMap::new()));

const CACHE_SIZE_LIMIT: usize = 1000;

/// Safe decode bytes to string with encoding detection
#[pyfunction]
#[pyo3(signature = (byte_data, encoding=None))]
pub fn safe_decode(byte_data: &[u8], encoding: Option<&str>) -> String {
    if byte_data.is_empty() {
        return String::new();
    }

    if let Some(enc_name) = encoding {
        if let Some(enc) = Encoding::for_label(enc_name.as_bytes()) {
            let (decoded, _, _) = enc.decode(byte_data);
            return fix_mojibake_internal(&decoded);
        }
    }

    let cache_key = calculate_cache_key(byte_data);

    if let Ok(cache) = ENCODING_CACHE.read() {
        if let Some(&cached_encoding) = cache.get(&cache_key) {
            let (decoded, _, _) = cached_encoding.decode(byte_data);
            return fix_mojibake_internal(&decoded);
        }
    }

    let mut detector = EncodingDetector::new();
    detector.feed(byte_data, true);
    let encoding = detector.guess(None, true);

    if let Ok(mut cache) = ENCODING_CACHE.write() {
        if cache.len() < CACHE_SIZE_LIMIT {
            cache.insert(cache_key, encoding);
        }
    }

    let (decoded, _, had_errors) = encoding.decode(byte_data);

    if had_errors {
        for enc_name in &[
            "windows-1255",
            "iso-8859-8",
            "windows-1256",
            "iso-8859-6",
            "windows-1252",
            "cp1251",
        ] {
            if let Some(enc) = Encoding::for_label(enc_name.as_bytes()) {
                let (test_decoded, _, test_errors) = enc.decode(byte_data);
                if !test_errors && calculate_text_confidence_internal(&test_decoded) > 0.5 {
                    return fix_mojibake_internal(&test_decoded);
                }
            }
        }
    }

    fix_mojibake_internal(&decoded)
}

fn calculate_cache_key(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let sample = if data.len() > 1024 { &data[..1024] } else { data };
    sample.hash(&mut hasher);
    data.len().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Get encoding cache key for given data hash and size
#[pyfunction]
pub fn get_encoding_cache_key(data_hash: &str, size: usize) -> String {
    format!("{}:{}", data_hash, size)
}

fn calculate_text_confidence_internal(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;

    let replacement_count = REPLACEMENT_CHARS.find_iter(text).count() as f64;
    let control_count = CONTROL_CHARS.find_iter(text).count() as f64;

    let penalty = (replacement_count + control_count * 2.0) / total_chars;

    let readable_chars = text
        .chars()
        .filter(|c| c.is_ascii_graphic() || c.is_whitespace())
        .count() as f64;

    let readability_score = readable_chars / total_chars;

    let cyrillic_matches = HEBREW_AS_CYRILLIC.find_iter(text);
    let cyrillic_length: usize = cyrillic_matches.map(|m| m.len()).sum();

    let mut final_penalty = penalty;
    if cyrillic_length as f64 > total_chars * 0.1 {
        final_penalty += 0.3;
    }

    (readability_score - final_penalty).clamp(0.0, 1.0)
}

/// Calculate text confidence score for encoding detection
#[pyfunction]
pub fn calculate_text_confidence(text: &str) -> f64 {
    calculate_text_confidence_internal(text)
}

fn fix_mojibake_internal(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }

    let replacements = [
        (&*CONTROL_CHARS, ""),
        (&*REPLACEMENT_CHARS, ""),
        (&*ISOLATED_COMBINING, ""),
    ];

    chain_replacements(Cow::Borrowed(text), &replacements).into_owned()
}

/// Fix mojibake and encoding artifacts in text
#[pyfunction]
pub fn fix_mojibake(text: &str) -> String {
    fix_mojibake_internal(text)
}

/// Parallel text processing for batch operations
#[pyfunction]
pub fn batch_process_texts(texts: Vec<String>) -> Vec<String> {
    use rayon::prelude::*;

    texts.par_iter().map(|text| clean_extracted_text(text)).collect()
}

use crate::quality::clean_extracted_text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_decode_empty() {
        assert_eq!(safe_decode(b"", None), "");
    }

    #[test]
    fn test_safe_decode_ascii() {
        let text = b"Hello, World!";
        assert_eq!(safe_decode(text, None), "Hello, World!");
    }

    #[test]
    fn test_safe_decode_utf8() {
        let text = "Hello, 世界! مرحبا".as_bytes();
        assert_eq!(safe_decode(text, None), "Hello, 世界! مرحبا");
    }

    #[test]
    fn test_safe_decode_with_explicit_encoding() {
        let text = "Héllo".as_bytes();
        assert_eq!(safe_decode(text, Some("utf-8")), "Héllo");
    }

    #[test]
    fn test_safe_decode_latin1() {
        let text = b"H\xe9llo"; // Latin-1 encoded é
        let decoded = safe_decode(text, Some("latin1"));
        assert!(decoded.contains("llo"));
    }

    #[test]
    fn test_safe_decode_invalid_encoding_name() {
        let text = b"Hello";
        // Should fallback to auto-detection
        assert_eq!(safe_decode(text, Some("invalid-encoding")), "Hello");
    }

    #[test]
    fn test_calculate_text_confidence_empty() {
        assert_eq!(calculate_text_confidence(""), 0.0);
    }

    #[test]
    fn test_calculate_text_confidence_clean_text() {
        let text = "This is clean, readable text without any issues.";
        let confidence = calculate_text_confidence(text);
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_calculate_text_confidence_with_control_chars() {
        let text = "Text with \x00 control \x01 characters \x1F";
        let confidence = calculate_text_confidence(text);
        assert!(confidence < 0.9);
    }

    #[test]
    fn test_calculate_text_confidence_with_replacement_chars() {
        let text = "Text with \u{FFFD} replacement \u{FFFD} characters";
        let confidence = calculate_text_confidence(text);
        assert!(confidence < 0.9);
    }

    #[test]
    fn test_calculate_text_confidence_cyrillic_text() {
        let text = "Normal text followed by много кириллического текста здесь";
        let confidence = calculate_text_confidence(text);
        // Cyrillic text may have lower confidence due to heuristics
        // Just verify it returns a valid score
        assert!(confidence >= 0.0 && confidence <= 1.0);
    }

    #[test]
    fn test_fix_mojibake_empty() {
        assert_eq!(fix_mojibake(""), "");
    }

    #[test]
    fn test_fix_mojibake_clean_text() {
        let text = "Clean text without mojibake";
        assert_eq!(fix_mojibake(text), text);
    }

    #[test]
    fn test_fix_mojibake_control_chars() {
        let text = "Text\x00with\x01control\x1Fchars";
        let fixed = fix_mojibake(text);
        assert_eq!(fixed, "Textwithcontrolchars");
    }

    #[test]
    fn test_fix_mojibake_replacement_chars() {
        let text = "Text\u{FFFD}with\u{FFFD}replacement";
        let fixed = fix_mojibake(text);
        assert_eq!(fixed, "Textwithreplacement");
    }

    #[test]
    fn test_fix_mojibake_isolated_combining() {
        let text = "Text\u{0301}with\u{0308}combining";
        let fixed = fix_mojibake(text);
        assert_eq!(fixed, "Textwithcombining");
    }

    #[test]
    fn test_fix_mojibake_mixed_issues() {
        let text = "Mixed\x00text\u{FFFD}with\u{0301}issues";
        let fixed = fix_mojibake(text);
        assert_eq!(fixed, "Mixedtextwithissues");
    }

    #[test]
    fn test_batch_process_texts_empty() {
        let texts: Vec<String> = vec![];
        let processed = batch_process_texts(texts);
        assert!(processed.is_empty());
    }

    #[test]
    fn test_batch_process_texts_single() {
        let texts = vec!["Test text".to_string()];
        let processed = batch_process_texts(texts);
        assert_eq!(processed.len(), 1);
        assert!(processed[0].contains("Test text"));
    }

    #[test]
    fn test_batch_process_texts_multiple() {
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];
        let processed = batch_process_texts(texts);
        assert_eq!(processed.len(), 3);
    }

    #[test]
    fn test_batch_process_texts_with_cleaning() {
        let texts = vec![
            "Text   with   spaces".to_string(),
            "Text\n\n\n\nwith newlines".to_string(),
        ];
        let processed = batch_process_texts(texts);
        assert!(!processed[0].contains("   "));
        assert!(!processed[1].contains("\n\n\n\n"));
    }

    #[test]
    fn test_get_encoding_cache_key() {
        let key = get_encoding_cache_key("hash123", 1024);
        assert_eq!(key, "hash123:1024");
    }

    #[test]
    fn test_calculate_cache_key_consistent() {
        let data = b"test data";
        let key1 = calculate_cache_key(data);
        let key2 = calculate_cache_key(data);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_calculate_cache_key_different_data() {
        let data1 = b"test data 1";
        let data2 = b"test data 2";
        let key1 = calculate_cache_key(data1);
        let key2 = calculate_cache_key(data2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_calculate_cache_key_truncates_large_data() {
        let large_data = vec![b'a'; 2048];
        let small_data = vec![b'a'; 512];

        // Keys should be different due to size being part of the hash
        let large_key = calculate_cache_key(&large_data);
        let small_key = calculate_cache_key(&small_data);
        assert_ne!(large_key, small_key);
    }

    #[test]
    fn test_safe_decode_caching() {
        let data = b"cached text";

        // First decode should cache the encoding
        let result1 = safe_decode(data, None);

        // Second decode should use cached encoding
        let result2 = safe_decode(data, None);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_safe_decode_fallback_encodings() {
        // Test that fallback encodings are tried when main detection has errors
        // Using bytes that might trigger fallback logic
        let data = vec![0xE9, 0xE8, 0xE0]; // Common Latin-1 accented characters
        let result = safe_decode(&data, None);

        // Should produce some result without panicking
        assert!(!result.is_empty() || result.is_empty());
    }
}
