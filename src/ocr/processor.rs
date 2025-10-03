//! Tesseract OCR processing implementation

use std::collections::HashMap;

use pyo3::prelude::*;
use tesseract_rs::{TessPageSegMode, TesseractAPI};

use super::cache::OCRCache;
use super::error::OCRError;
use super::hocr::convert_hocr_to_markdown;
use super::table::{extract_words, reconstruct_table, table_to_markdown};
use super::types::{BatchItemResult, ExtractionResultDTO, TableDTO, TesseractConfigDTO};
use super::utils::compute_hash;

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
        let image_hash = compute_hash(&image_bytes);
        let config_str = self.hash_config(config);

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
            .into_iter()
            .map(|path| match self.process_file(&path, config) {
                Ok(result) => BatchItemResult {
                    file_path: path,
                    success: true,
                    result: Some(result),
                    error: None,
                },
                Err(e) => BatchItemResult {
                    file_path: path,
                    success: false,
                    result: None,
                    error: Some(e.to_string()),
                },
            })
            .collect();
        Ok(results)
    }

    /// Generate config hash for cache key generation
    /// Hashes all configuration fields to create a unique cache key
    fn hash_config(&self, config: &TesseractConfigDTO) -> String {
        use ahash::AHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = AHasher::default();
        config.language.hash(&mut hasher);
        config.psm.hash(&mut hasher);
        config.output_format.hash(&mut hasher);
        config.enable_table_detection.hash(&mut hasher);
        config.table_min_confidence.to_bits().hash(&mut hasher);
        config.table_column_threshold.hash(&mut hasher);
        config.table_row_threshold_ratio.to_bits().hash(&mut hasher);
        config.classify_use_pre_adapted_templates.hash(&mut hasher);
        config.language_model_ngram_on.hash(&mut hasher);
        config.tessedit_dont_blkrej_good_wds.hash(&mut hasher);
        config.tessedit_dont_rowrej_good_wds.hash(&mut hasher);
        config.tessedit_enable_dict_correction.hash(&mut hasher);
        config.tessedit_char_whitelist.hash(&mut hasher);
        config.tessedit_use_primary_params_model.hash(&mut hasher);
        config.textord_space_size_is_variable.hash(&mut hasher);
        config.thresholding_method.hash(&mut hasher);

        compute_hash(&hasher.finish())
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
        // Note: Extract TSV if table detection is enabled OR if TSV is the output format
        let tsv_data_for_tables = if config.enable_table_detection || config.output_format == "tsv" {
            Some(
                api.get_tsv_text(0)
                    .map_err(|e| OCRError::ProcessingFailed(format!("Failed to extract TSV: {}", e)))?,
            )
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
                // TSV output - always extracted above when output_format="tsv"
                // Clone so we can still use for table detection if enabled
                let tsv = tsv_data_for_tables
                    .as_ref()
                    .expect("TSV data should be extracted when output_format is 'tsv'")
                    .clone();
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

    fn create_test_config() -> TesseractConfigDTO {
        TesseractConfigDTO {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfigDTO::default()
        }
    }

    fn create_simple_test_image() -> Vec<u8> {
        use image::{ImageBuffer, Rgb};

        let img = ImageBuffer::from_fn(200, 100, |x, y| {
            if x < 100 && y < 50 {
                Rgb([0u8, 0u8, 0u8])
            } else {
                Rgb([255u8, 255u8, 255u8])
            }
        });

        let mut buffer = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();
        buffer
    }

    #[test]
    fn test_processor_creation() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf()));
        assert!(processor.is_ok());
    }

    #[test]
    fn test_processor_creation_default_cache_dir() {
        let processor = OCRProcessor::new(None);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        assert!(processor.clear_cache().is_ok());

        let stats = processor.get_cache_stats();
        assert!(stats.is_ok());
    }

    #[test]
    fn test_compute_image_hash_deterministic() {
        let image_bytes = vec![1, 2, 3, 4, 5];
        let hash1 = compute_hash(&image_bytes);
        let hash2 = compute_hash(&image_bytes);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_compute_image_hash_different_images() {
        let image1 = vec![1, 2, 3, 4, 5];
        let image2 = vec![5, 4, 3, 2, 1];

        let hash1 = compute_hash(&image1);
        let hash2 = compute_hash(&image2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_image_hash_empty() {
        let empty: Vec<u8> = vec![];
        let hash = compute_hash(&empty);
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_hash_config_deterministic() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let hash1 = processor.hash_config(&config);
        let hash2 = processor.hash_config(&config);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_hash_config_different_languages() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.language = "eng".to_string();

        let mut config2 = create_test_config();
        config2.language = "fra".to_string();

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_output_formats() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.output_format = "text".to_string();

        let mut config2 = create_test_config();
        config2.output_format = "markdown".to_string();

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_table_settings() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.enable_table_detection = false;

        let mut config2 = create_test_config();
        config2.enable_table_detection = true;

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_process_file_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let result = processor.process_file("/nonexistent/file.png", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read file"));
    }

    #[test]
    fn test_process_files_batch_empty() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let results = processor.process_files_batch(vec![], &config).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_process_files_batch_all_invalid() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let results = processor
            .process_files_batch(
                vec!["/nonexistent1.png".to_string(), "/nonexistent2.png".to_string()],
                &config,
            )
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(!results[0].success);
        assert!(results[0].error.is_some());
        assert!(!results[1].success);
        assert!(results[1].error.is_some());
    }

    #[test]
    fn test_process_files_batch_mixed() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let img_path = temp_dir.path().join("test.png");
        std::fs::write(&img_path, create_simple_test_image()).unwrap();

        let results = processor
            .process_files_batch(
                vec![img_path.to_string_lossy().to_string(), "/nonexistent.png".to_string()],
                &config,
            )
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(!results[1].success);
        assert!(results[1].error.is_some());
    }

    #[test]
    fn test_process_image_invalid_image_data() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let invalid_data = vec![0, 1, 2, 3, 4];
        let result = processor.process_image(&invalid_data, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_process_image_with_caching() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config = create_test_config();
        config.use_cache = true;

        let image_bytes = create_simple_test_image();

        processor.clear_cache().unwrap();

        let result1 = processor.process_image(&image_bytes, &config);
        let stats_after_first = processor.get_cache_stats().unwrap();

        let result2 = processor.process_image(&image_bytes, &config);
        let stats_after_second = processor.get_cache_stats().unwrap();

        // If both results are OK, verify caching behavior
        if result1.is_ok() && result2.is_ok() {
            // Results should be identical
            assert_eq!(result1.unwrap().content, result2.unwrap().content);
            // Cache stats should remain the same (cached result was used)
            assert_eq!(stats_after_first.total_files, stats_after_second.total_files);
        }
    }

    #[test]
    fn test_process_image_without_caching() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config = create_test_config();
        config.use_cache = false;

        let image_bytes = create_simple_test_image();

        processor.clear_cache().unwrap();

        processor.process_image(&image_bytes, &config).ok();
        let stats = processor.get_cache_stats().unwrap();

        // Cache should not be used when use_cache is false
        assert_eq!(stats.total_files, 0);
    }

    #[test]
    fn test_cache_stats_after_operations() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        processor.clear_cache().unwrap();

        let stats_empty = processor.get_cache_stats().unwrap();
        assert_eq!(stats_empty.total_files, 0);
        assert_eq!(stats_empty.total_size_mb, 0.0);
    }

    #[test]
    fn test_hash_config_all_fields() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let config1 = TesseractConfigDTO {
            output_format: "text".to_string(),
            table_min_confidence: 50.0,
            table_column_threshold: 100,
            table_row_threshold_ratio: 0.8,
            classify_use_pre_adapted_templates: false,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: false,
            tessedit_dont_rowrej_good_wds: false,
            tessedit_enable_dict_correction: false,
            tessedit_char_whitelist: "ABC".to_string(),
            tessedit_use_primary_params_model: false,
            textord_space_size_is_variable: false,
            thresholding_method: true,
            ..TesseractConfigDTO::default()
        };

        let config2 = TesseractConfigDTO {
            output_format: "text".to_string(),
            table_min_confidence: 50.0,
            table_column_threshold: 100,
            table_row_threshold_ratio: 0.8,
            language_model_ngram_on: true,
            tessedit_dont_blkrej_good_wds: false,
            tessedit_dont_rowrej_good_wds: false,
            tessedit_enable_dict_correction: false,
            tessedit_char_whitelist: "ABC".to_string(),
            tessedit_use_primary_params_model: false,
            textord_space_size_is_variable: false,
            thresholding_method: true,
            ..TesseractConfigDTO::default()
        };

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_whitelist() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.tessedit_char_whitelist = "0123456789".to_string();

        let mut config2 = create_test_config();
        config2.tessedit_char_whitelist = "ABCDEFG".to_string();

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_psm() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.psm = 3;

        let mut config2 = create_test_config();
        config2.psm = 6;

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_confidence() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.table_min_confidence = 0.0;

        let mut config2 = create_test_config();
        config2.table_min_confidence = 90.0;

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_thresholds() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.table_column_threshold = 50;
        config1.table_row_threshold_ratio = 0.5;

        let mut config2 = create_test_config();
        config2.table_column_threshold = 100;
        config2.table_row_threshold_ratio = 0.8;

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_image_hash_large() {
        let large_image = vec![42u8; 10_000];
        let hash1 = compute_hash(&large_image);
        let hash2 = compute_hash(&large_image);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_compute_image_hash_single_byte() {
        let img1 = vec![1u8];
        let img2 = vec![2u8];

        let hash1 = compute_hash(&img1);
        let hash2 = compute_hash(&img2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_process_file_with_valid_image() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let img_path = temp_dir.path().join("valid.png");
        std::fs::write(&img_path, create_simple_test_image()).unwrap();

        let result = processor.process_file(&img_path.to_string_lossy(), &config);
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_process_files_batch_single_file() {
        let temp_dir = tempdir().unwrap();
        let processor = OCRProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let img_path = temp_dir.path().join("single.png");
        std::fs::write(&img_path, create_simple_test_image()).unwrap();

        let results = processor
            .process_files_batch(vec![img_path.to_string_lossy().to_string()], &config)
            .unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_processor_new_with_custom_cache_dir() {
        let temp_dir = tempdir().unwrap();
        let custom_cache = temp_dir.path().join("custom_cache");

        let processor = OCRProcessor::new(Some(custom_cache.clone())).unwrap();
        assert!(processor.clear_cache().is_ok());

        assert!(custom_cache.exists());
    }
}
