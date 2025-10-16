//! Post-processing pipeline orchestration.
//!
//! This module orchestrates the post-processing pipeline, executing validators,
//! quality processing, chunking, and custom hooks in the correct order.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::ProcessingStage;
use crate::types::ExtractionResult;

/// Run the post-processing pipeline on an extraction result.
///
/// Executes post-processing in the following order:
/// 1. Validators - Run validation hooks (can fail fast)
/// 2. Quality Processing - Text cleaning and quality scoring
/// 3. Chunking - Text splitting if enabled
/// 4. Post-Processors - Execute by stage (Early, Middle, Late)
/// 5. Custom Hooks - User-registered plugins
///
/// # Arguments
///
/// * `result` - The extraction result to process
/// * `config` - Extraction configuration
///
/// # Returns
///
/// The processed extraction result.
///
/// # Errors
///
/// - Validator errors bubble up immediately
/// - Post-processor errors are caught and recorded in metadata
/// - System errors (IO, RuntimeError equivalents) always bubble up
pub async fn run_pipeline(mut result: ExtractionResult, config: &ExtractionConfig) -> Result<ExtractionResult> {
    // 1. Run validators (fail fast on validation errors)
    {
        let validator_registry = crate::plugins::registry::get_validator_registry();
        let validators = {
            let registry = validator_registry
                .read()
                .map_err(|e| crate::KreuzbergError::Other(format!("Validator registry lock poisoned: {}", e)))?;
            registry.get_all()
        }; // Release lock

        for validator in validators {
            // Check if validator should process this result
            if validator.should_validate(&result, config) {
                validator.validate(&result, config).await?;
            }
        }
    }

    // 2. Quality processing (feature-gated)
    #[cfg(feature = "quality")]
    if config.enable_quality_processing {
        let quality_score = crate::text::quality::calculate_quality_score(
            &result.content,
            Some(
                &result
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect(),
            ),
        );
        result.metadata.insert(
            "quality_score".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(quality_score).unwrap_or(serde_json::Number::from(0)),
            ),
        );
    }

    #[cfg(not(feature = "quality"))]
    if config.enable_quality_processing {
        // Quality processing requested but feature not enabled
        result.metadata.insert(
            "quality_processing_error".to_string(),
            serde_json::Value::String("Quality processing feature not enabled".to_string()),
        );
    }

    // 3. Chunking (feature-gated)
    #[cfg(feature = "chunking")]
    if let Some(ref chunking_config) = config.chunking {
        // Convert config to chunking module's config type
        let chunk_config = crate::chunking::ChunkingConfig {
            max_characters: chunking_config.max_chars,
            overlap: chunking_config.max_overlap,
            trim: true,                                       // Default to trimming whitespace
            chunker_type: crate::chunking::ChunkerType::Text, // Default chunker type
        };

        match crate::chunking::chunk_text(&result.content, &chunk_config) {
            Ok(chunking_result) => {
                // Convert ChunkingResult to ExtractionResult chunks (currently empty, will be populated when chunks field is added)
                // For now, store chunk count in metadata
                result.metadata.insert(
                    "chunk_count".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(chunking_result.chunks.len())),
                );
            }
            Err(e) => {
                // Record chunking error in metadata, continue with degraded result
                result
                    .metadata
                    .insert("chunking_error".to_string(), serde_json::Value::String(e.to_string()));
            }
        }
    }

    #[cfg(not(feature = "chunking"))]
    if config.chunking.is_some() {
        // Chunking requested but feature not enabled
        result.metadata.insert(
            "chunking_error".to_string(),
            serde_json::Value::String("Chunking feature not enabled".to_string()),
        );
    }

    // 4. Language detection (feature-gated)
    #[cfg(feature = "language-detection")]
    if let Some(ref lang_config) = config.language_detection {
        match crate::language_detection::detect_languages(&result.content, lang_config) {
            Ok(detected) => {
                result.detected_languages = detected;
            }
            Err(e) => {
                // Record language detection error in metadata, continue with degraded result
                result.metadata.insert(
                    "language_detection_error".to_string(),
                    serde_json::Value::String(e.to_string()),
                );
            }
        }
    }

    #[cfg(not(feature = "language-detection"))]
    if config.language_detection.is_some() {
        // Language detection requested but feature not enabled
        result.metadata.insert(
            "language_detection_error".to_string(),
            serde_json::Value::String("Language detection feature not enabled".to_string()),
        );
    }

    // 5. Post-processors by stage (Early, Middle, Late)
    let processor_registry = crate::plugins::registry::get_post_processor_registry();

    for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
        let processors = {
            let registry = processor_registry
                .read()
                .map_err(|e| crate::KreuzbergError::Other(format!("Post-processor registry lock poisoned: {}", e)))?;
            registry.get_for_stage(stage)
        }; // Release lock

        for processor in processors {
            // Check if processor should process this result
            if processor.should_process(&result, config) {
                // Processors now take &mut ExtractionResult and return Result<()>,
                // which avoids unnecessary cloning of potentially large results.
                // On error, the original result is preserved (not modified).
                if let Err(e) = processor.process(&mut result, config).await {
                    // Record processor error in metadata, continue with current result
                    let error_key = format!("processing_error_{}", processor.name());
                    result
                        .metadata
                        .insert(error_key, serde_json::Value::String(e.to_string()));
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_pipeline_basic() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.content, "test");
    }

    #[tokio::test]
    #[cfg(feature = "quality")]
    async fn test_pipeline_with_quality_processing() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: "This is a test document with some meaningful content.".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let mut config = ExtractionConfig::default();
        config.enable_quality_processing = true;

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(processed.metadata.contains_key("quality_score"));
    }

    #[tokio::test]
    async fn test_pipeline_without_quality_processing() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let mut config = ExtractionConfig::default();
        config.enable_quality_processing = false;

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(!processed.metadata.contains_key("quality_score"));
    }

    #[tokio::test]
    #[cfg(feature = "chunking")]
    async fn test_pipeline_with_chunking() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: "This is a long text that should be chunked. ".repeat(100),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let mut config = ExtractionConfig::default();
        config.chunking = Some(crate::ChunkingConfig {
            max_chars: 500,
            max_overlap: 50,
        });

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(processed.metadata.contains_key("chunk_count"));
        let chunk_count = processed.metadata.get("chunk_count").unwrap();
        assert!(chunk_count.as_u64().unwrap() > 1);
    }

    #[tokio::test]
    async fn test_pipeline_without_chunking() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let mut config = ExtractionConfig::default();
        config.chunking = None;

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(!processed.metadata.contains_key("chunk_count"));
    }

    #[tokio::test]
    async fn test_pipeline_preserves_metadata() {
        use std::collections::HashMap;
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), serde_json::json!("test"));
        metadata.insert("page".to_string(), serde_json::json!(1));

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.metadata.get("source").unwrap(), &serde_json::json!("test"));
        assert_eq!(processed.metadata.get("page").unwrap(), &serde_json::json!(1));
    }

    #[tokio::test]
    async fn test_pipeline_preserves_tables() {
        use crate::types::Table;
        use std::collections::HashMap;

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![table],
            detected_languages: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.tables.len(), 1);
        assert_eq!(processed.tables[0].cells.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_empty_content() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: String::new(),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.content, "");
    }

    #[tokio::test]
    #[cfg(feature = "chunking")]
    async fn test_pipeline_with_all_features() {
        use std::collections::HashMap;
        let result = ExtractionResult {
            content: "This is a comprehensive test document. ".repeat(50),
            mime_type: "text/plain".to_string(),
            metadata: HashMap::new(),
            tables: vec![],
            detected_languages: None,
        };
        let mut config = ExtractionConfig::default();
        config.enable_quality_processing = true;
        config.chunking = Some(crate::ChunkingConfig {
            max_chars: 500,
            max_overlap: 50,
        });

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(processed.metadata.contains_key("quality_score"));
        assert!(processed.metadata.contains_key("chunk_count"));
    }
}
