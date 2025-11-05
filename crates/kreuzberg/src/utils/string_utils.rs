use ahash::AHashMap;
use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::env;
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

struct EncodingCache {
    entries: AHashMap<String, &'static Encoding>,
    order: VecDeque<String>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
}

impl EncodingCache {
    fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: AHashMap::new(),
            order: VecDeque::with_capacity(max_entries),
            max_entries,
            max_bytes,
            current_bytes: 0,
        }
    }

    fn get(&mut self, key: &str) -> Option<&'static Encoding> {
        if let Some(&encoding) = self.entries.get(key) {
            if let Some(pos) = self.order.iter().position(|existing| existing == key)
                && pos + 1 != self.order.len()
                && let Some(entry) = self.order.remove(pos)
            {
                self.order.push_back(entry);
            }
            return Some(encoding);
        }

        None
    }

    fn insert(&mut self, key: String, encoding: &'static Encoding) {
        let key_len = key.len();

        if let Some(pos) = self.order.iter().position(|existing| existing == &key) {
            self.order.remove(pos);
            // The key already exists in the order queue; `entries` will be updated below, so adjust bytes now.
            self.current_bytes = self.current_bytes.saturating_sub(key_len);
        }

        if self.entries.contains_key(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(key_len);
        }

        self.entries.insert(key.clone(), encoding);
        self.current_bytes = self.current_bytes.saturating_add(key_len);
        self.order.push_back(key);

        self.enforce_bounds();
    }

    fn enforce_bounds(&mut self) {
        while self.order.len() > self.max_entries || self.current_bytes > self.max_bytes {
            if let Some(oldest) = self.order.pop_front() {
                if self.entries.remove(&oldest).is_some() {
                    self.current_bytes = self.current_bytes.saturating_sub(oldest.len());
                }
            } else {
                break;
            }
        }
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.current_bytes = 0;
    }

    #[cfg(test)]
    fn set_limits(&mut self, max_entries: usize, max_bytes: usize) {
        self.max_entries = max_entries.max(1);
        self.max_bytes = max_bytes.max(1);
        self.enforce_bounds();
    }
}

const DEFAULT_CACHE_MAX_ENTRIES: usize = 256;
const DEFAULT_CACHE_MAX_BYTES: usize = 256 * 1024;
const CACHE_ENV_MAX_ENTRIES: &str = "KREUZBERG_ENCODING_CACHE_MAX_ENTRIES";
const CACHE_ENV_MAX_BYTES: &str = "KREUZBERG_ENCODING_CACHE_MAX_BYTES";

fn cache_limits() -> (usize, usize) {
    let max_entries = env::var(CACHE_ENV_MAX_ENTRIES)
        .ok()
        .and_then(|val| val.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_CACHE_MAX_ENTRIES);

    let max_bytes = env::var(CACHE_ENV_MAX_BYTES)
        .ok()
        .and_then(|val| val.parse::<usize>().ok())
        .filter(|&v| v >= 1)
        .unwrap_or(DEFAULT_CACHE_MAX_BYTES);

    (max_entries, max_bytes)
}

static ENCODING_CACHE: Lazy<RwLock<EncodingCache>> = Lazy::new(|| {
    let (entries, bytes) = cache_limits();
    RwLock::new(EncodingCache::new(entries, bytes))
});

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
    use ahash::AHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = AHasher::default();
    let sample = if data.len() > 1024 { &data[..1024] } else { data };
    sample.hash(&mut hasher);
    data.len().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Decode raw bytes into UTF-8, using heuristics and fallback encodings when necessary.
///
/// The function prefers an explicit `encoding`, falls back to the cached guess, probes
/// an encoding detector, and finally tries a small curated list before returning a
/// mojibake-cleaned string.
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

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    match ENCODING_CACHE.write() {
        Ok(mut cache) => {
            if let Some(cached_encoding) = cache.get(&cache_key) {
                let (decoded, _, _) = cached_encoding.decode(byte_data);
                return fix_mojibake_internal(&decoded);
            }
        }
        Err(e) => {
            // Lock poisoning should never happen in normal operation ~keep
            tracing::debug!(error = %e, "encoding cache read lock poisoned; continuing without cache");
            // Continue without cache - will detect encoding fresh
        }
    }

    let mut detector = EncodingDetector::new();
    detector.feed(byte_data, true);
    let encoding = detector.guess(None, true);

    // OSError/RuntimeError must bubble up - system errors need user reports ~keep
    match ENCODING_CACHE.write() {
        Ok(mut cache) => {
            cache.insert(cache_key, encoding);
        }
        Err(e) => {
            // Lock poisoning should never happen in normal operation ~keep
            tracing::debug!(error = %e, "encoding cache write lock poisoned; continuing without cache");
            // Continue without caching - extraction will still succeed
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

    let final_text = fix_mojibake_internal(&decoded);

    if had_errors {
        let confidence = calculate_text_confidence_internal(&final_text);
        if confidence < 0.6 {
            let preview: String = final_text.chars().filter(|c| !c.is_control()).take(80).collect();

            tracing::debug!(
                target: "kreuzberg::encoding",
                "safe_decode produced low-confidence output after fallback attempts; encoding={}, confidence={:.3}, len={}, preview=\"{}\"",
                encoding.name(),
                confidence,
                final_text.len(),
                preview
            );
        }
    }

    final_text
}

/// Estimate how trustworthy a decoded string is on a 0.0–1.0 scale.
///
/// Scores close to 1.0 indicate mostly printable characters, whereas lower scores
/// point to mojibake, control characters, or suspicious character mixes.
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

/// Strip control characters and replacement glyphs that typically arise from mojibake.
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
    use encoding_rs::Encoding;

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
    fn test_encoding_cache_eviction() {
        let mut cache = ENCODING_CACHE.write().unwrap();
        cache.clear();
        cache.set_limits(4, 64);

        let encoding = Encoding::for_label(b"utf-8").expect("utf-8 encoding should exist");

        for i in 0..8 {
            cache.insert(format!("key{}", i), encoding);
        }

        assert!(cache.entries.len() <= 4);
        assert!(!cache.entries.contains_key("key0"));
        assert!(cache.entries.contains_key("key7"));
    }

    #[test]
    fn test_encoding_cache_byte_limit_eviction() {
        let mut cache = ENCODING_CACHE.write().unwrap();
        cache.clear();
        cache.set_limits(16, 16);

        let encoding = Encoding::for_label(b"utf-8").expect("utf-8 encoding should exist");

        cache.insert("short".to_string(), encoding);
        cache.insert("much-longer-key".to_string(), encoding);

        assert!(cache.entries.contains_key("much-longer-key"));
        assert!(!cache.entries.contains_key("short"));
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
}
