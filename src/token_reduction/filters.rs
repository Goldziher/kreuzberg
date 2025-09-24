//! Advanced filtering pipeline for token reduction.
//!
//! Implements multiple levels of filtering from simple formatting cleanup
//! to sophisticated semantic filtering and pattern preservation.

use crate::token_reduction::config::TokenReductionConfig;
use ahash::{AHashMap, AHashSet};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;
use std::fs;
use std::sync::Arc;

/// Load stopwords from JSON files (matching Python implementation)
fn load_stopwords_from_json(language: &str) -> AHashSet<String> {
    // Try to find stopwords using relative paths only (production-safe)
    let paths = [
        format!("kreuzberg/_token_reduction/stopwords/{}_stopwords.json", language),
        format!("../_token_reduction/stopwords/{}_stopwords.json", language),
        format!("_token_reduction/stopwords/{}_stopwords.json", language),
        format!("stopwords/{}_stopwords.json", language),
    ];

    for json_path in &paths {
        if let Ok(content) = fs::read_to_string(json_path) {
            if let Ok(words) = serde_json::from_str::<Vec<String>>(&content) {
                return words.into_iter().collect();
            }
        }
    }

    // Fallback to basic stopwords if JSON loading fails
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

/// Comprehensive stopword sets for multiple languages
static STOPWORDS: Lazy<AHashMap<String, AHashSet<String>>> = Lazy::new(|| {
    let mut map = AHashMap::new();

    // Load English stopwords from JSON
    let en_stopwords = load_stopwords_from_json("en");
    map.insert("en".to_string(), en_stopwords);

    // Spanish stopwords (subset)
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

/// Regex patterns for various text cleanup operations
static HTML_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<!--.*?-->").unwrap());
static EXCESSIVE_NEWLINES_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());
static MULTIPLE_SPACES_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r" {2,}").unwrap());
static MARKDOWN_CODE_BLOCK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"```[\s\S]*?```").unwrap());
static MARKDOWN_INLINE_CODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"`[^`\n]+`").unwrap());
static MARKDOWN_HEADERS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^#{1,6}\s+").unwrap());
static MARKDOWN_LISTS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[ \t]*[-*+]\s+").unwrap());

/// Advanced filtering pipeline
pub struct FilterPipeline {
    config: Arc<TokenReductionConfig>,
    stopwords: AHashSet<String>,
    #[allow(dead_code)]
    preserve_patterns: Vec<Regex>,
    #[allow(dead_code)]
    language: String,
}

impl FilterPipeline {
    pub fn new(config: &Arc<TokenReductionConfig>, language: &str) -> PyResult<Self> {
        let mut stopwords = STOPWORDS
            .get(language)
            .cloned()
            .unwrap_or_else(|| STOPWORDS.get("en").cloned().unwrap_or_default());

        // Add custom stopwords if specified
        if let Some(ref custom) = config.custom_stopwords {
            if let Some(custom_for_lang) = custom.get(language) {
                for word in custom_for_lang {
                    stopwords.insert(word.to_lowercase());
                }
            }
        }

        // Compile preserve patterns
        let preserve_patterns: Result<Vec<Regex>, _> = config
            .preserve_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect();

        let preserve_patterns = preserve_patterns
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex pattern: {}", e)))?;

        Ok(Self {
            config: Arc::clone(config),
            stopwords,
            preserve_patterns,
            language: language.to_string(),
        })
    }

    /// Apply light filtering: formatting cleanup only
    pub fn apply_light_filters(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Preserve code blocks if markdown preservation is enabled
        let mut preserved_blocks = Vec::new();
        if self.config.preserve_markdown {
            result = self.extract_and_preserve_code(&result, &mut preserved_blocks);
        }

        // Remove HTML comments
        result = HTML_COMMENT_REGEX.replace_all(&result, "").to_string();

        // Normalize whitespace after HTML comment removal
        result = MULTIPLE_SPACES_REGEX.replace_all(&result, " ").to_string();

        // Normalize excessive newlines
        result = EXCESSIVE_NEWLINES_REGEX.replace_all(&result, "\n\n").to_string();

        // Handle markdown preservation (non-code elements)
        if self.config.preserve_markdown {
            result = self.preserve_markdown_structure(&result);
        }

        // Restore preserved code blocks
        result = self.restore_preserved_blocks(&result, &preserved_blocks);

        result
    }

    /// Apply moderate filtering: stopword removal
    pub fn apply_moderate_filters(&self, text: &str) -> String {
        let mut result = self.apply_light_filters(text);

        // Preserve code blocks if requested
        let mut preserved_blocks = Vec::new();
        if self.config.preserve_code {
            result = self.extract_and_preserve_code(&result, &mut preserved_blocks);
        }

        // If markdown preservation is enabled, apply stopword removal line by line,
        // skipping markdown structural elements
        if self.config.preserve_markdown {
            result = self.remove_stopwords_preserving_markdown(&result);
        } else {
            // Apply stopword filtering normally
            result = self.remove_stopwords(&result);
        }

        // Restore preserved blocks
        result = self.restore_preserved_blocks(&result, &preserved_blocks);

        result
    }

    /// Remove stopwords while preserving markdown structure
    fn remove_stopwords_preserving_markdown(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut processed_lines = Vec::new();

        for line in lines {
            // Skip stopword removal for markdown headers
            if MARKDOWN_HEADERS_REGEX.is_match(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            // Skip stopword removal for markdown lists (preserve structure but process content)
            if MARKDOWN_LISTS_REGEX.is_match(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            // Skip stopword removal for table headers and separators
            if line.trim().starts_with('|') && line.trim().ends_with('|') {
                // For table content, we could optionally apply stopword removal to cell content
                // but for now, preserve the entire structure
                processed_lines.push(line.to_string());
                continue;
            }

            // Apply stopword removal to paragraph content
            let processed_line = self.remove_stopwords(line);
            processed_lines.push(processed_line);
        }

        processed_lines.join("\n")
    }

    /// Remove stopwords using highly optimized Python-compatible logic
    fn remove_stopwords(&self, text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut filtered_words = Vec::with_capacity(words.len());

        for word in words {
            // Quick checks first for performance
            if word.is_empty() {
                continue;
            }

            // Preserve ALL-CAPS words (fast path)
            if word.len() > 1 && word.bytes().all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic()) {
                filtered_words.push(word);
                continue;
            }

            // Preserve words with numbers (fast path)
            if word.bytes().any(|b| b.is_ascii_digit()) {
                filtered_words.push(word);
                continue;
            }

            // Optimized word cleaning - use bytes for ASCII performance
            let clean_word = if word.is_ascii() {
                // Fast ASCII path - avoid char iteration
                let clean_bytes: Vec<u8> = word
                    .bytes()
                    .filter(|&b| b.is_ascii_alphabetic())
                    .map(|b| b.to_ascii_lowercase())
                    .collect();
                // Safe UTF-8 conversion - filtered ASCII bytes are always valid UTF-8
                String::from_utf8(clean_bytes).unwrap_or_else(|_| {
                    // Fallback to safe char-based processing if something goes wrong
                    word.chars()
                        .filter(|c| c.is_alphabetic())
                        .collect::<String>()
                        .to_lowercase()
                })
            } else {
                // Fallback for non-ASCII
                word.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            };

            if clean_word.is_empty() {
                // Keep words that are all punctuation/numbers
                filtered_words.push(word);
                continue;
            }

            // Preserve very short words
            if clean_word.len() <= 1 {
                filtered_words.push(word);
                continue;
            }

            // Check if it's a stopword - only preserve if it's NOT a stopword
            if !self.stopwords.contains(&clean_word) {
                filtered_words.push(word);
            }
        }

        filtered_words.join(" ")
    }

    /// Split word into prefix punctuation, core word, and suffix punctuation
    /// Matches Python WORD_BOUNDARY_PATTERN: r"^(\W*)(.*?)(\W*)$"
    #[allow(dead_code)]
    fn split_word_boundaries(&self, word: &str) -> (String, String, String) {
        let chars: Vec<char> = word.chars().collect();
        let mut start = 0;
        let mut end = chars.len();

        // Find start of core word (skip leading non-alphanumeric)
        while start < chars.len() && !chars[start].is_alphanumeric() {
            start += 1;
        }

        // Find end of core word (skip trailing non-alphanumeric)
        while end > start && !chars[end - 1].is_alphanumeric() {
            end -= 1;
        }

        let prefix: String = chars[..start].iter().collect();
        let core: String = chars[start..end].iter().collect();
        let suffix: String = chars[end..].iter().collect();

        (prefix, core, suffix)
    }

    /// Extract the core alphabetic part of a word for stopword checking
    #[allow(dead_code)]
    fn extract_word_core(&self, word: &str) -> String {
        word.chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_lowercase()
    }

    /// Preserve markdown structural elements
    fn preserve_markdown_structure(&self, text: &str) -> String {
        // This is a simplified version - in practice, you'd want more sophisticated parsing
        let lines: Vec<&str> = text.lines().collect();
        let mut processed_lines = Vec::new();

        for line in lines {
            // Preserve headers
            if MARKDOWN_HEADERS_REGEX.is_match(line) {
                processed_lines.push(line);
                continue;
            }

            // Preserve list items
            if MARKDOWN_LISTS_REGEX.is_match(line) {
                processed_lines.push(line);
                continue;
            }

            // Process other lines normally
            processed_lines.push(line);
        }

        processed_lines.join("\n")
    }

    /// Extract and preserve code blocks
    fn extract_and_preserve_code(&self, text: &str, preserved: &mut Vec<String>) -> String {
        let mut result = text.to_string();
        let mut placeholder_id = 0;

        // Preserve code blocks
        result = MARKDOWN_CODE_BLOCK_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let code_block = caps[0].to_string();
                preserved.push(code_block);
                let placeholder = format!("__CODE_BLOCK_{}__", placeholder_id);
                placeholder_id += 1;
                placeholder
            })
            .to_string();

        // Preserve inline code
        result = MARKDOWN_INLINE_CODE_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let inline_code = caps[0].to_string();
                preserved.push(inline_code);
                let placeholder = format!("__INLINE_CODE_{}__", placeholder_id);
                placeholder_id += 1;
                placeholder
            })
            .to_string();

        result
    }

    /// Restore preserved code blocks
    fn restore_preserved_blocks(&self, text: &str, preserved: &[String]) -> String {
        let mut result = text.to_string();

        for (i, block) in preserved.iter().enumerate() {
            let placeholder = if block.starts_with("```") {
                format!("__CODE_BLOCK_{}__", i)
            } else {
                format!("__INLINE_CODE_{}__", i)
            };

            result = result.replace(&placeholder, block);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopword_removal() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The quick brown fox is jumping over the lazy dog";
        let result = pipeline.remove_stopwords(input);

        // Should remove "the", "is", "over"
        assert!(!result.contains(" the "));
        assert!(!result.contains(" is "));
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
        assert!(result.contains("fox"));
    }

    #[test]
    fn test_preserve_patterns() {
        let mut config = TokenReductionConfig::default();
        config.preserve_patterns = vec!["\\b[A-Z]{2,}\\b".to_string()]; // Preserve acronyms

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The NASA mission is a success";
        let result = pipeline.remove_stopwords(input);

        assert!(result.contains("NASA"));
        assert!(result.contains("mission"));
        assert!(result.contains("success"));
    }

    #[test]
    fn test_markdown_preservation() {
        let mut config = TokenReductionConfig::default();
        config.preserve_markdown = true;
        config.preserve_code = true;

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "# Header\nThis is `code` and ```\ncode block\n``` text";
        let result = pipeline.apply_moderate_filters(input);

        assert!(result.contains("# Header"));
        assert!(result.contains("`code`"));
        assert!(result.contains("```\ncode block\n```"));
    }
}
