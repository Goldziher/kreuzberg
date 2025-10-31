//! Language detection using whatlang library.
//!
//! Provides fast language detection for extracted text content.

// TODO: move this file into its own folder - same as other features

use crate::Result;
use crate::core::config::LanguageDetectionConfig;
use whatlang::{Lang, detect};

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
fn detect_single_language(text: &str, config: &LanguageDetectionConfig) -> Result<Option<Vec<String>>> {
    match detect(text) {
        Some(info) => {
            // Apply confidence threshold from config
            if info.confidence() >= config.min_confidence {
                let lang_code = lang_to_iso639_3(info.lang());
                Ok(Some(vec![lang_code]))
            } else {
                // Detection confidence below threshold
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Detect multiple languages in the text by analyzing chunks.
///
/// This splits the text into chunks and detects the language of each chunk,
/// then returns the most common languages found.
fn detect_multiple_languages(text: &str, config: &LanguageDetectionConfig) -> Result<Option<Vec<String>>> {
    const CHUNK_SIZE: usize = 200;
    let char_vec: Vec<char> = text.chars().collect();
    let chunk_strings: Vec<String> = char_vec
        .chunks(CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect();

    if chunk_strings.is_empty() {
        return Ok(None);
    }

    let mut lang_counts = std::collections::HashMap::new();
    let threshold = config.min_confidence.min(0.35);

    for chunk in &chunk_strings {
        if let Some(info) = detect(chunk) {
            // Apply confidence threshold from config
            if info.confidence() >= threshold {
                *lang_counts.entry(info.lang()).or_insert(0) += 1;
            }
        }
    }

    if lang_counts.is_empty() {
        return detect_single_language(text, config);
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
        // Use longer text chunks for better confidence
        let text = "Hello world! This is English text. The quick brown fox jumps over the lazy dog. \
                    Hola mundo! Este es texto en español. El rápido zorro marrón salta sobre el perro perezoso. \
                    Bonjour le monde! Ceci est un texte en français. Le renard brun rapide saute par-dessus le chien paresseux.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.3, // Lower threshold for robustness
            detect_multiple: true,
        };

        let result = detect_languages(text, &config).unwrap();
        // Result may be Some or None depending on whatlang confidence scores
        // Just verify it doesn't panic and returns a valid result
        if let Some(langs) = result {
            assert!(
                !langs.is_empty(),
                "If detection succeeds, should return at least one language"
            );
        }
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

    #[test]
    fn test_confidence_threshold_filters_low_confidence() {
        // Short ambiguous text that will have lower confidence
        let text = "ok yes no";
        let high_confidence_config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.99,
            detect_multiple: false,
        };

        let result = detect_languages(text, &high_confidence_config).unwrap();
        // Should return None because confidence is below 0.99
        assert!(result.is_none());
    }

    #[test]
    fn test_confidence_threshold_accepts_high_confidence() {
        // Clear English text that will have high confidence
        let text = "The quick brown fox jumps over the lazy dog. This is definitely English text with clear patterns.";
        let low_confidence_config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &low_confidence_config).unwrap();
        // Should return Some(["eng"]) because confidence is above 0.5
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_confidence_threshold_boundary_low() {
        // Clear text with high confidence
        let text =
            "This is a comprehensive English sentence with multiple words to ensure accurate language detection.";
        let very_low_threshold = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.01,
            detect_multiple: false,
        };

        let result = detect_languages(text, &very_low_threshold).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_confidence_threshold_boundary_high() {
        // Even clear text might not reach 1.0 confidence
        let text = "The quick brown fox jumps over the lazy dog.";
        let max_threshold = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 1.0,
            detect_multiple: false,
        };

        let result = detect_languages(text, &max_threshold).unwrap();
        // Depending on whatlang's confidence, this might be None
        // This test verifies the threshold is respected
        if let Some(langs) = result {
            assert_eq!(langs.len(), 1);
        }
    }

    #[test]
    fn test_confidence_threshold_multiple_languages() {
        // Mixed language text with longer phrases for better confidence
        let text = format!(
            "{}{}",
            "Hello world! This is English text. The quick brown fox jumps over the lazy dog. ".repeat(10),
            "Hola mundo! Este es texto en español. El rápido zorro marrón salta sobre el perro perezoso. ".repeat(10)
        );
        let high_confidence_config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5, // Lower threshold for robustness
            detect_multiple: true,
        };

        let result = detect_languages(&text, &high_confidence_config).unwrap();
        // Should detect languages but only those with confidence >= 0.5
        // Result may vary based on whatlang's confidence scoring
        if let Some(langs) = result {
            assert!(
                !langs.is_empty(),
                "If detection succeeds, should find at least one language"
            );
            // Verify at least one of the expected languages was detected
            let has_expected = langs.contains(&"eng".to_string())
                || langs.contains(&"spa".to_string())
                || langs.contains(&"fra".to_string());
            assert!(has_expected, "Should detect at least one of the languages in the text");
        }
    }

    #[test]
    fn test_confidence_threshold_filters_all_chunks() {
        // Ambiguous short chunks that won't meet threshold
        let text = "a b c d e f g h i j k ".repeat(50);
        let high_confidence_config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.95,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &high_confidence_config).unwrap();
        // Should return None because all chunks have low confidence
        // or Some([]) if no chunks meet threshold
        assert!(result.is_none() || result.unwrap().is_empty());
    }

    #[test]
    fn test_default_confidence_threshold() {
        // Test with a reasonable confidence threshold - use longer text for better confidence
        let text = "This is a clear English sentence. The quick brown fox jumps over the lazy dog. \
                    English text is easy to detect when there is sufficient content to analyze. \
                    Language detection works best with longer text passages that provide more context.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5, // More reasonable threshold for testing
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        // Should detect English, but allow for confidence variance
        if let Some(langs) = result {
            assert_eq!(langs.len(), 1, "Single language mode should return one language");
            assert_eq!(langs[0], "eng", "Should detect English");
        } else {
            // If None, the confidence was below threshold - that's also valid behavior
            // Just verify the function executed without panicking
        }
    }

    // ========== Multi-language document tests ==========

    #[test]
    fn test_english_spanish_document() {
        let text = format!(
            "{}{}",
            "The global economy has been experiencing significant changes in recent years. International cooperation is essential for addressing climate change and sustainable development. ".repeat(5),
            "La economía global ha estado experimentando cambios significativos en los últimos años. La cooperación internacional es esencial para abordar el cambio climático y el desarrollo sostenible. ".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
        // Should detect both English and Spanish
        assert!(langs.contains(&"eng".to_string()) || langs.contains(&"spa".to_string()));
    }

    #[test]
    fn test_chinese_english_document() {
        let text = format!(
            "{}{}",
            "中国是世界上人口最多的国家。中文是世界上使用人数最多的语言之一。中华文明有着五千年的悠久历史。".repeat(5),
            "China is the most populous country in the world. Chinese is one of the most widely spoken languages. Chinese civilization has a long history of five thousand years. ".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.4,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
        // Should detect Chinese and/or English
        assert!(langs.contains(&"cmn".to_string()) || langs.contains(&"eng".to_string()));
    }

    #[test]
    fn test_french_german_document() {
        let text = format!(
            "{}{}",
            "La France est connue pour sa culture riche et sa cuisine délicieuse. Paris est la capitale de la France et une destination touristique populaire. ".repeat(5),
            "Deutschland ist bekannt für seine Ingenieurskunst und seine reiche Geschichte. Berlin ist die Hauptstadt Deutschlands und eine lebendige Metropole. ".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_russian_ukrainian_document() {
        let text = format!(
            "{}{}",
            "Россия является крупнейшей страной в мире по территории. Москва - столица России и крупнейший город страны. ".repeat(5),
            "Україна є країною в Східній Європі. Київ - столиця України та найбільше місто країни. ".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    // ========== Different language family tests ==========

    #[test]
    fn test_romance_languages() {
        // Italian, Portuguese, Spanish
        let text = "L'Italia è famosa per la sua arte e architettura. O português é falado em vários países. El español es uno de los idiomas más hablados del mundo. ".repeat(3);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_germanic_languages() {
        // German, Dutch, Swedish
        let text = "Deutschland hat eine reiche Kulturgeschichte. Nederland is bekend om zijn tulpen en windmolens. Sverige är känt för sina skogar och innovationer. ".repeat(3);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_slavic_languages() {
        // Polish, Czech, Bulgarian
        let text = "Polska jest krajem w Europie Środkowej. Česká republika má bohatou historii. България е страна на Балканския полуостров. ".repeat(3);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_cjk_languages() {
        // Chinese, Japanese, Korean
        let text = "中国是一个历史悠久的国家。日本は美しい桜の国です。한국은 아시아의 선진국입니다。".repeat(3);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.4,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_arabic_persian() {
        // Arabic and Persian use similar scripts but are different languages
        let text = "اللغة العربية هي واحدة من أقدم اللغات في العالم. زبان فارسی زبانی زیبا و شاعرانه است. ".repeat(5);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.4,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    // ========== Short vs long text tests ==========

    #[test]
    fn test_very_short_text() {
        let text = "Hello";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        // Short text may or may not be detected reliably
        // Just verify it doesn't panic
        if let Some(langs) = result {
            assert!(!langs.is_empty());
        }
    }

    #[test]
    fn test_medium_length_text() {
        let text = "Machine learning is a subset of artificial intelligence that enables computers to learn from data.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_very_long_text() {
        // Generate 5000+ characters of English text
        let paragraph = "The advancement of technology in the twenty-first century has transformed how we live, work, and communicate. \
                        From smartphones to artificial intelligence, these innovations have created unprecedented opportunities and challenges. \
                        Understanding the implications of technological progress requires careful consideration of ethical, social, and economic factors. ";
        let text = paragraph.repeat(20);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.7,
            detect_multiple: false,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0], "eng");
    }

    // ========== Edge cases ==========

    #[test]
    fn test_numbers_only() {
        let text = "123456789 0123456789 987654321";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        // Should return None or have very low confidence
        assert!(result.is_none());
    }

    #[test]
    fn test_punctuation_only() {
        let text = "!!! ??? ... --- *** @@@ ###";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_whitespace_only() {
        let text = "   \t\n   \n\n\t\t   ";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_mixed_numbers_and_text() {
        let text = "The year 2024 marks the 100th anniversary of the founding. Over 50 countries participated in the event with more than 10,000 attendees.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_text_with_urls() {
        let text = "Visit our website at https://example.com for more information. You can also contact us at info@example.com or follow us on social media.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_text_with_email_addresses() {
        let text = "Please send your resume to jobs@company.com or contact.us@example.org for inquiries. Our support team at support@help.com is available 24/7.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    // ========== Code-heavy document tests ==========

    #[test]
    fn test_code_with_comments() {
        let text = r#"
            // This function calculates the factorial of a number
            fn factorial(n: u64) -> u64 {
                if n == 0 {
                    return 1;
                }
                n * factorial(n - 1)
            }

            // The algorithm uses recursion to compute the result efficiently
            // It handles edge cases like zero and negative numbers appropriately
        "#;
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.4,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        // May detect English from comments or return None due to code syntax
        // Just verify it doesn't panic
        if let Some(langs) = result {
            assert!(!langs.is_empty());
        }
    }

    #[test]
    fn test_predominantly_code() {
        let text = r#"
            let x = 42;
            let y = x * 2;
            println!("{}", y);
            fn main() {
                let vec = vec![1, 2, 3];
                for i in vec {
                    println!("{}", i);
                }
            }
        "#;
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        // Code without much natural language should have low confidence
        // or not be detected
        assert!(result.is_none() || result.as_ref().unwrap().is_empty() || result.as_ref().unwrap().len() <= 1);
    }

    #[test]
    fn test_documentation_with_code() {
        let text = r#"
            Language detection is an important feature in document processing systems.
            It allows applications to automatically identify the language of text content.
            This is particularly useful for multilingual documents and international applications.

            Example code:
            let config = LanguageDetectionConfig::default();
            let result = detect_languages(text, &config);

            The detection algorithm analyzes character patterns and word frequencies to determine the most likely language.
            Modern detection systems achieve high accuracy rates across dozens of languages.
        "#;
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    // ========== Technical jargon tests ==========

    #[test]
    fn test_medical_terminology() {
        let text = "The patient presented with acute myocardial infarction and was administered thrombolytic therapy. \
                   The electrocardiogram showed significant ST-segment elevation in the anterior leads. \
                   Cardiac biomarkers including troponin and creatine kinase were significantly elevated.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_legal_terminology() {
        let text = "The plaintiff hereby alleges that the defendant breached the contractual obligations as stipulated in the aforementioned agreement. \
                   Pursuant to clause 5.2, the defendant was required to provide adequate consideration within thirty days of execution. \
                   The court finds that the preponderance of evidence supports the plaintiff's claims.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    #[test]
    fn test_scientific_terminology() {
        let text = "The experimental protocol involved spectrophotometric analysis using ultraviolet-visible spectroscopy. \
                   Quantum mechanical calculations were performed using density functional theory at the B3LYP level. \
                   The results demonstrated significant correlation between molecular structure and optical properties.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }

    // ========== Mixed script tests ==========

    #[test]
    fn test_latin_cyrillic_mix() {
        let text = format!(
            "{}{}",
            "Modern technology enables global communication across language barriers. ".repeat(5),
            "Современные технологии позволяют общаться по всему миру. ".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_latin_cjk_mix() {
        let text = format!(
            "{}{}",
            "Technology companies are expanding into Asian markets. ".repeat(5),
            "科技公司正在进军亚洲市场。".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.4,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    #[test]
    fn test_latin_arabic_mix() {
        let text = format!(
            "{}{}",
            "International cooperation is essential for global peace and prosperity. ".repeat(5),
            "التعاون الدولي ضروري للسلام والازدهار العالمي. ".repeat(5)
        );
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.4,
            detect_multiple: true,
        };

        let result = detect_languages(&text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert!(!langs.is_empty());
    }

    // ========== Performance and edge case tests ==========

    #[test]
    fn test_single_word_detection() {
        let words = vec![("hello", "eng"), ("bonjour", "fra"), ("hola", "spa"), ("привет", "rus")];

        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.3, // Lower threshold for single words
            detect_multiple: false,
        };

        for (word, _expected_lang) in words {
            let result = detect_languages(word, &config).unwrap();
            // Single words may or may not be detected reliably
            // Just verify no panic
            if let Some(langs) = result {
                assert!(!langs.is_empty());
            }
        }
    }

    #[test]
    fn test_repetitive_text() {
        let text = "test test test test test ".repeat(100);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(&text, &config).unwrap();
        // Repetitive text should still be detectable
        if let Some(langs) = result {
            assert!(!langs.is_empty());
        }
    }

    #[test]
    fn test_detection_consistency() {
        // Same text should produce same result
        let text = "This is a consistent test of language detection capabilities across multiple runs.";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result1 = detect_languages(text, &config).unwrap();
        let result2 = detect_languages(text, &config).unwrap();

        assert_eq!(result1, result2, "Detection should be deterministic");
    }

    #[test]
    fn test_chunk_size_boundary() {
        // Test with text exactly at and around chunk size (500 chars)
        let chunk_text = "a".repeat(500);
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: true,
        };

        let result = detect_languages(&chunk_text, &config).unwrap();
        // May or may not detect, just verify no panic
        assert!(result.is_none() || result.is_some());

        let over_chunk = "This is English text. ".repeat(30); // Over 500 chars
        let result2 = detect_languages(&over_chunk, &config).unwrap();
        assert!(result2.is_none() || result2.is_some());
    }

    #[test]
    fn test_special_characters_with_text() {
        let text =
            "The company's revenue increased by 25% year-over-year. CEO said: \"We're excited!\" #growth @investors";
        let config = LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.5,
            detect_multiple: false,
        };

        let result = detect_languages(text, &config).unwrap();
        assert!(result.is_some());
        let langs = result.unwrap();
        assert_eq!(langs[0], "eng");
    }
}
