//! Tesseract OCR processing implementation

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use ahash::AHasher;
use pyo3::prelude::*;
use tesseract_rs::{TessPageSegMode, TesseractAPI};

use super::cache::OCRCache;
use super::error::OCRError;
use super::hocr::convert_hocr_to_markdown;
use super::table::{extract_words, reconstruct_table, table_to_markdown};
use super::types::{BatchItemResult, ExtractionResultDTO, TableDTO, TesseractConfigDTO};

/// Compute hash of image bytes for caching
fn compute_image_hash(image_bytes: &[u8]) -> String {
    let mut hasher = AHasher::default();
    image_bytes.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)
}

/// OCR Processor for handling Tesseract operations
#[pyclass]
pub struct OCRProcessor {
    cache: OCRCache,
}

#[pymethods]
impl OCRProcessor {
    /// Create a new OCR processor with optional cache directory
    #[new]
    #[pyo3(signature = (cache_dir = None))]
    pub fn new(cache_dir: Option<std::path::PathBuf>) -> PyResult<Self> {
        let cache = OCRCache::new(cache_dir).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { cache })
    }

    /// Process image bytes with Tesseract OCR
    pub fn process_image(&self, image_bytes: &[u8], config: &TesseractConfigDTO) -> PyResult<ExtractionResultDTO> {
        // Generate cache key
        let image_hash = compute_image_hash(image_bytes);
        let config_str = self.serialize_config(config);

        // Check cache if enabled
        if config.use_cache
            && let Some(cached_result) = self
                .cache
                .get_cached_result(&image_hash, "tesseract", &config_str)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?
        {
            return Ok(cached_result);
        }

        // Perform OCR
        let result = self
            .perform_ocr(image_bytes, config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        // Cache result if enabled
        if config.use_cache {
            let _ = self
                .cache
                .set_cached_result(&image_hash, "tesseract", &config_str, &result);
        }

        Ok(result)
    }

    /// Clear all cached OCR results
    pub fn clear_cache(&self) -> PyResult<()> {
        self.cache
            .clear()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> PyResult<super::cache::OCRCacheStats> {
        self.cache
            .get_stats()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Process image file with Tesseract OCR
    pub fn process_file(&self, file_path: &str, config: &TesseractConfigDTO) -> PyResult<ExtractionResultDTO> {
        let image_bytes = std::fs::read(file_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(format!("Failed to read file '{}': {}", file_path, e)))?;
        self.process_image(&image_bytes, config)
    }

    /// Process multiple image files sequentially
    /// Note: Using sequential processing instead of parallel to avoid potential
    /// deadlocks and thread safety issues with Tesseract C API
    pub fn process_files_batch(
        &self,
        file_paths: Vec<String>,
        config: &TesseractConfigDTO,
    ) -> PyResult<Vec<BatchItemResult>> {
        let results: Vec<BatchItemResult> = file_paths
            .iter()
            .map(|path| match self.process_file(path, config) {
                Ok(result) => BatchItemResult {
                    file_path: path.clone(),
                    success: true,
                    result: Some(result),
                    error: None,
                },
                Err(e) => BatchItemResult {
                    file_path: path.clone(),
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                },
            })
            .collect();
        Ok(results)
    }

    /// Serialize config for cache key generation
    fn serialize_config(&self, config: &TesseractConfigDTO) -> String {
        format!(
            "lang={}&psm={}&format={}&table_detection={}&classify_pre_adapted={}&ngram={}&blkrej={}&rowrej={}&dict_correction={}&whitelist={}&primary_params={}&space_variable={}&threshold={}",
            config.language,
            config.psm,
            config.output_format,
            config.enable_table_detection,
            config.classify_use_pre_adapted_templates,
            config.language_model_ngram_on,
            config.tessedit_dont_blkrej_good_wds,
            config.tessedit_dont_rowrej_good_wds,
            config.tessedit_enable_dict_correction,
            config.tessedit_char_whitelist,
            config.tessedit_use_primary_params_model,
            config.textord_space_size_is_variable,
            config.thresholding_method
        )
    }

    /// Perform the actual OCR processing
    fn perform_ocr(&self, image_bytes: &[u8], config: &TesseractConfigDTO) -> Result<ExtractionResultDTO, OCRError> {
        // Decode image to get dimensions and pixel data
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| OCRError::ImageReadError(format!("Failed to decode image: {}", e)))?;

        // Convert to RGB8 for Tesseract
        let rgb_image = img.to_rgb8();
        let (width, height) = rgb_image.dimensions();
        let bytes_per_pixel = 3; // RGB
        let bytes_per_line = width * bytes_per_pixel;

        // Initialize Tesseract API
        let api = TesseractAPI::new();

        // Try to find tessdata directory
        let tessdata_path = std::env::var("TESSDATA_PREFIX")
            .ok()
            .or_else(|| {
                // Try common tessdata directory paths
                let paths = vec![
                    "/opt/homebrew/opt/tesseract/share/tessdata", // Homebrew on ARM Mac
                    "/usr/local/opt/tesseract/share/tessdata",    // Homebrew on Intel Mac
                    "/usr/share/tessdata",                        // Linux
                    "/usr/local/share/tessdata",                  // Linux/macOS
                ];
                paths
                    .into_iter()
                    .find(|p| std::path::Path::new(p).exists())
                    .map(String::from)
            })
            .unwrap_or_default();

        // Initialize with language
        api.init(&tessdata_path, &config.language).map_err(|e| {
            OCRError::InitializationFailed(format!("Failed to initialize language '{}': {}", config.language, e))
        })?;

        // Set PSM mode using enum
        let psm_mode = TessPageSegMode::from_int(config.psm as i32);
        api.set_page_seg_mode(psm_mode)
            .map_err(|e| OCRError::ConfigurationError(format!("Failed to set PSM mode: {}", e)))?;

        // Apply additional Tesseract configuration variables
        api.set_variable(
            "classify_use_pre_adapted_templates",
            &config.classify_use_pre_adapted_templates.to_string(),
        )
        .map_err(|e| {
            OCRError::ConfigurationError(format!("Failed to set classify_use_pre_adapted_templates: {}", e))
        })?;

        api.set_variable("language_model_ngram_on", &config.language_model_ngram_on.to_string())
            .map_err(|e| OCRError::ConfigurationError(format!("Failed to set language_model_ngram_on: {}", e)))?;

        api.set_variable(
            "tessedit_dont_blkrej_good_wds",
            &config.tessedit_dont_blkrej_good_wds.to_string(),
        )
        .map_err(|e| OCRError::ConfigurationError(format!("Failed to set tessedit_dont_blkrej_good_wds: {}", e)))?;

        api.set_variable(
            "tessedit_dont_rowrej_good_wds",
            &config.tessedit_dont_rowrej_good_wds.to_string(),
        )
        .map_err(|e| OCRError::ConfigurationError(format!("Failed to set tessedit_dont_rowrej_good_wds: {}", e)))?;

        api.set_variable(
            "tessedit_enable_dict_correction",
            &config.tessedit_enable_dict_correction.to_string(),
        )
        .map_err(|e| OCRError::ConfigurationError(format!("Failed to set tessedit_enable_dict_correction: {}", e)))?;

        // Only set whitelist if non-empty
        if !config.tessedit_char_whitelist.is_empty() {
            api.set_variable("tessedit_char_whitelist", &config.tessedit_char_whitelist)
                .map_err(|e| OCRError::ConfigurationError(format!("Failed to set tessedit_char_whitelist: {}", e)))?;
        }

        api.set_variable(
            "tessedit_use_primary_params_model",
            &config.tessedit_use_primary_params_model.to_string(),
        )
        .map_err(|e| OCRError::ConfigurationError(format!("Failed to set tessedit_use_primary_params_model: {}", e)))?;

        api.set_variable(
            "textord_space_size_is_variable",
            &config.textord_space_size_is_variable.to_string(),
        )
        .map_err(|e| OCRError::ConfigurationError(format!("Failed to set textord_space_size_is_variable: {}", e)))?;

        api.set_variable("thresholding_method", &config.thresholding_method.to_string())
            .map_err(|e| OCRError::ConfigurationError(format!("Failed to set thresholding_method: {}", e)))?;

        // Set image data
        api.set_image(
            rgb_image.as_raw(),
            width as i32,
            height as i32,
            bytes_per_pixel as i32,
            bytes_per_line as i32,
        )
        .map_err(|e| OCRError::ProcessingFailed(format!("Failed to set image: {}", e)))?;

        // Extract text based on output format
        // Note: If table detection is enabled, we need TSV data regardless of output format
        let tsv_data_for_tables =
            if config.enable_table_detection {
                Some(api.get_tsv_text(0).map_err(|e| {
                    OCRError::ProcessingFailed(format!("Failed to extract TSV for table detection: {}", e))
                })?)
            } else {
                None
            };

        let (content, mime_type) = match config.output_format.as_str() {
            "text" => {
                // Plain text output
                let text = api
                    .get_utf8_text()
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract text: {}", e)))?;
                (text, "text/plain".to_string())
            }
            "markdown" => {
                // Markdown output - extract hOCR and convert to markdown with table extraction
                let hocr = api
                    .get_hocr_text(0)
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;

                // Configure hOCR conversion with table extraction if enabled
                let options = if config.enable_table_detection {
                    Some(html_to_markdown::ConversionOptions {
                        hocr_extract_tables: true,
                        hocr_table_column_threshold: config.table_column_threshold,
                        hocr_table_row_threshold_ratio: config.table_row_threshold_ratio,
                        ..Default::default()
                    })
                } else {
                    None
                };

                let markdown = convert_hocr_to_markdown(&hocr, options)?;
                (markdown, "text/markdown".to_string())
            }
            "hocr" => {
                // hOCR output
                let hocr = api
                    .get_hocr_text(0)
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;
                (hocr, "text/html".to_string())
            }
            "tsv" => {
                // TSV output - reuse data if already extracted for table detection
                let tsv = if let Some(ref tsv) = tsv_data_for_tables {
                    tsv.clone()
                } else {
                    api.get_tsv_text(0)
                        .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract TSV: {}", e)))?
                };
                (tsv, "text/tab-separated-values".to_string())
            }
            _ => {
                return Err(OCRError::ConfigurationError(format!(
                    "Unsupported output format: {}",
                    config.output_format
                )));
            }
        };

        // Build metadata
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), config.language.clone());
        metadata.insert("psm".to_string(), config.psm.to_string());
        metadata.insert("output_format".to_string(), config.output_format.clone());

        // Tables vector to store extracted tables
        let mut tables = Vec::new();

        // Perform table detection if enabled
        if config.enable_table_detection {
            // Use the TSV data we already extracted
            let tsv_data = tsv_data_for_tables.unwrap(); // Safe: we only reach here if enable_table_detection is true

            // Extract words from TSV using configured minimum confidence
            let words = extract_words(&tsv_data, config.table_min_confidence)?;

            if !words.is_empty() {
                // Reconstruct tables from words using configured thresholds
                match reconstruct_table(&words, config.table_column_threshold, config.table_row_threshold_ratio) {
                    Ok(table) if !table.is_empty() => {
                        // Add table count to metadata
                        metadata.insert("table_count".to_string(), "1".to_string());
                        metadata.insert("table_rows".to_string(), table.len().to_string());
                        metadata.insert("table_cols".to_string(), table[0].len().to_string());

                        // Create markdown representation
                        let markdown_table = table_to_markdown(&table);

                        // Note: For markdown output, tables are already included via hOCR conversion
                        // For other outputs (text, tsv, hocr), we keep the TableDTO for API compatibility

                        // Create TableDTO and add to tables vector
                        tables.push(TableDTO {
                            cells: table,
                            markdown: markdown_table,
                            page_number: 0, // Single image, page 0
                        });
                    }
                    _ => {
                        metadata.insert("table_count".to_string(), "0".to_string());
                    }
                }
            } else {
                metadata.insert("table_count".to_string(), "0".to_string());
            }
        }

        Ok(ExtractionResultDTO {
            content,
            mime_type,
            metadata,
            tables,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_processor_creation() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf()));
        assert!(processor.is_ok());
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        // Clear cache should succeed
        assert!(processor.clear_cache().is_ok());

        // Get stats should succeed
        let stats = processor.get_cache_stats();
        assert!(stats.is_ok());
    }
}
