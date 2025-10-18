//! Image extractors for various image formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::image::extract_image_metadata;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;

#[cfg(feature = "ocr")]
use crate::ocr::OcrProcessor;

/// Image extractor for various image formats.
///
/// Supports: PNG, JPEG, WebP, BMP, TIFF, GIF.
/// Extracts dimensions, format, and EXIF metadata.
/// Optionally runs OCR when configured.
pub struct ImageExtractor;

impl ImageExtractor {
    /// Create a new image extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract text from image using OCR.
    #[cfg(feature = "ocr")]
    async fn extract_with_ocr(&self, content: &[u8], config: &ExtractionConfig) -> Result<String> {
        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "OCR config required for image OCR".to_string(),
            source: None,
        })?;

        // Get TesseractConfig from OcrConfig
        let tess_config = ocr_config.tesseract_config.as_ref().cloned().unwrap_or_default();

        let tess_config_clone = tess_config.clone();
        let image_data = content.to_vec();

        // Run OCR on blocking thread pool
        let ocr_result = tokio::task::spawn_blocking(move || {
            // Use cache directory from environment or default
            let cache_dir = std::env::var("KREUZBERG_CACHE_DIR").ok().map(std::path::PathBuf::from);

            let proc = OcrProcessor::new(cache_dir)?;

            // Convert TesseractConfig using From trait
            let ocr_tess_config: crate::ocr::types::TesseractConfig = (&tess_config_clone).into();

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

        Ok(ocr_result.content)
    }
}

impl Default for ImageExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ImageExtractor {
    fn name(&self) -> &str {
        "image-extractor"
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

    fn description(&self) -> &str {
        "Extracts dimensions, format, and EXIF data from images (PNG, JPEG, WebP, BMP, TIFF, GIF)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for ImageExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let extraction_metadata = extract_image_metadata(content)?;

        // Build typed metadata - keep the exif_data as-is from extraction module
        let image_metadata = crate::types::ImageMetadata {
            width: extraction_metadata.width,
            height: extraction_metadata.height,
            format: extraction_metadata.format.clone(),
            exif: extraction_metadata.exif_data,
        };

        // Check if OCR is configured
        let content_text = if config.ocr.is_some() {
            // Run OCR to extract text from image
            #[cfg(feature = "ocr")]
            {
                self.extract_with_ocr(content, config).await?
            }
            #[cfg(not(feature = "ocr"))]
            {
                // OCR feature not enabled, fall back to metadata description
                format!(
                    "Image: {} {}x{}",
                    extraction_metadata.format, extraction_metadata.width, extraction_metadata.height
                )
            }
        } else {
            // No OCR configured, return metadata description
            format!(
                "Image: {} {}x{}",
                extraction_metadata.format, extraction_metadata.width, extraction_metadata.height
            )
        };

        Ok(ExtractionResult {
            content: content_text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                image: Some(image_metadata),
                format: Some(extraction_metadata.format),
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/webp",
            "image/bmp",
            "image/tiff",
            "image/gif",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_image_extractor_invalid_image() {
        let extractor = ImageExtractor::new();
        let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
        let config = ExtractionConfig::default();

        let result = extractor.extract_bytes(&invalid_bytes, "image/png", &config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_image_plugin_interface() {
        let extractor = ImageExtractor::new();
        assert_eq!(extractor.name(), "image-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert!(extractor.supported_mime_types().contains(&"image/png"));
        assert!(extractor.supported_mime_types().contains(&"image/jpeg"));
        assert!(extractor.supported_mime_types().contains(&"image/webp"));
        assert_eq!(extractor.priority(), 50);
    }

    #[test]
    fn test_image_extractor_default() {
        let extractor = ImageExtractor;
        assert_eq!(extractor.name(), "image-extractor");
    }
}
