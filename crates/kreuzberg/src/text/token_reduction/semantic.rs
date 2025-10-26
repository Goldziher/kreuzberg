use ahash::AHashMap;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
struct ScoredToken {
    token: String,
    position: usize,
    importance_score: f32,
    #[allow(dead_code)]
    context_boost: f32,
    #[allow(dead_code)]
    frequency_score: f32,
}

impl PartialEq for ScoredToken {
    fn eq(&self, other: &Self) -> bool {
        self.importance_score == other.importance_score
    }
}

impl Eq for ScoredToken {}

impl PartialOrd for ScoredToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredToken {
    fn cmp(&self, other: &Self) -> Ordering {
        self.importance_score
            .partial_cmp(&other.importance_score)
            .unwrap_or(Ordering::Equal)
    }
}

pub struct SemanticAnalyzer {
    #[allow(dead_code)]
    language: String,
    importance_weights: AHashMap<String, f32>,
    hypernyms: AHashMap<String, String>,
    semantic_clusters: AHashMap<String, Vec<String>>,
}

impl SemanticAnalyzer {
    pub fn new(language: &str) -> Self {
        let mut analyzer = Self {
            language: language.to_string(),
            importance_weights: AHashMap::new(),
            hypernyms: AHashMap::new(),
            semantic_clusters: AHashMap::new(),
        };

        analyzer.initialize_importance_weights();
        analyzer.initialize_hypernyms();
        analyzer.initialize_semantic_clusters();

        analyzer
    }

    pub fn apply_semantic_filtering(&self, text: &str, threshold: f32) -> String {
        let tokens = self.tokenize_and_score(text);
        let filtered_tokens = self.filter_by_importance(tokens, threshold);
        self.reconstruct_text(filtered_tokens)
    }

    pub fn apply_hypernym_compression(&self, text: &str, target_reduction: Option<f32>) -> String {
        let tokens = self.tokenize_and_score(text);
        let compressed_tokens = self.compress_with_hypernyms(tokens, target_reduction);
        self.reconstruct_text(compressed_tokens)
    }

    fn tokenize_and_score(&self, text: &str) -> Vec<ScoredToken> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut scored_tokens = Vec::with_capacity(words.len());

        let mut word_freq = AHashMap::new();
        for word in &words {
            let clean_word = self.clean_word(word);
            *word_freq.entry(clean_word).or_insert(0) += 1;
        }

        for (position, word) in words.iter().enumerate() {
            let clean_word = self.clean_word(word);
            let base_importance = self.calculate_base_importance(&clean_word);
            let context_boost = self.calculate_context_boost(&clean_word, position, &words);
            let frequency_score = self.calculate_frequency_score(&clean_word, &word_freq, words.len());

            let total_score = base_importance + context_boost + frequency_score;

            scored_tokens.push(ScoredToken {
                token: word.to_string(),
                position,
                importance_score: total_score,
                context_boost,
                frequency_score,
            });
        }

        scored_tokens
    }

    fn filter_by_importance(&self, tokens: Vec<ScoredToken>, threshold: f32) -> Vec<ScoredToken> {
        tokens
            .into_iter()
            .filter(|token| token.importance_score >= threshold)
            .collect()
    }

    fn compress_with_hypernyms(&self, tokens: Vec<ScoredToken>, target_reduction: Option<f32>) -> Vec<ScoredToken> {
        let mut result = tokens;

        if let Some(target) = target_reduction {
            let target_count = ((1.0 - target) * result.len() as f32) as usize;

            result.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());

            for token in result.iter_mut().skip(target_count) {
                if let Some(hypernym) = self.get_hypernym(&token.token) {
                    token.token = hypernym;
                    token.importance_score *= 0.8;
                }
            }

            result.truncate(target_count.max(1));
        } else {
            for token in &mut result {
                if token.importance_score < 0.5
                    && let Some(hypernym) = self.get_hypernym(&token.token)
                {
                    token.token = hypernym; // FIXME: check value is used after being moved
                }
            }
        }

        result.sort_by_key(|token| token.position);
        result
    }

    fn reconstruct_text(&self, tokens: Vec<ScoredToken>) -> String {
        tokens
            .into_iter()
            .map(|token| token.token)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn calculate_base_importance(&self, word: &str) -> f32 {
        if let Some(&weight) = self.importance_weights.get(word) {
            return weight;
        }

        let mut score = 0.3;

        score += (word.len() as f32 * 0.02).min(0.2);

        if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            score += 0.2;
        }

        if word.chars().any(|c| c.is_numeric()) {
            score += 0.15;
        }

        if self.is_technical_term(word) {
            score += 0.25;
        }

        score.min(1.0)
    }

    fn calculate_context_boost(&self, word: &str, position: usize, words: &[&str]) -> f32 {
        let mut boost = 0.0;

        if position == 0 || position == words.len() - 1 {
            boost += 0.1;
        }

        let window = 2;
        let start = position.saturating_sub(window);
        let end = (position + window + 1).min(words.len());

        for &context_word in &words[start..end] {
            if context_word != word {
                boost += self.calculate_contextual_weight(word, context_word);
            }
        }

        boost.min(0.3)
    }

    fn calculate_frequency_score(&self, word: &str, word_freq: &AHashMap<String, i32>, total_words: usize) -> f32 {
        if let Some(&freq) = word_freq.get(word) {
            let tf = freq as f32 / total_words as f32;

            (tf.ln() + 1.0) * 0.1
        } else {
            0.0
        }
    }

    fn calculate_contextual_weight(&self, word: &str, context_word: &str) -> f32 {
        if self.is_technical_term(word) && self.is_technical_term(context_word) {
            0.05
        } else if context_word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            0.02
        } else {
            0.0
        }
    }

    fn is_technical_term(&self, word: &str) -> bool {
        word.len() > 6
            && (word.contains("_")
                || word.chars().filter(|&c| c.is_uppercase()).count() > 1
                || word.ends_with("tion")
                || word.ends_with("ment")
                || word.ends_with("ing"))
    }

    fn get_hypernym(&self, word: &str) -> Option<String> {
        let clean_word = self.clean_word(word).to_lowercase();
        self.hypernyms.get(&clean_word).cloned()
    }

    fn clean_word(&self, word: &str) -> String {
        word.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    }

    fn initialize_importance_weights(&mut self) {
        let high_importance = [
            ("result", 0.8),
            ("conclusion", 0.8),
            ("important", 0.7),
            ("significant", 0.7),
            ("analysis", 0.7),
            ("method", 0.6),
            ("data", 0.6),
            ("system", 0.6),
            ("performance", 0.6),
            ("improvement", 0.6),
        ];

        for (word, score) in &high_importance {
            self.importance_weights.insert(word.to_string(), *score);
        }

        let medium_importance = [
            ("process", 0.5),
            ("algorithm", 0.5),
            ("function", 0.5),
            ("model", 0.5),
            ("implementation", 0.5),
        ];

        for (word, score) in &medium_importance {
            self.importance_weights.insert(word.to_string(), *score);
        }
    }

    fn initialize_hypernyms(&mut self) {
        let hypernym_pairs = [
            ("car", "vehicle"),
            ("dog", "animal"),
            ("apple", "fruit"),
            ("chair", "furniture"),
            ("book", "publication"),
            ("computer", "device"),
            ("algorithm", "method"),
            ("implementation", "approach"),
            ("optimization", "improvement"),
            ("analysis", "study"),
        ];

        for (word, hypernym) in &hypernym_pairs {
            self.hypernyms.insert(word.to_string(), hypernym.to_string());
        }
    }

    fn initialize_semantic_clusters(&mut self) {
        self.semantic_clusters.insert(
            "computing".to_string(),
            vec![
                "computer".to_string(),
                "algorithm".to_string(),
                "software".to_string(),
                "programming".to_string(),
                "code".to_string(),
            ],
        );

        self.semantic_clusters.insert(
            "analysis".to_string(),
            vec![
                "analysis".to_string(),
                "study".to_string(),
                "research".to_string(),
                "investigation".to_string(),
                "examination".to_string(),
            ],
        );

        self.semantic_clusters.insert(
            "performance".to_string(),
            vec![
                "performance".to_string(),
                "speed".to_string(),
                "efficiency".to_string(),
                "optimization".to_string(),
                "improvement".to_string(),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_filtering() {
        let analyzer = SemanticAnalyzer::new("en");
        let input = "The quick brown fox jumps over the lazy dog with great performance";
        let result = analyzer.apply_semantic_filtering(input, 0.4);

        assert!(result.contains("performance") || result.contains("fox") || result.contains("dog"));
        assert!(result.len() < input.len());
    }

    #[test]
    fn test_hypernym_compression() {
        let analyzer = SemanticAnalyzer::new("en");
        let input = "The car drove past the dog near the apple tree";
        let result = analyzer.apply_hypernym_compression(input, Some(0.5));

        let original_words = input.split_whitespace().count();
        let result_words = result.split_whitespace().count();
        assert!(result_words <= (original_words as f32 * 0.5) as usize + 1);
    }

    #[test]
    fn test_importance_scoring() {
        let analyzer = SemanticAnalyzer::new("en");
        let tokens = analyzer.tokenize_and_score("The important analysis shows significant results");

        let important_token = tokens.iter().find(|t| t.token == "important").unwrap();
        let analysis_token = tokens.iter().find(|t| t.token == "analysis").unwrap();
        let the_token = tokens.iter().find(|t| t.token == "The").unwrap();

        assert!(important_token.importance_score > the_token.importance_score);
        assert!(analysis_token.importance_score > the_token.importance_score);
    }

    #[test]
    fn test_semantic_filtering_empty_text() {
        let analyzer = SemanticAnalyzer::new("en");
        let result = analyzer.apply_semantic_filtering("", 0.5);
        assert_eq!(result, "");
    }

    #[test]
    fn test_semantic_filtering_high_threshold() {
        let analyzer = SemanticAnalyzer::new("en");
        let input = "The quick brown fox";
        let result = analyzer.apply_semantic_filtering(input, 0.9);
        assert!(result.len() <= input.len());
    }

    #[test]
    fn test_hypernym_compression_without_target() {
        let analyzer = SemanticAnalyzer::new("en");
        let input = "The car drove past the dog";
        let result = analyzer.apply_hypernym_compression(input, None);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_technical_term_detection() {
        let analyzer = SemanticAnalyzer::new("en");

        assert!(analyzer.is_technical_term("implementation"));
        assert!(analyzer.is_technical_term("optimization"));
        assert!(analyzer.is_technical_term("processing"));
        assert!(analyzer.is_technical_term("HTTP_SERVER"));
        assert!(!analyzer.is_technical_term("cat"));
        assert!(!analyzer.is_technical_term("dog"));
    }

    #[test]
    fn test_clean_word() {
        let analyzer = SemanticAnalyzer::new("en");

        assert_eq!(analyzer.clean_word("Hello!"), "hello");
        assert_eq!(analyzer.clean_word("test123"), "test123");
        assert_eq!(analyzer.clean_word("word,"), "word");
        assert_eq!(analyzer.clean_word("(test)"), "test");
    }

    #[test]
    fn test_calculate_base_importance() {
        let analyzer = SemanticAnalyzer::new("en");

        let result_score = analyzer.calculate_base_importance("result");
        let conclusion_score = analyzer.calculate_base_importance("conclusion");

        assert!(result_score > 0.5);
        assert!(conclusion_score > 0.5);

        let process_score = analyzer.calculate_base_importance("process");
        assert!(process_score >= 0.4);

        let regular_score = analyzer.calculate_base_importance("cat");
        assert!(regular_score < result_score);
    }

    #[test]
    fn test_calculate_base_importance_uppercase() {
        let analyzer = SemanticAnalyzer::new("en");

        let uppercase_score = analyzer.calculate_base_importance("Test");
        let lowercase_score = analyzer.calculate_base_importance("test");

        assert!(uppercase_score > lowercase_score);
    }

    #[test]
    fn test_calculate_base_importance_with_numbers() {
        let analyzer = SemanticAnalyzer::new("en");

        let with_number = analyzer.calculate_base_importance("test123");
        let without_number = analyzer.calculate_base_importance("test");

        assert!(with_number > without_number);
    }

    #[test]
    fn test_calculate_base_importance_length_bonus() {
        let analyzer = SemanticAnalyzer::new("en");

        let long_word = analyzer.calculate_base_importance("verylongword");
        let short_word = analyzer.calculate_base_importance("cat");

        assert!(long_word > short_word);
    }

    #[test]
    fn test_get_hypernym() {
        let analyzer = SemanticAnalyzer::new("en");

        assert_eq!(analyzer.get_hypernym("car"), Some("vehicle".to_string()));
        assert_eq!(analyzer.get_hypernym("dog"), Some("animal".to_string()));
        assert_eq!(analyzer.get_hypernym("apple"), Some("fruit".to_string()));
        assert_eq!(analyzer.get_hypernym("unknown"), None);
    }

    #[test]
    fn test_get_hypernym_case_insensitive() {
        let analyzer = SemanticAnalyzer::new("en");

        assert_eq!(analyzer.get_hypernym("CAR"), Some("vehicle".to_string()));
        assert_eq!(analyzer.get_hypernym("Dog"), Some("animal".to_string()));
    }

    #[test]
    fn test_tokenize_and_score_positions() {
        let analyzer = SemanticAnalyzer::new("en");
        let tokens = analyzer.tokenize_and_score("first middle last");

        assert_eq!(tokens[0].position, 0);
        assert_eq!(tokens[1].position, 1);
        assert_eq!(tokens[2].position, 2);
    }

    #[test]
    fn test_context_boost_for_edge_positions() {
        let analyzer = SemanticAnalyzer::new("en");
        let tokens = analyzer.tokenize_and_score("first middle last");

        assert!(tokens[0].importance_score > 0.0);
        assert!(tokens[2].importance_score > 0.0);
    }

    #[test]
    fn test_frequency_score() {
        let analyzer = SemanticAnalyzer::new("en");
        let tokens = analyzer.tokenize_and_score("test test test other");

        let test_token = tokens.iter().find(|t| t.token == "test").unwrap();
        let other_token = tokens.iter().find(|t| t.token == "other").unwrap();

        assert!(test_token.frequency_score > other_token.frequency_score);
    }

    #[test]
    fn test_scored_token_ordering() {
        let token1 = ScoredToken {
            token: "a".to_string(),
            position: 0,
            importance_score: 0.5,
            context_boost: 0.0,
            frequency_score: 0.0,
        };

        let token2 = ScoredToken {
            token: "b".to_string(),
            position: 1,
            importance_score: 0.7,
            context_boost: 0.0,
            frequency_score: 0.0,
        };

        assert!(token2 > token1);
        assert_eq!(token1, token1.clone());
    }

    #[test]
    fn test_reconstruct_text() {
        let analyzer = SemanticAnalyzer::new("en");
        let tokens = vec![
            ScoredToken {
                token: "Hello".to_string(),
                position: 0,
                importance_score: 0.5,
                context_boost: 0.0,
                frequency_score: 0.0,
            },
            ScoredToken {
                token: "world".to_string(),
                position: 1,
                importance_score: 0.5,
                context_boost: 0.0,
                frequency_score: 0.0,
            },
        ];

        let result = analyzer.reconstruct_text(tokens);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_compress_with_hypernyms_respects_target() {
        let analyzer = SemanticAnalyzer::new("en");
        let tokens = vec![
            ScoredToken {
                token: "car".to_string(),
                position: 0,
                importance_score: 0.3,
                context_boost: 0.0,
                frequency_score: 0.0,
            },
            ScoredToken {
                token: "dog".to_string(),
                position: 1,
                importance_score: 0.3,
                context_boost: 0.0,
                frequency_score: 0.0,
            },
            ScoredToken {
                token: "test".to_string(),
                position: 2,
                importance_score: 0.8,
                context_boost: 0.0,
                frequency_score: 0.0,
            },
        ];

        let result = analyzer.compress_with_hypernyms(tokens, Some(0.5));
        assert!(result.len() <= 2);
    }

    #[test]
    fn test_initialize_importance_weights() {
        let analyzer = SemanticAnalyzer::new("en");

        assert!(analyzer.importance_weights.contains_key("result"));
        assert!(analyzer.importance_weights.contains_key("conclusion"));
        assert!(analyzer.importance_weights.contains_key("important"));
        assert!(analyzer.importance_weights.contains_key("process"));
    }

    #[test]
    fn test_initialize_hypernyms() {
        let analyzer = SemanticAnalyzer::new("en");

        assert!(analyzer.hypernyms.contains_key("car"));
        assert!(analyzer.hypernyms.contains_key("dog"));
        assert!(analyzer.hypernyms.contains_key("apple"));
    }

    #[test]
    fn test_initialize_semantic_clusters() {
        let analyzer = SemanticAnalyzer::new("en");

        assert!(analyzer.semantic_clusters.contains_key("computing"));
        assert!(analyzer.semantic_clusters.contains_key("analysis"));
        assert!(analyzer.semantic_clusters.contains_key("performance"));
    }

    #[test]
    fn test_contextual_weight_technical_terms() {
        let analyzer = SemanticAnalyzer::new("en");

        let weight = analyzer.calculate_contextual_weight("implementation", "optimization");
        assert!(weight > 0.0);
    }

    #[test]
    fn test_hypernym_compression_zero_target() {
        let analyzer = SemanticAnalyzer::new("en");
        let input = "The car drove fast";
        let result = analyzer.apply_hypernym_compression(input, Some(0.0));
        assert!(!result.is_empty());
    }
}
