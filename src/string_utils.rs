use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::sync::RwLock;

// Mojibake patterns
static CONTROL_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x08\x0B-\x0C\x0E-\x1F\x7F-\x9F]").unwrap());
static REPLACEMENT_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\u{FFFD}+").unwrap());
static ISOLATED_COMBINING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:^|[^\u{0300}-\u{036F}])([\u{0300}-\u{036F}]+)").unwrap());
static HEBREW_AS_CYRILLIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\u{0400}-\u{04FF}]{3,}").unwrap());

// Encoding cache with size limit
static ENCODING_CACHE: Lazy<RwLock<HashMap<String, &'static Encoding>>> = Lazy::new(|| RwLock::new(HashMap::new()));

const CACHE_SIZE_LIMIT: usize = 1000;

/// Safe decode bytes to string with encoding detection
#[pyfunction]
#[pyo3(signature = (byte_data, encoding=None))]
pub fn safe_decode_rust(byte_data: &[u8], encoding: Option<&str>) -> String {
    if byte_data.is_empty() {
        return String::new();
    }

    // Try provided encoding first
    if let Some(enc_name) = encoding {
        if let Some(enc) = Encoding::for_label(enc_name.as_bytes()) {
            let (decoded, _, _) = enc.decode(byte_data);
            return fix_mojibake(&decoded);
        }
    }

    // Calculate cache key
    let cache_key = calculate_cache_key(byte_data);

    // Check cache
    if let Ok(cache) = ENCODING_CACHE.read() {
        if let Some(&cached_encoding) = cache.get(&cache_key) {
            let (decoded, _, _) = cached_encoding.decode(byte_data);
            return fix_mojibake(&decoded);
        }
    }

    // Use chardetng for detection
    let mut detector = EncodingDetector::new();
    detector.feed(byte_data, true);
    let encoding = detector.guess(None, true);

    // Cache the result
    if let Ok(mut cache) = ENCODING_CACHE.write() {
        if cache.len() < CACHE_SIZE_LIMIT {
            cache.insert(cache_key, encoding);
        }
    }

    let (decoded, _, had_errors) = encoding.decode(byte_data);

    // If decoding had errors, try fallback encodings
    if had_errors {
        for enc_name in &[
            "windows-1255", // Hebrew
            "iso-8859-8",   // Hebrew
            "windows-1256", // Arabic
            "iso-8859-6",   // Arabic
            "windows-1252", // Western European
            "cp1251",       // Cyrillic
        ] {
            if let Some(enc) = Encoding::for_label(enc_name.as_bytes()) {
                let (test_decoded, _, test_errors) = enc.decode(byte_data);
                if !test_errors && calculate_text_confidence(&test_decoded) > 0.5 {
                    return fix_mojibake(&test_decoded);
                }
            }
        }
    }

    fix_mojibake(&decoded)
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

fn calculate_text_confidence(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let total_chars = text.len() as f64;

    // Count problematic characters
    let replacement_count = REPLACEMENT_CHARS.find_iter(text).count() as f64;
    let control_count = CONTROL_CHARS.find_iter(text).count() as f64;

    let penalty = (replacement_count + control_count * 2.0) / total_chars;

    // Count readable characters
    let readable_chars = text
        .chars()
        .filter(|c| c.is_ascii_graphic() || c.is_whitespace())
        .count() as f64;

    let readability_score = readable_chars / total_chars;

    // Check for Hebrew misinterpreted as Cyrillic
    let cyrillic_matches = HEBREW_AS_CYRILLIC.find_iter(text);
    let cyrillic_length: usize = cyrillic_matches.map(|m| m.len()).sum();

    let mut final_penalty = penalty;
    if cyrillic_length as f64 > total_chars * 0.1 {
        final_penalty += 0.3;
    }

    (readability_score - final_penalty).clamp(0.0, 1.0)
}

fn fix_mojibake(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();

    // Remove control characters
    result = CONTROL_CHARS.replace_all(&result, "").to_string();

    // Remove replacement characters
    result = REPLACEMENT_CHARS.replace_all(&result, "").to_string();

    // Remove isolated combining characters
    result = ISOLATED_COMBINING.replace_all(&result, "$1").to_string();

    result
}

/// Parallel text processing for batch operations
#[pyfunction]
pub fn batch_process_texts_rust(texts: Vec<String>) -> Vec<String> {
    use rayon::prelude::*;

    texts
        .par_iter()
        .map(|text| {
            // Apply quality cleaning in parallel
            clean_extracted_text_rust(text)
        })
        .collect()
}

// Re-export clean function for batch processing
use crate::quality::clean_extracted_text_rust;
