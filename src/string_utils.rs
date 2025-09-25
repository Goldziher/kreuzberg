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
