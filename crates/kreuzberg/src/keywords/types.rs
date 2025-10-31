//! Shared types for keyword extraction.

use serde::{Deserialize, Serialize};

/// Keyword algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeywordAlgorithm {
    /// YAKE (Yet Another Keyword Extractor) - statistical approach
    #[cfg(feature = "keywords-yake")]
    Yake,

    /// RAKE (Rapid Automatic Keyword Extraction) - co-occurrence based
    #[cfg(feature = "keywords-rake")]
    Rake,
}

impl Default for KeywordAlgorithm {
    fn default() -> Self {
        #[cfg(feature = "keywords-yake")]
        return Self::Yake;

        #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
        return Self::Rake;

        #[cfg(not(any(feature = "keywords-yake", feature = "keywords-rake")))]
        compile_error!("At least one keyword extraction feature must be enabled");
    }
}

/// Extracted keyword with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyword {
    /// The keyword text.
    pub text: String,

    /// Relevance score (higher is better, algorithm-specific range).
    pub score: f32,

    /// Algorithm that extracted this keyword.
    pub algorithm: KeywordAlgorithm,

    /// Optional positions where keyword appears in text (character offsets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positions: Option<Vec<usize>>,
}

impl Keyword {
    /// Create a new keyword.
    pub fn new(text: String, score: f32, algorithm: KeywordAlgorithm) -> Self {
        Self {
            text,
            score,
            algorithm,
            positions: None,
        }
    }

    /// Create a new keyword with positions.
    pub fn with_positions(text: String, score: f32, algorithm: KeywordAlgorithm, positions: Vec<usize>) -> Self {
        Self {
            text,
            score,
            algorithm,
            positions: Some(positions),
        }
    }
}
