//! Configuration for modern token reduction system.

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// Token reduction levels with different semantic preservation strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(eq, eq_int)]
pub enum ReductionLevelDTO {
    /// No reduction, return original text
    Off = 0,
    /// Light formatting cleanup only (normalize whitespace, punctuation)
    Light = 1,
    /// Moderate reduction with traditional stopword removal
    Moderate = 2,
    /// Aggressive reduction with semantic importance scoring
    Aggressive = 3,
    /// Maximum compression using semantic field constriction (90%+ reduction)
    Maximum = 4,
}

#[pymethods]
impl ReductionLevelDTO {
    fn __str__(&self) -> &'static str {
        match self {
            ReductionLevelDTO::Off => "off",
            ReductionLevelDTO::Light => "light",
            ReductionLevelDTO::Moderate => "moderate",
            ReductionLevelDTO::Aggressive => "aggressive",
            ReductionLevelDTO::Maximum => "maximum",
        }
    }

    fn __repr__(&self) -> String {
        format!("ReductionLevel.{}", self.__str__().to_uppercase())
    }
}

impl From<&str> for ReductionLevelDTO {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "off" => ReductionLevelDTO::Off,
            "light" => ReductionLevelDTO::Light,
            "moderate" => ReductionLevelDTO::Moderate,
            "aggressive" => ReductionLevelDTO::Aggressive,
            "maximum" => ReductionLevelDTO::Maximum,
            _ => ReductionLevelDTO::Moderate,
        }
    }
}

/// Modern token reduction configuration with semantic-aware options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[pyclass]
pub struct TokenReductionConfigDTO {
    /// Reduction level determining the aggressiveness of token removal
    #[pyo3(get, set)]
    pub level: ReductionLevelDTO,

    /// Language hint for language-specific optimizations
    #[pyo3(get, set)]
    pub language_hint: Option<String>,

    /// Preserve markdown structure during reduction
    #[pyo3(get, set)]
    pub preserve_markdown: bool,

    /// Preserve code blocks and inline code
    #[pyo3(get, set)]
    pub preserve_code: bool,

    /// Minimum semantic importance score (0.0-1.0) to preserve tokens
    #[pyo3(get, set)]
    pub semantic_threshold: f32,

    /// Enable parallel processing for large texts
    #[pyo3(get, set)]
    pub enable_parallel: bool,

    /// Use SIMD optimizations when available
    #[pyo3(get, set)]
    pub use_simd: bool,

    /// Custom stopwords by language
    #[pyo3(get, set)]
    pub custom_stopwords: Option<std::collections::HashMap<String, Vec<String>>>,

    /// Preserve tokens with specific patterns (regex)
    #[pyo3(get, set)]
    pub preserve_patterns: Vec<String>,

    /// Target reduction percentage (for adaptive algorithms)
    #[pyo3(get, set)]
    pub target_reduction: Option<f32>,

    /// Enable semantic clustering for hypernym compression
    #[pyo3(get, set)]
    pub enable_semantic_clustering: bool,
}

impl Default for TokenReductionConfigDTO {
    fn default() -> Self {
        Self {
            level: ReductionLevelDTO::Moderate,
            language_hint: None,
            preserve_markdown: false,
            preserve_code: true,
            semantic_threshold: 0.3,
            enable_parallel: true,
            use_simd: true,
            custom_stopwords: None,
            preserve_patterns: vec![],
            target_reduction: None,
            enable_semantic_clustering: false,
        }
    }
}

#[pymethods]
impl TokenReductionConfigDTO {
    #[new]
    #[pyo3(signature = (
        level = ReductionLevelDTO::Moderate,
        language_hint = None,
        preserve_markdown = false,
        preserve_code = true,
        semantic_threshold = 0.3,
        enable_parallel = true,
        use_simd = true,
        custom_stopwords = None,
        preserve_patterns = None,
        target_reduction = None,
        enable_semantic_clustering = false
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        level: ReductionLevelDTO,
        language_hint: Option<String>,
        preserve_markdown: bool,
        preserve_code: bool,
        semantic_threshold: f32,
        enable_parallel: bool,
        use_simd: bool,
        custom_stopwords: Option<std::collections::HashMap<String, Vec<String>>>,
        preserve_patterns: Option<Vec<String>>,
        target_reduction: Option<f32>,
        enable_semantic_clustering: bool,
    ) -> Self {
        Self {
            level,
            language_hint,
            preserve_markdown,
            preserve_code,
            semantic_threshold: semantic_threshold.clamp(0.0, 1.0),
            enable_parallel,
            use_simd,
            custom_stopwords,
            preserve_patterns: preserve_patterns.unwrap_or_default(),
            target_reduction: target_reduction.map(|t| t.clamp(0.0, 1.0)),
            enable_semantic_clustering,
        }
    }

    fn __str__(&self) -> String {
        format!(
            "TokenReductionConfig(level={}, lang={:?}, semantic_threshold={})",
            self.level.__str__(),
            self.language_hint,
            self.semantic_threshold
        )
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}
