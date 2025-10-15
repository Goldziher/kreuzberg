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
            let registry = validator_registry.read().unwrap();
            registry.get_all()
        }; // Release lock

        for validator in validators {
            // Check if validator should process this result
            if validator.should_validate(&result, config) {
                validator.validate(&result, config).await?;
            }
        }
    }

    // 2. Quality processing
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

    // 3. Chunking
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

    // 4. Post-processors by stage (Early, Middle, Late)
    let processor_registry = crate::plugins::registry::get_post_processor_registry();

    for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
        let processors = {
            let registry = processor_registry.read().unwrap();
            registry.get_for_stage(stage)
        }; // Release lock

        for processor in processors {
            // Check if processor should process this result
            if processor.should_process(&result, config) {
                match processor.process(result.clone(), config).await {
                    Ok(processed) => result = processed,
                    Err(e) => {
                        // Record processor error in metadata, continue with original result
                        let error_key = format!("processing_error_{}", processor.name());
                        result
                            .metadata
                            .insert(error_key, serde_json::Value::String(e.to_string()));
                    }
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
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.content, "test");
    }
}
