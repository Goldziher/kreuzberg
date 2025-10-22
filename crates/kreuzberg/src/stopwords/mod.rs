//! Stopwords management for text processing.
//!
//! Provides language-specific stopword collections used by keyword extraction
//! and token reduction features. Stopwords are common words (the, is, and, etc.)
//! that should be filtered out from text analysis.
//!
//! # Supported Languages
//!
//! - English (`en`): 78+ common English stopwords
//! - Spanish (`es`): 250+ common Spanish stopwords
//!
//! Additional languages can be loaded from JSON files in the `stopwords/` directory.
//!
//! # Usage
//!
//! ```rust
//! use kreuzberg::stopwords::STOPWORDS;
//!
//! // Get English stopwords
//! let en_stopwords = STOPWORDS.get("en").unwrap();
//! assert!(en_stopwords.contains("the"));
//!
//! // Get Spanish stopwords
//! let es_stopwords = STOPWORDS.get("es").unwrap();
//! assert!(es_stopwords.contains("el"));
//! ```

// TODO: serious bug in this file. We should retrieve the stop words from the main branch and put under this rust package, we should embed the jsons part of our binary. We can compress them to reduce size.
// TODO: load from JSON and memoize the stop words
// TODO: allow the user to pass stop words and either MERGE with or REPLACE the stop words (default behavior should be merge)

use ahash::{AHashMap, AHashSet};
use once_cell::sync::Lazy;
use std::fs;

/// Load stopwords from JSON files.
///
/// Attempts to load stopwords from multiple file locations:
/// 1. `kreuzberg/_token_reduction/stopwords/{lang}_stopwords.json`
/// 2. `../_token_reduction/stopwords/{lang}_stopwords.json`
/// 3. `_token_reduction/stopwords/{lang}_stopwords.json`
/// 4. `stopwords/{lang}_stopwords.json`
///
/// Falls back to hardcoded stopwords for English if loading fails.
///
/// # Arguments
///
/// * `language` - Language code (e.g., "en", "es")
///
/// # Returns
///
/// A set of stopwords for the specified language.
fn load_stopwords_from_json(language: &str) -> AHashSet<String> {
    let paths = [
        format!("kreuzberg/_token_reduction/stopwords/{}_stopwords.json", language),
        format!("../_token_reduction/stopwords/{}_stopwords.json", language),
        format!("_token_reduction/stopwords/{}_stopwords.json", language),
        format!("stopwords/{}_stopwords.json", language),
    ];

    for json_path in &paths {
        if let Ok(content) = fs::read_to_string(json_path)
            && let Ok(words) = serde_json::from_str::<Vec<String>>(&content)
        {
            return words.into_iter().collect();
        }
    }

    match language {
        "en" => [
            "a", "an", "and", "are", "as", "at", "be", "been", "by", "for", "from", "has", "have", "had", "he", "him",
            "his", "her", "hers", "she", "in", "is", "it", "its", "of", "on", "that", "the", "to", "was", "were",
            "will", "with", "would", "this", "these", "they", "them", "their", "but", "or", "if", "then", "than",
            "when", "where", "who", "which", "what", "how", "why", "do", "does", "did", "can", "could", "should",
            "shall", "may", "might", "must", "up", "down", "out", "over", "under", "again", "further", "once", "here",
            "there", "all", "any", "both", "each", "few", "more", "most", "other", "some", "such", "no", "nor", "not",
            "only", "own", "same", "so", "too", "very", "can", "just", "should", "now",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        _ => AHashSet::new(),
    }
}

/// Global stopwords registry.
///
/// A lazy-initialized map of language codes to stopword sets.
/// Includes built-in stopwords for English and Spanish, with support
/// for loading additional languages from JSON files.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::stopwords::STOPWORDS;
///
/// // Access English stopwords
/// let en_stopwords = STOPWORDS.get("en").unwrap();
/// assert!(en_stopwords.contains("the"));
/// assert!(en_stopwords.contains("is"));
///
/// // Access Spanish stopwords
/// let es_stopwords = STOPWORDS.get("es").unwrap();
/// assert!(es_stopwords.contains("el"));
/// assert!(es_stopwords.contains("es"));
/// ```
pub static STOPWORDS: Lazy<AHashMap<String, AHashSet<String>>> = Lazy::new(|| {
    let mut map = AHashMap::new();

    let en_stopwords = load_stopwords_from_json("en");
    map.insert("en".to_string(), en_stopwords);

    let es_stopwords: AHashSet<String> = [
        "a",
        "al",
        "algo",
        "algunas",
        "algunos",
        "ante",
        "antes",
        "como",
        "con",
        "contra",
        "de",
        "del",
        "desde",
        "donde",
        "durante",
        "e",
        "el",
        "ella",
        "ellas",
        "ellos",
        "en",
        "entre",
        "era",
        "erais",
        "eran",
        "eras",
        "eres",
        "es",
        "esa",
        "esas",
        "ese",
        "esos",
        "esta",
        "estaba",
        "estabais",
        "estaban",
        "estabas",
        "estad",
        "estada",
        "estadas",
        "estado",
        "estados",
        "estais",
        "estamos",
        "estan",
        "estando",
        "estar",
        "estaras",
        "estare",
        "estareis",
        "estaremos",
        "estaran",
        "estaras",
        "estaria",
        "estariais",
        "estariamos",
        "estarian",
        "estarias",
        "estas",
        "este",
        "esteis",
        "estemos",
        "esten",
        "estes",
        "esto",
        "estos",
        "estoy",
        "estuve",
        "estuviera",
        "estuvierais",
        "estuvieran",
        "estuvieras",
        "estuvieron",
        "estuviese",
        "estuvieseis",
        "estuviesen",
        "estuvieses",
        "estuvimos",
        "estuviste",
        "estuvisteis",
        "estuvo",
        "fue",
        "fuera",
        "fuerais",
        "fueran",
        "fueras",
        "fueron",
        "fuese",
        "fueseis",
        "fuesen",
        "fueses",
        "fui",
        "fuimos",
        "fuiste",
        "fuisteis",
        "ha",
        "habeis",
        "habia",
        "habiais",
        "habian",
        "habias",
        "habida",
        "habidas",
        "habido",
        "habidos",
        "habiendo",
        "habra",
        "habras",
        "habre",
        "habreis",
        "habremos",
        "habran",
        "habria",
        "habriais",
        "habriamos",
        "habrian",
        "habrias",
        "has",
        "hasta",
        "hay",
        "haya",
        "hayais",
        "hayan",
        "hayas",
        "he",
        "hemos",
        "hube",
        "hubiera",
        "hubierais",
        "hubieran",
        "hubieras",
        "hubieron",
        "hubiese",
        "hubieseis",
        "hubiesen",
        "hubieses",
        "hubimos",
        "hubiste",
        "hubisteis",
        "hubo",
        "la",
        "las",
        "le",
        "les",
        "lo",
        "los",
        "me",
        "mi",
        "mis",
        "mucho",
        "muchos",
        "muy",
        "mas",
        "mia",
        "mias",
        "mio",
        "mios",
        "nada",
        "ni",
        "no",
        "nos",
        "nosotras",
        "nosotros",
        "nuestra",
        "nuestras",
        "nuestro",
        "nuestros",
        "o",
        "os",
        "otra",
        "otras",
        "otro",
        "otros",
        "para",
        "pero",
        "poco",
        "por",
        "porque",
        "que",
        "quien",
        "quienes",
        "qué",
        "se",
        "sea",
        "seais",
        "seamos",
        "sean",
        "seas",
        "sentid",
        "sentida",
        "sentidas",
        "sentido",
        "sentidos",
        "sera",
        "seran",
        "seras",
        "sere",
        "sereis",
        "seremos",
        "seria",
        "seriais",
        "seriamos",
        "serian",
        "serias",
        "si",
        "siente",
        "sin",
        "sintiendo",
        "sobre",
        "sois",
        "somos",
        "son",
        "soy",
        "su",
        "sus",
        "suya",
        "suyas",
        "suyo",
        "suyos",
        "también",
        "te",
        "tendrán",
        "tendrás",
        "tendremos",
        "tengo",
        "ti",
        "tiene",
        "tienen",
        "tienes",
        "todo",
        "todos",
        "tu",
        "tus",
        "tuya",
        "tuyas",
        "tuyo",
        "tuyos",
        "tú",
        "un",
        "una",
        "uno",
        "unos",
        "vosotras",
        "vosotros",
        "vuestra",
        "vuestras",
        "vuestro",
        "vuestros",
        "y",
        "ya",
        "yo",
        "él",
        "éramos",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    map.insert("es".to_string(), es_stopwords);

    map
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopwords_lazy_initialization() {
        let stopwords = &*STOPWORDS;
        assert!(stopwords.contains_key("en"));
        assert!(stopwords.contains_key("es"));
        assert!(!stopwords.get("en").unwrap().is_empty());
        assert!(!stopwords.get("es").unwrap().is_empty());
    }

    #[test]
    fn test_english_stopwords() {
        let en_stopwords = STOPWORDS.get("en").unwrap();

        assert!(en_stopwords.contains("the"));
        assert!(en_stopwords.contains("is"));
        assert!(en_stopwords.contains("and"));
        assert!(en_stopwords.contains("a"));
        assert!(en_stopwords.contains("of"));

        assert!(en_stopwords.len() >= 70);
    }

    #[test]
    fn test_spanish_stopwords() {
        let es_stopwords = STOPWORDS.get("es").unwrap();

        assert!(es_stopwords.contains("el"));
        assert!(es_stopwords.contains("la"));
        assert!(es_stopwords.contains("es"));
        assert!(es_stopwords.contains("en"));
        assert!(es_stopwords.contains("de"));

        assert!(es_stopwords.len() >= 200);
    }

    #[test]
    fn test_unknown_language_returns_empty() {
        assert!(!STOPWORDS.contains_key("xx"));
    }

    #[test]
    fn test_load_stopwords_from_json() {
        let en_stopwords = load_stopwords_from_json("en");
        assert!(!en_stopwords.is_empty());
        assert!(en_stopwords.contains("the"));

        let unknown = load_stopwords_from_json("unknown");
        assert!(unknown.is_empty());
    }
}
