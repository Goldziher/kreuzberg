//! Image extractors for various image formats.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::image::extract_image_metadata;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;

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
    async fn extract_with_ocr(&self, content: &[u8], config: &ExtractionConfig) -> Result<ExtractionResult> {
        use crate::plugins::registry::get_ocr_backend_registry;

        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "OCR config required for image OCR".to_string(),
            source: None,
        })?;

        let backend = {
            let registry = get_ocr_backend_registry();
            let registry = registry.read().map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("Failed to acquire read lock on OCR backend registry: {}", e),
                plugin_name: "ocr-registry".to_string(),
            })?;
            registry.get(&ocr_config.backend)?
        };

        backend.process_image(content, ocr_config).await
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
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let extraction_metadata = extract_image_metadata(content)?;

        let image_metadata = crate::types::ImageMetadata {
            width: extraction_metadata.width,
            height: extraction_metadata.height,
            format: extraction_metadata.format.clone(),
            exif: extraction_metadata.exif_data,
        };

        if config.ocr.is_some() {
            #[cfg(feature = "ocr")]
            {
                let mut ocr_result = self.extract_with_ocr(content, config).await?;

                ocr_result.metadata.format = Some(crate::types::FormatMetadata::Image(image_metadata));
                ocr_result.mime_type = mime_type.to_string();

                return Ok(ocr_result);
            }
            #[cfg(not(feature = "ocr"))]
            {
                let content_text = format!(
                    "Image: {} {}x{}",
                    extraction_metadata.format, extraction_metadata.width, extraction_metadata.height
                );

                return Ok(ExtractionResult {
                    content: content_text,
                    mime_type: mime_type.to_string(),
                    metadata: Metadata {
                        format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                        ..Default::default()
                    },
                    pages: None,
                    tables: vec![],
                    detected_languages: None,
                    chunks: None,
                    images: None,
                });
            }
        }

        Ok(ExtractionResult {
            content: format!(
                "Image: {} {}x{}",
                extraction_metadata.format, extraction_metadata.width, extraction_metadata.height
            ),
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                format: Some(crate::types::FormatMetadata::Image(image_metadata)),
                ..Default::default()
            },
            pages: None,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
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
