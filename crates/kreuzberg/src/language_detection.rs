//! Language detection using whatlang library.
//!
//! Provides fast language detection for extracted text content.

// TODO: move this file into its own folder - same as other features

use crate::Result;
use crate::core::config::LanguageDetectionConfig;
use whatlang::{Lang, detect_lang};

/// Detect languages in text using whatlang.
///
/// Returns a list of detected language codes (ISO 639-3 format).
/// Returns `None` if no languages could be detected with sufficient confidence.
///
/// # Arguments
///
/// * `text` - The text to analyze for language detection
/// * `config` - Optional configuration for language detection
///
/// # Example
///
/// ```rust
/// use kreuzberg::language_detection::detect_languages;
/// use kreuzberg::core::config::LanguageDetectionConfig;
///
/// let text = "Hello world! This is English text.";
/// let config = LanguageDetectionConfig {
///     enabled: true,
///     min_confidence: 0.8,
///     detect_multiple: false,
/// };
/// let languages = detect_languages(text, &config).unwrap();
/// assert!(languages.is_some());
/// ```
pub fn detect_languages(text: &str, config: &LanguageDetectionConfig) -> Result<Option<Vec<String>>> {
    if !config.enabled {
        return Ok(None);
    }

    if text.trim().is_empty() {
        return Ok(None);
    }

    if !config.detect_multiple {
        return detect_single_language(text, config);
    }

    detect_multiple_languages(text, config)
}

/// Detect a single primary language in the text.
fn detect_single_language(text: &str, _config: &LanguageDetectionConfig) -> Result<Option<Vec<String>>> {
    match detect_lang(text) {
        Some(lang) => {
            let lang_code = lang_to_iso639_3(lang);
            Ok(Some(vec![lang_code]))
        }
        None => Ok(None),
    }
}

/// Detect multiple languages in the text by analyzing chunks.
///
/// This splits the text into chunks and detects the language of each chunk,
/// then returns the most common languages found.
fn detect_multiple_languages(text: &str, _config: &LanguageDetectionConfig) -> Result<Option<Vec<String>>> {
    const CHUNK_SIZE: usize = 500;
    let char_vec: Vec<char> = text.chars().collect();
    let chunk_strings: Vec<String> = char_vec
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect();

    if chunk_strings.is_empty() {
        return Ok(None);
    }

    let mut lang_counts = std::collections::HashMap::new();
    for chunk in &chunk_strings {
        if let Some(lang) = detect_lang(chunk) {
            *lang_counts.entry(lang).or_insert(0) += 1;
        }
    }

    if lang_counts.is_empty() {
        return Ok(None);
    }

    let mut lang_vec: Vec<(Lang, usize)> = lang_counts.into_iter().collect();
    lang_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let languages: Vec<String> = lang_vec.iter().map(|(lang, _)| lang_to_iso639_3(*lang)).collect();

    Ok(Some(languages))
}

/// Convert whatlang Lang enum to ISO 639-3 language code.
///
/// Maps whatlang's language codes to standardized ISO 639-3 codes.
fn lang_to_iso639_3(lang: Lang) -> String {
    match lang {
        Lang::Eng => "eng",
        Lang::Rus => "rus",
        Lang::Cmn => "cmn",
        Lang::Spa => "spa",
        Lang::Por => "por",
        Lang::Ita => "ita",
        Lang::Fra => "fra",
        Lang::Deu => "deu",
        Lang::Ukr => "ukr",
        Lang::Kat => "kat",
        Lang::Ara => "ara",
        Lang::Hin => "hin",
        Lang::Jpn => "jpn",
        Lang::Heb => "heb",
        Lang::Yid => "yid",
        Lang::Pol => "pol",
        Lang::Amh => "amh",
        Lang::Jav => "jav",
        Lang::Kor => "kor",
        Lang::Nob => "nob",
        Lang::Dan => "dan",
        Lang::Swe => "swe",
        Lang::Fin => "fin",
        Lang::Tur => "tur",
        Lang::Nld => "nld",
        Lang::Hun => "hun",
        Lang::Ces => "ces",
        Lang::Ell => "ell",
        Lang::Bul => "bul",
        Lang::Bel => "bel",
        Lang::Mar => "mar",
        Lang::Kan => "kan",
        Lang::Ron => "ron",
        Lang::Slv => "slv",
        Lang::Hrv => "hrv",
        Lang::Srp => "srp",
        Lang::Mkd => "mkd",
        Lang::Lit => "lit",
        Lang::Lav => "lav",
        Lang::Est => "est",
        Lang::Tam => "tam",
        Lang::Vie => "vie",
        Lang::Urd => "urd",
        Lang::Tha => "tha",
        Lang::Guj => "guj",
        Lang::Uzb => "uzb",
        Lang::Pan => "pan",
        Lang::Aze => "aze",
        Lang::Ind => "ind",
        Lang::Tel => "tel",
        Lang::Pes => "pes",
        Lang::Mal => "mal",
        Lang::Ori => "ori",
        Lang::Mya => "mya",
        Lang::Nep => "nep",
        Lang::Sin => "sin",
        Lang::Khm => "khm",
        Lang::Tuk => "tuk",
        Lang::Aka => "aka",
        Lang::Zul => "zul",
        Lang::Sna => "sna",
        Lang::Afr => "afr",
        Lang::Lat => "lat",
        Lang::Slk => "slk",
        Lang::Cat => "cat",
        Lang::Tgl => "tgl",
        Lang::Hye => "hye",
        Lang::Epo => "epo",
        Lang::Ben => "ben",
        Lang::Cym => "cym",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_single_language_english() {
        let text = "Hello world! This is a test of the language detection system.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_detect_single_language_spanish() {
        let text = "Hola mundo! Esta es una prueba del sistema de detección de idiomas.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], "spa");
    }

    #[test]
    fn test_detect_multiple_languages() {
        let text = "Hello world! Hola mundo! Bonjour le monde! こんにちは世界! مرحبا بالعالم!";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: true,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_detect_disabled() {
        let text = "Hello world!";
        let config = LanguageDetectionConfig {
            enabled: false,
            min_confidence: 0.8,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_empty_text() {
        let text = "";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_lang_to_iso639_3() {
        assert_eq!(lang_to_iso639_3(Lang::Eng), "eng");
        assert_eq!(lang_to_iso639_3(Lang::Spa), "spa");
        assert_eq!(lang_to_iso639_3(Lang::Fra), "fra");
        assert_eq!(lang_to_iso639_3(Lang::Deu), "deu");
        assert_eq!(lang_to_iso639_3(Lang::Cmn), "cmn");
    }
}
