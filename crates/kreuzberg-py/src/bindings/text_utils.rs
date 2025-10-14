use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::collections::HashMap;

// Token reduction types (PyO3 wrappers for Rust types)
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReductionLevelDTO {
    Off = 0,
    Light = 1,
    Moderate = 2,
    Aggressive = 3,
    Maximum = 4,
}

impl From<ReductionLevelDTO> for kreuzberg::text::ReductionLevel {
    fn from(dto: ReductionLevelDTO) -> Self {
        match dto {
            ReductionLevelDTO::Off => kreuzberg::text::ReductionLevel::Off,
            ReductionLevelDTO::Light => kreuzberg::text::ReductionLevel::Light,
            ReductionLevelDTO::Moderate => kreuzberg::text::ReductionLevel::Moderate,
            ReductionLevelDTO::Aggressive => kreuzberg::text::ReductionLevel::Aggressive,
            ReductionLevelDTO::Maximum => kreuzberg::text::ReductionLevel::Maximum,
        }
    }
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

#[pyclass]
#[derive(Debug, Clone)]
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
    pub custom_stopwords: Option<HashMap<String, Vec<String>>>,

    #[pyo3(get, set)]
    pub preserve_patterns: Vec<String>,

    #[pyo3(get, set)]
    pub target_reduction: Option<f32>,

    #[pyo3(get, set)]
    pub enable_semantic_clustering: bool,
}

impl From<TokenReductionConfigDTO> for kreuzberg::text::TokenReductionConfig {
    fn from(dto: TokenReductionConfigDTO) -> Self {
        kreuzberg::text::TokenReductionConfig {
            level: dto.level.into(),
            language_hint: dto.language_hint,
            preserve_markdown: dto.preserve_markdown,
            preserve_code: dto.preserve_code,
            semantic_threshold: dto.semantic_threshold,
            enable_parallel: dto.enable_parallel,
            use_simd: dto.use_simd,
            custom_stopwords: dto.custom_stopwords,
            preserve_patterns: dto.preserve_patterns,
            target_reduction: dto.target_reduction,
            enable_semantic_clustering: dto.enable_semantic_clustering,
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

// Quality and string utils functions
#[pyfunction]
pub fn calculate_quality_score(text: &str, metadata: Option<HashMap<String, String>>) -> PyResult<f64> {
    Ok(kreuzberg::text::calculate_quality_score(text, metadata.as_ref()))
}

#[pyfunction]
pub fn clean_extracted_text(text: &str) -> PyResult<String> {
    Ok(kreuzberg::text::clean_extracted_text(text))
}

#[pyfunction]
pub fn normalize_spaces(text: &str) -> PyResult<String> {
    Ok(kreuzberg::text::normalize_spaces(text))
}

#[pyfunction]
pub fn safe_decode(data: &[u8], encoding: Option<&str>) -> PyResult<String> {
    Ok(kreuzberg::text::safe_decode(data, encoding))
}

#[pyfunction]
pub fn calculate_text_confidence(text: &str) -> PyResult<f64> {
    Ok(kreuzberg::text::calculate_text_confidence(text))
}

#[pyfunction]
pub fn fix_mojibake(text: &str) -> PyResult<String> {
    Ok(kreuzberg::text::fix_mojibake(text))
}

#[pyfunction]
pub fn get_encoding_cache_key(data_hash: &str, size: usize) -> PyResult<String> {
    Ok(kreuzberg::text::get_encoding_cache_key(data_hash, size))
}

// Token reduction functions
#[pyfunction]
#[pyo3(signature = (text, config, language_hint=None))]
pub fn reduce_tokens(text: &str, config: TokenReductionConfigDTO, language_hint: Option<&str>) -> PyResult<String> {
    let rust_config: kreuzberg::text::TokenReductionConfig = config.into();
    kreuzberg::text::reduce_tokens(text, &rust_config, language_hint)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Token reduction error: {}", e)))
}

#[pyfunction]
#[pyo3(signature = (texts, config, language_hint=None))]
pub fn batch_reduce_tokens(
    texts: Vec<String>,
    config: TokenReductionConfigDTO,
    language_hint: Option<&str>,
) -> PyResult<Vec<String>> {
    let rust_config: kreuzberg::text::TokenReductionConfig = config.into();
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    kreuzberg::text::batch_reduce_tokens(&text_refs, &rust_config, language_hint)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Batch token reduction error: {}", e)))
}

#[pyfunction]
pub fn get_reduction_statistics(original: &str, reduced: &str) -> PyResult<(f64, f64, usize, usize, usize, usize)> {
    Ok(kreuzberg::text::get_reduction_statistics(original, reduced))
}

pub fn register_text_utils_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Quality and string utils
    m.add_function(wrap_pyfunction!(calculate_quality_score, m)?)?;
    m.add_function(wrap_pyfunction!(clean_extracted_text, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_spaces, m)?)?;
    m.add_function(wrap_pyfunction!(safe_decode, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_text_confidence, m)?)?;
    m.add_function(wrap_pyfunction!(fix_mojibake, m)?)?;
    m.add_function(wrap_pyfunction!(get_encoding_cache_key, m)?)?;

    // Token reduction types
    m.add_class::<ReductionLevelDTO>()?;
    m.add_class::<TokenReductionConfigDTO>()?;

    // Token reduction functions
    m.add_function(wrap_pyfunction!(reduce_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(batch_reduce_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(get_reduction_statistics, m)?)?;

    Ok(())
}
