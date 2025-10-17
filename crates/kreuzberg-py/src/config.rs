//! Configuration type bindings
//!
//! Provides Python-friendly wrappers around the Rust configuration structs.
//! All types support both construction and field access from Python.

use pyo3::prelude::*;

// ============================================================================
// ExtractionConfig - Main configuration
// ============================================================================

/// Main extraction configuration.
///
/// Controls all aspects of document extraction including OCR, PDF rendering,
/// chunking, caching, and post-processing.
///
/// Example:
///     >>> from kreuzberg import ExtractionConfig, OcrConfig
///     >>> config = ExtractionConfig(
///     ...     ocr=OcrConfig(language="eng"),
///     ...     use_cache=True
///     ... )
#[pyclass(name = "ExtractionConfig", module = "kreuzberg")]
#[derive(Clone, Default)]
pub struct ExtractionConfig {
    inner: kreuzberg::ExtractionConfig,
}

#[pymethods]
impl ExtractionConfig {
    #[new]
    #[pyo3(signature = (
        use_cache=None,
        enable_quality_processing=None,
        ocr=None,
        force_ocr=None,
        chunking=None,
        images=None,
        pdf_options=None,
        token_reduction=None,
        language_detection=None,
        postprocessor=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        use_cache: Option<bool>,
        enable_quality_processing: Option<bool>,
        ocr: Option<OcrConfig>,
        force_ocr: Option<bool>,
        chunking: Option<ChunkingConfig>,
        images: Option<ImageExtractionConfig>,
        pdf_options: Option<PdfConfig>,
        token_reduction: Option<TokenReductionConfig>,
        language_detection: Option<LanguageDetectionConfig>,
        postprocessor: Option<PostProcessorConfig>,
    ) -> Self {
        Self {
            inner: kreuzberg::ExtractionConfig {
                use_cache: use_cache.unwrap_or(true),
                enable_quality_processing: enable_quality_processing.unwrap_or(true),
                ocr: ocr.map(Into::into),
                force_ocr: force_ocr.unwrap_or(false),
                chunking: chunking.map(Into::into),
                images: images.map(Into::into),
                pdf_options: pdf_options.map(Into::into),
                token_reduction: token_reduction.map(Into::into),
                language_detection: language_detection.map(Into::into),
                postprocessor: postprocessor.map(Into::into),
            },
        }
    }

    #[getter]
    fn use_cache(&self) -> bool {
        self.inner.use_cache
    }

    #[setter]
    fn set_use_cache(&mut self, value: bool) {
        self.inner.use_cache = value;
    }

    #[getter]
    fn enable_quality_processing(&self) -> bool {
        self.inner.enable_quality_processing
    }

    #[setter]
    fn set_enable_quality_processing(&mut self, value: bool) {
        self.inner.enable_quality_processing = value;
    }

    #[getter]
    fn ocr(&self) -> Option<OcrConfig> {
        self.inner.ocr.clone().map(Into::into)
    }

    #[setter]
    fn set_ocr(&mut self, value: Option<OcrConfig>) {
        self.inner.ocr = value.map(Into::into);
    }

    #[getter]
    fn force_ocr(&self) -> bool {
        self.inner.force_ocr
    }

    #[setter]
    fn set_force_ocr(&mut self, value: bool) {
        self.inner.force_ocr = value;
    }

    #[getter]
    fn chunking(&self) -> Option<ChunkingConfig> {
        self.inner.chunking.clone().map(Into::into)
    }

    #[setter]
    fn set_chunking(&mut self, value: Option<ChunkingConfig>) {
        self.inner.chunking = value.map(Into::into);
    }

    #[getter]
    fn images(&self) -> Option<ImageExtractionConfig> {
        self.inner.images.clone().map(Into::into)
    }

    #[setter]
    fn set_images(&mut self, value: Option<ImageExtractionConfig>) {
        self.inner.images = value.map(Into::into);
    }

    #[getter]
    fn pdf_options(&self) -> Option<PdfConfig> {
        self.inner.pdf_options.clone().map(Into::into)
    }

    #[setter]
    fn set_pdf_options(&mut self, value: Option<PdfConfig>) {
        self.inner.pdf_options = value.map(Into::into);
    }

    #[getter]
    fn token_reduction(&self) -> Option<TokenReductionConfig> {
        self.inner.token_reduction.clone().map(Into::into)
    }

    #[setter]
    fn set_token_reduction(&mut self, value: Option<TokenReductionConfig>) {
        self.inner.token_reduction = value.map(Into::into);
    }

    #[getter]
    fn language_detection(&self) -> Option<LanguageDetectionConfig> {
        self.inner.language_detection.clone().map(Into::into)
    }

    #[setter]
    fn set_language_detection(&mut self, value: Option<LanguageDetectionConfig>) {
        self.inner.language_detection = value.map(Into::into);
    }

    #[getter]
    fn postprocessor(&self) -> Option<PostProcessorConfig> {
        self.inner.postprocessor.clone().map(Into::into)
    }

    #[setter]
    fn set_postprocessor(&mut self, value: Option<PostProcessorConfig>) {
        self.inner.postprocessor = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "ExtractionConfig(use_cache={}, enable_quality_processing={}, ocr={}, force_ocr={})",
            self.inner.use_cache,
            self.inner.enable_quality_processing,
            if self.inner.ocr.is_some() { "Some(...)" } else { "None" },
            self.inner.force_ocr
        )
    }
}

impl From<ExtractionConfig> for kreuzberg::ExtractionConfig {
    fn from(config: ExtractionConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::ExtractionConfig> for ExtractionConfig {
    fn from(config: kreuzberg::ExtractionConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// OcrConfig
// ============================================================================

/// OCR configuration.
///
/// Example:
///     >>> from kreuzberg import OcrConfig
///     >>> config = OcrConfig(backend="tesseract", language="eng")
#[pyclass(name = "OcrConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct OcrConfig {
    inner: kreuzberg::OcrConfig,
}

#[pymethods]
impl OcrConfig {
    #[new]
    #[pyo3(signature = (backend=None, language=None))]
    fn new(backend: Option<String>, language: Option<String>) -> Self {
        Self {
            inner: kreuzberg::OcrConfig {
                backend: backend.unwrap_or_else(|| "tesseract".to_string()),
                language: language.unwrap_or_else(|| "eng".to_string()),
            },
        }
    }

    #[getter]
    fn backend(&self) -> String {
        self.inner.backend.clone()
    }

    #[setter]
    fn set_backend(&mut self, value: String) {
        self.inner.backend = value;
    }

    #[getter]
    fn language(&self) -> String {
        self.inner.language.clone()
    }

    #[setter]
    fn set_language(&mut self, value: String) {
        self.inner.language = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "OcrConfig(backend='{}', language='{}')",
            self.inner.backend, self.inner.language
        )
    }
}

impl From<OcrConfig> for kreuzberg::OcrConfig {
    fn from(config: OcrConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::OcrConfig> for OcrConfig {
    fn from(config: kreuzberg::OcrConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// ChunkingConfig
// ============================================================================

/// Chunking configuration.
///
/// Example:
///     >>> from kreuzberg import ChunkingConfig
///     >>> config = ChunkingConfig(max_chars=2000, max_overlap=300)
#[pyclass(name = "ChunkingConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct ChunkingConfig {
    inner: kreuzberg::ChunkingConfig,
}

#[pymethods]
impl ChunkingConfig {
    #[new]
    #[pyo3(signature = (max_chars=None, max_overlap=None))]
    fn new(max_chars: Option<usize>, max_overlap: Option<usize>) -> Self {
        Self {
            inner: kreuzberg::ChunkingConfig {
                max_chars: max_chars.unwrap_or(1000),
                max_overlap: max_overlap.unwrap_or(200),
            },
        }
    }

    #[getter]
    fn max_chars(&self) -> usize {
        self.inner.max_chars
    }

    #[setter]
    fn set_max_chars(&mut self, value: usize) {
        self.inner.max_chars = value;
    }

    #[getter]
    fn max_overlap(&self) -> usize {
        self.inner.max_overlap
    }

    #[setter]
    fn set_max_overlap(&mut self, value: usize) {
        self.inner.max_overlap = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ChunkingConfig(max_chars={}, max_overlap={})",
            self.inner.max_chars, self.inner.max_overlap
        )
    }
}

impl From<ChunkingConfig> for kreuzberg::ChunkingConfig {
    fn from(config: ChunkingConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::ChunkingConfig> for ChunkingConfig {
    fn from(config: kreuzberg::ChunkingConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// ImageExtractionConfig
// ============================================================================

/// Image extraction configuration.
///
/// Example:
///     >>> from kreuzberg import ImageExtractionConfig
///     >>> config = ImageExtractionConfig(target_dpi=300, max_image_dimension=4096)
#[pyclass(name = "ImageExtractionConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct ImageExtractionConfig {
    inner: kreuzberg::ImageExtractionConfig,
}

#[pymethods]
impl ImageExtractionConfig {
    #[new]
    #[pyo3(signature = (
        extract_images=None,
        target_dpi=None,
        max_image_dimension=None,
        auto_adjust_dpi=None,
        min_dpi=None,
        max_dpi=None
    ))]
    fn new(
        extract_images: Option<bool>,
        target_dpi: Option<i32>,
        max_image_dimension: Option<i32>,
        auto_adjust_dpi: Option<bool>,
        min_dpi: Option<i32>,
        max_dpi: Option<i32>,
    ) -> Self {
        Self {
            inner: kreuzberg::ImageExtractionConfig {
                extract_images: extract_images.unwrap_or(true),
                target_dpi: target_dpi.unwrap_or(300),
                max_image_dimension: max_image_dimension.unwrap_or(4096),
                auto_adjust_dpi: auto_adjust_dpi.unwrap_or(true),
                min_dpi: min_dpi.unwrap_or(72),
                max_dpi: max_dpi.unwrap_or(600),
            },
        }
    }

    #[getter]
    fn extract_images(&self) -> bool {
        self.inner.extract_images
    }

    #[setter]
    fn set_extract_images(&mut self, value: bool) {
        self.inner.extract_images = value;
    }

    #[getter]
    fn target_dpi(&self) -> i32 {
        self.inner.target_dpi
    }

    #[setter]
    fn set_target_dpi(&mut self, value: i32) {
        self.inner.target_dpi = value;
    }

    #[getter]
    fn max_image_dimension(&self) -> i32 {
        self.inner.max_image_dimension
    }

    #[setter]
    fn set_max_image_dimension(&mut self, value: i32) {
        self.inner.max_image_dimension = value;
    }

    #[getter]
    fn auto_adjust_dpi(&self) -> bool {
        self.inner.auto_adjust_dpi
    }

    #[setter]
    fn set_auto_adjust_dpi(&mut self, value: bool) {
        self.inner.auto_adjust_dpi = value;
    }

    #[getter]
    fn min_dpi(&self) -> i32 {
        self.inner.min_dpi
    }

    #[setter]
    fn set_min_dpi(&mut self, value: i32) {
        self.inner.min_dpi = value;
    }

    #[getter]
    fn max_dpi(&self) -> i32 {
        self.inner.max_dpi
    }

    #[setter]
    fn set_max_dpi(&mut self, value: i32) {
        self.inner.max_dpi = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageExtractionConfig(extract_images={}, target_dpi={}, max_image_dimension={})",
            self.inner.extract_images, self.inner.target_dpi, self.inner.max_image_dimension
        )
    }
}

impl From<ImageExtractionConfig> for kreuzberg::ImageExtractionConfig {
    fn from(config: ImageExtractionConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::ImageExtractionConfig> for ImageExtractionConfig {
    fn from(config: kreuzberg::ImageExtractionConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// PdfConfig
// ============================================================================

/// PDF-specific configuration.
///
/// Example:
///     >>> from kreuzberg import PdfConfig
///     >>> config = PdfConfig(extract_images=True, passwords=["pass1", "pass2"])
#[pyclass(name = "PdfConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct PdfConfig {
    inner: kreuzberg::PdfConfig,
}

#[pymethods]
impl PdfConfig {
    #[new]
    #[pyo3(signature = (extract_images=None, passwords=None, extract_metadata=None))]
    fn new(extract_images: Option<bool>, passwords: Option<Vec<String>>, extract_metadata: Option<bool>) -> Self {
        Self {
            inner: kreuzberg::PdfConfig {
                extract_images: extract_images.unwrap_or(false),
                passwords,
                extract_metadata: extract_metadata.unwrap_or(true),
            },
        }
    }

    #[getter]
    fn extract_images(&self) -> bool {
        self.inner.extract_images
    }

    #[setter]
    fn set_extract_images(&mut self, value: bool) {
        self.inner.extract_images = value;
    }

    #[getter]
    fn passwords(&self) -> Option<Vec<String>> {
        self.inner.passwords.clone()
    }

    #[setter]
    fn set_passwords(&mut self, value: Option<Vec<String>>) {
        self.inner.passwords = value;
    }

    #[getter]
    fn extract_metadata(&self) -> bool {
        self.inner.extract_metadata
    }

    #[setter]
    fn set_extract_metadata(&mut self, value: bool) {
        self.inner.extract_metadata = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "PdfConfig(extract_images={}, extract_metadata={}, passwords={})",
            self.inner.extract_images,
            self.inner.extract_metadata,
            if self.inner.passwords.is_some() {
                "Some([...])"
            } else {
                "None"
            }
        )
    }
}

impl From<PdfConfig> for kreuzberg::PdfConfig {
    fn from(config: PdfConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::PdfConfig> for PdfConfig {
    fn from(config: kreuzberg::PdfConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// TokenReductionConfig
// ============================================================================

/// Token reduction configuration.
///
/// Example:
///     >>> from kreuzberg import TokenReductionConfig
///     >>> config = TokenReductionConfig(mode="aggressive", preserve_important_words=True)
#[pyclass(name = "TokenReductionConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct TokenReductionConfig {
    inner: kreuzberg::TokenReductionConfig,
}

#[pymethods]
impl TokenReductionConfig {
    #[new]
    #[pyo3(signature = (mode=None, preserve_important_words=None))]
    fn new(mode: Option<String>, preserve_important_words: Option<bool>) -> Self {
        Self {
            inner: kreuzberg::TokenReductionConfig {
                mode: mode.unwrap_or_else(|| "off".to_string()),
                preserve_important_words: preserve_important_words.unwrap_or(true),
            },
        }
    }

    #[getter]
    fn mode(&self) -> String {
        self.inner.mode.clone()
    }

    #[setter]
    fn set_mode(&mut self, value: String) {
        self.inner.mode = value;
    }

    #[getter]
    fn preserve_important_words(&self) -> bool {
        self.inner.preserve_important_words
    }

    #[setter]
    fn set_preserve_important_words(&mut self, value: bool) {
        self.inner.preserve_important_words = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "TokenReductionConfig(mode='{}', preserve_important_words={})",
            self.inner.mode, self.inner.preserve_important_words
        )
    }
}

impl From<TokenReductionConfig> for kreuzberg::TokenReductionConfig {
    fn from(config: TokenReductionConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::TokenReductionConfig> for TokenReductionConfig {
    fn from(config: kreuzberg::TokenReductionConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// LanguageDetectionConfig
// ============================================================================

/// Language detection configuration.
///
/// Example:
///     >>> from kreuzberg import LanguageDetectionConfig
///     >>> config = LanguageDetectionConfig(enabled=True, min_confidence=0.9)
#[pyclass(name = "LanguageDetectionConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct LanguageDetectionConfig {
    inner: kreuzberg::LanguageDetectionConfig,
}

#[pymethods]
impl LanguageDetectionConfig {
    #[new]
    #[pyo3(signature = (enabled=None, min_confidence=None, detect_multiple=None))]
    fn new(enabled: Option<bool>, min_confidence: Option<f64>, detect_multiple: Option<bool>) -> Self {
        Self {
            inner: kreuzberg::LanguageDetectionConfig {
                enabled: enabled.unwrap_or(true),
                min_confidence: min_confidence.unwrap_or(0.8),
                detect_multiple: detect_multiple.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled
    }

    #[setter]
    fn set_enabled(&mut self, value: bool) {
        self.inner.enabled = value;
    }

    #[getter]
    fn min_confidence(&self) -> f64 {
        self.inner.min_confidence
    }

    #[setter]
    fn set_min_confidence(&mut self, value: f64) {
        self.inner.min_confidence = value;
    }

    #[getter]
    fn detect_multiple(&self) -> bool {
        self.inner.detect_multiple
    }

    #[setter]
    fn set_detect_multiple(&mut self, value: bool) {
        self.inner.detect_multiple = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "LanguageDetectionConfig(enabled={}, min_confidence={}, detect_multiple={})",
            self.inner.enabled, self.inner.min_confidence, self.inner.detect_multiple
        )
    }
}

impl From<LanguageDetectionConfig> for kreuzberg::LanguageDetectionConfig {
    fn from(config: LanguageDetectionConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::LanguageDetectionConfig> for LanguageDetectionConfig {
    fn from(config: kreuzberg::LanguageDetectionConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// PostProcessorConfig
// ============================================================================

/// Post-processor configuration.
///
/// Example:
///     >>> from kreuzberg import PostProcessorConfig
///     >>> config = PostProcessorConfig(enabled=True, enabled_processors=["entity_extraction"])
#[pyclass(name = "PostProcessorConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct PostProcessorConfig {
    inner: kreuzberg::PostProcessorConfig,
}

#[pymethods]
impl PostProcessorConfig {
    #[new]
    #[pyo3(signature = (enabled=None, enabled_processors=None, disabled_processors=None))]
    fn new(
        enabled: Option<bool>,
        enabled_processors: Option<Vec<String>>,
        disabled_processors: Option<Vec<String>>,
    ) -> Self {
        Self {
            inner: kreuzberg::PostProcessorConfig {
                enabled: enabled.unwrap_or(true),
                enabled_processors,
                disabled_processors,
            },
        }
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled
    }

    #[setter]
    fn set_enabled(&mut self, value: bool) {
        self.inner.enabled = value;
    }

    #[getter]
    fn enabled_processors(&self) -> Option<Vec<String>> {
        self.inner.enabled_processors.clone()
    }

    #[setter]
    fn set_enabled_processors(&mut self, value: Option<Vec<String>>) {
        self.inner.enabled_processors = value;
    }

    #[getter]
    fn disabled_processors(&self) -> Option<Vec<String>> {
        self.inner.disabled_processors.clone()
    }

    #[setter]
    fn set_disabled_processors(&mut self, value: Option<Vec<String>>) {
        self.inner.disabled_processors = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "PostProcessorConfig(enabled={}, enabled_processors={:?}, disabled_processors={:?})",
            self.inner.enabled, self.inner.enabled_processors, self.inner.disabled_processors
        )
    }
}

impl From<PostProcessorConfig> for kreuzberg::PostProcessorConfig {
    fn from(config: PostProcessorConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::PostProcessorConfig> for PostProcessorConfig {
    fn from(config: kreuzberg::PostProcessorConfig) -> Self {
        Self { inner: config }
    }
}
