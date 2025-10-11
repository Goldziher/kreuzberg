use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[pyclass(eq, eq_int)]
pub enum ReductionLevelDTO {
    Off = 0,
    Light = 1,
    Moderate = 2,
    Aggressive = 3,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[pyclass]
pub struct TokenReductionConfigDTO {
    #[pyo3(get, set)]
    pub level: ReductionLevelDTO,

    #[pyo3(get, set)]
    pub language_hint: Option<String>,

    #[pyo3(get, set)]
    pub preserve_markdown: bool,

    #[pyo3(get, set)]
    pub preserve_code: bool,

    #[pyo3(get, set)]
    pub semantic_threshold: f32,

    #[pyo3(get, set)]
    pub enable_parallel: bool,

    #[pyo3(get, set)]
    pub use_simd: bool,

    #[pyo3(get, set)]
    pub custom_stopwords: Option<std::collections::HashMap<String, Vec<String>>>,

    #[pyo3(get, set)]
    pub preserve_patterns: Vec<String>,

    #[pyo3(get, set)]
    pub target_reduction: Option<f32>,

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
