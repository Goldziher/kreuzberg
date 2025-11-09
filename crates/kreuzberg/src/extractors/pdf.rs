//! PDF document extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::path::Path;

#[cfg(feature = "pdf")]
use crate::pdf::error::PdfError;
#[cfg(feature = "ocr")]
use crate::pdf::rendering::{PageRenderOptions, PdfRenderer};
#[cfg(feature = "pdf")]
use pdfium_render::prelude::Pdfium;

#[cfg(feature = "ocr")]
const MIN_TOTAL_NON_WHITESPACE: usize = 64;
#[cfg(feature = "ocr")]
const MIN_NON_WHITESPACE_PER_PAGE: f64 = 32.0;
#[cfg(feature = "ocr")]
const MIN_MEANINGFUL_WORD_LEN: usize = 4;
#[cfg(feature = "ocr")]
const MIN_MEANINGFUL_WORDS: usize = 3;
#[cfg(feature = "ocr")]
const MIN_ALNUM_RATIO: f64 = 0.3;

#[cfg(feature = "ocr")]
struct NativeTextStats {
    non_whitespace: usize,
    alnum: usize,
    meaningful_words: usize,
    alnum_ratio: f64,
}

#[cfg(feature = "ocr")]
struct OcrFallbackDecision {
    stats: NativeTextStats,
    avg_non_whitespace: f64,
    avg_alnum: f64,
    fallback: bool,
}

#[cfg(feature = "ocr")]
impl NativeTextStats {
    fn from(text: &str) -> Self {
        let mut non_whitespace = 0usize;
        let mut alnum = 0usize;

        for ch in text.chars() {
            if !ch.is_whitespace() {
                non_whitespace += 1;
                if ch.is_alphanumeric() {
                    alnum += 1;
                }
            }
        }

        let meaningful_words = text
            .split_whitespace()
            .filter(|word| {
                word.chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(MIN_MEANINGFUL_WORD_LEN)
                    .count()
                    >= MIN_MEANINGFUL_WORD_LEN
            })
            .take(MIN_MEANINGFUL_WORDS)
            .count();

        let alnum_ratio = if non_whitespace == 0 {
            0.0
        } else {
            alnum as f64 / non_whitespace as f64
        };

        Self {
            non_whitespace,
            alnum,
            meaningful_words,
            alnum_ratio,
        }
    }
}

#[cfg(feature = "ocr")]
fn evaluate_native_text_for_ocr(native_text: &str, page_count: Option<usize>) -> OcrFallbackDecision {
    let trimmed = native_text.trim();

    if trimmed.is_empty() {
        let empty_stats = NativeTextStats {
            non_whitespace: 0,
            alnum: 0,
            meaningful_words: 0,
            alnum_ratio: 0.0,
        };
        return OcrFallbackDecision {
            stats: empty_stats,
            avg_non_whitespace: 0.0,
            avg_alnum: 0.0,
            fallback: true,
        };
    }

    let stats = NativeTextStats::from(trimmed);
    let pages = page_count.unwrap_or(1).max(1) as f64;
    let avg_non_whitespace = stats.non_whitespace as f64 / pages;
    let avg_alnum = stats.alnum as f64 / pages;

    let has_substantial_text = stats.non_whitespace >= MIN_TOTAL_NON_WHITESPACE
        && avg_non_whitespace >= MIN_NON_WHITESPACE_PER_PAGE
        && stats.meaningful_words >= MIN_MEANINGFUL_WORDS;

    let fallback = if stats.non_whitespace == 0 || stats.alnum == 0 {
        true
    } else if has_substantial_text {
        false
    } else if (stats.alnum_ratio < MIN_ALNUM_RATIO && avg_alnum < MIN_NON_WHITESPACE_PER_PAGE)
        || (stats.non_whitespace < MIN_TOTAL_NON_WHITESPACE && avg_non_whitespace < MIN_NON_WHITESPACE_PER_PAGE)
    {
        true
    } else {
        stats.meaningful_words == 0 && avg_non_whitespace < MIN_NON_WHITESPACE_PER_PAGE
    };

    OcrFallbackDecision {
        stats,
        avg_non_whitespace,
        avg_alnum,
        fallback,
    }
}

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
        use crate::plugins::registry::get_ocr_backend_registry;
        use image::ImageEncoder;
        use image::codecs::png::PngEncoder;
        use std::io::Cursor;

        let ocr_config = config.ocr.as_ref().ok_or_else(|| crate::KreuzbergError::Parsing {
            message: "OCR config required for force_ocr".to_string(),
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
        };

        let mut page_texts = Vec::with_capacity(images.len());

        for image in images {
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

            let ocr_result = backend.process_image(&image_data, ocr_config).await?;

            page_texts.push(ocr_result.content);
        }

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
        #[cfg(feature = "pdf")]
        let (pdf_metadata, native_text) = if config._internal_batch_mode {
            // Batch mode: Move PDF extraction to blocking thread pool to enable parallelism
            let content_owned = content.to_vec();
            tokio::task::spawn_blocking(move || {
                let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                    .or_else(|_| Pdfium::bind_to_system_library())
                    .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to initialize Pdfium: {}", e)))?;

                let pdfium = Pdfium::new(bindings);

                let document = pdfium.load_pdf_from_byte_slice(&content_owned, None).map_err(|e| {
                    let err_msg = e.to_string();
                    if err_msg.contains("password") || err_msg.contains("Password") {
                        PdfError::PasswordRequired
                    } else {
                        PdfError::InvalidPdf(err_msg)
                    }
                })?;

                let metadata = crate::pdf::metadata::extract_metadata_from_document(&document)?;
                let native_text = crate::pdf::text::extract_text_from_pdf_document(&document)?;

                Ok::<_, crate::error::KreuzbergError>((metadata, native_text))
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::Other(format!("PDF extraction task failed: {}", e)))??
        } else {
            // Single-file mode: Direct extraction (no spawn overhead)
            let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to initialize Pdfium: {}", e)))?;

            let pdfium = Pdfium::new(bindings);

            let document = pdfium.load_pdf_from_byte_slice(content, None).map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("password") || err_msg.contains("Password") {
                    PdfError::PasswordRequired
                } else {
                    PdfError::InvalidPdf(err_msg)
                }
            })?;

            let metadata = crate::pdf::metadata::extract_metadata_from_document(&document)?;
            let native_text = crate::pdf::text::extract_text_from_pdf_document(&document)?;

            (metadata, native_text)
        };

        #[cfg(feature = "ocr")]
        let text = if config.force_ocr {
            if config.ocr.is_some() {
                self.extract_with_ocr(content, config).await?
            } else {
                native_text
            }
        } else if config.ocr.is_some() {
            let decision = evaluate_native_text_for_ocr(&native_text, pdf_metadata.page_count);

            if std::env::var("KREUZBERG_DEBUG_OCR").is_ok() {
                eprintln!(
                    "[kreuzberg::pdf::ocr] fallback={} non_whitespace={} alnum={} meaningful_words={} \
                     avg_non_whitespace={:.2} avg_alnum={:.2} alnum_ratio={:.3} pages={}",
                    decision.fallback,
                    decision.stats.non_whitespace,
                    decision.stats.alnum,
                    decision.stats.meaningful_words,
                    decision.avg_non_whitespace,
                    decision.avg_alnum,
                    decision.stats.alnum_ratio,
                    pdf_metadata.page_count.unwrap_or(0)
                );
            }

            if decision.fallback {
                self.extract_with_ocr(content, config).await?
            } else {
                native_text
            }
        } else {
            native_text
        };

        #[cfg(not(feature = "ocr"))]
        let text = native_text;

        let images = if config.images.is_some() {
            match crate::pdf::images::extract_images_from_pdf(content) {
                Ok(pdf_images) => Some(
                    pdf_images
                        .into_iter()
                        .enumerate()
                        .map(|(idx, img)| {
                            let format = img.filters.first().cloned().unwrap_or_else(|| "unknown".to_string());
                            crate::types::ExtractedImage {
                                data: img.data,
                                format,
                                image_index: idx,
                                page_number: Some(img.page_number),
                                width: Some(img.width as u32),
                                height: Some(img.height as u32),
                                colorspace: img.color_space,
                                bits_per_component: img.bits_per_component.map(|b| b as u32),
                                is_mask: false,
                                description: None,
                                ocr_result: None,
                            }
                        })
                        .collect(),
                ),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                #[cfg(feature = "pdf")]
                format: Some(crate::types::FormatMetadata::Pdf(pdf_metadata)),
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images,
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
        50
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

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_to_ocr_for_empty_text() {
        assert!(evaluate_native_text_for_ocr("", Some(1)).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_not_fallback_for_meaningful_text() {
        let sample = "This page has searchable vector text and should avoid OCR.";
        assert!(!evaluate_native_text_for_ocr(sample, Some(1)).fallback);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_should_fallback_for_punctuation_only_text() {
        let sample = " . , ; : -- -- ";
        assert!(evaluate_native_text_for_ocr(sample, Some(2)).fallback);
    }
}
