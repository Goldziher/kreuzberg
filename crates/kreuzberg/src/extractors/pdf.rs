//! PDF document extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::path::Path;

#[cfg(feature = "ocr")]
use crate::ocr::OcrProcessor;
#[cfg(feature = "ocr")]
use crate::pdf::rendering::{PageRenderOptions, PdfRenderer};

/// PDF document extractor using pypdfium2 and playa-pdf.
pub struct PdfExtractor;

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract text from PDF using OCR.
    ///
    /// Renders all pages to images and processes them with OCR.
    #[cfg(feature = "ocr")]
    async fn extract_with_ocr(&self, content: &[u8], config: &ExtractionConfig) -> Result<String> {
        use image::ImageEncoder;
        use image::codecs::png::PngEncoder;
        use std::io::Cursor;

        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "OCR config required for force_ocr".to_string(),
            source: None,
        })?;

        // Get TesseractConfig from OcrConfig
        let tess_config = ocr_config.tesseract_config.as_ref().cloned().unwrap_or_default();

        // Render all PDF pages to images (synchronous, drops renderer before async processing)
        let images = {
            let render_options = PageRenderOptions::default();
            let renderer = PdfRenderer::new().map_err(|e| crate::KreuzbergError::Parsing {
                message: format!("Failed to initialize PDF renderer: {}", e),
                source: None,
            })?;

            renderer
                .render_all_pages(content, &render_options)
                .map_err(|e| crate::KreuzbergError::Parsing {
                    message: format!("Failed to render PDF pages: {}", e),
                    source: None,
                })?
            // renderer is dropped here, before async processing
        };

        // Process each page with OCR
        let mut page_texts = Vec::with_capacity(images.len());

        for image in images {
            // Convert DynamicImage to PNG bytes
            let rgb_image = image.to_rgb8();
            let (width, height) = rgb_image.dimensions();

            let mut image_bytes = Cursor::new(Vec::new());
            let encoder = PngEncoder::new(&mut image_bytes);
            encoder
                .write_image(&rgb_image, width, height, image::ColorType::Rgb8.into())
                .map_err(|e| crate::KreuzbergError::Parsing {
                    message: format!("Failed to encode image: {}", e),
                    source: None,
                })?;

            let image_data = image_bytes.into_inner();
            let tess_config_clone = tess_config.clone();

            // Run OCR on this page in blocking task
            let ocr_result = tokio::task::spawn_blocking(move || {
                // Use cache directory from environment or default
                let cache_dir = std::env::var("KREUZBERG_CACHE_DIR").ok().map(std::path::PathBuf::from);

                let proc = OcrProcessor::new(cache_dir)?;

                // Convert TesseractConfig to ocr::types::TesseractConfig
                let ocr_tess_config = crate::ocr::types::TesseractConfig {
                    psm: tess_config_clone.psm as u8,
                    language: tess_config_clone.language.clone(),
                    output_format: tess_config_clone.output_format.clone(),
                    oem: tess_config_clone.oem as u8,
                    min_confidence: tess_config_clone.min_confidence,
                    preprocessing: tess_config_clone.preprocessing.clone(),
                    enable_table_detection: tess_config_clone.enable_table_detection,
                    table_min_confidence: tess_config_clone.table_min_confidence,
                    table_column_threshold: tess_config_clone.table_column_threshold as u32,
                    table_row_threshold_ratio: tess_config_clone.table_row_threshold_ratio,
                    use_cache: tess_config_clone.use_cache,
                    classify_use_pre_adapted_templates: tess_config_clone.classify_use_pre_adapted_templates,
                    language_model_ngram_on: tess_config_clone.language_model_ngram_on,
                    tessedit_dont_blkrej_good_wds: tess_config_clone.tessedit_dont_blkrej_good_wds,
                    tessedit_dont_rowrej_good_wds: tess_config_clone.tessedit_dont_rowrej_good_wds,
                    tessedit_enable_dict_correction: tess_config_clone.tessedit_enable_dict_correction,
                    tessedit_char_whitelist: tess_config_clone.tessedit_char_whitelist.clone(),
                    tessedit_char_blacklist: tess_config_clone.tessedit_char_blacklist.clone(),
                    tessedit_use_primary_params_model: tess_config_clone.tessedit_use_primary_params_model,
                    textord_space_size_is_variable: tess_config_clone.textord_space_size_is_variable,
                    thresholding_method: tess_config_clone.thresholding_method,
                };

                proc.process_image(&image_data, &ocr_tess_config)
            })
            .await
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("OCR task failed: {}", e),
                source: None,
            })?
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("OCR processing failed: {}", e),
                source: None,
            })?;

            page_texts.push(ocr_result.content);
        }

        // Combine all page texts with page breaks
        Ok(page_texts.join("\n\n"))
    }
}

impl Plugin for PdfExtractor {
    fn name(&self) -> &str {
        "pdf-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for PdfExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Extract metadata first (needed regardless of text extraction method)
        let pdf_metadata = crate::pdf::metadata::extract_metadata(content)?;

        // Determine if we should use OCR instead of native text extraction
        let should_use_ocr = config.force_ocr && config.ocr.is_some();

        let text = if should_use_ocr {
            // Force OCR mode: render pages and run OCR
            #[cfg(feature = "ocr")]
            {
                self.extract_with_ocr(content, config).await?
            }
            #[cfg(not(feature = "ocr"))]
            {
                // OCR feature not enabled, fall back to native extraction
                crate::pdf::text::extract_text_from_pdf(content)?
            }
        } else {
            // Normal mode: extract native text
            let native_text = crate::pdf::text::extract_text_from_pdf(content)?;

            // If native extraction produced empty content and OCR is available, try OCR
            #[cfg(feature = "ocr")]
            {
                if native_text.trim().is_empty() && config.ocr.is_some() {
                    self.extract_with_ocr(content, config).await?
                } else {
                    native_text
                }
            }
            #[cfg(not(feature = "ocr"))]
            {
                native_text
            }
        };

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                #[cfg(feature = "pdf")]
                pdf: Some(pdf_metadata),
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }

    fn priority(&self) -> i32 {
        50 // Default priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_extractor_plugin_interface() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.name(), "pdf-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pdf_extractor_supported_mime_types() {
        let extractor = PdfExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 1);
        assert!(mime_types.contains(&"application/pdf"));
    }

    #[test]
    fn test_pdf_extractor_priority() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.priority(), 50);
    }
}
