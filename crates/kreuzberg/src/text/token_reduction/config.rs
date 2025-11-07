use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReductionLevel {
    Off = 0,
    Light = 1,
    Moderate = 2,
    Aggressive = 3,
    Maximum = 4,
}

impl ReductionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReductionLevel::Off => "off",
            ReductionLevel::Light => "light",
            ReductionLevel::Moderate => "moderate",
            ReductionLevel::Aggressive => "aggressive",
            ReductionLevel::Maximum => "maximum",
        }
    }
}

impl From<&str> for ReductionLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "off" => ReductionLevel::Off,
            "light" => ReductionLevel::Light,
            "moderate" => ReductionLevel::Moderate,
            "aggressive" => ReductionLevel::Aggressive,
            "maximum" => ReductionLevel::Maximum,
            _ => ReductionLevel::Moderate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenReductionConfig {
    pub level: ReductionLevel,
    pub language_hint: Option<String>,
    pub preserve_markdown: bool,
    pub preserve_code: bool,
    pub semantic_threshold: f32,
    pub enable_parallel: bool,
    pub use_simd: bool,
    pub custom_stopwords: Option<HashMap<String, Vec<String>>>,
    pub preserve_patterns: Vec<String>,
    pub target_reduction: Option<f32>,
    pub enable_semantic_clustering: bool,
}

impl Default for TokenReductionConfig {
    fn default() -> Self {
        Self {
            level: ReductionLevel::Moderate,
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

impl TokenReductionConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        level: ReductionLevel,
        language_hint: Option<String>,
        preserve_markdown: bool,
        preserve_code: bool,
        semantic_threshold: f32,
        enable_parallel: bool,
        use_simd: bool,
        custom_stopwords: Option<HashMap<String, Vec<String>>>,
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
}
