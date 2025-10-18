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
    #[pyo3(signature = (backend=None, language=None, tesseract_config=None))]
    fn new(backend: Option<String>, language: Option<String>, tesseract_config: Option<TesseractConfig>) -> Self {
        Self {
            inner: kreuzberg::OcrConfig {
                backend: backend.unwrap_or_else(|| "tesseract".to_string()),
                language: language.unwrap_or_else(|| "eng".to_string()),
                tesseract_config: tesseract_config.map(Into::into),
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

    #[getter]
    fn tesseract_config(&self) -> Option<TesseractConfig> {
        self.inner.tesseract_config.clone().map(Into::into)
    }

    #[setter]
    fn set_tesseract_config(&mut self, value: Option<TesseractConfig>) {
        self.inner.tesseract_config = value.map(Into::into);
    }

    fn __repr__(&self) -> String {
        format!(
            "OcrConfig(backend='{}', language='{}', tesseract_config={})",
            self.inner.backend,
            self.inner.language,
            if self.inner.tesseract_config.is_some() {
                "Some(...)"
            } else {
                "None"
            }
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

// ============================================================================
// ImagePreprocessingConfig
// ============================================================================

/// Image preprocessing configuration for OCR.
///
/// Controls how images are preprocessed before OCR to improve text recognition.
///
/// Example:
///     >>> from kreuzberg import ImagePreprocessingConfig
///     >>> config = ImagePreprocessingConfig(
///     ...     target_dpi=600,
///     ...     denoise=True,
///     ...     contrast_enhance=True
///     ... )
#[pyclass(name = "ImagePreprocessingConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct ImagePreprocessingConfig {
    inner: kreuzberg::types::ImagePreprocessingConfig,
}

#[pymethods]
impl ImagePreprocessingConfig {
    #[new]
    #[pyo3(signature = (
        target_dpi=None,
        auto_rotate=None,
        deskew=None,
        denoise=None,
        contrast_enhance=None,
        binarization_method=None,
        invert_colors=None
    ))]
    fn new(
        target_dpi: Option<i32>,
        auto_rotate: Option<bool>,
        deskew: Option<bool>,
        denoise: Option<bool>,
        contrast_enhance: Option<bool>,
        binarization_method: Option<String>,
        invert_colors: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::types::ImagePreprocessingConfig {
                target_dpi: target_dpi.unwrap_or(300),
                auto_rotate: auto_rotate.unwrap_or(true),
                deskew: deskew.unwrap_or(true),
                denoise: denoise.unwrap_or(false),
                contrast_enhance: contrast_enhance.unwrap_or(false),
                binarization_method: binarization_method.unwrap_or_else(|| "otsu".to_string()),
                invert_colors: invert_colors.unwrap_or(false),
            },
        }
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
    fn auto_rotate(&self) -> bool {
        self.inner.auto_rotate
    }

    #[setter]
    fn set_auto_rotate(&mut self, value: bool) {
        self.inner.auto_rotate = value;
    }

    #[getter]
    fn deskew(&self) -> bool {
        self.inner.deskew
    }

    #[setter]
    fn set_deskew(&mut self, value: bool) {
        self.inner.deskew = value;
    }

    #[getter]
    fn denoise(&self) -> bool {
        self.inner.denoise
    }

    #[setter]
    fn set_denoise(&mut self, value: bool) {
        self.inner.denoise = value;
    }

    #[getter]
    fn contrast_enhance(&self) -> bool {
        self.inner.contrast_enhance
    }

    #[setter]
    fn set_contrast_enhance(&mut self, value: bool) {
        self.inner.contrast_enhance = value;
    }

    #[getter]
    fn binarization_method(&self) -> String {
        self.inner.binarization_method.clone()
    }

    #[setter]
    fn set_binarization_method(&mut self, value: String) {
        self.inner.binarization_method = value;
    }

    #[getter]
    fn invert_colors(&self) -> bool {
        self.inner.invert_colors
    }

    #[setter]
    fn set_invert_colors(&mut self, value: bool) {
        self.inner.invert_colors = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ImagePreprocessingConfig(target_dpi={}, auto_rotate={}, deskew={}, denoise={})",
            self.inner.target_dpi, self.inner.auto_rotate, self.inner.deskew, self.inner.denoise
        )
    }
}

impl From<ImagePreprocessingConfig> for kreuzberg::types::ImagePreprocessingConfig {
    fn from(config: ImagePreprocessingConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::types::ImagePreprocessingConfig> for ImagePreprocessingConfig {
    fn from(config: kreuzberg::types::ImagePreprocessingConfig) -> Self {
        Self { inner: config }
    }
}

// ============================================================================
// TesseractConfig
// ============================================================================

/// Tesseract OCR configuration.
///
/// Provides fine-grained control over Tesseract OCR behavior including
/// page segmentation mode, table detection, and various Tesseract-specific options.
///
/// Example:
///     >>> from kreuzberg import TesseractConfig
///     >>> config = TesseractConfig(
///     ...     language="eng",
///     ...     psm=6,
///     ...     enable_table_detection=True,
///     ...     tessedit_char_whitelist="0123456789"
///     ... )
#[pyclass(name = "TesseractConfig", module = "kreuzberg")]
#[derive(Clone)]
pub struct TesseractConfig {
    inner: kreuzberg::types::TesseractConfig,
}

#[pymethods]
impl TesseractConfig {
    #[new]
    #[pyo3(signature = (
        language=None,
        psm=None,
        output_format=None,
        oem=None,
        min_confidence=None,
        preprocessing=None,
        enable_table_detection=None,
        table_min_confidence=None,
        table_column_threshold=None,
        table_row_threshold_ratio=None,
        use_cache=None,
        classify_use_pre_adapted_templates=None,
        language_model_ngram_on=None,
        tessedit_dont_blkrej_good_wds=None,
        tessedit_dont_rowrej_good_wds=None,
        tessedit_enable_dict_correction=None,
        tessedit_char_whitelist=None,
        tessedit_char_blacklist=None,
        tessedit_use_primary_params_model=None,
        textord_space_size_is_variable=None,
        thresholding_method=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        language: Option<String>,
        psm: Option<i32>,
        output_format: Option<String>,
        oem: Option<i32>,
        min_confidence: Option<f64>,
        preprocessing: Option<ImagePreprocessingConfig>,
        enable_table_detection: Option<bool>,
        table_min_confidence: Option<f64>,
        table_column_threshold: Option<i32>,
        table_row_threshold_ratio: Option<f64>,
        use_cache: Option<bool>,
        classify_use_pre_adapted_templates: Option<bool>,
        language_model_ngram_on: Option<bool>,
        tessedit_dont_blkrej_good_wds: Option<bool>,
        tessedit_dont_rowrej_good_wds: Option<bool>,
        tessedit_enable_dict_correction: Option<bool>,
        tessedit_char_whitelist: Option<String>,
        tessedit_char_blacklist: Option<String>,
        tessedit_use_primary_params_model: Option<bool>,
        textord_space_size_is_variable: Option<bool>,
        thresholding_method: Option<bool>,
    ) -> Self {
        Self {
            inner: kreuzberg::types::TesseractConfig {
                language: language.unwrap_or_else(|| "eng".to_string()),
                psm: psm.unwrap_or(3),
                output_format: output_format.unwrap_or_else(|| "markdown".to_string()),
                oem: oem.unwrap_or(3),
                min_confidence: min_confidence.unwrap_or(0.0),
                preprocessing: preprocessing.map(Into::into),
                enable_table_detection: enable_table_detection.unwrap_or(true),
                table_min_confidence: table_min_confidence.unwrap_or(0.0),
                table_column_threshold: table_column_threshold.unwrap_or(50),
                table_row_threshold_ratio: table_row_threshold_ratio.unwrap_or(0.5),
                use_cache: use_cache.unwrap_or(true),
                classify_use_pre_adapted_templates: classify_use_pre_adapted_templates.unwrap_or(true),
                language_model_ngram_on: language_model_ngram_on.unwrap_or(false),
                tessedit_dont_blkrej_good_wds: tessedit_dont_blkrej_good_wds.unwrap_or(true),
                tessedit_dont_rowrej_good_wds: tessedit_dont_rowrej_good_wds.unwrap_or(true),
                tessedit_enable_dict_correction: tessedit_enable_dict_correction.unwrap_or(true),
                tessedit_char_whitelist: tessedit_char_whitelist.unwrap_or_default(),
                tessedit_char_blacklist: tessedit_char_blacklist.unwrap_or_default(),
                tessedit_use_primary_params_model: tessedit_use_primary_params_model.unwrap_or(true),
                textord_space_size_is_variable: textord_space_size_is_variable.unwrap_or(true),
                thresholding_method: thresholding_method.unwrap_or(false),
            },
        }
    }

    #[getter]
    fn language(&self) -> String {
        self.inner.language.clone()
    }

    #[setter]
    fn set_language(&mut self, value: String) {
        self.inner.language = value;
    }

    #[getter]
    fn psm(&self) -> i32 {
        self.inner.psm
    }

    #[setter]
    fn set_psm(&mut self, value: i32) {
        self.inner.psm = value;
    }

    #[getter]
    fn output_format(&self) -> String {
        self.inner.output_format.clone()
    }

    #[setter]
    fn set_output_format(&mut self, value: String) {
        self.inner.output_format = value;
    }

    #[getter]
    fn oem(&self) -> i32 {
        self.inner.oem
    }

    #[setter]
    fn set_oem(&mut self, value: i32) {
        self.inner.oem = value;
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
    fn preprocessing(&self) -> Option<ImagePreprocessingConfig> {
        self.inner.preprocessing.clone().map(Into::into)
    }

    #[setter]
    fn set_preprocessing(&mut self, value: Option<ImagePreprocessingConfig>) {
        self.inner.preprocessing = value.map(Into::into);
    }

    #[getter]
    fn enable_table_detection(&self) -> bool {
        self.inner.enable_table_detection
    }

    #[setter]
    fn set_enable_table_detection(&mut self, value: bool) {
        self.inner.enable_table_detection = value;
    }

    #[getter]
    fn table_min_confidence(&self) -> f64 {
        self.inner.table_min_confidence
    }

    #[setter]
    fn set_table_min_confidence(&mut self, value: f64) {
        self.inner.table_min_confidence = value;
    }

    #[getter]
    fn table_column_threshold(&self) -> i32 {
        self.inner.table_column_threshold
    }

    #[setter]
    fn set_table_column_threshold(&mut self, value: i32) {
        self.inner.table_column_threshold = value;
    }

    #[getter]
    fn table_row_threshold_ratio(&self) -> f64 {
        self.inner.table_row_threshold_ratio
    }

    #[setter]
    fn set_table_row_threshold_ratio(&mut self, value: f64) {
        self.inner.table_row_threshold_ratio = value;
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
    fn classify_use_pre_adapted_templates(&self) -> bool {
        self.inner.classify_use_pre_adapted_templates
    }

    #[setter]
    fn set_classify_use_pre_adapted_templates(&mut self, value: bool) {
        self.inner.classify_use_pre_adapted_templates = value;
    }

    #[getter]
    fn language_model_ngram_on(&self) -> bool {
        self.inner.language_model_ngram_on
    }

    #[setter]
    fn set_language_model_ngram_on(&mut self, value: bool) {
        self.inner.language_model_ngram_on = value;
    }

    #[getter]
    fn tessedit_dont_blkrej_good_wds(&self) -> bool {
        self.inner.tessedit_dont_blkrej_good_wds
    }

    #[setter]
    fn set_tessedit_dont_blkrej_good_wds(&mut self, value: bool) {
        self.inner.tessedit_dont_blkrej_good_wds = value;
    }

    #[getter]
    fn tessedit_dont_rowrej_good_wds(&self) -> bool {
        self.inner.tessedit_dont_rowrej_good_wds
    }

    #[setter]
    fn set_tessedit_dont_rowrej_good_wds(&mut self, value: bool) {
        self.inner.tessedit_dont_rowrej_good_wds = value;
    }

    #[getter]
    fn tessedit_enable_dict_correction(&self) -> bool {
        self.inner.tessedit_enable_dict_correction
    }

    #[setter]
    fn set_tessedit_enable_dict_correction(&mut self, value: bool) {
        self.inner.tessedit_enable_dict_correction = value;
    }

    #[getter]
    fn tessedit_char_whitelist(&self) -> String {
        self.inner.tessedit_char_whitelist.clone()
    }

    #[setter]
    fn set_tessedit_char_whitelist(&mut self, value: String) {
        self.inner.tessedit_char_whitelist = value;
    }

    #[getter]
    fn tessedit_char_blacklist(&self) -> String {
        self.inner.tessedit_char_blacklist.clone()
    }

    #[setter]
    fn set_tessedit_char_blacklist(&mut self, value: String) {
        self.inner.tessedit_char_blacklist = value;
    }

    #[getter]
    fn tessedit_use_primary_params_model(&self) -> bool {
        self.inner.tessedit_use_primary_params_model
    }

    #[setter]
    fn set_tessedit_use_primary_params_model(&mut self, value: bool) {
        self.inner.tessedit_use_primary_params_model = value;
    }

    #[getter]
    fn textord_space_size_is_variable(&self) -> bool {
        self.inner.textord_space_size_is_variable
    }

    #[setter]
    fn set_textord_space_size_is_variable(&mut self, value: bool) {
        self.inner.textord_space_size_is_variable = value;
    }

    #[getter]
    fn thresholding_method(&self) -> bool {
        self.inner.thresholding_method
    }

    #[setter]
    fn set_thresholding_method(&mut self, value: bool) {
        self.inner.thresholding_method = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "TesseractConfig(language='{}', psm={}, output_format='{}', enable_table_detection={})",
            self.inner.language, self.inner.psm, self.inner.output_format, self.inner.enable_table_detection
        )
    }
}

impl From<TesseractConfig> for kreuzberg::types::TesseractConfig {
    fn from(config: TesseractConfig) -> Self {
        config.inner
    }
}

impl From<kreuzberg::types::TesseractConfig> for TesseractConfig {
    fn from(config: kreuzberg::types::TesseractConfig) -> Self {
        Self { inner: config }
    }
}
