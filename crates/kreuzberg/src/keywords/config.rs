//! Configuration for keyword extraction.

use super::types::KeywordAlgorithm;
use serde::{Deserialize, Serialize};

/// YAKE-specific parameters.
#[cfg(feature = "keywords-yake")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YakeParams {
    /// Window size for co-occurrence analysis (default: 2).
    ///
    /// Controls the context window for computing co-occurrence statistics.
    pub window_size: usize,
}

#[cfg(feature = "keywords-yake")]
impl Default for YakeParams {
    fn default() -> Self {
        Self { window_size: 2 }
    }
}

/// RAKE-specific parameters.
#[cfg(feature = "keywords-rake")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RakeParams {
    /// Minimum word length to consider (default: 1).
    pub min_word_length: usize,

    /// Maximum words in a keyword phrase (default: 3).
    pub max_words_per_phrase: usize,
}

#[cfg(feature = "keywords-rake")]
impl Default for RakeParams {
    fn default() -> Self {
        Self {
            min_word_length: 1,
            max_words_per_phrase: 3,
        }
    }
}

/// Keyword extraction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordConfig {
    /// Algorithm to use for extraction.
    pub algorithm: KeywordAlgorithm,

    /// Maximum number of keywords to extract (default: 10).
    pub max_keywords: usize,

    /// Minimum score threshold (0.0-1.0, default: 0.0).
    ///
    /// Keywords with scores below this threshold are filtered out.
    /// Note: Score ranges differ between algorithms.
    pub min_score: f32,

    /// N-gram range for keyword extraction (min, max).
    ///
    /// (1, 1) = unigrams only
    /// (1, 2) = unigrams and bigrams
    /// (1, 3) = unigrams, bigrams, and trigrams (default)
    pub ngram_range: (usize, usize),

    /// Language code for stopword filtering (e.g., "en", "de", "fr").
    ///
    /// If None, no stopword filtering is applied.
    pub language: Option<String>,

    /// YAKE-specific tuning parameters.
    #[cfg(feature = "keywords-yake")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yake_params: Option<YakeParams>,

    /// RAKE-specific tuning parameters.
    #[cfg(feature = "keywords-rake")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rake_params: Option<RakeParams>,
}

impl Default for KeywordConfig {
    fn default() -> Self {
        Self {
            algorithm: KeywordAlgorithm::default(),
            max_keywords: 10,
            min_score: 0.0,
            ngram_range: (1, 3),
            language: Some("en".to_string()),
            #[cfg(feature = "keywords-yake")]
            yake_params: None,
            #[cfg(feature = "keywords-rake")]
            rake_params: None,
        }
    }
}

impl KeywordConfig {
    /// Create a new configuration with YAKE algorithm.
    #[cfg(feature = "keywords-yake")]
    pub fn yake() -> Self {
        Self {
            algorithm: KeywordAlgorithm::Yake,
            ..Default::default()
        }
    }

    /// Create a new configuration with RAKE algorithm.
    #[cfg(feature = "keywords-rake")]
    pub fn rake() -> Self {
        Self {
            algorithm: KeywordAlgorithm::Rake,
            ..Default::default()
        }
    }

    /// Set maximum number of keywords to extract.
    pub fn with_max_keywords(mut self, max: usize) -> Self {
        self.max_keywords = max;
        self
    }

    /// Set minimum score threshold.
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = score;
        self
    }

    /// Set n-gram range.
    pub fn with_ngram_range(mut self, min: usize, max: usize) -> Self {
        self.ngram_range = (min, max);
        self
    }

    /// Set language for stopword filtering.
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Set YAKE-specific parameters.
    #[cfg(feature = "keywords-yake")]
    pub fn with_yake_params(mut self, params: YakeParams) -> Self {
        self.yake_params = Some(params);
        self
    }

    /// Set RAKE-specific parameters.
    #[cfg(feature = "keywords-rake")]
    pub fn with_rake_params(mut self, params: RakeParams) -> Self {
        self.rake_params = Some(params);
        self
    }
}
