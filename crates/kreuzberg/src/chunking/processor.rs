//! Text chunking post-processor.
//!
//! This module provides a PostProcessor plugin that chunks text content in
//! extraction results.

use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
use crate::{ExtractionConfig, ExtractionResult, KreuzbergError, Result};
use async_trait::async_trait;

/// Post-processor that chunks text in document content.
///
/// This processor:
/// - Runs in the Middle processing stage
/// - Only processes when `config.chunking` is configured
/// - Stores chunks in `result.chunks`
/// - Uses configurable chunk size and overlap
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::plugins::{Plugin, PostProcessor};
/// use kreuzberg::chunking::processor::ChunkingProcessor;
///
/// let processor = ChunkingProcessor;
/// assert_eq!(processor.name(), "text-chunking");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ChunkingProcessor;

impl Plugin for ChunkingProcessor {
    fn name(&self) -> &str {
        "text-chunking"
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

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PostProcessor for ChunkingProcessor {
    async fn process(&self, result: &mut ExtractionResult, config: &ExtractionConfig) -> Result<()> {
        let chunking_config = match &config.chunking {
            Some(cfg) => cfg,
            None => return Ok(()),
        };

        let chunk_config = crate::chunking::ChunkingConfig {
            max_characters: chunking_config.max_chars,
            overlap: chunking_config.max_overlap,
            trim: true,
            chunker_type: crate::chunking::ChunkerType::Text,
        };

        let chunking_result = crate::chunking::chunk_text(&result.content, &chunk_config, None)
            .map_err(|e| KreuzbergError::Other(format!("Chunking failed: {}", e)))?;
        result.chunks = Some(chunking_result.chunks);

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(&self, _result: &ExtractionResult, config: &ExtractionConfig) -> bool {
        config.chunking.is_some()
    }

    fn estimated_duration_ms(&self, result: &ExtractionResult) -> u64 {
        let text_length = result.content.len();
        // Chunking is fast: ~1ms per 10KB
        (text_length / 10240).max(1) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ChunkingConfig;
    use crate::types::Metadata;

    #[tokio::test]
    async fn test_chunking_processor() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig {
            chunking: Some(ChunkingConfig {
                max_chars: 100,
                max_overlap: 10,
                embedding: None,
                preset: None,
            }),
            ..Default::default()
        };

        let mut result = ExtractionResult {
	            content: "This is a longer text that should be split into multiple chunks to test the chunking processor functionality.".to_string(),
	            mime_type: "text/plain".to_string(),
	            metadata: Metadata::default(),
	            tables: vec![],
	            detected_languages: None,
	            chunks: None,
	            images: None,
	            pages: None,
	        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.chunks.is_some());
        let chunks = result.chunks.unwrap();
        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_chunking_processor_no_config() {
        let processor = ChunkingProcessor;
        let config = ExtractionConfig::default();

        let mut result = ExtractionResult {
            content: "Some text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.chunks.is_none());
    }

    #[test]
    fn test_chunking_processor_plugin_interface() {
        let processor = ChunkingProcessor;
        assert_eq!(processor.name(), "text-chunking");
        assert!(!processor.version().is_empty());
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[test]
    fn test_chunking_processor_stage() {
        let processor = ChunkingProcessor;
        assert_eq!(processor.processing_stage(), ProcessingStage::Middle);
    }

    #[test]
    fn test_chunking_processor_should_process() {
        let processor = ChunkingProcessor;

        let result = ExtractionResult {
            content: "Sample text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        };

        let config_with_chunking = ExtractionConfig {
            chunking: Some(crate::core::config::ChunkingConfig {
                max_chars: 100,
                max_overlap: 10,
                embedding: None,
                preset: None,
            }),
            ..Default::default()
        };
        assert!(processor.should_process(&result, &config_with_chunking));

        let config_without_chunking = ExtractionConfig::default();
        assert!(!processor.should_process(&result, &config_without_chunking));
    }

    #[test]
    fn test_chunking_processor_estimated_duration() {
        let processor = ChunkingProcessor;

        let short_result = ExtractionResult {
            content: "Short".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        };

        let long_result = ExtractionResult {
            content: "a".repeat(100000),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        };

        let short_duration = processor.estimated_duration_ms(&short_result);
        let long_duration = processor.estimated_duration_ms(&long_result);

        assert!(long_duration > short_duration);
    }
}
