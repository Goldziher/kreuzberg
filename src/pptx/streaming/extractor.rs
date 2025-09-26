//! Main streaming PPTX extractor

use crate::pptx::config::ParserConfig;
use crate::pptx::container::PptxContainer;
use crate::pptx::content_builder::ContentBuilder;
use crate::pptx::metadata::extract_metadata;
use crate::pptx::notes::extract_all_notes;
use crate::pptx::streaming::iterator::SlideIterator;
use crate::pptx::types::{PptxExtractionResultDTO, PptxMetadataDTO, Result};
use pyo3::prelude::*;
use std::path::Path;

/// Streaming PPTX extractor DTO for memory-efficient processing
#[pyclass]
pub struct StreamingPptxExtractorDTO {
    config: ParserConfig,
}

#[pymethods]
impl StreamingPptxExtractorDTO {
    #[new]
    pub fn new(extract_images: Option<bool>, max_cache_mb: Option<usize>) -> Self {
        let config = ParserConfig {
            extract_images: extract_images.unwrap_or(true),
            max_cache_size_mb: max_cache_mb.unwrap_or(256),
            ..Default::default()
        };

        Self { config }
    }

    /// Extract PPTX from file path using streaming approach
    pub fn extract_from_path(&self, path: String) -> PyResult<PptxExtractionResultDTO> {
        let result = self
            .extract_streaming(&path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("PPTX extraction failed: {}", e)))?;
        Ok(result)
    }
}

impl StreamingPptxExtractorDTO {
    /// Internal streaming extraction method
    fn extract_streaming<P: AsRef<Path>>(&self, path: P) -> Result<PptxExtractionResultDTO> {
        let mut container = PptxContainer::open(path)?;

        let metadata = self.extract_metadata(&mut container)?;

        let notes = extract_all_notes(&mut container)?;

        let mut iterator = SlideIterator::new(container, self.config.clone());
        let slide_count = iterator.slide_count();

        let estimated_capacity = slide_count * 1024;
        let mut content_builder = ContentBuilder::with_capacity(estimated_capacity);

        let mut total_image_count = 0;
        let mut total_table_count = 0;

        while let Some(slide) = iterator.next_slide()? {
            content_builder.add_slide_header(slide.slide_number);

            let slide_content = slide.to_markdown(&self.config);
            content_builder.add_text(&slide_content);

            if let Some(slide_notes) = notes.get(&slide.slide_number) {
                content_builder.add_notes(slide_notes);
            }

            total_image_count += slide.image_count();
            total_table_count += slide.table_count();
        }

        Ok(PptxExtractionResultDTO {
            content: content_builder.build(),
            metadata,
            slide_count,
            image_count: total_image_count,
            table_count: total_table_count,
        })
    }

    fn extract_metadata(&self, container: &mut PptxContainer) -> Result<PptxMetadataDTO> {
        match container.read_file("docProps/core.xml") {
            Ok(core_xml) => extract_metadata(&core_xml),
            Err(_) => Ok(PptxMetadataDTO {
                title: None,
                author: None,
                description: None,
                summary: None,
                fonts: Vec::new(),
            }),
        }
    }
}
