use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use kreuzberg_tesseract::{TessPageSegMode, TesseractAPI};

use super::cache::OcrCache;
use super::error::OcrError;
use super::hocr::convert_hocr_to_markdown;
use super::table::{extract_words_from_tsv, reconstruct_table, table_to_markdown};
use super::types::{BatchItemResult, TesseractConfig};
use crate::types::{OcrExtractionResult, OcrTable};

fn strip_control_characters(text: &str) -> String {
    if text
        .chars()
        .any(|c| matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}') && c != '\n' && c != '\r' && c != '\t')
    {
        text.chars()
            .filter(|c| !matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}') || matches!(c, '\n' | '\r' | '\t'))
            .collect()
    } else {
        text.to_string()
    }
}

fn log_ci_debug<F>(enabled: bool, stage: &str, details: F)
where
    F: FnOnce() -> String,
{
    if !enabled {
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    tracing::debug!("[ci-debug][ocr::processor::{stage}] {timestamp:.3}s {}", details());
}

pub struct OcrProcessor {
    cache: OcrCache,
}

impl OcrProcessor {
    pub fn new(cache_dir: Option<std::path::PathBuf>) -> Result<Self, OcrError> {
        let cache = OcrCache::new(cache_dir)?;
        Ok(Self { cache })
    }

    pub fn process_image(&self, image_bytes: &[u8], config: &TesseractConfig) -> Result<OcrExtractionResult, OcrError> {
        config.validate().map_err(OcrError::InvalidConfiguration)?;

        let mut hasher = ahash::AHasher::default();
        use std::hash::{Hash, Hasher};
        image_bytes.hash(&mut hasher);
        let image_hash = format!("{:016x}", hasher.finish());

        let config_str = self.hash_config(config);

        if config.use_cache
            && let Some(cached_result) = self.cache.get_cached_result(&image_hash, "tesseract", &config_str)?
        {
            return Ok(cached_result);
        }

        let result = self.perform_ocr(image_bytes, config)?;

        if config.use_cache {
            let _ = self
                .cache
                .set_cached_result(&image_hash, "tesseract", &config_str, &result);
        }

        Ok(result)
    }

    pub fn clear_cache(&self) -> Result<(), OcrError> {
        self.cache.clear()
    }

    pub fn get_cache_stats(&self) -> Result<super::cache::OcrCacheStats, OcrError> {
        self.cache.get_stats()
    }

    pub fn process_file(&self, file_path: &str, config: &TesseractConfig) -> Result<OcrExtractionResult, OcrError> {
        let image_bytes = std::fs::read(file_path)
            .map_err(|e| OcrError::IOError(format!("Failed to read file '{}': {}", file_path, e)))?;
        self.process_image(&image_bytes, config)
    }

    /// Process multiple image files in parallel using Rayon.
    ///
    /// This method processes OCR operations in parallel across CPU cores for improved throughput.
    /// Results are returned in the same order as the input file paths.
    pub fn process_files_batch(&self, file_paths: Vec<String>, config: &TesseractConfig) -> Vec<BatchItemResult> {
        use rayon::prelude::*;

        file_paths
            .par_iter()
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
            .collect()
    }

    fn hash_config(&self, config: &TesseractConfig) -> String {
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

        format!("{:016x}", hasher.finish())
    }

    fn perform_ocr(&self, image_bytes: &[u8], config: &TesseractConfig) -> Result<OcrExtractionResult, OcrError> {
        let ci_debug_enabled = env::var_os("KREUZBERG_CI_DEBUG").is_some();
        log_ci_debug(ci_debug_enabled, "perform_ocr:start", || {
            format!(
                "bytes={} language={} output={} use_cache={}",
                image_bytes.len(),
                config.language,
                config.output_format,
                config.use_cache
            )
        });

        let img = image::load_from_memory(image_bytes)
            .map_err(|e| OcrError::ImageProcessingFailed(format!("Failed to decode image: {}", e)))?;

        let rgb_image = img.to_rgb8();
        let (width, height) = rgb_image.dimensions();
        let bytes_per_pixel = 3;
        let bytes_per_line = width * bytes_per_pixel;

        log_ci_debug(ci_debug_enabled, "image", || {
            format!(
                "dimensions={}x{} bytes_per_line={} color_type=RGB8",
                width, height, bytes_per_line
            )
        });

        let api = TesseractAPI::new();

        let tessdata_env = env::var("TESSDATA_PREFIX").ok();
        let fallback_paths = [
            "/opt/homebrew/share/tessdata",
            "/opt/homebrew/opt/tesseract/share/tessdata",
            "/usr/local/opt/tesseract/share/tessdata",
            "/usr/share/tessdata",
            "/usr/local/share/tessdata",
            r#"C:\Program Files\Tesseract-OCR\tessdata"#,
            r#"C:\ProgramData\Tesseract-OCR\tessdata"#,
        ];
        let tessdata_path = tessdata_env
            .clone()
            .or_else(|| {
                fallback_paths
                    .iter()
                    .find(|p| Path::new(p).exists())
                    .map(|p| (*p).to_string())
            })
            .unwrap_or_default();

        log_ci_debug(ci_debug_enabled, "tessdata", || {
            let path_preview = env::var_os("PATH").map(|paths| {
                env::split_paths(&paths)
                    .take(6)
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            let resolved_exists = !tessdata_path.is_empty() && Path::new(&tessdata_path).exists();
            let available_fallbacks = fallback_paths
                .iter()
                .filter(|p| Path::new(p).exists())
                .map(|p| (*p).to_string())
                .collect::<Vec<_>>();

            format!(
                "env={:?} resolved={} exists={} fallbacks_found={:?} path_preview={:?}",
                tessdata_env,
                if tessdata_path.is_empty() {
                    "unset"
                } else {
                    &tessdata_path
                },
                resolved_exists,
                available_fallbacks,
                path_preview
            )
        });

        log_ci_debug(ci_debug_enabled, "tesseract_version", || {
            format!("version={}", TesseractAPI::version())
        });

        // Validate language before initializing to prevent segfault ~keep
        // tesseract-rs can crash on empty language or missing language files
        if config.language.trim().is_empty() {
            return Err(OcrError::TesseractInitializationFailed(
                "Language cannot be empty. Please specify a valid language code (e.g., 'eng')".to_string(),
            ));
        }

        // Validate language file exists before initializing to prevent segfault ~keep
        // tesseract-rs can crash if language file is missing instead of returning error
        if !tessdata_path.is_empty() {
            let languages: Vec<&str> = config.language.split('+').collect();
            for lang in languages {
                let lang = lang.trim();
                if lang.is_empty() {
                    continue;
                }
                let traineddata_path = Path::new(&tessdata_path).join(format!("{}.traineddata", lang));
                if !traineddata_path.exists() {
                    return Err(OcrError::TesseractInitializationFailed(format!(
                        "Language '{}' not found. Traineddata file does not exist: {}",
                        lang,
                        traineddata_path.display()
                    )));
                }
            }
        }

        let init_result = api.init(&tessdata_path, &config.language);
        log_ci_debug(ci_debug_enabled, "init", || match &init_result {
            Ok(_) => format!("language={} datapath='{}'", config.language, tessdata_path),
            Err(err) => format!(
                "language={} datapath='{}' error={:?}",
                config.language, tessdata_path, err
            ),
        });

        init_result.map_err(|e| {
            OcrError::TesseractInitializationFailed(format!(
                "Failed to initialize language '{}': {}",
                config.language, e
            ))
        })?;

        if ci_debug_enabled {
            match api.get_available_languages() {
                Ok(languages) => {
                    log_ci_debug(ci_debug_enabled, "available_languages", move || {
                        let preview = languages.iter().take(10).cloned().collect::<Vec<_>>();
                        format!("count={} preview={:?}", languages.len(), preview)
                    });
                }
                Err(err) => {
                    log_ci_debug(ci_debug_enabled, "available_languages_error", move || {
                        format!("error={:?}", err)
                    });
                }
            }
        }

        let psm_mode = TessPageSegMode::from_int(config.psm as i32);
        let psm_result = api.set_page_seg_mode(psm_mode);
        log_ci_debug(ci_debug_enabled, "set_psm", || match &psm_result {
            Ok(_) => format!("mode={}", config.psm),
            Err(err) => format!("error={:?}", err),
        });
        psm_result.map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set PSM mode: {}", e)))?;

        api.set_variable(
            "classify_use_pre_adapted_templates",
            &config.classify_use_pre_adapted_templates.to_string(),
        )
        .map_err(|e| {
            OcrError::InvalidConfiguration(format!("Failed to set classify_use_pre_adapted_templates: {}", e))
        })?;

        api.set_variable("language_model_ngram_on", &config.language_model_ngram_on.to_string())
            .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set language_model_ngram_on: {}", e)))?;

        api.set_variable(
            "tessedit_dont_blkrej_good_wds",
            &config.tessedit_dont_blkrej_good_wds.to_string(),
        )
        .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_dont_blkrej_good_wds: {}", e)))?;

        api.set_variable(
            "tessedit_dont_rowrej_good_wds",
            &config.tessedit_dont_rowrej_good_wds.to_string(),
        )
        .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_dont_rowrej_good_wds: {}", e)))?;

        api.set_variable(
            "tessedit_enable_dict_correction",
            &config.tessedit_enable_dict_correction.to_string(),
        )
        .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_enable_dict_correction: {}", e)))?;

        if !config.tessedit_char_whitelist.is_empty() {
            api.set_variable("tessedit_char_whitelist", &config.tessedit_char_whitelist)
                .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set tessedit_char_whitelist: {}", e)))?;
        }

        api.set_variable(
            "tessedit_use_primary_params_model",
            &config.tessedit_use_primary_params_model.to_string(),
        )
        .map_err(|e| {
            OcrError::InvalidConfiguration(format!("Failed to set tessedit_use_primary_params_model: {}", e))
        })?;

        api.set_variable(
            "textord_space_size_is_variable",
            &config.textord_space_size_is_variable.to_string(),
        )
        .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set textord_space_size_is_variable: {}", e)))?;

        api.set_variable("thresholding_method", &config.thresholding_method.to_string())
            .map_err(|e| OcrError::InvalidConfiguration(format!("Failed to set thresholding_method: {}", e)))?;

        api.set_image(
            rgb_image.as_raw(),
            width as i32,
            height as i32,
            bytes_per_pixel as i32,
            bytes_per_line as i32,
        )
        .map_err(|e| OcrError::ProcessingFailed(format!("Failed to set image: {}", e)))?;

        log_ci_debug(ci_debug_enabled, "set_image", || {
            format!(
                "width={} height={} bytes_per_pixel={} bytes_per_line={}",
                width, height, bytes_per_pixel, bytes_per_line
            )
        });

        let tsv_data_for_tables = if config.enable_table_detection || config.output_format == "tsv" {
            Some(
                api.get_tsv_text(0)
                    .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract TSV: {}", e)))?,
            )
        } else {
            None
        };

        let (raw_content, mime_type) = match config.output_format.as_str() {
            "text" => {
                let text = api
                    .get_utf8_text()
                    .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract text: {}", e)))?;
                (text, "text/plain".to_string())
            }
            "markdown" => {
                let hocr = api
                    .get_hocr_text(0)
                    .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;

                let markdown = convert_hocr_to_markdown(&hocr, None)?;
                (markdown, "text/markdown".to_string())
            }
            "hocr" => {
                let hocr = api
                    .get_hocr_text(0)
                    .map_err(|e| OcrError::ProcessingFailed(format!("Failed to extract hOCR: {}", e)))?;
                (hocr, "text/html".to_string())
            }
            "tsv" => {
                let tsv = tsv_data_for_tables
                    .as_ref()
                    .expect("TSV data should be extracted when output_format is 'tsv'")
                    .clone();
                (tsv, "text/plain".to_string())
            }
            _ => {
                return Err(OcrError::InvalidConfiguration(format!(
                    "Unsupported output format: {}",
                    config.output_format
                )));
            }
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "language".to_string(),
            serde_json::Value::String(config.language.clone()),
        );
        metadata.insert("psm".to_string(), serde_json::Value::String(config.psm.to_string()));
        metadata.insert(
            "output_format".to_string(),
            serde_json::Value::String(config.output_format.clone()),
        );
        metadata.insert("table_count".to_string(), serde_json::Value::String("0".to_string()));
        metadata.insert(
            "tables_detected".to_string(),
            serde_json::Value::String("0".to_string()),
        );
        if config.output_format == "markdown" {
            metadata.insert(
                "source_format".to_string(),
                serde_json::Value::String("hocr".to_string()),
            );
        }

        let mut tables = Vec::new();

        if config.enable_table_detection {
            let tsv_data = tsv_data_for_tables.unwrap();

            let words = extract_words_from_tsv(&tsv_data, config.table_min_confidence)?;

            if !words.is_empty() {
                let table = reconstruct_table(
                    &words,
                    config.table_column_threshold,
                    config.table_row_threshold_ratio,
                    true,
                );
                if !table.is_empty() {
                    metadata.insert("table_count".to_string(), serde_json::Value::String("1".to_string()));
                    metadata.insert(
                        "tables_detected".to_string(),
                        serde_json::Value::String("1".to_string()),
                    );
                    metadata.insert(
                        "table_rows".to_string(),
                        serde_json::Value::String(table.len().to_string()),
                    );
                    metadata.insert(
                        "table_cols".to_string(),
                        serde_json::Value::String(table[0].len().to_string()),
                    );

                    let markdown_table = table_to_markdown(&table);
                    tables.push(OcrTable {
                        cells: table,
                        markdown: markdown_table,
                        page_number: 0,
                    });
                }
            }
        }

        let content = strip_control_characters(&raw_content);

        Ok(OcrExtractionResult {
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

    fn create_test_config() -> TesseractConfig {
        TesseractConfig {
            output_format: "text".to_string(),
            enable_table_detection: false,
            use_cache: false,
            ..TesseractConfig::default()
        }
    }

    #[allow(dead_code)]
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
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf()));
        assert!(processor.is_ok());
    }

    #[test]
    fn test_processor_creation_default_cache_dir() {
        let processor = OcrProcessor::new(None);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        assert!(processor.clear_cache().is_ok());

        let stats = processor.get_cache_stats();
        assert!(stats.is_ok());
    }

    #[test]
    fn test_compute_image_hash_deterministic() {
        use ahash::AHasher;
        use std::hash::{Hash, Hasher};

        let image_bytes = vec![1, 2, 3, 4, 5];

        let mut hasher1 = AHasher::default();
        image_bytes.hash(&mut hasher1);
        let hash1 = format!("{:016x}", hasher1.finish());

        let mut hasher2 = AHasher::default();
        image_bytes.hash(&mut hasher2);
        let hash2 = format!("{:016x}", hasher2.finish());

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_hash_config_deterministic() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let hash1 = processor.hash_config(&config);
        let hash2 = processor.hash_config(&config);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_process_file_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let result = processor.process_file("/nonexistent/file.png", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read file"));
    }

    #[test]
    fn test_process_files_batch_empty() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let results = processor.process_files_batch(vec![], &config);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_process_image_invalid_image_data() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let invalid_data = vec![0, 1, 2, 3, 4];
        let result = processor.process_image(&invalid_data, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_strip_control_characters() {
        let input = "Hello\x00World\x01Test";
        let output = strip_control_characters(input);
        assert_eq!(output, "HelloWorldTest");

        let input_with_newlines = "Hello\nWorld\rTest\t!";
        let output = strip_control_characters(input_with_newlines);
        assert_eq!(output, "Hello\nWorld\rTest\t!");
    }

    #[test]
    fn test_strip_control_characters_all_control() {
        let input = "\x00\x01\x02\x03";
        let output = strip_control_characters(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_control_characters_no_control() {
        let input = "Hello World Test";
        let output = strip_control_characters(input);
        assert_eq!(output, "Hello World Test");
    }

    #[test]
    fn test_strip_control_characters_delete_char() {
        let input = "Hello\x7FWorld";
        let output = strip_control_characters(input);
        assert_eq!(output, "HelloWorld");
    }

    #[test]
    fn test_process_files_batch_single_file() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let results = processor.process_files_batch(vec!["/nonexistent.png".to_string()], &config);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].error.is_some());
        assert!(results[0].result.is_none());
    }

    #[test]
    fn test_process_files_batch_multiple_files() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let file_paths = vec![
            "/nonexistent1.png".to_string(),
            "/nonexistent2.png".to_string(),
            "/nonexistent3.png".to_string(),
        ];

        let results = processor.process_files_batch(file_paths, &config);
        assert_eq!(results.len(), 3);

        for result in &results {
            assert!(!result.success);
            assert!(result.error.is_some());
            assert!(result.result.is_none());
        }
    }

    #[test]
    fn test_batch_item_result_structure() {
        let success_result = BatchItemResult {
            file_path: "test.png".to_string(),
            success: true,
            result: Some(OcrExtractionResult {
                content: "test".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: HashMap::new(),
                tables: vec![],
            }),
            error: None,
        };

        assert!(success_result.success);
        assert!(success_result.result.is_some());
        assert!(success_result.error.is_none());

        let error_result = BatchItemResult {
            file_path: "error.png".to_string(),
            success: false,
            result: None,
            error: Some("Test error".to_string()),
        };

        assert!(!error_result.success);
        assert!(error_result.result.is_none());
        assert!(error_result.error.is_some());
    }

    #[test]
    fn test_hash_config_different_languages() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.language = "eng".to_string();

        let mut config2 = create_test_config();
        config2.language = "fra".to_string();

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_psm() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.psm = 3;

        let mut config2 = create_test_config();
        config2.psm = 6;

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_different_output_format() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.output_format = "text".to_string();

        let mut config2 = create_test_config();
        config2.output_format = "markdown".to_string();

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_table_detection_flag() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.enable_table_detection = false;

        let mut config2 = create_test_config();
        config2.enable_table_detection = true;

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_config_whitelist() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config1 = create_test_config();
        config1.tessedit_char_whitelist = "".to_string();

        let mut config2 = create_test_config();
        config2.tessedit_char_whitelist = "0123456789".to_string();

        let hash1 = processor.hash_config(&config1);
        let hash2 = processor.hash_config(&config2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_process_image_with_cache_disabled() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();

        let mut config = create_test_config();
        config.use_cache = false;

        let invalid_data = vec![0, 1, 2, 3];
        let result = processor.process_image(&invalid_data, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_process_files_batch_preserves_order() {
        let temp_dir = tempdir().unwrap();
        let processor = OcrProcessor::new(Some(temp_dir.path().to_path_buf())).unwrap();
        let config = create_test_config();

        let file_paths = vec![
            "file1.png".to_string(),
            "file2.png".to_string(),
            "file3.png".to_string(),
        ];

        let results = processor.process_files_batch(file_paths.clone(), &config);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].file_path, "file1.png");
        assert_eq!(results[1].file_path, "file2.png");
        assert_eq!(results[2].file_path, "file3.png");
    }

    #[test]
    fn test_compute_image_hash_different_data() {
        use ahash::AHasher;
        use std::hash::{Hash, Hasher};

        let image_bytes1 = vec![1, 2, 3, 4, 5];
        let image_bytes2 = vec![5, 4, 3, 2, 1];

        let mut hasher1 = AHasher::default();
        image_bytes1.hash(&mut hasher1);
        let hash1 = format!("{:016x}", hasher1.finish());

        let mut hasher2 = AHasher::default();
        image_bytes2.hash(&mut hasher2);
        let hash2 = format!("{:016x}", hasher2.finish());

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_log_ci_debug_disabled() {
        log_ci_debug(false, "test_stage", || "test message".to_string());
    }

    #[test]
    fn test_log_ci_debug_enabled() {
        log_ci_debug(true, "test_stage", || "test message".to_string());
    }
}
