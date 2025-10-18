//! Native Tesseract OCR backend.
//!
//! This module provides the native Tesseract backend that implements the OcrBackend
//! trait, bridging the plugin system with the low-level OcrProcessor.

use crate::Result;
use crate::core::config::OcrConfig;
use crate::ocr::processor::OcrProcessor;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

// Internal OCR types (different from public API types)
use crate::ocr::types::TesseractConfig as InternalTesseractConfig;

/// Native Tesseract OCR backend.
///
/// This backend wraps the OcrProcessor and implements the OcrBackend trait,
/// allowing it to be used through the plugin system.
///
/// # Thread Safety
///
/// Uses Arc for shared ownership and is thread-safe (Send + Sync).
pub struct TesseractBackend {
    processor: Arc<OcrProcessor>,
}

impl TesseractBackend {
    /// Create a new Tesseract backend with default cache directory.
    pub fn new() -> Result<Self> {
        let processor = OcrProcessor::new(None).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to create Tesseract processor: {}", e),
            source: Some(Box::new(e)),
        })?;
        Ok(Self {
            processor: Arc::new(processor),
        })
    }

    /// Create a new Tesseract backend with custom cache directory.
    pub fn with_cache_dir(cache_dir: std::path::PathBuf) -> Result<Self> {
        let processor = OcrProcessor::new(Some(cache_dir)).map_err(|e| crate::KreuzbergError::Ocr {
            message: format!("Failed to create Tesseract processor: {}", e),
            source: Some(Box::new(e)),
        })?;
        Ok(Self {
            processor: Arc::new(processor),
        })
    }

    /// Convert public API TesseractConfig to internal TesseractConfig.
    ///
    /// The public API types (crate::types) use i32 for compatibility with PyO3,
    /// while internal types (crate::ocr::types) use u8/u32 for efficiency.
    fn convert_config(public_config: &crate::types::TesseractConfig) -> InternalTesseractConfig {
        InternalTesseractConfig {
            language: public_config.language.clone(),
            psm: public_config.psm as u8,
            output_format: public_config.output_format.clone(),
            enable_table_detection: public_config.enable_table_detection,
            table_min_confidence: public_config.table_min_confidence,
            table_column_threshold: public_config.table_column_threshold as u32,
            table_row_threshold_ratio: public_config.table_row_threshold_ratio,
            use_cache: public_config.use_cache,
            classify_use_pre_adapted_templates: public_config.classify_use_pre_adapted_templates,
            language_model_ngram_on: public_config.language_model_ngram_on,
            tessedit_dont_blkrej_good_wds: public_config.tessedit_dont_blkrej_good_wds,
            tessedit_dont_rowrej_good_wds: public_config.tessedit_dont_rowrej_good_wds,
            tessedit_enable_dict_correction: public_config.tessedit_enable_dict_correction,
            tessedit_char_whitelist: public_config.tessedit_char_whitelist.clone(),
            tessedit_use_primary_params_model: public_config.tessedit_use_primary_params_model,
            textord_space_size_is_variable: public_config.textord_space_size_is_variable,
            thresholding_method: public_config.thresholding_method,
        }
    }

    /// Convert OcrConfig to internal TesseractConfig.
    ///
    /// Uses tesseract_config from OcrConfig if provided, otherwise uses defaults
    /// with the language from OcrConfig.
    fn config_to_tesseract(&self, config: &OcrConfig) -> InternalTesseractConfig {
        match &config.tesseract_config {
            Some(tess_config) => Self::convert_config(tess_config),
            None => {
                // Use defaults but override language
                InternalTesseractConfig {
                    language: config.language.clone(),
                    ..Default::default()
                }
            }
        }
    }
}

impl Default for TesseractBackend {
    fn default() -> Self {
        // Use unwrap here as this should only fail if cache directory creation fails,
        // which would be a fatal error anyway
        Self::new().unwrap()
    }
}

impl Plugin for TesseractBackend {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> String {
        // Use the Tesseract library version
        tesseract_rs::TesseractAPI::version()
    }

    fn initialize(&self) -> Result<()> {
        // Tesseract is initialized lazily on first use
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        // Clear cache on shutdown
        self.processor.clear_cache().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to clear Tesseract cache: {}", e),
            plugin_name: "tesseract".to_string(),
        })
    }
}

#[async_trait]
impl OcrBackend for TesseractBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let tess_config = self.config_to_tesseract(config);
        let tess_config_clone = tess_config.clone(); // Clone for metadata

        // Run OCR on blocking thread pool (Tesseract is blocking)
        let processor = Arc::clone(&self.processor);
        let image_bytes = image_bytes.to_vec();

        let ocr_result = tokio::task::spawn_blocking(move || processor.process_image(&image_bytes, &tess_config_clone))
            .await
            .map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("Tesseract task panicked: {}", e),
                plugin_name: "tesseract".to_string(),
            })?
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Tesseract OCR failed: {}", e),
                source: Some(Box::new(e)),
            })?;

        // Convert OcrExtractionResult to ExtractionResult
        // Convert metadata from HashMap to Metadata struct
        let metadata = crate::types::Metadata {
            ocr: Some(crate::types::OcrMetadata {
                language: tess_config.language.clone(),
                psm: tess_config.psm as i32,
                output_format: tess_config.output_format.clone(),
                table_count: ocr_result.tables.len(),
                table_rows: ocr_result.tables.first().map(|t| t.cells.len()),
                table_cols: ocr_result
                    .tables
                    .first()
                    .and_then(|t| t.cells.first().map(|row| row.len())),
            }),
            additional: ocr_result.metadata,
            ..Default::default()
        };

        Ok(ExtractionResult {
            content: ocr_result.content,
            mime_type: ocr_result.mime_type,
            metadata,
            tables: ocr_result
                .tables
                .into_iter()
                .map(|t| crate::types::Table {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number,
                })
                .collect(),
            detected_languages: None,
        })
    }

    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult> {
        let tess_config = self.config_to_tesseract(config);
        let tess_config_clone = tess_config.clone(); // Clone for metadata

        // Run OCR on blocking thread pool
        let processor = Arc::clone(&self.processor);
        let path_str = path.to_string_lossy().to_string();

        let ocr_result = tokio::task::spawn_blocking(move || processor.process_file(&path_str, &tess_config_clone))
            .await
            .map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("Tesseract task panicked: {}", e),
                plugin_name: "tesseract".to_string(),
            })?
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("Tesseract OCR failed: {}", e),
                source: Some(Box::new(e)),
            })?;

        // Convert OcrExtractionResult to ExtractionResult
        // Convert metadata from HashMap to Metadata struct
        let metadata = crate::types::Metadata {
            ocr: Some(crate::types::OcrMetadata {
                language: tess_config.language.clone(),
                psm: tess_config.psm as i32,
                output_format: tess_config.output_format.clone(),
                table_count: ocr_result.tables.len(),
                table_rows: ocr_result.tables.first().map(|t| t.cells.len()),
                table_cols: ocr_result
                    .tables
                    .first()
                    .and_then(|t| t.cells.first().map(|row| row.len())),
            }),
            additional: ocr_result.metadata,
            ..Default::default()
        };

        Ok(ExtractionResult {
            content: ocr_result.content,
            mime_type: ocr_result.mime_type,
            metadata,
            tables: ocr_result
                .tables
                .into_iter()
                .map(|t| crate::types::Table {
                    cells: t.cells,
                    markdown: t.markdown,
                    page_number: t.page_number,
                })
                .collect(),
            detected_languages: None,
        })
    }

    fn supports_language(&self, lang: &str) -> bool {
        // Tesseract supports 100+ languages
        // For now, return true for common languages
        // TODO: Query Tesseract for available languages
        matches!(
            lang,
            "eng"
                | "deu"
                | "fra"
                | "spa"
                | "ita"
                | "por"
                | "rus"
                | "chi_sim"
                | "chi_tra"
                | "jpn"
                | "kor"
                | "ara"
                | "hin"
                | "ben"
                | "tha"
                | "vie"
                | "heb"
                | "tur"
                | "pol"
                | "nld"
                | "swe"
                | "dan"
                | "fin"
                | "nor"
                | "ces"
                | "hun"
                | "ron"
                | "ukr"
                | "bul"
                | "hrv"
                | "srp"
                | "slk"
                | "slv"
                | "lit"
                | "lav"
                | "est"
        )
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Tesseract
    }

    fn supports_table_detection(&self) -> bool {
        true
    }

    fn supported_languages(&self) -> Vec<String> {
        // Return common Tesseract languages
        // TODO: Query Tesseract API for available languages dynamically
        vec![
            "eng", "deu", "fra", "spa", "ita", "por", "rus", "chi_sim", "chi_tra", "jpn", "kor", "ara", "hin", "ben",
            "tha", "vie", "heb", "tur", "pol", "nld", "swe", "dan", "fin", "nor", "ces", "hun", "ron", "ukr", "bul",
            "hrv", "srp", "slk", "slv", "lit", "lav", "est",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_backend_creation() {
        let backend = TesseractBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_tesseract_backend_plugin_interface() {
        let backend = TesseractBackend::new().unwrap();
        assert_eq!(backend.name(), "tesseract");
        assert!(!backend.version().is_empty());
        assert!(backend.initialize().is_ok());
    }

    #[test]
    fn test_tesseract_backend_type() {
        let backend = TesseractBackend::new().unwrap();
        assert_eq!(backend.backend_type(), OcrBackendType::Tesseract);
    }

    #[test]
    fn test_tesseract_backend_supports_language() {
        let backend = TesseractBackend::new().unwrap();
        assert!(backend.supports_language("eng"));
        assert!(backend.supports_language("deu"));
        assert!(backend.supports_language("fra"));
        assert!(!backend.supports_language("xyz"));
    }

    #[test]
    fn test_tesseract_backend_supports_table_detection() {
        let backend = TesseractBackend::new().unwrap();
        assert!(backend.supports_table_detection());
    }

    #[test]
    fn test_tesseract_backend_supported_languages() {
        let backend = TesseractBackend::new().unwrap();
        let languages = backend.supported_languages();
        assert!(languages.contains(&"eng".to_string()));
        assert!(languages.contains(&"deu".to_string()));
        assert!(languages.len() > 30);
    }

    #[test]
    fn test_config_to_tesseract_with_none() {
        let backend = TesseractBackend::new().unwrap();
        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: "deu".to_string(),
            tesseract_config: None,
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        assert_eq!(tess_config.language, "deu");
        // Should use defaults for other fields
        assert_eq!(tess_config.psm, InternalTesseractConfig::default().psm);
    }

    #[test]
    fn test_config_to_tesseract_with_some() {
        let backend = TesseractBackend::new().unwrap();
        let custom_tess_config = crate::types::TesseractConfig {
            language: "fra".to_string(),
            psm: 6,
            enable_table_detection: true,
            ..Default::default()
        };

        let ocr_config = OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(), // This should be ignored
            tesseract_config: Some(custom_tess_config),
        };

        let tess_config = backend.config_to_tesseract(&ocr_config);
        // Should use tesseract_config, not language from OcrConfig
        assert_eq!(tess_config.language, "fra");
        assert_eq!(tess_config.psm, 6);
        assert!(tess_config.enable_table_detection);
    }

    #[test]
    fn test_tesseract_backend_default() {
        let backend = TesseractBackend::default();
        assert_eq!(backend.name(), "tesseract");
    }
}
