//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::path::Path;

#[cfg(feature = "ocr")]
use crate::ocr::OcrProcessor;

/// PowerPoint presentation extractor.
///
/// Supports: .pptx, .pptm, .ppsx
pub struct PptxExtractor;

impl Default for PptxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Process extracted images with OCR if configured.
    #[cfg(feature = "ocr")]
    async fn process_images_with_ocr(
        &self,
        mut images: Vec<crate::types::ExtractedImage>,
        config: &ExtractionConfig,
    ) -> Result<Vec<crate::types::ExtractedImage>> {
        if config.ocr.is_none() {
            return Ok(images);
        }

        let ocr_config = config.ocr.as_ref().unwrap();
        let tess_config = ocr_config.tesseract_config.as_ref().cloned().unwrap_or_default();

        for image in &mut images {
            let image_data = image.data.clone();
            let tess_config_clone = tess_config.clone();
            let span = tracing::Span::current();

            let ocr_result = tokio::task::spawn_blocking(move || {
                let _guard = span.entered();
                let cache_dir = std::env::var("KREUZBERG_CACHE_DIR").ok().map(std::path::PathBuf::from);

                let proc = OcrProcessor::new(cache_dir)?;
                let ocr_tess_config: crate::ocr::types::TesseractConfig = (&tess_config_clone).into();
                proc.process_image(&image_data, &ocr_tess_config)
            })
            .await
            .map_err(|e| crate::KreuzbergError::Ocr {
                message: format!("OCR task failed: {}", e),
                source: None,
            })?;

            match ocr_result {
                Ok(ocr_extraction) => {
                    let extraction_result = ExtractionResult {
                        content: ocr_extraction.content,
                        mime_type: image.format.clone(),
                        metadata: Metadata::default(),
                        tables: vec![],
                        detected_languages: None,
                        chunks: None,
                        images: None,
                        pages: None,
                    };
                    image.ocr_result = Some(Box::new(extraction_result));
                }
                Err(_) => {
                    image.ocr_result = None;
                }
            }
        }

        Ok(images)
    }
}

impl Plugin for PptxExtractor {
    fn name(&self) -> &str {
        "pptx-extractor"
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
impl DocumentExtractor for PptxExtractor {
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
        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);

        let pages_config = config.pages.clone();
        let pptx_result = if crate::core::batch_mode::is_batch_mode() {
            let content_owned = content.to_vec();
            let span = tracing::Span::current();
            tokio::task::spawn_blocking(move || {
                let _guard = span.entered();
                crate::extraction::pptx::extract_pptx_from_bytes(&content_owned, extract_images, pages_config.as_ref())
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("PPTX extraction task failed: {}", e)))??
        } else {
            crate::extraction::pptx::extract_pptx_from_bytes(content, extract_images, config.pages.as_ref())?
        };

        let mut additional = std::collections::HashMap::new();
        additional.insert("slide_count".to_string(), serde_json::json!(pptx_result.slide_count));
        additional.insert("image_count".to_string(), serde_json::json!(pptx_result.image_count));
        additional.insert("table_count".to_string(), serde_json::json!(pptx_result.table_count));

        let images = if !pptx_result.images.is_empty() {
            #[cfg(feature = "ocr")]
            {
                let processed_images = self.process_images_with_ocr(pptx_result.images, config).await?;
                Some(processed_images)
            }
            #[cfg(not(feature = "ocr"))]
            {
                Some(pptx_result.images)
            }
        } else {
            None
        };

        // Build BaseMetadata with PageStructure if available
        let mut metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Pptx(pptx_result.metadata)),
            additional,
            ..Default::default()
        };

        // Set pages field in BaseMetadata if PageStructure was built
        if let Some(page_structure) = pptx_result.page_structure {
            metadata.pages = Some(page_structure);
        }

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string(),
            metadata,
            pages: pptx_result.page_contents,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images,
        })
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);

        let pptx_result =
            crate::extraction::pptx::extract_pptx_from_path(path_str, extract_images, config.pages.as_ref())?;

        let mut additional = std::collections::HashMap::new();
        additional.insert("slide_count".to_string(), serde_json::json!(pptx_result.slide_count));
        additional.insert("image_count".to_string(), serde_json::json!(pptx_result.image_count));
        additional.insert("table_count".to_string(), serde_json::json!(pptx_result.table_count));

        let images = if !pptx_result.images.is_empty() {
            #[cfg(feature = "ocr")]
            {
                let processed_images = self.process_images_with_ocr(pptx_result.images, config).await?;
                Some(processed_images)
            }
            #[cfg(not(feature = "ocr"))]
            {
                Some(pptx_result.images)
            }
        } else {
            None
        };

        // Build BaseMetadata with PageStructure if available
        let mut metadata = Metadata {
            format: Some(crate::types::FormatMetadata::Pptx(pptx_result.metadata)),
            additional,
            ..Default::default()
        };

        // Set pages field in BaseMetadata if PageStructure was built
        if let Some(page_structure) = pptx_result.page_structure {
            metadata.pages = Some(page_structure);
        }

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string(),
            metadata,
            pages: pptx_result.page_contents,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pptx_extractor_plugin_interface() {
        let extractor = PptxExtractor::new();
        assert_eq!(extractor.name(), "pptx-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pptx_extractor_supported_mime_types() {
        let extractor = PptxExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 3);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.presentationml.presentation"));
    }
}
