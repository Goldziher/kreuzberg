//! Tesseract OCR processing implementation

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use ahash::AHasher;
use pyo3::prelude::*;
use tesseract_rs::{TessPageSegMode, TesseractAPI};

use super::cache::OCRCache;
use super::error::OCRError;
use super::table::{extract_words, reconstruct_table, table_to_markdown};
use super::types::{ExtractionResultDTO, TesseractConfigDTO};

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

    /// Serialize config for cache key generation
    fn serialize_config(&self, config: &TesseractConfigDTO) -> String {
        format!(
            "lang={}&psm={}&format={}&table_detection={}",
            config.language, config.psm, config.output_format, config.enable_table_detection
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

        // Initialize with language (empty string for auto-detected tessdata directory)
        api.init("", &config.language).map_err(|e| {
            OCRError::InitializationFailed(format!("Failed to initialize language '{}': {}", config.language, e))
        })?;

        // Set PSM mode using enum
        let psm_mode = TessPageSegMode::from_int(config.psm as i32);
        api.set_page_seg_mode(psm_mode)
            .map_err(|e| OCRError::ConfigurationError(format!("Failed to set PSM mode: {}", e)))?;

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
        let (mut content, mime_type) = match config.output_format.as_str() {
            "text" | "markdown" => {
                // Extract UTF-8 text
                let text = api
                    .get_utf8_text()
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract text: {}", e)))?;
                let mime = if config.output_format == "markdown" {
                    "text/markdown"
                } else {
                    "text/plain"
                };
                (text, mime.to_string())
            }
            "hocr" => {
                // hOCR output
                let hocr = api
                    .get_hocr_text(0)
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;
                (hocr, "text/html".to_string())
            }
            "tsv" => {
                // TSV output
                let tsv = api
                    .get_tsv_text(0)
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract TSV: {}", e)))?;
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

        // Perform table detection if enabled
        if config.enable_table_detection {
            // Get TSV data for table detection
            let tsv_data = api
                .get_tsv_text(0)
                .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract TSV for table detection: {}", e)))?;

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

                        // If output format is markdown, append the table to content
                        if config.output_format == "markdown" {
                            let markdown_table = table_to_markdown(&table);
                            if !content.is_empty() && !content.ends_with('\n') {
                                content.push_str("\n\n");
                            }
                            content.push_str(&markdown_table);
                        }
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
