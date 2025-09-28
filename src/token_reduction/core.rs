//! Core token reduction engine with parallel processing and semantic awareness.

use crate::token_reduction::{
    cjk_utils::CjkTokenizer,
    config::{ReductionLevelDTO, TokenReductionConfigDTO},
    filters::FilterPipeline,
    semantic::SemanticAnalyzer,
    simd_text::{SimdTextProcessor, chunk_text_for_parallel},
};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use rayon::prelude::*;
use regex::Regex;
use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;

/// Pre-compiled regex patterns for performance
static REPEATED_EXCLAMATION: Lazy<Regex> = Lazy::new(|| Regex::new(r"[!]{2,}").unwrap());
static REPEATED_QUESTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"[?]{2,}").unwrap());
static REPEATED_COMMA: Lazy<Regex> = Lazy::new(|| Regex::new(r"[,]{2,}").unwrap());

/// High-performance token reduction engine
pub struct TokenReducer {
    config: Arc<TokenReductionConfigDTO>,
    text_processor: SimdTextProcessor,
    filter_pipeline: FilterPipeline,
    semantic_analyzer: Option<SemanticAnalyzer>,
    cjk_tokenizer: CjkTokenizer,
    #[allow(dead_code)]
    language: String,
}

impl TokenReducer {
    /// Create a new token reducer with the given configuration
    pub fn new(config: &TokenReductionConfigDTO, language_hint: Option<&str>) -> PyResult<Self> {
        let config = Arc::new(config.clone());
        let language = language_hint
            .or(config.language_hint.as_deref())
            .unwrap_or("en")
            .to_string();

        let text_processor = SimdTextProcessor::new();
        let filter_pipeline = FilterPipeline::new(&config, &language)?;

        let semantic_analyzer = if matches!(config.level, ReductionLevelDTO::Aggressive | ReductionLevelDTO::Maximum) {
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
        if text.is_empty() || matches!(self.config.level, ReductionLevelDTO::Off) {
            return text.to_string();
        }

        let working_text = if text.is_ascii() {
            text
        } else {
            &text.nfc().collect::<String>()
        };

        match self.config.level {
            ReductionLevelDTO::Off => working_text.to_string(),
            ReductionLevelDTO::Light => self.apply_light_reduction_optimized(working_text),
            ReductionLevelDTO::Moderate => self.apply_moderate_reduction_optimized(working_text),
            ReductionLevelDTO::Aggressive => self.apply_aggressive_reduction_optimized(working_text),
            ReductionLevelDTO::Maximum => self.apply_maximum_reduction_optimized(working_text),
        }
    }

    /// Reduce tokens in multiple texts using optimized parallel processing
    pub fn batch_reduce(&self, texts: &[&str]) -> Vec<String> {
        if !self.config.enable_parallel || texts.len() < 2 {
            return texts.iter().map(|text| self.reduce(text)).collect();
        }

        texts.par_iter().map(|text| self.reduce(text)).collect()
    }

    /// Apply optimized light reduction: formatting cleanup only
    fn apply_light_reduction_optimized(&self, text: &str) -> String {
        let mut result = if self.config.use_simd {
            self.text_processor.clean_punctuation(text)
        } else {
            self.clean_punctuation_optimized(text)
        };

        result = self.filter_pipeline.apply_light_filters(&result);
        result.trim().to_string()
    }

    /// Apply optimized moderate reduction: traditional stopword removal
    fn apply_moderate_reduction_optimized(&self, text: &str) -> String {
        let mut result = self.apply_light_reduction_optimized(text);

        result = if self.config.enable_parallel && text.len() > 1000 {
            self.apply_parallel_moderate_reduction(&result)
        } else {
            self.filter_pipeline.apply_moderate_filters(&result)
        };

        result
    }

    /// Apply optimized aggressive reduction: moderate + semantic filtering
    fn apply_aggressive_reduction_optimized(&self, text: &str) -> String {
        let mut result = self.apply_moderate_reduction_optimized(text);

        result = self.remove_additional_common_words(&result);
        result = self.apply_sentence_selection(&result);

        if let Some(ref analyzer) = self.semantic_analyzer {
            result = analyzer.apply_semantic_filtering(&result, self.config.semantic_threshold);
        }

        result
    }

    /// Apply optimized maximum reduction: hypernym compression
    fn apply_maximum_reduction_optimized(&self, text: &str) -> String {
        let mut result = self.apply_aggressive_reduction_optimized(text);

        if let Some(ref analyzer) = self.semantic_analyzer
            && self.config.enable_semantic_clustering
        {
            result = analyzer.apply_hypernym_compression(&result, self.config.target_reduction);
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

    /// Optimized punctuation cleanup using pre-compiled regex patterns
    fn clean_punctuation_optimized(&self, text: &str) -> String {
        let mut result = text.to_string();

        result = REPEATED_EXCLAMATION.replace_all(&result, "!").to_string();
        result = REPEATED_QUESTION.replace_all(&result, "?").to_string();
        result = REPEATED_COMMA.replace_all(&result, ",").to_string();

        result
    }

    /// Remove additional words using statistical frequency analysis (language-agnostic)
    fn remove_additional_common_words(&self, text: &str) -> String {
        let words = self.universal_tokenize(text);

        if words.len() < 4 {
            return text.to_string();
        }

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
                word_lengths.push(clean_word.chars().count());
            }
        }

        let avg_length = if !word_lengths.is_empty() {
            word_lengths.iter().sum::<usize>() as f32 / word_lengths.len() as f32
        } else {
            5.0
        };

        let original_count = words.len();

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
                    return true;
                }

                let freq = word_freq.get(&clean_word).unwrap_or(&0);
                let word_len = clean_word.chars().count() as f32;

                self.has_important_characteristics(word)
                    || (*freq <= 2 && word_len >= avg_length * 0.8)
                    || (word_len >= avg_length * 1.5)
            })
            .cloned()
            .collect();

        let has_cjk_content = text.chars().any(|c| c as u32 >= 0x4E00 && (c as u32) <= 0x9FFF);
        let fallback_threshold = if has_cjk_content {
            original_count / 5
        } else {
            original_count / 3
        };

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
            tokens.join("")
        } else {
            tokens.join(" ")
        }
    }

    /// Check for language-agnostic important word characteristics
    fn has_important_characteristics(&self, word: &str) -> bool {
        if word.len() > 1 && word.chars().all(|c| c.is_uppercase()) {
            return true;
        }

        if word.chars().any(|c| c.is_numeric()) {
            return true;
        }

        if word.len() > 10 {
            return true;
        }

        let uppercase_count = word.chars().filter(|c| c.is_uppercase()).count();
        if uppercase_count > 1 && uppercase_count < word.len() {
            return true;
        }

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

        let has_cjk = chars.iter().any(|&c| c as u32 >= 0x4E00 && (c as u32) <= 0x9FFF);
        if !has_cjk {
            return false;
        }

        let important_radicals = [
            '学', '智', '能', '技', '术', '法', '算', '理', '科', '研', '究', '发', '展', '系', '统', '模', '型', '方',
            '式', '过', '程', '结', '构', '功', '效', '应', '分', '析', '计', '算', '数', '据', '信', '息', '处', '理',
            '语', '言', '文', '生', '成', '产', '用', '作', '为', '成', '变', '化', '转', '换', '提', '高', '网', '络',
            '神', '经', '机', '器', '人', '工', '智', '能', '自', '然', '复',
        ];

        for &char in &chars {
            if important_radicals.contains(&char) {
                return true;
            }
        }

        if chars.len() == 2 && has_cjk {
            let has_technical = chars.iter().any(|&c| {
                let code = c as u32;
                (0x4E00..=0x4FFF).contains(&code)
                    || (0x5000..=0x51FF).contains(&code)
                    || (0x6700..=0x68FF).contains(&code)
                    || (0x7500..=0x76FF).contains(&code)
            });

            if has_technical {
                return true;
            }
        }

        false
    }

    /// Apply sentence-level selection to keep only most important sentences
    fn apply_sentence_selection(&self, text: &str) -> String {
        let sentences: Vec<&str> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if sentences.len() <= 2 {
            return text.to_string();
        }

        let mut scored_sentences: Vec<(usize, f32, &str)> = sentences
            .iter()
            .enumerate()
            .map(|(i, sentence)| {
                let score = self.score_sentence_importance(sentence, i, sentences.len());
                (i, score, *sentence)
            })
            .collect();

        scored_sentences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let keep_count = ((sentences.len() as f32 * 0.4).ceil() as usize).max(1);
        let mut selected_indices: Vec<usize> = scored_sentences[..keep_count].iter().map(|(i, _, _)| *i).collect();

        selected_indices.sort();

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

        if position == 0 || position == total_sentences - 1 {
            score += 0.3;
        }

        let words: Vec<&str> = sentence.split_whitespace().collect();
        if words.is_empty() {
            return score;
        }

        let word_count = words.len();
        if (3..=25).contains(&word_count) {
            score += 0.2;
        }

        let mut numeric_count = 0;
        let mut caps_count = 0;
        let mut long_word_count = 0;
        let mut punct_density = 0;

        for word in &words {
            if word.chars().any(|c| c.is_numeric()) {
                numeric_count += 1;
            }

            if word.len() > 1 && word.chars().all(|c| c.is_uppercase()) {
                caps_count += 1;
            }

            if word.len() > 8 {
                long_word_count += 1;
            }

            punct_density += word.chars().filter(|c| c.is_ascii_punctuation()).count();
        }

        score += (numeric_count as f32 / words.len() as f32) * 0.3;
        score += (caps_count as f32 / words.len() as f32) * 0.25;
        score += (long_word_count as f32 / words.len() as f32) * 0.2;
        score += (punct_density as f32 / sentence.len() as f32) * 0.15;

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

        let char_entropy = self.calculate_char_entropy(sentence);
        score += char_entropy * 0.1;

        score
    }

    /// Universal tokenization that works for languages with and without whitespace
    fn universal_tokenize(&self, text: &str) -> Vec<String> {
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
                if p > 0.0 { -p * p.log2() } else { 0.0 }
            })
            .sum::<f32>()
            .min(5.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_reduction() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Light,
            use_simd: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Hello   world!!!   How are you???";
        let result = reducer.reduce(input);

        assert!(result.len() < input.len());
        assert!(!result.contains("   "));
    }

    #[test]
    fn test_moderate_reduction() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Moderate,
            use_simd: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("en")).unwrap();
        let input = "The quick brown fox is jumping over the lazy dog";
        let result = reducer.reduce(input);

        assert!(result.len() < input.len());
        assert!(result.contains("quick"));
        assert!(result.contains("brown"));
        assert!(result.contains("fox"));
    }

    #[test]
    fn test_batch_processing() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Light,
            enable_parallel: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let inputs = vec!["Hello  world!", "How   are you?", "Fine,  thanks!"];
        let results = reducer.batch_reduce(&inputs);

        assert_eq!(results.len(), inputs.len());
        for result in &results {
            assert!(!result.contains("  "));
        }
    }

    #[test]
    fn test_aggressive_reduction() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Aggressive,
            use_simd: false,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("en")).unwrap();
        let input = "The quick brown fox is jumping over the lazy dog and running through the forest";
        let result = reducer.reduce(input);

        assert!(result.len() < input.len());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_maximum_reduction() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Maximum,
            use_simd: false,
            enable_semantic_clustering: true,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("en")).unwrap();
        let input = "The quick brown fox is jumping over the lazy dog and running through the forest";
        let result = reducer.reduce(input);

        assert!(result.len() < input.len());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_empty_text_handling() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        assert_eq!(reducer.reduce(""), "");
        let result = reducer.reduce("   ");
        assert!(result == "   " || result == "");
    }

    #[test]
    fn test_off_mode_preserves_text() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Off,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Text   with    multiple   spaces!!!";
        assert_eq!(reducer.reduce(input), input);
    }

    #[test]
    fn test_parallel_batch_processing() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Light,
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
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, Some("zh")).unwrap();
        let input = "这是中文文本测试";
        let result = reducer.reduce(input);

        assert!(!result.is_empty());
    }

    #[test]
    fn test_mixed_language_text() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Moderate,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "This is English text 这是中文 and some more English";
        let result = reducer.reduce(input);

        assert!(!result.is_empty());
        assert!(result.contains("English") || result.contains("中"));
    }

    #[test]
    fn test_punctuation_normalization() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Light,
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
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "First sentence here. Second sentence with more words. Third one. Fourth sentence is even longer than the others.";
        let result = reducer.reduce(input);

        assert!(result.len() < input.len());
        assert!(result.split(". ").count() < 4);
    }

    #[test]
    fn test_unicode_normalization_ascii() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Light,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Pure ASCII text without special characters";
        let result = reducer.reduce(input);

        assert!(result.contains("ASCII"));
    }

    #[test]
    fn test_unicode_normalization_non_ascii() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Light,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "Café naïve résumé";
        let result = reducer.reduce(input);

        assert!(result.contains("Café") || result.contains("Cafe"));
    }

    #[test]
    fn test_single_text_vs_batch() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Moderate,
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
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "The IMPORTANT word COVID-19 and 12345 numbers should be preserved";
        let result = reducer.reduce(input);

        assert!(result.contains("IMPORTANT") || result.contains("COVID") || result.contains("12345"));
    }

    #[test]
    fn test_technical_terms_preservation() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();
        let input = "The implementation uses PyTorch and TensorFlow frameworks";
        let result = reducer.reduce(input);

        assert!(result.contains("PyTorch") || result.contains("TensorFlow"));
    }

    #[test]
    fn test_calculate_char_entropy() {
        let config = TokenReductionConfigDTO::default();
        let reducer = TokenReducer::new(&config, None).unwrap();

        let low_entropy = reducer.calculate_char_entropy("aaaaaaa");
        assert!(low_entropy < 1.0);

        let high_entropy = reducer.calculate_char_entropy("abcdefg123");
        assert!(high_entropy > low_entropy);
    }

    #[test]
    fn test_universal_tokenize_english() {
        let config = TokenReductionConfigDTO::default();
        let reducer = TokenReducer::new(&config, None).unwrap();

        let tokens = reducer.universal_tokenize("hello world test");
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_universal_tokenize_cjk() {
        let config = TokenReductionConfigDTO::default();
        let reducer = TokenReducer::new(&config, None).unwrap();

        let tokens = reducer.universal_tokenize("中文");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_fallback_threshold() {
        let config = TokenReductionConfigDTO {
            level: ReductionLevelDTO::Aggressive,
            ..Default::default()
        };

        let reducer = TokenReducer::new(&config, None).unwrap();

        let input = "a the is of to in for on at by";
        let result = reducer.reduce(input);

        assert!(!result.is_empty());
    }
}
