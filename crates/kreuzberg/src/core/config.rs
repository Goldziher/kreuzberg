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

// Default value helpers
fn default_true() -> bool { true }
fn default_eng() -> String { "eng".to_string() }
fn default_chunk_size() -> usize { 1000 }
fn default_chunk_overlap() -> usize { 200 }

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            use_cache: true,
            enable_quality_processing: true,
            ocr: None,
            force_ocr: false,
            chunking: None,
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
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| KreuzbergError::Validation(format!("Failed to read config file {}: {}", path.as_ref().display(), e)))?;

        toml::from_str(&content)
            .map_err(|e| KreuzbergError::Validation(format!("Invalid TOML in {}: {}", path.as_ref().display(), e)))
    }

    /// Load configuration from a YAML file.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| KreuzbergError::Validation(format!("Failed to read config file {}: {}", path.as_ref().display(), e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| KreuzbergError::Validation(format!("Invalid YAML in {}: {}", path.as_ref().display(), e)))
    }

    /// Load configuration from a JSON file.
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| KreuzbergError::Validation(format!("Failed to read config file {}: {}", path.as_ref().display(), e)))?;

        serde_json::from_str(&content)
            .map_err(|e| KreuzbergError::Validation(format!("Invalid JSON in {}: {}", path.as_ref().display(), e)))
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
        let mut current = std::env::current_dir()
            .map_err(KreuzbergError::Io)?;

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

        fs::write(&config_path, r#"
use_cache = false
enable_quality_processing = true
        "#).unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(!config.use_cache);
        assert!(config.enable_quality_processing);
    }

    #[test]
    fn test_discover_kreuzberg_toml() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        fs::write(&config_path, r#"
use_cache = false
enable_quality_processing = true
        "#).unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let config = ExtractionConfig::discover().unwrap();
        assert!(config.is_some());
        assert!(!config.unwrap().use_cache);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_discover_no_config() {
        let dir = tempdir().unwrap();

        // Change to temp directory with no config
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let config = ExtractionConfig::discover().unwrap();
        assert!(config.is_none());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_v4_config_with_ocr_and_chunking() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kreuzberg.toml");

        // Valid V4 config
        fs::write(&config_path, r#"
use_cache = true
enable_quality_processing = false

[ocr]
backend = "tesseract"
language = "eng"

[chunking]
max_chars = 2000
max_overlap = 300
        "#).unwrap();

        let config = ExtractionConfig::from_toml_file(&config_path).unwrap();
        assert!(config.use_cache);
        assert!(!config.enable_quality_processing);
        assert!(config.ocr.is_some());
        assert_eq!(config.ocr.unwrap().backend, "tesseract");
        assert!(config.chunking.is_some());
        assert_eq!(config.chunking.unwrap().max_chars, 2000);
    }
}
