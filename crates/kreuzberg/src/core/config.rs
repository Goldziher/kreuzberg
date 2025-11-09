//! Configuration loading and management.
//!
//! This module provides utilities for loading extraction configuration from various
//! sources (TOML, YAML, JSON) and discovering configuration files in the project hierarchy.

use crate::{KreuzbergError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main extraction configuration.
///
/// This struct contains all configuration options for the extraction process.
/// It can be loaded from TOML, YAML, or JSON files, or created programmatically.
///
/// # Example
///
/// ```rust
/// use kreuzberg::core::config::ExtractionConfig;
///
/// // Create with defaults
/// let config = ExtractionConfig::default();
///
/// // Load from TOML file
/// // let config = ExtractionConfig::from_toml_file("kreuzberg.toml")?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Enable caching of extraction results
    #[serde(default = "default_true")]
    pub use_cache: bool,

    /// Enable quality post-processing
    #[serde(default = "default_true")]
    pub enable_quality_processing: bool,

    /// OCR configuration (None = OCR disabled)
    #[serde(default)]
    pub ocr: Option<OcrConfig>,

    /// Force OCR even for searchable PDFs
    #[serde(default)]
    pub force_ocr: bool,

    /// Text chunking configuration (None = chunking disabled)
    #[serde(default)]
    pub chunking: Option<ChunkingConfig>,

    /// Image extraction configuration (None = no image extraction)
    #[serde(default)]
    pub images: Option<ImageExtractionConfig>,

    /// PDF-specific options (None = use defaults)
    #[serde(default)]
    pub pdf_options: Option<PdfConfig>,

    /// Token reduction configuration (None = no token reduction)
    #[serde(default)]
    pub token_reduction: Option<TokenReductionConfig>,

    /// Language detection configuration (None = no language detection)
    #[serde(default)]
    pub language_detection: Option<LanguageDetectionConfig>,

    /// Keyword extraction configuration (None = no keyword extraction)
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    #[serde(default)]
    pub keywords: Option<crate::keywords::KeywordConfig>,

    /// Post-processor configuration (None = use defaults)
    #[serde(default)]
    pub postprocessor: Option<PostProcessorConfig>,

    /// HTML conversion options (None = use defaults)
    ///
    /// Note: This field cannot be deserialized from TOML/YAML/JSON files.
    /// Set it programmatically after loading config.
    #[cfg(feature = "html")]
    #[serde(skip)]
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,

    /// Maximum concurrent extractions in batch operations (None = num_cpus * 2).
    ///
    /// Limits parallelism to prevent resource exhaustion when processing
    /// large batches. Defaults to twice the number of CPU cores.
    #[serde(default)]
    pub max_concurrent_extractions: Option<usize>,
}

/// Post-processor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessorConfig {
    /// Enable post-processors
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Whitelist of processor names to run (None = all enabled)
    #[serde(default)]
    pub enabled_processors: Option<Vec<String>>,

    /// Blacklist of processor names to skip (None = none disabled)
    #[serde(default)]
    pub disabled_processors: Option<Vec<String>>,
}

/// OCR configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// OCR backend: tesseract, easyocr, paddleocr
    #[serde(default = "default_tesseract_backend")]
    pub backend: String,

    /// Language code (e.g., "eng", "deu")
    #[serde(default = "default_eng")]
    pub language: String,

    /// Tesseract-specific configuration (optional)
    #[serde(default)]
    pub tesseract_config: Option<crate::types::TesseractConfig>,
}

/// Chunking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Maximum characters per chunk
    #[serde(default = "default_chunk_size")]
    pub max_chars: usize,

    /// Overlap between chunks in characters
    #[serde(default = "default_chunk_overlap")]
    pub max_overlap: usize,

    /// Optional embedding configuration for chunk embeddings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<EmbeddingConfig>,

    /// Use a preset configuration (overrides individual settings if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

/// Embedding configuration for text chunks.
///
/// Configures embedding generation using ONNX models via fastembed-rs.
/// Requires the `embeddings` feature to be enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// The embedding model to use
    pub model: EmbeddingModelType,

    /// Whether to normalize embedding vectors (recommended for cosine similarity)
    #[serde(default = "default_normalize")]
    pub normalize: bool,

    /// Batch size for embedding generation
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Show model download progress
    #[serde(default)]
    pub show_download_progress: bool,

    /// Custom cache directory for model files
    ///
    /// Defaults to `~/.cache/kreuzberg/embeddings/` if not specified.
    /// Allows full customization of model download location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<std::path::PathBuf>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: EmbeddingModelType::Preset {
                name: "balanced".to_string(),
            },
            normalize: true,
            batch_size: 32,
            show_download_progress: false,
            cache_dir: None,
        }
    }
}

/// Embedding model types supported by Kreuzberg.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbeddingModelType {
    /// Use a preset model configuration (recommended)
    Preset { name: String },

    /// Use a specific fastembed model by name
    #[cfg(feature = "embeddings")]
    FastEmbed { model: String, dimensions: usize },

    /// Use a custom ONNX model from HuggingFace
    Custom { model_id: String, dimensions: usize },
}

/// Image extraction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageExtractionConfig {
    /// Extract images from documents
    #[serde(default = "default_true")]
    pub extract_images: bool,

    /// Target DPI for image normalization
    #[serde(default = "default_target_dpi")]
    pub target_dpi: i32,

    /// Maximum dimension for images (width or height)
    #[serde(default = "default_max_dimension")]
    pub max_image_dimension: i32,

    /// Automatically adjust DPI based on image content
    #[serde(default = "default_true")]
    pub auto_adjust_dpi: bool,

    /// Minimum DPI threshold
    #[serde(default = "default_min_dpi")]
    pub min_dpi: i32,

    /// Maximum DPI threshold
    #[serde(default = "default_max_dpi")]
    pub max_dpi: i32,
}

/// PDF-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    /// Extract images from PDF
    #[serde(default)]
    pub extract_images: bool,

    /// List of passwords to try when opening encrypted PDFs
    #[serde(default)]
    pub passwords: Option<Vec<String>>,

    /// Extract PDF metadata
    #[serde(default = "default_true")]
    pub extract_metadata: bool,
}

/// Token reduction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenReductionConfig {
    /// Reduction mode: "off", "light", "moderate", "aggressive", "maximum"
    #[serde(default = "default_reduction_mode")]
    pub mode: String,

    /// Preserve important words (capitalized, technical terms)
    #[serde(default = "default_true")]
    pub preserve_important_words: bool,
}

/// Language detection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionConfig {
    /// Enable language detection
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Minimum confidence threshold (0.0-1.0)
    #[serde(default = "default_confidence")]
    pub min_confidence: f64,

    /// Detect multiple languages in the document
    #[serde(default)]
    pub detect_multiple: bool,
}

fn default_true() -> bool {
    true
}
fn default_eng() -> String {
    "eng".to_string()
}
fn default_tesseract_backend() -> String {
    "tesseract".to_string()
}
fn default_chunk_size() -> usize {
    1000
}
fn default_chunk_overlap() -> usize {
    200
}
fn default_normalize() -> bool {
    true
}
fn default_batch_size() -> usize {
    32
}
fn default_target_dpi() -> i32 {
    300
}
fn default_max_dimension() -> i32 {
    4096
}
fn default_min_dpi() -> i32 {
    72
}
fn default_max_dpi() -> i32 {
    600
}
fn default_reduction_mode() -> String {
    "off".to_string()
}
fn default_confidence() -> f64 {
    0.8
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            use_cache: true,
            enable_quality_processing: true,
            ocr: None,
            force_ocr: false,
            chunking: None,
            images: None,
            pdf_options: None,
            token_reduction: None,
            language_detection: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            keywords: None,
            postprocessor: None,
            #[cfg(feature = "html")]
            html_options: None,
            max_concurrent_extractions: None,
        }
    }
}

impl Default for PostProcessorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enabled_processors: None,
            disabled_processors: None,
        }
    }
}

impl ExtractionConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML file
    ///
    /// # Errors
    ///
    /// Returns `KreuzbergError::Validation` if file doesn't exist or is invalid TOML.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            KreuzbergError::validation(format!("Failed to read config file {}: {}", path.as_ref().display(), e))
        })?;

        toml::from_str(&content)
            .map_err(|e| KreuzbergError::validation(format!("Invalid TOML in {}: {}", path.as_ref().display(), e)))
    }

    /// Load configuration from a YAML file.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            KreuzbergError::validation(format!("Failed to read config file {}: {}", path.as_ref().display(), e))
        })?;

        serde_yaml_ng::from_str(&content)
            .map_err(|e| KreuzbergError::validation(format!("Invalid YAML in {}: {}", path.as_ref().display(), e)))
    }

    /// Load configuration from a JSON file.
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            KreuzbergError::validation(format!("Failed to read config file {}: {}", path.as_ref().display(), e))
        })?;

        serde_json::from_str(&content)
            .map_err(|e| KreuzbergError::validation(format!("Invalid JSON in {}: {}", path.as_ref().display(), e)))
    }

    /// Discover configuration file in parent directories.
    ///
    /// Searches for `kreuzberg.toml` in current directory and parent directories.
    ///
    /// # Returns
    ///
    /// - `Some(config)` if found
    /// - `None` if no config file found
    pub fn discover() -> Result<Option<Self>> {
        let mut current = std::env::current_dir().map_err(KreuzbergError::Io)?;

        loop {
            let kreuzberg_toml = current.join("kreuzberg.toml");
            if kreuzberg_toml.exists() {
                return Ok(Some(Self::from_toml_file(kreuzberg_toml)?));
            }

            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = ExtractionConfig::default();
        assert!(config.use_cache);
        assert!(config.enable_quality_processing);
        assert!(config.ocr.is_none());
    }

    #[test]
    fn test_from_toml_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = false
enable_quality_processing = true
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(!config.use_cache);
        assert!(config.enable_quality_processing);
    }

    #[test]
    fn test_discover_kreuzberg_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = false
enable_quality_processing = true
        "#,
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let result = std::panic::catch_unwind(|| {
            let config = ExtractionConfig::discover().unwrap();
            assert!(config.is_some());
            assert!(!config.unwrap().use_cache);
        });

        std::env::set_current_dir(&original_dir).unwrap();

        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn test_v4_config_with_ocr_and_chunking() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true
enable_quality_processing = false

[ocr]
backend = "tesseract"
language = "eng"

[chunking]
max_chars = 2000
max_overlap = 300
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.use_cache);
        assert!(!config.enable_quality_processing);
        assert!(config.ocr.is_some());
        assert_eq!(config.ocr.unwrap().backend, "tesseract");
        assert!(config.chunking.is_some());
        assert_eq!(config.chunking.unwrap().max_chars, 2000);
    }

    #[test]
    fn test_config_with_image_extraction() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true

[images]
extract_images = true
target_dpi = 300
max_image_dimension = 4096
auto_adjust_dpi = true
min_dpi = 72
max_dpi = 600
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.images.is_some());
        let images = config.images.unwrap();
        assert!(images.extract_images);
        assert_eq!(images.target_dpi, 300);
        assert_eq!(images.max_image_dimension, 4096);
        assert!(images.auto_adjust_dpi);
        assert_eq!(images.min_dpi, 72);
        assert_eq!(images.max_dpi, 600);
    }

    #[test]
    fn test_config_with_pdf_options() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true

[pdf_options]
extract_images = true
passwords = ["password1", "password2"]
extract_metadata = true
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.pdf_options.is_some());
        let pdf = config.pdf_options.unwrap();
        assert!(pdf.extract_images);
        assert!(pdf.extract_metadata);
        assert!(pdf.passwords.is_some());
        let passwords = pdf.passwords.unwrap();
        assert_eq!(passwords.len(), 2);
        assert_eq!(passwords[0], "password1");
        assert_eq!(passwords[1], "password2");
    }

    #[test]
    fn test_config_with_token_reduction() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true

[token_reduction]
mode = "aggressive"
preserve_important_words = true
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.token_reduction.is_some());
        let token = config.token_reduction.unwrap();
        assert_eq!(token.mode, "aggressive");
        assert!(token.preserve_important_words);
    }

    #[test]
    fn test_config_with_language_detection() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true

[language_detection]
enabled = true
min_confidence = 0.9
detect_multiple = true
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.language_detection.is_some());
        let lang = config.language_detection.unwrap();
        assert!(lang.enabled);
        assert_eq!(lang.min_confidence, 0.9);
        assert!(lang.detect_multiple);
    }

    #[test]
    fn test_config_with_all_optional_fields() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true
enable_quality_processing = true
force_ocr = false

[ocr]
backend = "tesseract"
language = "eng"

[chunking]
max_chars = 1500
max_overlap = 250

[images]
extract_images = true
target_dpi = 300

[pdf_options]
extract_images = false
extract_metadata = true

[token_reduction]
mode = "moderate"

[language_detection]
enabled = true
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.use_cache);
        assert!(config.enable_quality_processing);
        assert!(!config.force_ocr);
        assert!(config.ocr.is_some());
        assert!(config.chunking.is_some());
        assert!(config.images.is_some());
        assert!(config.pdf_options.is_some());
        assert!(config.token_reduction.is_some());
        assert!(config.language_detection.is_some());
    }

    #[test]
    fn test_image_config_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[images]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let images = config.images.unwrap();
        assert!(images.extract_images);
        assert_eq!(images.target_dpi, 300);
        assert_eq!(images.max_image_dimension, 4096);
        assert!(images.auto_adjust_dpi);
        assert_eq!(images.min_dpi, 72);
        assert_eq!(images.max_dpi, 600);
    }

    #[test]
    fn test_token_reduction_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[token_reduction]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let token = config.token_reduction.unwrap();
        assert_eq!(token.mode, "off");
        assert!(token.preserve_important_words);
    }

    #[test]
    fn test_language_detection_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[language_detection]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let lang = config.language_detection.unwrap();
        assert!(lang.enabled);
        assert_eq!(lang.min_confidence, 0.8);
        assert!(!lang.detect_multiple);
    }

    #[test]
    fn test_pdf_config_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[pdf_options]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let pdf = config.pdf_options.unwrap();
        assert!(!pdf.extract_images);
        assert!(pdf.extract_metadata);
        assert!(pdf.passwords.is_none());
    }

    #[test]
    fn test_config_with_postprocessor() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true

[postprocessor]
enabled = true
enabled_processors = ["entity_extraction", "keyword_extraction"]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.postprocessor.is_some());
        let pp = config.postprocessor.unwrap();
        assert!(pp.enabled);
        assert!(pp.enabled_processors.is_some());
        let enabled = pp.enabled_processors.unwrap();
        assert_eq!(enabled.len(), 2);
        assert_eq!(enabled[0], "entity_extraction");
        assert_eq!(enabled[1], "keyword_extraction");
        assert!(pp.disabled_processors.is_none());
    }

    #[test]
    fn test_postprocessor_config_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[postprocessor]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let pp = config.postprocessor.unwrap();
        assert!(pp.enabled);
        assert!(pp.enabled_processors.is_none());
        assert!(pp.disabled_processors.is_none());
    }

    #[test]
    fn test_postprocessor_config_disabled() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[postprocessor]
enabled = false
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let pp = config.postprocessor.unwrap();
        assert!(!pp.enabled);
    }

    #[test]
    fn test_postprocessor_config_blacklist() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[postprocessor]
disabled_processors = ["category_extraction"]
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        let pp = config.postprocessor.unwrap();
        assert!(pp.enabled);
        assert!(pp.disabled_processors.is_some());
        let disabled = pp.disabled_processors.unwrap();
        assert_eq!(disabled.len(), 1);
        assert_eq!(disabled[0], "category_extraction");
    }

    #[test]
    fn test_config_with_tesseract_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
use_cache = true

[ocr]
backend = "tesseract"
language = "eng"

[ocr.tesseract_config]
psm = 6
output_format = "text"
enable_table_detection = false
tessedit_char_whitelist = "0123456789"
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.ocr.is_some());
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "tesseract");
        assert_eq!(ocr.language, "eng");
        assert!(ocr.tesseract_config.is_some());

        let tess = ocr.tesseract_config.unwrap();
        assert_eq!(tess.psm, 6);
        assert_eq!(tess.output_format, "text");
        assert!(!tess.enable_table_detection);
        assert_eq!(tess.tessedit_char_whitelist, "0123456789");
    }

    #[test]
    fn test_ocr_config_without_tesseract_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(
            &config_path,
            r#"
[ocr]
backend = "easyocr"
language = "eng"
        "#,
        )
        .unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.ocr.is_some());
        let ocr = config.ocr.unwrap();
        assert_eq!(ocr.backend, "easyocr");
        assert_eq!(ocr.language, "eng");
        assert!(ocr.tesseract_config.is_none());
    }

    #[test]
    fn test_tesseract_config_defaults() {
        let tess = crate::types::TesseractConfig::default();
        assert_eq!(tess.language, "eng");
        assert_eq!(tess.psm, 3);
        assert_eq!(tess.output_format, "markdown");
        assert!(tess.enable_table_detection);
        assert_eq!(tess.table_min_confidence, 0.0);
        assert_eq!(tess.table_column_threshold, 50);
        assert_eq!(tess.table_row_threshold_ratio, 0.5);
        assert!(tess.use_cache);
        assert!(tess.classify_use_pre_adapted_templates);
        assert!(!tess.language_model_ngram_on);
        assert!(tess.tessedit_dont_blkrej_good_wds);
        assert!(tess.tessedit_dont_rowrej_good_wds);
        assert!(tess.tessedit_enable_dict_correction);
        assert_eq!(tess.tessedit_char_whitelist, "");
        assert!(tess.tessedit_use_primary_params_model);
        assert!(tess.textord_space_size_is_variable);
        assert!(!tess.thresholding_method);
    }
}
