use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::RwLock;

// ============================================================================

static CONTROL_CHARS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\x00-\x08\x0B-\x0C\x0E-\x1F\x7F-\x9F]")
        .expect("Control chars regex pattern is valid and should compile")
});
static REPLACEMENT_CHARS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\u{FFFD}+").expect("Replacement chars regex pattern is valid and should compile"));
static ISOLATED_COMBINING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{0300}-\u{036F}]+")
        .expect("Isolated combining diacritics regex pattern is valid and should compile")
});
static HEBREW_AS_CYRILLIC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{0400}-\u{04FF}]{3,}")
        .expect("Hebrew misencoded as Cyrillic regex pattern is valid and should compile")
});

static ENCODING_CACHE: Lazy<RwLock<HashMap<String, &'static Encoding>>> = Lazy::new(|| RwLock::new(HashMap::new()));

const CACHE_SIZE_LIMIT: usize = 1000;

#[inline]
fn chain_replacements<'a>(mut text: Cow<'a, str>, replacements: &[(&Regex, &str)]) -> Cow<'a, str> {
    for (pattern, replacement) in replacements {
        if pattern.is_match(&text) {
            text = Cow::Owned(pattern.replace_all(&text, *replacement).into_owned());
        }
    }
    text
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

pub fn safe_decode(byte_data: &[u8], encoding: Option<&str>) -> String {
    if byte_data.is_empty() {
        return String::new();
    }

    if let Some(enc_name) = encoding
        && let Some(enc) = Encoding::for_label(enc_name.as_bytes())
    {
        let (decoded, _, _) = enc.decode(byte_data);
        return fix_mojibake_internal(&decoded);
    }

    let cache_key = calculate_cache_key(byte_data);

    if let Ok(cache) = ENCODING_CACHE.read()
        && let Some(&cached_encoding) = cache.get(&cache_key)
    {
        let (decoded, _, _) = cached_encoding.decode(byte_data);
        return fix_mojibake_internal(&decoded);
    }

    let mut detector = EncodingDetector::new();
    detector.feed(byte_data, true);
    let encoding = detector.guess(None, true);

    if let Ok(mut cache) = ENCODING_CACHE.write()
        && cache.len() < CACHE_SIZE_LIMIT
    {
        cache.insert(cache_key, encoding);
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

pub fn get_encoding_cache_key(data_hash: &str, size: usize) -> String {
    format!("{}:{}", data_hash, size)
}

pub fn calculate_text_confidence(text: &str) -> f64 {
    calculate_text_confidence_internal(text)
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

pub fn fix_mojibake(text: &str) -> String {
    fix_mojibake_internal(text)
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
    fn test_get_encoding_cache_key() {
        let key = get_encoding_cache_key("hash123", 1024);
        assert_eq!(key, "hash123:1024");
    }
}
