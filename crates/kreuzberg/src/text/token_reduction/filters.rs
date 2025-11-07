use crate::error::{KreuzbergError, Result};
use crate::stopwords::STOPWORDS;
use crate::text::token_reduction::config::TokenReductionConfig;
use ahash::{AHashMap, AHashSet};
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

static HTML_COMMENT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--.*?-->").expect("HTML comment regex pattern is valid and should compile"));
static EXCESSIVE_NEWLINES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n{3,}").expect("Excessive newlines regex pattern is valid and should compile"));
static MULTIPLE_SPACES_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r" {2,}").expect("Multiple spaces regex pattern is valid and should compile"));
static MARKDOWN_CODE_BLOCK_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"```[\s\S]*?```").expect("Markdown code block regex pattern is valid and should compile"));
static MARKDOWN_INLINE_CODE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"`[^`\n]+`").expect("Markdown inline code regex pattern is valid and should compile"));
static MARKDOWN_HEADERS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#{1,6}\s+").expect("Markdown headers regex pattern is valid and should compile"));
static MARKDOWN_LISTS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[ \t]*[-*+]\s+").expect("Markdown lists regex pattern is valid and should compile"));

pub struct FilterPipeline {
    config: Arc<TokenReductionConfig>,
    stopwords: AHashSet<String>,
    preserve_patterns: Vec<Regex>,
    language: String,
}

impl FilterPipeline {
    pub fn new(config: &Arc<TokenReductionConfig>, language: &str) -> Result<Self> {
        // Try requested language first, fall back to English (which is always available in the embedded stopwords).
        // If English stopwords are missing, it indicates a build/compilation issue that must be exposed.
        let mut stopwords = STOPWORDS.get(language).cloned().unwrap_or_else(|| {
            STOPWORDS
                .get("en")
                .cloned()
                .expect("English stopwords must be available - indicates build failure if missing")
        });

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
            preserve_patterns.map_err(|e| KreuzbergError::validation(format!("Invalid regex pattern: {}", e)))?;

        Ok(Self {
            config: Arc::clone(config),
            stopwords,
            preserve_patterns,
            language: language.to_string(),
        })
    }

    pub fn apply_light_filters(&self, text: &str) -> String {
        let mut result = text.to_string();

        let mut preserved_blocks = AHashMap::new();
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

        let mut preserved_blocks = AHashMap::new();
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

            // Check if word matches any preserve pattern
            if self.should_preserve_word(word) {
                filtered_words.push(word);
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

    /// Get the language code for this filter pipeline.
    ///
    /// Primarily useful for testing and debugging to verify language configuration.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Check if a word should be preserved based on configured patterns.
    fn should_preserve_word(&self, word: &str) -> bool {
        self.preserve_patterns.iter().any(|pattern| pattern.is_match(word))
    }

    /// Split a word into prefix (non-alphanumeric), core (alphanumeric), and suffix (non-alphanumeric).
    ///
    /// This is useful for handling punctuation-wrapped words like "(hello)" or "world!".
    /// Currently used in tests; reserved for future word boundary-aware filtering.
    #[cfg_attr(not(test), allow(dead_code))]
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

    fn extract_and_preserve_code(&self, text: &str, preserved: &mut AHashMap<String, String>) -> String {
        let mut result = text.to_string();
        let mut code_block_id = 0;
        let mut inline_code_id = 0;

        // Extract code blocks and store in HashMap with unique placeholders
        result = MARKDOWN_CODE_BLOCK_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let code_block = caps[0].to_string();
                let placeholder = format!("__CODEBLOCK_{}__", code_block_id);
                code_block_id += 1;
                preserved.insert(placeholder.clone(), code_block);
                placeholder
            })
            .to_string();

        // Extract inline code and store in HashMap with unique placeholders
        result = MARKDOWN_INLINE_CODE_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let inline_code = caps[0].to_string();
                let placeholder = format!("__INLINECODE_{}__", inline_code_id);
                inline_code_id += 1;
                preserved.insert(placeholder.clone(), inline_code);
                placeholder
            })
            .to_string();

        result
    }

    fn restore_preserved_blocks(&self, text: &str, preserved: &AHashMap<String, String>) -> String {
        let mut result = text.to_string();

        // Replace all placeholders with their original content from HashMap
        for (placeholder, original_content) in preserved {
            result = result.replace(placeholder, original_content);
        }

        result
    }
}

#[cfg(all(test, feature = "stopwords"))]
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

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
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

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
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

        assert!(result.contains("perro"));
        assert!(result.contains("grande"));
        assert!(result.contains("bonito"));
        let words: Vec<&str> = result.split_whitespace().collect();
        assert!(!words.contains(&"el"));
        assert!(!words.contains(&"El"));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
    #[test]
    fn test_unknown_language_fallback() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "unknown").unwrap();

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

        let mut preserved = AHashMap::new();
        let input = "Text before\n```rust\nfn main() {}\n```\nText after";
        let result = pipeline.extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved.values().any(|v| v.contains("fn main()")));
        assert!(result.contains("__CODEBLOCK_0__"));
    }

    #[test]
    fn test_inline_code_preservation() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = AHashMap::new();
        let input = "Use the `println!` macro";
        let result = pipeline.extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved.values().any(|v| v == "`println!`"));
        assert!(result.contains("__INLINECODE_0__"));
    }

    #[test]
    fn test_restore_preserved_blocks() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let mut preserved = AHashMap::new();
        preserved.insert("__CODEBLOCK_0__".to_string(), "```code```".to_string());
        preserved.insert("__INLINECODE_0__".to_string(), "`inline`".to_string());
        let input = "Text __CODEBLOCK_0__ and __INLINECODE_0__ here";
        let result = pipeline.restore_preserved_blocks(input, &preserved);

        assert!(result.contains("```code```"));
        assert!(result.contains("`inline`"));
        assert!(!result.contains("__CODEBLOCK_0__"));
        assert!(!result.contains("__INLINECODE_0__"));
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
            assert!(matches!(err, KreuzbergError::Validation { .. }));
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

        assert!(result.contains("I"));
        assert!(result.contains("x"));
    }

    #[cfg_attr(coverage, ignore = "coverage instrumentation disables SIMD stopword paths")]
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
        let _ = &*HTML_COMMENT_REGEX;
        let _ = &*EXCESSIVE_NEWLINES_REGEX;
        let _ = &*MULTIPLE_SPACES_REGEX;
        let _ = &*MARKDOWN_CODE_BLOCK_REGEX;
        let _ = &*MARKDOWN_INLINE_CODE_REGEX;
        let _ = &*MARKDOWN_HEADERS_REGEX;
        let _ = &*MARKDOWN_LISTS_REGEX;
    }

    #[test]
    fn test_multiple_code_blocks_hashmap_approach() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        // Test with multiple code blocks and inline code mixed
        let input =
            "Start ```rust\nlet x = 1;\n``` middle `inline1` text ```python\nprint('hi')\n``` and `inline2` end";
        let mut preserved = AHashMap::new();
        let result = pipeline.extract_and_preserve_code(input, &mut preserved);

        // Verify all blocks are preserved with unique IDs
        assert_eq!(preserved.len(), 4);
        assert!(preserved.contains_key("__CODEBLOCK_0__"));
        assert!(preserved.contains_key("__CODEBLOCK_1__"));
        assert!(preserved.contains_key("__INLINECODE_0__"));
        assert!(preserved.contains_key("__INLINECODE_1__"));

        // Verify content is correct
        assert_eq!(preserved.get("__CODEBLOCK_0__").unwrap(), "```rust\nlet x = 1;\n```");
        assert_eq!(preserved.get("__CODEBLOCK_1__").unwrap(), "```python\nprint('hi')\n```");
        assert_eq!(preserved.get("__INLINECODE_0__").unwrap(), "`inline1`");
        assert_eq!(preserved.get("__INLINECODE_1__").unwrap(), "`inline2`");

        // Verify restoration works correctly
        let restored = pipeline.restore_preserved_blocks(&result, &preserved);
        assert!(restored.contains("```rust\nlet x = 1;\n```"));
        assert!(restored.contains("```python\nprint('hi')\n```"));
        assert!(restored.contains("`inline1`"));
        assert!(restored.contains("`inline2`"));
        assert!(!restored.contains("__CODEBLOCK_"));
        assert!(!restored.contains("__INLINECODE_"));
    }

    #[test]
    fn test_hashmap_order_independence() {
        let config = Arc::new(TokenReductionConfig {
            preserve_code: true,
            ..Default::default()
        });
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        // Test that order doesn't matter with HashMap approach
        let input = "Text `a` and `b` and `c` here";
        let mut preserved = AHashMap::new();
        let result = pipeline.extract_and_preserve_code(input, &mut preserved);

        // All inline codes should be preserved regardless of iteration order
        assert_eq!(preserved.len(), 3);
        let restored = pipeline.restore_preserved_blocks(&result, &preserved);

        assert!(restored.contains("`a`"));
        assert!(restored.contains("`b`"));
        assert!(restored.contains("`c`"));
        assert_eq!(restored, "Text `a` and `b` and `c` here");
    }

    #[test]
    fn test_preserve_patterns_regex() {
        let config = TokenReductionConfig {
            preserve_patterns: vec![
                r"\b[A-Z]{2,}\b".to_string(),     // All caps words like "NASA", "HTTP"
                r"\b\d+\.\d+\.\d+\b".to_string(), // Version numbers like "1.2.3"
                r"@\w+".to_string(),              // Mentions like "@user"
            ],
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The NASA and HTTP protocols version 1.2.3 by @john";
        let result = pipeline.remove_stopwords(input);

        // Preserved patterns should remain
        assert!(result.contains("NASA"));
        assert!(result.contains("HTTP"));
        assert!(result.contains("1.2.3"));
        assert!(result.contains("@john"));

        // Stopwords should be removed
        assert!(!result.contains(" the "));
        assert!(!result.contains(" and "));
        assert!(!result.contains(" by "));
    }

    #[test]
    fn test_language_specific_stopwords() {
        // Test English stopwords
        let config_en = Arc::new(TokenReductionConfig::default());
        let pipeline_en = FilterPipeline::new(&config_en, "en").unwrap();
        assert_eq!(pipeline_en.language(), "en");

        let input_en = "the quick brown fox";
        let result_en = pipeline_en.remove_stopwords(input_en);
        assert!(!result_en.contains(" the "));

        // Test German stopwords
        let config_de = Arc::new(TokenReductionConfig::default());
        let pipeline_de = FilterPipeline::new(&config_de, "de").unwrap();
        assert_eq!(pipeline_de.language(), "de");

        let input_de = "der schnelle braune fuchs";
        let result_de = pipeline_de.remove_stopwords(input_de);
        // "der" is a German stopword and should be removed
        assert!(!result_de.contains(" der "));
        assert!(result_de.contains("schnelle"));
    }

    #[test]
    fn test_language_fallback_to_english() {
        let config = Arc::new(TokenReductionConfig::default());

        // Use an unsupported language - should fall back to English
        let pipeline = FilterPipeline::new(&config, "unsupported_lang").unwrap();
        assert_eq!(pipeline.language(), "unsupported_lang");

        let input = "the quick brown fox";
        let result = pipeline.remove_stopwords(input);

        // Should use English stopwords as fallback
        assert!(!result.contains(" the "));
        assert!(result.contains("quick"));
    }

    #[test]
    fn test_split_word_boundaries() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        // Test word with punctuation
        let (prefix, core, suffix) = pipeline.split_word_boundaries("(hello)");
        assert_eq!(prefix, "(");
        assert_eq!(core, "hello");
        assert_eq!(suffix, ")");

        // Test word with trailing punctuation
        let (prefix2, core2, suffix2) = pipeline.split_word_boundaries("world!");
        assert_eq!(prefix2, "");
        assert_eq!(core2, "world");
        assert_eq!(suffix2, "!");

        // Test word with leading punctuation
        let (prefix3, core3, suffix3) = pipeline.split_word_boundaries("'test");
        assert_eq!(prefix3, "'");
        assert_eq!(core3, "test");
        assert_eq!(suffix3, "");

        // Test word with no punctuation
        let (prefix4, core4, suffix4) = pipeline.split_word_boundaries("simple");
        assert_eq!(prefix4, "");
        assert_eq!(core4, "simple");
        assert_eq!(suffix4, "");

        // Test complex case
        let (prefix5, core5, suffix5) = pipeline.split_word_boundaries("\"example!!!\"");
        assert_eq!(prefix5, "\"");
        assert_eq!(core5, "example");
        assert_eq!(suffix5, "!!!\"");
    }

    #[test]
    fn test_split_word_boundaries_edge_cases() {
        let config = Arc::new(TokenReductionConfig::default());
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        // Only punctuation
        let (prefix, core, suffix) = pipeline.split_word_boundaries("!!!");
        assert_eq!(prefix, "!!!");
        assert_eq!(core, "");
        assert_eq!(suffix, "");

        // Empty string
        let (prefix2, core2, suffix2) = pipeline.split_word_boundaries("");
        assert_eq!(prefix2, "");
        assert_eq!(core2, "");
        assert_eq!(suffix2, "");

        // Single character
        let (prefix3, core3, suffix3) = pipeline.split_word_boundaries("a");
        assert_eq!(prefix3, "");
        assert_eq!(core3, "a");
        assert_eq!(suffix3, "");

        // Unicode characters
        let (prefix4, core4, suffix4) = pipeline.split_word_boundaries("(café)");
        assert_eq!(prefix4, "(");
        assert_eq!(core4, "café");
        assert_eq!(suffix4, ")");
    }

    #[test]
    fn test_custom_stopwords_with_preserve_patterns() {
        use std::collections::HashMap;

        let mut custom_stopwords = HashMap::new();
        custom_stopwords.insert("en".to_string(), vec!["custom".to_string(), "stopword".to_string()]);

        let config = TokenReductionConfig {
            custom_stopwords: Some(custom_stopwords),
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "this is a custom stopword test";
        let result = pipeline.remove_stopwords(input);

        // Custom stopwords should be removed
        assert!(!result.contains(" custom "));
        assert!(!result.contains(" stopword "));
        // Regular stopwords also removed
        assert!(!result.contains(" is "));
        assert!(!result.contains(" a "));
        // Non-stopwords preserved
        assert!(result.contains("test"));
    }

    #[test]
    fn test_preserve_patterns_empty() {
        let config = TokenReductionConfig {
            preserve_patterns: vec![],
            ..Default::default()
        };

        let config = Arc::new(config);
        let pipeline = FilterPipeline::new(&config, "en").unwrap();

        let input = "The quick brown fox";
        let result = pipeline.remove_stopwords(input);

        // With no preserve patterns, normal stopword removal applies
        assert!(!result.contains(" The "));
        assert!(result.contains("quick"));
    }

    #[test]
    fn test_invalid_preserve_pattern() {
        let config = TokenReductionConfig {
            preserve_patterns: vec!["[invalid".to_string()],
            ..Default::default()
        };

        let config = Arc::new(config);
        let result = FilterPipeline::new(&config, "en");

        // Should return validation error for invalid regex
        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                KreuzbergError::Validation { message, .. } => {
                    assert!(message.contains("Invalid regex pattern"));
                }
                _ => panic!("Expected ValidationError"),
            }
        }
    }
}
