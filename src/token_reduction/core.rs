//! Core token reduction engine with parallel processing and semantic awareness.

use crate::token_reduction::{
    cjk_utils::CjkTokenizer,
    config::{ReductionLevel, TokenReductionConfig},
    filters::FilterPipeline,
    semantic::SemanticAnalyzer,
    simd_text::{chunk_text_for_parallel, SimdTextProcessor},
};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use rayon::prelude::*;
use regex::Regex;
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

/// Pre-compiled regex patterns for performance
#[allow(dead_code)]
static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static REPEATED_EXCLAMATION: Lazy<Regex> = Lazy::new(|| Regex::new(r"[!]{2,}").unwrap());
static REPEATED_QUESTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"[?]{2,}").unwrap());
static REPEATED_COMMA: Lazy<Regex> = Lazy::new(|| Regex::new(r"[,]{2,}").unwrap());

/// High-performance token reduction engine
pub struct TokenReducer {
    config: Arc<TokenReductionConfig>,
    text_processor: SimdTextProcessor,
    filter_pipeline: FilterPipeline,
    semantic_analyzer: Option<SemanticAnalyzer>,
    cjk_tokenizer: CjkTokenizer,
    #[allow(dead_code)]
    language: String,
}

impl TokenReducer {
    /// Create a new token reducer with the given configuration
    pub fn new(config: &TokenReductionConfig, language_hint: Option<&str>) -> PyResult<Self> {
        let config = Arc::new(config.clone());
        let language = language_hint
            .or(config.language_hint.as_deref())
            .unwrap_or("en")
            .to_string();

        let text_processor = SimdTextProcessor::new();
        let filter_pipeline = FilterPipeline::new(&config, &language)?;

        let semantic_analyzer = if matches!(config.level, ReductionLevel::Aggressive | ReductionLevel::Maximum) {
            Some(SemanticAnalyzer::new(&language)?)
        } else {
            None
        };

        Ok(Self {
            config,
            text_processor,
            filter_pipeline,
            semantic_analyzer,
            cjk_tokenizer: CjkTokenizer::new(),
            language,
        })
    }

    /// Reduce tokens in a single text with optimized processing
    pub fn reduce(&self, text: &str) -> String {
        if text.is_empty() || matches!(self.config.level, ReductionLevel::Off) {
            return text.to_string();
        }

        // Skip Unicode normalization if text appears to be ASCII
        let working_text = if text.is_ascii() {
            text
        } else {
            // Only normalize non-ASCII text to avoid overhead
            &text.nfc().collect::<String>()
        };

        // Apply reduction based on level with optimized paths
        match self.config.level {
            ReductionLevel::Off => working_text.to_string(),
            ReductionLevel::Light => self.apply_light_reduction_optimized(working_text),
            ReductionLevel::Moderate => self.apply_moderate_reduction_optimized(working_text),
            ReductionLevel::Aggressive => self.apply_aggressive_reduction_optimized(working_text),
            ReductionLevel::Maximum => self.apply_maximum_reduction_optimized(working_text),
        }
    }

    /// Reduce tokens in multiple texts using optimized parallel processing
    pub fn batch_reduce(&self, texts: &[&str]) -> Vec<String> {
        // Lower threshold for parallel processing - benefit starts with 2+ texts
        if !self.config.enable_parallel || texts.len() < 2 {
            return texts.iter().map(|text| self.reduce(text)).collect();
        }

        // Use rayon's optimal work-stealing for parallel processing
        texts.par_iter().map(|text| self.reduce(text)).collect()
    }

    /// Apply optimized light reduction: formatting cleanup only
    fn apply_light_reduction_optimized(&self, text: &str) -> String {
        // For light mode, skip aggressive whitespace normalization
        // and let apply_light_filters handle it with precise regex patterns
        let mut result = if self.config.use_simd {
            // Only apply punctuation cleaning, skip whitespace normalization
            self.text_processor.clean_punctuation(text)
        } else {
            self.clean_punctuation_optimized(text)
        };

        // Apply basic filters (includes proper newline handling)
        result = self.filter_pipeline.apply_light_filters(&result);
        result.trim().to_string()
    }

    /// Apply optimized moderate reduction: traditional stopword removal
    fn apply_moderate_reduction_optimized(&self, text: &str) -> String {
        // First apply light reduction
        let mut result = self.apply_light_reduction_optimized(text);

        // Apply stopword filtering with lower parallel threshold for better performance
        result = if self.config.enable_parallel && text.len() > 1000 {
            self.apply_parallel_moderate_reduction(&result)
        } else {
            self.filter_pipeline.apply_moderate_filters(&result)
        };

        result
    }

    /// Apply optimized aggressive reduction: moderate + semantic filtering
    fn apply_aggressive_reduction_optimized(&self, text: &str) -> String {
        // Start with moderate reduction as the base
        let mut result = self.apply_moderate_reduction_optimized(text);

        // Apply additional aggressive filtering
        result = self.remove_additional_common_words(&result);
        result = self.apply_sentence_selection(&result);

        // Apply semantic filtering if available
        if let Some(ref analyzer) = self.semantic_analyzer {
            result = analyzer.apply_semantic_filtering(&result, self.config.semantic_threshold);
        }

        result
    }

    /// Apply optimized maximum reduction: hypernym compression
    fn apply_maximum_reduction_optimized(&self, text: &str) -> String {
        let mut result = self.apply_aggressive_reduction_optimized(text);

        if let Some(ref analyzer) = self.semantic_analyzer {
            if self.config.enable_semantic_clustering {
                result = analyzer.apply_hypernym_compression(&result, self.config.target_reduction);
            }
        }

        result
    }

    /// Parallel processing for moderate reduction on large texts
    fn apply_parallel_moderate_reduction(&self, text: &str) -> String {
        let num_threads = rayon::current_num_threads();
        let chunks = chunk_text_for_parallel(text, num_threads);

        let processed_chunks: Vec<String> = chunks
            .par_iter()
            .map(|chunk| self.filter_pipeline.apply_moderate_filters(chunk))
            .collect();

        processed_chunks.join(" ")
    }

    /// Optimized whitespace normalization using pre-compiled regex
    #[allow(dead_code)]
    fn normalize_whitespace_optimized(&self, text: &str) -> String {
        WHITESPACE_REGEX.replace_all(text, " ").to_string()
    }

    /// Optimized punctuation cleanup using pre-compiled regex patterns
    fn clean_punctuation_optimized(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Use pre-compiled regex patterns for better performance
        result = REPEATED_EXCLAMATION.replace_all(&result, "!").to_string();
        result = REPEATED_QUESTION.replace_all(&result, "?").to_string();
        result = REPEATED_COMMA.replace_all(&result, ",").to_string();

        result
    }

    /// Remove additional words using statistical frequency analysis (language-agnostic)
    fn remove_additional_common_words(&self, text: &str) -> String {
        let words = self.universal_tokenize(text);

        if words.len() < 4 {
            return text.to_string(); // Don't over-reduce very short texts
        }

        // Build frequency and length statistics
        let mut word_freq = std::collections::HashMap::new();
        let mut word_lengths = Vec::new();

        for word in &words {
            let clean_word = if word.chars().all(|c| c.is_alphabetic()) {
                word.to_lowercase()
            } else {
                word.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            };

            if !clean_word.is_empty() {
                *word_freq.entry(clean_word.clone()).or_insert(0) += 1;
                word_lengths.push(clean_word.chars().count()); // Use char count for Unicode
            }
        }

        // Calculate average word length for filtering threshold
        let avg_length = if !word_lengths.is_empty() {
            word_lengths.iter().sum::<usize>() as f32 / word_lengths.len() as f32
        } else {
            5.0
        };

        let original_count = words.len();

        // More aggressive filtering - remove frequent words AND short words
        let filtered_words: Vec<String> = words
            .iter()
            .filter(|word| {
                let clean_word = if word.chars().all(|c| c.is_alphabetic()) {
                    word.to_lowercase()
                } else {
                    word.chars()
                        .filter(|c| c.is_alphabetic())
                        .collect::<String>()
                        .to_lowercase()
                };

                if clean_word.is_empty() {
                    return true; // Keep punctuation/numbers
                }

                // Aggressive filtering criteria - more balanced approach
                let freq = word_freq.get(&clean_word).unwrap_or(&0);
                let word_len = clean_word.chars().count() as f32;

                // Keep if:
                // 1. Has important characteristics (numbers, caps, etc.), OR
                // 2. Appears infrequently (1-2 times) AND is reasonably long (>= avg * 0.8), OR
                // 3. Is very long (much above average)
                self.has_important_characteristics(word)
                    || (*freq <= 2 && word_len >= avg_length * 0.8)
                    || (word_len >= avg_length * 1.5)
            })
            .cloned()
            .collect();

        // Adjust fallback threshold for CJK languages where tokens are shorter
        let has_cjk_content = text.chars().any(|c| c as u32 >= 0x4E00 && (c as u32) <= 0x9FFF);
        let fallback_threshold = if has_cjk_content {
            // More lenient threshold for CJK - allow more aggressive filtering
            original_count / 5 // Allow keeping just 20% instead of 33%
        } else {
            original_count / 3 // Standard threshold for other languages
        };

        // If we filtered too aggressively, fall back to keeping longer words
        if filtered_words.len() < fallback_threshold {
            let fallback_words: Vec<String> = words
                .iter()
                .filter(|word| {
                    let clean_word = if word.chars().all(|c| c.is_alphabetic()) {
                        (*word).clone()
                    } else {
                        word.chars().filter(|c| c.is_alphabetic()).collect::<String>()
                    };

                    clean_word.is_empty() || clean_word.chars().count() >= 3 || self.has_important_characteristics(word)
                })
                .cloned()
                .collect();
            self.smart_join(&fallback_words, has_cjk_content)
        } else {
            self.smart_join(&filtered_words, has_cjk_content)
        }
    }

    /// Smart join function that handles CJK and non-CJK languages appropriately
    fn smart_join(&self, tokens: &[String], has_cjk_content: bool) -> String {
        if has_cjk_content {
            // For CJK languages, concatenate without spaces as they don't use whitespace
            tokens.join("")
        } else {
            // For other languages, use space separation
            tokens.join(" ")
        }
    }

    /// Check for language-agnostic important word characteristics
    fn has_important_characteristics(&self, word: &str) -> bool {
        // ALL-CAPS (emphasis, acronyms) - universal
        if word.len() > 1 && word.chars().all(|c| c.is_uppercase()) {
            return true;
        }

        // Contains numbers - universal importance
        if word.chars().any(|c| c.is_numeric()) {
            return true;
        }

        // Very long words (technical terms) - universal pattern
        if word.len() > 10 {
            return true;
        }

        // Mixed case (proper nouns) - universal pattern
        let uppercase_count = word.chars().filter(|c| c.is_uppercase()).count();
        if uppercase_count > 1 && uppercase_count < word.len() {
            return true;
        }

        // CJK-specific importance indicators (2025 research-based)
        if self.has_cjk_importance(word) {
            return true;
        }

        false
    }

    /// CJK importance detection using common semantic radicals
    /// NOTE: This is a simplified heuristic approach - production systems should use
    /// proper CJK segmentation libraries (e.g., jieba, mecab) for optimal results
    fn has_cjk_importance(&self, word: &str) -> bool {
        let chars: Vec<char> = word.chars().collect();

        // Check if word contains CJK characters
        let has_cjk = chars.iter().any(|&c| c as u32 >= 0x4E00 && (c as u32) <= 0x9FFF);
        if !has_cjk {
            return false;
        }

        // Common semantic radicals for technical/abstract concepts (heuristic approach)
        let important_radicals = [
            // Technology and learning
            '学', '智', '能', '技', '术', '法', '算', '理', '科', '研', '究', '发', '展',
            // Abstract concepts and processes
            '系', '统', '模', '型', '方', '式', '过', '程', '结', '构', '功', '效', '应',
            // Analysis and measurement
            '分', '析', '计', '算', '数', '据', '信', '息', '处', '理', '语', '言', '文',
            // Important action verbs
            '生', '成', '产', '用', '作', '为', '成', '变', '化', '转', '换', '提', '高',
            // Key nouns and concepts
            '网', '络', '神', '经', '机', '器', '人', '工', '智', '能', '自', '然', '复',
        ];

        // Check for important semantic radicals
        for &char in &chars {
            if important_radicals.contains(&char) {
                return true;
            }
        }

        // 2025 research: 2-character CJK compounds are semantically dense
        // Preserve them more aggressively as they often represent key concepts
        if chars.len() == 2 && has_cjk {
            // Additional check for technical/abstract character ranges
            let has_technical = chars.iter().any(|&c| {
                let code = c as u32;
                // High semantic density ranges based on 2025 research
                (0x4E00..=0x4FFF).contains(&code) ||  // Common ideographs
                (0x5000..=0x51FF).contains(&code) ||  // Semantic radicals
                (0x6700..=0x68FF).contains(&code) ||  // Technical concepts
                (0x7500..=0x76FF).contains(&code) // Modern terms
            });

            if has_technical {
                return true;
            }
        }

        false
    }

    /// Apply sentence-level selection to keep only most important sentences
    fn apply_sentence_selection(&self, text: &str) -> String {
        // Split into sentences
        let sentences: Vec<&str> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if sentences.len() <= 2 {
            return text.to_string(); // Don't reduce very short texts further
        }

        // Score sentences based on content importance
        let mut scored_sentences: Vec<(usize, f32, &str)> = sentences
            .iter()
            .enumerate()
            .map(|(i, sentence)| {
                let score = self.score_sentence_importance(sentence, i, sentences.len());
                (i, score, *sentence)
            })
            .collect();

        // Sort by score (highest first)
        scored_sentences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Keep top 40% of sentences for aggressive reduction (more aggressive than 60%)
        let keep_count = ((sentences.len() as f32 * 0.4).ceil() as usize).max(1);
        let mut selected_indices: Vec<usize> = scored_sentences[..keep_count].iter().map(|(i, _, _)| *i).collect();

        // Sort indices to maintain original order
        selected_indices.sort();

        // Reconstruct text with selected sentences
        let selected_sentences: Vec<&str> = selected_indices
            .iter()
            .filter_map(|&i| sentences.get(i))
            .copied()
            .collect();

        if selected_sentences.is_empty() {
            text.to_string()
        } else {
            selected_sentences.join(". ")
        }
    }

    /// Score individual sentence importance using universal patterns (language-agnostic)
    fn score_sentence_importance(&self, sentence: &str, position: usize, total_sentences: usize) -> f32 {
        let mut score = 0.0;

        // Position bonus (universal structural importance)
        if position == 0 || position == total_sentences - 1 {
            score += 0.3;
        }

        let words: Vec<&str> = sentence.split_whitespace().collect();
        if words.is_empty() {
            return score;
        }

        // Length scoring (universal information density pattern)
        let word_count = words.len();
        if (3..=25).contains(&word_count) {
            score += 0.2;
        }

        // Universal importance markers
        let mut numeric_count = 0;
        let mut caps_count = 0;
        let mut long_word_count = 0;
        let mut punct_density = 0;

        for word in &words {
            // Numbers (universal importance across all languages)
            if word.chars().any(|c| c.is_numeric()) {
                numeric_count += 1;
            }

            // ALL-CAPS words (universal emphasis pattern)
            if word.len() > 1 && word.chars().all(|c| c.is_uppercase()) {
                caps_count += 1;
            }

            // Long words (likely technical/important terms - universal)
            if word.len() > 8 {
                long_word_count += 1;
            }

            // Punctuation density (structural importance)
            punct_density += word.chars().filter(|c| c.is_ascii_punctuation()).count();
        }

        // Apply universal scoring
        score += (numeric_count as f32 / words.len() as f32) * 0.3; // Number density
        score += (caps_count as f32 / words.len() as f32) * 0.25; // Emphasis density
        score += (long_word_count as f32 / words.len() as f32) * 0.2; // Technical term density
        score += (punct_density as f32 / sentence.len() as f32) * 0.15; // Structural density

        // Lexical diversity (universal information measure)
        let unique_words: std::collections::HashSet<_> = words
            .iter()
            .map(|w| {
                w.chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase()
            })
            .collect();
        let diversity_ratio = unique_words.len() as f32 / words.len() as f32;
        score += diversity_ratio * 0.15;

        // Character entropy (universal information density)
        let char_entropy = self.calculate_char_entropy(sentence);
        score += char_entropy * 0.1;

        score
    }

    /// Universal tokenization that works for languages with and without whitespace
    fn universal_tokenize(&self, text: &str) -> Vec<String> {
        // Use CJK tokenizer for automatic detection and handling
        self.cjk_tokenizer.tokenize_mixed_text(text)
    }

    /// Calculate character-level entropy for information density (language-agnostic)
    fn calculate_char_entropy(&self, text: &str) -> f32 {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return 0.0;
        }

        let mut char_freq = std::collections::HashMap::new();
        for &ch in &chars {
            *char_freq.entry(ch.to_lowercase().next().unwrap_or(ch)).or_insert(0) += 1;
        }

        let total_chars = chars.len() as f32;
        char_freq
            .values()
            .map(|&freq| {
                let p = freq as f32 / total_chars;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            .min(5.0) // Cap entropy for normalization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_reduction() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Light,
            use_simd: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Hello   world!!!   How are you???";
        let result = reducer.reduce(input);

        assert!(result.len() < input.len());
        assert!(!result.contains("   ")); // Multiple spaces removed
    }

    #[test]
    fn test_moderate_reduction() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Moderate,
            use_simd: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("en")).unwrap();
        let input = "The quick brown fox is jumping over the lazy dog";
        let result = reducer.reduce(input);

        // Should remove some stopwords like "the", "is", "over"
        assert!(result.len() < input.len());
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
        assert!(result.contains("fox"));
    }

    #[test]
    fn test_batch_processing() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Light,
            enable_parallel: false, // Test sequential first
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let inputs = vec!["Hello  world!", "How   are you?", "Fine,  thanks!"];
        let results = reducer.batch_reduce(&inputs);

        assert_eq!(results.len(), inputs.len());
        for result in &results {
            assert!(!result.contains("  ")); // No double spaces
        }
    }

    #[test]
    fn test_aggressive_reduction() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Aggressive,
            use_simd: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("en")).unwrap();
        let input = "The quick brown fox is jumping over the lazy dog and running through the forest";
        let result = reducer.reduce(input);

        // Should reduce significantly
        assert!(result.len() < input.len());
        // Should keep some content
        assert!(!result.is_empty());
    }

    #[test]
    fn test_maximum_reduction() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Maximum,
            use_simd: false,
            enable_semantic_clustering: true,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("en")).unwrap();
        let input = "The quick brown fox is jumping over the lazy dog and running through the forest";
        let result = reducer.reduce(input);

        // Should apply significant reduction
        assert!(result.len() < input.len());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_empty_text_handling() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        assert_eq!(reducer.reduce(""), "");
        // Whitespace-only text might be trimmed in some modes
        let result = reducer.reduce("   ");
        assert!(result == "   " || result == "");
    }

    #[test]
    fn test_off_mode_preserves_text() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Off,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Text   with    multiple   spaces!!!";
        assert_eq!(reducer.reduce(input), input);
    }

    #[test]
    fn test_parallel_batch_processing() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Light,
            enable_parallel: true,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let inputs = vec![
            "First text  with spaces",
            "Second  text with  spaces",
            "Third   text  with spaces",
        ];
        let results = reducer.batch_reduce(&inputs);

        assert_eq!(results.len(), inputs.len());
        for result in &results {
            assert!(!result.contains("  "));
        }
    }

    #[test]
    fn test_cjk_text_handling() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("zh")).unwrap();
        let input = "这是中文文本测试";
        let result = reducer.reduce(input);

        // Should handle CJK text properly
        assert!(!result.is_empty());
    }

    #[test]
    fn test_mixed_language_text() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "This is English text 这是中文 and some more English";
        let result = reducer.reduce(input);

        assert!(!result.is_empty());
        // Should preserve some content from both languages
        assert!(result.contains("English") || result.contains("中"));
    }

    #[test]
    fn test_punctuation_normalization() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Light,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Text!!!!!! with????? excessive,,,,,, punctuation";
        let result = reducer.reduce(input);

        assert!(!result.contains("!!!!!!"));
        assert!(!result.contains("?????"));
        assert!(!result.contains(",,,,,,"));
    }

    #[test]
    fn test_sentence_selection() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "First sentence here. Second sentence with more words. Third one. Fourth sentence is even longer than the others.";
        let result = reducer.reduce(input);

        // Should keep some but not all sentences
        assert!(result.len() < input.len());
        assert!(result.split(". ").count() < 4);
    }

    #[test]
    fn test_unicode_normalization_ascii() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Light,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Pure ASCII text without special characters";
        let result = reducer.reduce(input);

        // ASCII text should skip normalization for performance
        assert!(result.contains("ASCII"));
    }

    #[test]
    fn test_unicode_normalization_non_ascii() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Light,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Café naïve résumé"; // Contains non-ASCII characters
        let result = reducer.reduce(input);

        assert!(result.contains("Café") || result.contains("Cafe"));
    }

    #[test]
    fn test_single_text_vs_batch() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let text = "The quick brown fox jumps over the lazy dog";

        let single_result = reducer.reduce(text);
        let batch_results = reducer.batch_reduce(&vec![text]);

        assert_eq!(single_result, batch_results[0]);
    }

    #[test]
    fn test_important_word_preservation() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "The IMPORTANT word COVID-19 and 12345 numbers should be preserved";
        let result = reducer.reduce(input);

        // Should keep all-caps words, words with numbers
        assert!(result.contains("IMPORTANT") || result.contains("COVID") || result.contains("12345"));
    }

    #[test]
    fn test_technical_terms_preservation() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "The implementation uses PyTorch and TensorFlow frameworks";
        let result = reducer.reduce(input);

        // Should keep technical terms (mixed case)
        assert!(result.contains("PyTorch") || result.contains("TensorFlow"));
    }

    #[test]
    fn test_calculate_char_entropy() {
        let config = TokenReductionConfig::default();
        let reducer = TokenReducer::new(&config, None).unwrap();

        // Repetitive text should have low entropy
        let low_entropy = reducer.calculate_char_entropy("aaaaaaa");
        assert!(low_entropy < 1.0);

        // Diverse text should have higher entropy
        let high_entropy = reducer.calculate_char_entropy("abcdefg123");
        assert!(high_entropy > low_entropy);
    }

    #[test]
    fn test_universal_tokenize_english() {
        let config = TokenReductionConfig::default();
        let reducer = TokenReducer::new(&config, None).unwrap();

        let tokens = reducer.universal_tokenize("hello world test");
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_universal_tokenize_cjk() {
        let config = TokenReductionConfig::default();
        let reducer = TokenReducer::new(&config, None).unwrap();

        let tokens = reducer.universal_tokenize("中文");
        // Should split into character pairs for CJK
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_fallback_threshold() {
        let config = TokenReductionConfig {
            level: ReductionLevel::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();

        // Text with mostly short common words
        let input = "a the is of to in for on at by";
        let result = reducer.reduce(input);

        // Should not reduce to empty even with aggressive filtering
        assert!(!result.is_empty());
    }
}
