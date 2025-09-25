//! Main streaming PPTX extractor

use crate::pptx::config::ParserConfig;
use crate::pptx::container::PptxContainer;
use crate::pptx::content_builder::ContentBuilder;
use crate::pptx::metadata::extract_metadata;
use crate::pptx::notes::extract_all_notes;
use crate::pptx::streaming::iterator::SlideIterator;
use crate::pptx::types::{PptxExtractionResult, PptxMetadata, Result};
use pyo3::prelude::*;
use std::path::Path;

/// Streaming PPTX extractor for memory-efficient processing
#[pyclass]
pub struct StreamingPptxExtractor {
    config: ParserConfig,
}

#[pymethods]
impl StreamingPptxExtractor {
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
    pub fn extract_from_path(&self, path: String) -> PyResult<PptxExtractionResult> {
        let result = self
            .extract_streaming(&path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("PPTX extraction failed: {}", e)))?;
        Ok(result)
    }

    // Note: slide_iterator removed from Python interface for now
    // Can be added later with proper PyO3 iterator implementation
}

impl StreamingPptxExtractor {
    /// Internal streaming extraction method
    fn extract_streaming<P: AsRef<Path>>(&self, path: P) -> Result<PptxExtractionResult> {
        let mut container = PptxContainer::open(path)?;

        // Extract metadata
        let metadata = self.extract_metadata(&mut container)?;

        // Extract notes
        let notes = extract_all_notes(&mut container)?;

        // Create slide iterator
        let mut iterator = SlideIterator::new(container, self.config.clone());
        let slide_count = iterator.slide_count();

        // Estimate content capacity based on slide count
        let estimated_capacity = slide_count * 1024; // ~1KB per slide estimate
        let mut content_builder = ContentBuilder::with_capacity(estimated_capacity);

        // Process slides one by one
        let mut total_image_count = 0;
        let mut total_table_count = 0;

        while let Some(slide) = iterator.next_slide()? {
            // Add slide header
            content_builder.add_slide_header(slide.slide_number);

            // Convert slide to markdown
            let slide_content = slide.to_markdown(&self.config);
            content_builder.add_text(&slide_content);

            // Add notes if available
            if let Some(slide_notes) = notes.get(&slide.slide_number) {
                content_builder.add_notes(slide_notes);
            }

            // Update counters
            total_image_count += slide.image_count();
            total_table_count += slide.table_count();
        }

        Ok(PptxExtractionResult {
            content: content_builder.build(),
            metadata,
            slide_count,
            image_count: total_image_count,
            table_count: total_table_count,
        })
    }

    fn extract_metadata(&self, container: &mut PptxContainer) -> Result<PptxMetadata> {
        // Try to extract from docProps/core.xml
        match container.read_file("docProps/core.xml") {
            Ok(core_xml) => extract_metadata(&core_xml),
            Err(_) => {
                // Return default metadata if not found
                Ok(PptxMetadata {
                    title: None,
                    author: None,
                    description: None,
                    summary: None,
                    fonts: Vec::new(),
                })
            }
        }
    }
}
