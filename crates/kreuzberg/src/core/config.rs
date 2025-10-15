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
}

/// OCR configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// OCR backend: tesseract, easyocr, paddleocr
    pub backend: String,

    /// Language code (e.g., "eng", "deu")
    #[serde(default = "default_eng")]
    pub language: String,
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

// Default value helpers
fn default_true() -> bool {
    true
}
fn default_eng() -> String {
    "eng".to_string()
}
fn default_chunk_size() -> usize {
    1000
}
fn default_chunk_overlap() -> usize {
    200
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

        serde_yaml::from_str(&content)
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
            // Check for kreuzberg.toml
            let kreuzberg_toml = current.join("kreuzberg.toml");
            if kreuzberg_toml.exists() {
                return Ok(Some(Self::from_toml_file(kreuzberg_toml)?));
            }

            // Move to parent directory
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

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        // Run test and ensure we restore directory even if test fails
        let result = std::panic::catch_unwind(|| {
            let config = ExtractionConfig::discover().unwrap();
            assert!(config.is_some());
            assert!(!config.unwrap().use_cache);
        });

        // Restore original directory before dropping temp dir
        std::env::set_current_dir(&original_dir).unwrap();

        // Re-panic if test failed
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    // Note: test_discover_no_config removed because it's unreliable - discovery walks
    // parent directories and may find the project's own kreuzberg.toml during testing

    #[test]
    fn test_v4_config_with_ocr_and_chunking() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        // Valid V4 config
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
}
