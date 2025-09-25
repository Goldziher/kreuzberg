//! Semantic analysis and advanced token reduction techniques.
//!
//! Implements modern semantic-aware filtering approaches including:
//! - Semantic importance scoring
//! - Hypernym-based compression
//! - Context-aware token preservation

use ahash::AHashMap;
use pyo3::prelude::*;
use std::cmp::Ordering;

/// Token with semantic importance score
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

/// Semantic analyzer for advanced token reduction
pub struct SemanticAnalyzer {
    #[allow(dead_code)]
    language: String,
    importance_weights: AHashMap<String, f32>,
    hypernyms: AHashMap<String, String>,
    semantic_clusters: AHashMap<String, Vec<String>>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer for the specified language
    pub fn new(language: &str) -> PyResult<Self> {
        let mut analyzer = Self {
            language: language.to_string(),
            importance_weights: AHashMap::new(),
            hypernyms: AHashMap::new(),
            semantic_clusters: AHashMap::new(),
        };

        analyzer.initialize_importance_weights();
        analyzer.initialize_hypernyms();
        analyzer.initialize_semantic_clusters();

        Ok(analyzer)
    }

    /// Apply semantic filtering based on importance scores
    pub fn apply_semantic_filtering(&self, text: &str, threshold: f32) -> String {
        let tokens = self.tokenize_and_score(text);
        let filtered_tokens = self.filter_by_importance(tokens, threshold);
        self.reconstruct_text(filtered_tokens)
    }

    /// Apply hypernym-based compression for maximum reduction
    pub fn apply_hypernym_compression(&self, text: &str, target_reduction: Option<f32>) -> String {
        let tokens = self.tokenize_and_score(text);
        let compressed_tokens = self.compress_with_hypernyms(tokens, target_reduction);
        self.reconstruct_text(compressed_tokens)
    }

    /// Tokenize text and assign importance scores
    fn tokenize_and_score(&self, text: &str) -> Vec<ScoredToken> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut scored_tokens = Vec::with_capacity(words.len());

        // Calculate word frequencies for TF component
        let mut word_freq = AHashMap::new();
        for word in &words {
            let clean_word = self.clean_word(word);
            *word_freq.entry(clean_word).or_insert(0) += 1;
        }

        // Score each token
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

    /// Filter tokens by importance threshold
    fn filter_by_importance(&self, tokens: Vec<ScoredToken>, threshold: f32) -> Vec<ScoredToken> {
        tokens
            .into_iter()
            .filter(|token| token.importance_score >= threshold)
            .collect()
    }

    /// Compress using hypernym substitution
    fn compress_with_hypernyms(&self, tokens: Vec<ScoredToken>, target_reduction: Option<f32>) -> Vec<ScoredToken> {
        let mut result = tokens;

        if let Some(target) = target_reduction {
            let target_count = ((1.0 - target) * result.len() as f32) as usize;

            // Sort by importance (highest first)
            result.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());

            // Apply hypernym substitution to less important tokens
            for token in result.iter_mut().skip(target_count) {
                if let Some(hypernym) = self.get_hypernym(&token.token) {
                    token.token = hypernym;
                    token.importance_score *= 0.8; // Slightly reduce score for hypernyms
                }
            }

            // Remove the least important tokens if we still exceed target
            result.truncate(target_count.max(1));
        } else {
            // Apply hypernym substitution based on semantic clusters
            for token in &mut result {
                if token.importance_score < 0.5 {
                    if let Some(hypernym) = self.get_hypernym(&token.token) {
                        token.token = hypernym;
                    }
                }
            }
        }

        // Sort back by position for natural text flow
        result.sort_by_key(|token| token.position);
        result
    }

    /// Reconstruct text from filtered tokens
    fn reconstruct_text(&self, tokens: Vec<ScoredToken>) -> String {
        tokens
            .into_iter()
            .map(|token| token.token)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Calculate base importance score for a word
    fn calculate_base_importance(&self, word: &str) -> f32 {
        // Check predefined importance weights
        if let Some(&weight) = self.importance_weights.get(word) {
            return weight;
        }

        // Heuristic scoring based on word characteristics
        let mut score = 0.3; // Base score

        // Length bonus (longer words often more important)
        score += (word.len() as f32 * 0.02).min(0.2);

        // Capitalization bonus (proper nouns)
        if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            score += 0.2;
        }

        // Numeric content bonus
        if word.chars().any(|c| c.is_numeric()) {
            score += 0.15;
        }

        // Technical term detection (basic heuristics)
        if self.is_technical_term(word) {
            score += 0.25;
        }

        score.min(1.0)
    }

    /// Calculate context boost based on surrounding words
    fn calculate_context_boost(&self, word: &str, position: usize, words: &[&str]) -> f32 {
        let mut boost = 0.0;

        // Position-based boost (beginning and end of sentences are often important)
        if position == 0 || position == words.len() - 1 {
            boost += 0.1;
        }

        // Check surrounding context for importance indicators
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

    /// Calculate frequency-based score (TF component)
    fn calculate_frequency_score(&self, word: &str, word_freq: &AHashMap<String, i32>, total_words: usize) -> f32 {
        if let Some(&freq) = word_freq.get(word) {
            let tf = freq as f32 / total_words as f32;

            // Apply logarithmic scaling to prevent very frequent words from dominating
            (tf.ln() + 1.0) * 0.1
        } else {
            0.0
        }
    }

    /// Calculate contextual weight between two words
    fn calculate_contextual_weight(&self, word: &str, context_word: &str) -> f32 {
        // Simple heuristic: technical terms boost each other
        if self.is_technical_term(word) && self.is_technical_term(context_word) {
            0.05
        } else if context_word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            0.02 // Proximity to proper nouns
        } else {
            0.0
        }
    }

    /// Check if a word is likely a technical term
    fn is_technical_term(&self, word: &str) -> bool {
        // Basic heuristics for technical terms
        word.len() > 6
            && (word.contains("_")
                || word.chars().filter(|&c| c.is_uppercase()).count() > 1
                || word.ends_with("tion")
                || word.ends_with("ment")
                || word.ends_with("ing"))
    }

    /// Get hypernym for a word (simplified implementation)
    fn get_hypernym(&self, word: &str) -> Option<String> {
        let clean_word = self.clean_word(word).to_lowercase();
        self.hypernyms.get(&clean_word).cloned()
    }

    /// Clean word for processing (remove punctuation, normalize case)
    fn clean_word(&self, word: &str) -> String {
        word.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    }

    /// Initialize importance weights for common word patterns
    fn initialize_importance_weights(&mut self) {
        // High importance words
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

        // Medium importance words
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

    /// Initialize hypernym mappings for compression
    fn initialize_hypernyms(&mut self) {
        // Example hypernym mappings (in practice, use WordNet or similar)
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

    /// Initialize semantic clusters for related terms
    fn initialize_semantic_clusters(&mut self) {
        // Group semantically related terms
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
        let analyzer = SemanticAnalyzer::new("en").unwrap();
        let input = "The quick brown fox jumps over the lazy dog with great performance";
        let result = analyzer.apply_semantic_filtering(input, 0.4);

        // Should preserve some important words
        assert!(result.contains("performance") || result.contains("fox") || result.contains("dog"));
        // Should be shorter than original
        assert!(result.len() < input.len());
    }

    #[test]
    fn test_hypernym_compression() {
        let analyzer = SemanticAnalyzer::new("en").unwrap();
        let input = "The car drove past the dog near the apple tree";
        let result = analyzer.apply_hypernym_compression(input, Some(0.5));

        // Should achieve significant compression
        let original_words = input.split_whitespace().count();
        let result_words = result.split_whitespace().count();
        assert!(result_words <= (original_words as f32 * 0.5) as usize + 1);
    }

    #[test]
    fn test_importance_scoring() {
        let analyzer = SemanticAnalyzer::new("en").unwrap();
        let tokens = analyzer.tokenize_and_score("The important analysis shows significant results");

        // Find the "important" and "analysis" tokens
        let important_token = tokens.iter().find(|t| t.token == "important").unwrap();
        let analysis_token = tokens.iter().find(|t| t.token == "analysis").unwrap();
        let the_token = tokens.iter().find(|t| t.token == "The").unwrap();

        // Important words should have higher scores
        assert!(important_token.importance_score > the_token.importance_score);
        assert!(analysis_token.importance_score > the_token.importance_score);
    }
}
