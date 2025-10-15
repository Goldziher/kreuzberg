use crate::error::{KreuzbergError, Result};
use crate::text::token_reduction::config::TokenReductionConfig;
use ahash::{AHashMap, AHashSet};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::sync::Arc;

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

static STOPWORDS: Lazy<AHashMap<String, AHashSet<String>>> = Lazy::new(|| {
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

static HTML_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<!--.*?-->").unwrap());
static EXCESSIVE_NEWLINES_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());
static MULTIPLE_SPACES_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r" {2,}").unwrap());
static MARKDOWN_CODE_BLOCK_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"```[\s\S]*?```").unwrap());
static MARKDOWN_INLINE_CODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"`[^`\n]+`").unwrap());
static MARKDOWN_HEADERS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^#{1,6}\s+").unwrap());
static MARKDOWN_LISTS_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[ \t]*[-*+]\s+").unwrap());

pub struct FilterPipeline {
    config: Arc<TokenReductionConfig>,
    stopwords: AHashSet<String>,
    #[allow(dead_code)]
    preserve_patterns: Vec<Regex>,
    #[allow(dead_code)]
    language: String,
}

impl FilterPipeline {
    pub fn new(config: &Arc<TokenReductionConfig>, language: &str) -> Result<Self> {
        let mut stopwords = STOPWORDS
            .get(language)
            .cloned()
            .unwrap_or_else(|| STOPWORDS.get("en").cloned().unwrap_or_default());

        if let Some(ref custom) = config.custom_stopwords
            && let Some(custom_for_lang) = custom.get(language)
        {
            for word in custom_for_lang {
                stopwords.insert(word.to_lowercase());
            }
        }

        let preserve_patterns: std::result::Result<Vec<Regex>, _> = config
            .preserve_patterns
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect();

        let preserve_patterns =
            preserve_patterns.map_err(|e| KreuzbergError::Validation(format!("Invalid regex pattern: {}", e)))?;

        Ok(Self {
            config: Arc::clone(config),
            stopwords,
            preserve_patterns,
            language: language.to_string(),
        })
    }

    pub fn apply_light_filters(&self, text: &str) -> String {
        let mut result = text.to_string();

        let mut preserved_blocks = Vec::new();
        if self.config.preserve_markdown {
            result = self.extract_and_preserve_code(&result, &mut preserved_blocks);
        }

        result = HTML_COMMENT_REGEX.replace_all(&result, "").to_string();

        result = MULTIPLE_SPACES_REGEX.replace_all(&result, " ").to_string();

        result = EXCESSIVE_NEWLINES_REGEX.replace_all(&result, "\n\n").to_string();

        if self.config.preserve_markdown {
            result = self.preserve_markdown_structure(&result);
        }

        result = self.restore_preserved_blocks(&result, &preserved_blocks);

        result
    }

    pub fn apply_moderate_filters(&self, text: &str) -> String {
        let mut result = self.apply_light_filters(text);

        let mut preserved_blocks = Vec::new();
        if self.config.preserve_code {
            result = self.extract_and_preserve_code(&result, &mut preserved_blocks);
        }

        if self.config.preserve_markdown {
            result = self.remove_stopwords_preserving_markdown(&result);
        } else {
            result = self.remove_stopwords(&result);
        }

        result = self.restore_preserved_blocks(&result, &preserved_blocks);

        result
    }

    fn remove_stopwords_preserving_markdown(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut processed_lines = Vec::new();

        for line in lines {
            if MARKDOWN_HEADERS_REGEX.is_match(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            if MARKDOWN_LISTS_REGEX.is_match(line) {
                processed_lines.push(line.to_string());
                continue;
            }

            if line.trim().starts_with('|') && line.trim().ends_with('|') {
                processed_lines.push(line.to_string());
                continue;
            }

            let processed_line = self.remove_stopwords(line);
            processed_lines.push(processed_line);
        }

        processed_lines.join("\n")
    }

    fn remove_stopwords(&self, text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut filtered_words = Vec::with_capacity(words.len());

        for word in words {
            if word.is_empty() {
                continue;
            }

            if word.len() > 1 && word.bytes().all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic()) {
                filtered_words.push(word);
                continue;
            }

            if word.bytes().any(|b| b.is_ascii_digit()) {
                filtered_words.push(word);
                continue;
            }

            let clean_word = if word.is_ascii() {
                let clean_bytes: Vec<u8> = word
                    .bytes()
                    .filter(|&b| b.is_ascii_alphabetic())
                    .map(|b| b.to_ascii_lowercase())
                    .collect();
                String::from_utf8(clean_bytes).unwrap_or_else(|_| {
                    word.chars()
                        .filter(|c| c.is_alphabetic())
                        .collect::<String>()
                        .to_lowercase()
                })
            } else {
                word.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            };

            if clean_word.is_empty() {
                filtered_words.push(word);
                continue;
            }

            if clean_word.len() <= 1 {
                filtered_words.push(word);
                continue;
            }

            if !self.stopwords.contains(&clean_word) {
                filtered_words.push(word);
            }
        }

        filtered_words.join(" ")
    }

    #[allow(dead_code)]
    fn split_word_boundaries(&self, word: &str) -> (String, String, String) {
        let chars: Vec<char> = word.chars().collect();
        let mut start = 0;
        let mut end = chars.len();

        while start < chars.len() && !chars[start].is_alphanumeric() {
            start += 1;
        }

        while end > start && !chars[end - 1].is_alphanumeric() {
            end -= 1;
        }

        let prefix: String = chars[..start].iter().collect();
        let core: String = chars[start..end].iter().collect();
        let suffix: String = chars[end..].iter().collect();

        (prefix, core, suffix)
    }

    #[allow(dead_code)]
    fn extract_word_core(&self, word: &str) -> String {
        word.chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_lowercase()
    }

    fn preserve_markdown_structure(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut processed_lines = Vec::new();

        for line in lines {
            if MARKDOWN_HEADERS_REGEX.is_match(line) {
                processed_lines.push(line);
                continue;
            }

            if MARKDOWN_LISTS_REGEX.is_match(line) {
                processed_lines.push(line);
                continue;
            }

            processed_lines.push(line);
        }

        processed_lines.join("\n")
    }

    fn extract_and_preserve_code(&self, text: &str, preserved: &mut Vec<String>) -> String {
        let mut result = text.to_string();
        let mut placeholder_id = 0;

        result = MARKDOWN_CODE_BLOCK_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let code_block = caps[0].to_string();
                preserved.push(code_block);
                let placeholder = format!("__CODE_BLOCK_{}__", placeholder_id);
                placeholder_id += 1;
                placeholder
            })
            .to_string();

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

        assert!(!result.contains(" the "));
        assert!(!result.contains(" is "));
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
        assert!(result.contains("fox"));
    }

    #[test]
    fn test_preserve_patterns() {
        let config = TokenReductionConfig {
            preserve_patterns: vec!["\\b[A-Z]{2,}\\b".to_string()],
            ..Default::default()
        };

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
        let config = TokenReductionConfig {
            preserve_markdown: true,
            preserve_code: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "# Header\nThis is `code` and ```\ncode block\n``` text";
        let result = pipeline.apply_moderate_filters(input);

        assert!(result.contains("# Header"));
        assert!(result.contains("`code`"));
        assert!(result.contains("```\ncode block\n```"));
    }

    #[test]
    fn test_apply_light_filters_removes_html_comments() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Text before <!-- comment --> text after";
        let result = pipeline.apply_light_filters(input);

        assert!(!result.contains("<!-- comment -->"));
        assert!(result.contains("Text before"));
        assert!(result.contains("text after"));
    }

    #[test]
    fn test_apply_light_filters_normalizes_whitespace() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Text  with    multiple     spaces";
        let result = pipeline.apply_light_filters(input);

        assert!(!result.contains("  "));
        assert!(result.contains("Text with multiple spaces"));
    }

    #[test]
    fn test_apply_light_filters_reduces_newlines() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Paragraph 1\n\n\n\n\nParagraph 2";
        let result = pipeline.apply_light_filters(input);

        assert!(!result.contains("\n\n\n"));
        assert!(result.contains("Paragraph 1"));
        assert!(result.contains("Paragraph 2"));
    }

    #[test]
    fn test_stopword_removal_preserves_uppercase() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The API is working WITH the SDK";
        let result = pipeline.remove_stopwords(input);

        assert!(result.contains("API"));
        assert!(result.contains("SDK"));
        assert!(result.contains("WITH"));
        assert!(!result.contains("The "));
        assert!(!result.contains(" is "));
    }

    #[test]
    fn test_stopword_removal_preserves_numbers() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The version is 3.14 and the count is 42";
        let result = pipeline.remove_stopwords(input);

        assert!(result.contains("3.14"));
        assert!(result.contains("42"));
        assert!(result.contains("version"));
        assert!(result.contains("count"));
    }

    #[test]
    fn test_stopword_removal_handles_punctuation() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "Hello, the world! This is great.";
        let result = pipeline.remove_stopwords(input);

        assert!(result.contains("Hello,"));
        assert!(result.contains("world!"));
        assert!(result.contains("great."));
    }

    #[test]
    fn test_custom_stopwords() {
        use std::collections::HashMap;

        let mut custom_stopwords = HashMap::new();
        custom_stopwords.insert("en".to_string(), vec!["custom".to_string(), "word".to_string()]);

        let config = TokenReductionConfig {
            custom_stopwords: Some(custom_stopwords),
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "This is a custom word test";
        let result = pipeline.remove_stopwords(input);

        assert!(!result.contains("custom"));
        assert!(!result.contains("word"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_spanish_stopwords() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "es").unwrap();

        let input = "El perro grande bonito tiene";
        let result = pipeline.remove_stopwords(input);

        // Check that Spanish stopwords are filtered and content words preserved
        assert!(result.contains("perro"));
        assert!(result.contains("grande"));
        assert!(result.contains("bonito"));
        // Verify some common Spanish stopwords are removed
        let words: Vec<&str> = result.split_whitespace().collect();
        assert!(!words.contains(&"el"));
        assert!(!words.contains(&"El"));
    }

    #[test]
    fn test_unknown_language_fallback() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "unknown").unwrap();

        // Should fall back to English stopwords
        let input = "The quick test with unknown language";
        let result = pipeline.remove_stopwords(input);

        assert!(!result.contains("The "));
        assert!(result.contains("quick"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_markdown_header_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "# Header 1\n## Header 2\n### Header 3\nRegular text";
        let result = pipeline.remove_stopwords_preserving_markdown(input);

        assert!(result.contains("# Header 1"));
        assert!(result.contains("## Header 2"));
        assert!(result.contains("### Header 3"));
    }

    #[test]
    fn test_markdown_list_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "- Item 1\n* Item 2\n+ Item 3";
        let result = pipeline.remove_stopwords_preserving_markdown(input);

        assert!(result.contains("- Item 1"));
        assert!(result.contains("* Item 2"));
        assert!(result.contains("+ Item 3"));
    }

    #[test]
    fn test_markdown_table_preservation() {
        let config = TokenReductionConfig {
            preserve_markdown: true,
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let result = pipeline.remove_stopwords_preserving_markdown(input);

        assert!(result.contains("| Header 1 | Header 2 |"));
        assert!(result.contains("|----------|----------|"));
    }

    #[test]
    fn test_code_block_preservation() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = Vec::new();
        let input = "Text before\n```rust\nfn main() {}\n```\nText after";
        let result = pipeline.extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved[0].contains("fn main()"));
        assert!(result.contains("__CODE_BLOCK_0__"));
    }

    #[test]
    fn test_inline_code_preservation() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = Vec::new();
        let input = "Use the `println!` macro";
        let result = pipeline.extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert_eq!(preserved[0], "`println!`");
        assert!(result.contains("__INLINE_CODE_0__"));
    }

    #[test]
    fn test_restore_preserved_blocks() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let preserved = vec!["```code```".to_string(), "`inline`".to_string()];
        let input = "Text __CODE_BLOCK_0__ and __INLINE_CODE_1__ here";
        let result = pipeline.restore_preserved_blocks(input, &preserved);

        assert!(result.contains("```code```"));
        assert!(result.contains("`inline`"));
        assert!(!result.contains("__CODE_BLOCK_0__"));
        assert!(!result.contains("__INLINE_CODE_1__"));
    }

    #[test]
    fn test_apply_moderate_filters_with_stopwords() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The quick brown fox is jumping";
        let result = pipeline.apply_moderate_filters(input);

        assert!(!result.contains("The "));
        assert!(!result.contains(" is "));
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
    }

    #[test]
    fn test_invalid_regex_pattern() {
        let config = TokenReductionConfig {
            preserve_patterns: vec!["[invalid".to_string()],
            ..Default::default()
        };

        let config = Arc::new(config);
        let result = FilterPipeline::new(&config, "en");

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, KreuzbergError::Validation(_)));
        }
    }

    #[test]
    fn test_empty_input() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let result = pipeline.apply_light_filters("");
        assert_eq!(result, "");

        let result = pipeline.apply_moderate_filters("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_stopword_removal_single_letter_words() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "I a x test";
        let result = pipeline.remove_stopwords(input);

        // Single letter words should be preserved
        assert!(result.contains("I"));
        assert!(result.contains("x"));
    }

    #[test]
    fn test_stopword_removal_mixed_case() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The Test Is Working";
        let result = pipeline.remove_stopwords(input);

        assert!(!result.contains("The"));
        assert!(!result.contains("Is"));
        assert!(result.contains("Test"));
        assert!(result.contains("Working"));
    }

    #[test]
    fn test_lazy_regex_initialization() {
        // Access each lazy static to ensure they initialize without panic
        let _ = &*HTML_COMMENT_REGEX;
        let _ = &*EXCESSIVE_NEWLINES_REGEX;
        let _ = &*MULTIPLE_SPACES_REGEX;
        let _ = &*MARKDOWN_CODE_BLOCK_REGEX;
        let _ = &*MARKDOWN_INLINE_CODE_REGEX;
        let _ = &*MARKDOWN_HEADERS_REGEX;
        let _ = &*MARKDOWN_LISTS_REGEX;
    }

    #[test]
    fn test_stopwords_lazy_initialization() {
        // Access stopwords to ensure it initializes
        let stopwords = &*STOPWORDS;
        assert!(stopwords.contains_key("en"));
        assert!(stopwords.contains_key("es"));
        assert!(!stopwords.get("en").unwrap().is_empty());
        assert!(!stopwords.get("es").unwrap().is_empty());
    }
}
