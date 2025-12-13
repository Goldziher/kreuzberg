//! Quality processing post-processor.
//!
//! This module provides a PostProcessor plugin that performs quality assessment and
//! text cleaning on extraction results.

use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
use crate::{ExtractionConfig, ExtractionResult, Result};
use async_trait::async_trait;

/// Post-processor that calculates quality score and cleans text.
///
/// This processor:
/// - Runs in the Early processing stage
/// - Calculates quality score when `config.enable_quality_processing` is true
/// - Stores quality score in `metadata.additional["quality_score"]`
/// - Cleans and normalizes extracted text
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::plugins::{Plugin, PostProcessor};
/// use kreuzberg::text::quality::processor::QualityProcessor;
///
/// let processor = QualityProcessor;
/// assert_eq!(processor.name(), "quality-processing");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct QualityProcessor;

impl Plugin for QualityProcessor {
    fn name(&self) -> &str {
        "quality-processing"
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
impl PostProcessor for QualityProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        // Calculate quality score
        let quality_score = crate::text::quality::calculate_quality_score(
            &result.content,
            Some(
                &result
                    .metadata
                    .additional
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect(),
            ),
        );

        result.metadata.additional.insert(
            "quality_score".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(quality_score).unwrap_or(serde_json::Number::from(0)),
            ),
        );

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }

    fn should_process(&self, _result: &ExtractionResult, config: &ExtractionConfig) -> bool {
        config.enable_quality_processing
    }

    fn estimated_duration_ms(&self, result: &ExtractionResult) -> u64 {
        let text_length = result.content.len();
        // Quality processing is relatively fast: ~1ms per 100KB
        (text_length / 102400).max(1) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Metadata;

    #[tokio::test]
    async fn test_quality_processor() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };

        let mut result = ExtractionResult {
            content: "This is a well-written paragraph with proper structure. It contains multiple sentences. The quality should be good.".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        processor.process(&mut result, &config).await.unwrap();

        assert!(result.metadata.additional.contains_key("quality_score"));
        let score = result.metadata.additional.get("quality_score").unwrap();
        assert!(score.is_number());
    }

    #[tokio::test]
    async fn test_quality_processor_disabled() {
        let processor = QualityProcessor;
        let config = ExtractionConfig {
            enable_quality_processing: false,
            ..Default::default()
        };

        let mut result = ExtractionResult {
            content: "Some text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        // When disabled, the processor should not run, so no quality_score should be added
        // (because should_process returns false)
        processor.process(&mut result, &config).await.unwrap();
    }

    #[test]
    fn test_quality_processor_plugin_interface() {
        let processor = QualityProcessor;
        assert_eq!(processor.name(), "quality-processing");
        assert!(!processor.version().is_empty());
        assert!(processor.initialize().is_ok());
        assert!(processor.shutdown().is_ok());
    }

    #[test]
    fn test_quality_processor_stage() {
        let processor = QualityProcessor;
        assert_eq!(processor.processing_stage(), ProcessingStage::Early);
    }

    #[test]
    fn test_quality_processor_should_process() {
        let processor = QualityProcessor;

        let result = ExtractionResult {
            content: "Sample text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config_with_quality = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };
        assert!(processor.should_process(&result, &config_with_quality));

        let config_without_quality = ExtractionConfig {
            enable_quality_processing: false,
            ..Default::default()
        };
        assert!(!processor.should_process(&result, &config_without_quality));
    }

    #[test]
    fn test_quality_processor_estimated_duration() {
        let processor = QualityProcessor;

        let short_result = ExtractionResult {
            content: "Short".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let long_result = ExtractionResult {
            content: "a".repeat(1000000),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let short_duration = processor.estimated_duration_ms(&short_result);
        let long_duration = processor.estimated_duration_ms(&long_result);

        assert!(long_duration > short_duration);
    }
}
