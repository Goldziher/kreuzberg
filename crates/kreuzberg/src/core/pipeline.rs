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
    {
        let validator_registry = crate::plugins::registry::get_validator_registry();
        let validators = {
            let registry = validator_registry
                .read()
                .map_err(|e| crate::KreuzbergError::Other(format!("Validator registry lock poisoned: {}", e)))?;
            registry.get_all()
        };

        for validator in validators {
            if validator.should_validate(&result, config) {
                validator.validate(&result, config).await?;
            }
        }
    }

    #[cfg(feature = "quality")]
    if config.enable_quality_processing {
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
    }

    #[cfg(not(feature = "quality"))]
    if config.enable_quality_processing {
        result.metadata.additional.insert(
            "quality_processing_error".to_string(),
            serde_json::Value::String("Quality processing feature not enabled".to_string()),
        );
    }

    #[cfg(feature = "chunking")]
    if let Some(ref chunking_config) = config.chunking {
        let chunk_config = crate::chunking::ChunkingConfig {
            max_characters: chunking_config.max_chars,
            overlap: chunking_config.max_overlap,
            trim: true,
            chunker_type: crate::chunking::ChunkerType::Text,
        };

        match crate::chunking::chunk_text(&result.content, &chunk_config) {
            Ok(chunking_result) => {
                result.chunks = Some(chunking_result.chunks);

                if let Some(ref chunks) = result.chunks {
                    result.metadata.additional.insert(
                        "chunk_count".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(chunks.len())),
                    );
                }
            }
            Err(e) => {
                result
                    .metadata
                    .additional
                    .insert("chunking_error".to_string(), serde_json::Value::String(e.to_string()));
            }
        }
    }

    #[cfg(not(feature = "chunking"))]
    if config.chunking.is_some() {
        result.metadata.additional.insert(
            "chunking_error".to_string(),
            serde_json::Value::String("Chunking feature not enabled".to_string()),
        );
    }

    #[cfg(feature = "language-detection")]
    if let Some(ref lang_config) = config.language_detection {
        match crate::language_detection::detect_languages(&result.content, lang_config) {
            Ok(detected) => {
                result.detected_languages = detected;
            }
            Err(e) => {
                result.metadata.additional.insert(
                    "language_detection_error".to_string(),
                    serde_json::Value::String(e.to_string()),
                );
            }
        }
    }

    #[cfg(not(feature = "language-detection"))]
    if config.language_detection.is_some() {
        result.metadata.additional.insert(
            "language_detection_error".to_string(),
            serde_json::Value::String("Language detection feature not enabled".to_string()),
        );
    }

    let pp_config = config.postprocessor.as_ref();
    let postprocessing_enabled = pp_config.is_none_or(|c| c.enabled);

    if postprocessing_enabled {
        #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
        {
            let _ = crate::keywords::ensure_initialized();
        }

        let processor_registry = crate::plugins::registry::get_post_processor_registry();

        for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
            let processors = {
                let registry = processor_registry.read().map_err(|e| {
                    crate::KreuzbergError::Other(format!("Post-processor registry lock poisoned: {}", e))
                })?;
                registry.get_for_stage(stage)
            };

            for processor in processors {
                let processor_name = processor.name();

                let should_run = if let Some(config) = pp_config {
                    if let Some(ref enabled) = config.enabled_processors {
                        enabled.iter().any(|name| name == processor_name)
                    } else if let Some(ref disabled) = config.disabled_processors {
                        !disabled.iter().any(|name| name == processor_name)
                    } else {
                        true
                    }
                } else {
                    true
                };

                if should_run
                    && processor.should_process(&result, config)
                    && let Err(e) = processor.process(&mut result, config).await
                {
                    let error_key = format!("processing_error_{}", processor_name);
                    result
                        .metadata
                        .additional
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
    use crate::types::Metadata;

    #[tokio::test]
    async fn test_run_pipeline_basic() {
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.content, "test");
    }

    #[tokio::test]
    #[cfg(feature = "quality")]
    async fn test_pipeline_with_quality_processing() {
        let result = ExtractionResult {
            content: "This is a test document with some meaningful content.".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(processed.metadata.additional.contains_key("quality_score"));
    }

    #[tokio::test]
    async fn test_pipeline_without_quality_processing() {
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig {
            enable_quality_processing: false,
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(!processed.metadata.additional.contains_key("quality_score"));
    }

    #[tokio::test]
    #[cfg(feature = "chunking")]
    async fn test_pipeline_with_chunking() {
        let result = ExtractionResult {
            content: "This is a long text that should be chunked. ".repeat(100),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig {
            chunking: Some(crate::ChunkingConfig {
                max_chars: 500,
                max_overlap: 50,
            }),
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(processed.metadata.additional.contains_key("chunk_count"));
        let chunk_count = processed.metadata.additional.get("chunk_count").unwrap();
        assert!(chunk_count.as_u64().unwrap() > 1);
    }

    #[tokio::test]
    async fn test_pipeline_without_chunking() {
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig {
            chunking: None,
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(!processed.metadata.additional.contains_key("chunk_count"));
    }

    #[tokio::test]
    async fn test_pipeline_preserves_metadata() {
        use std::collections::HashMap;
        let mut additional = HashMap::new();
        additional.insert("source".to_string(), serde_json::json!("test"));
        additional.insert("page".to_string(), serde_json::json!(1));

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata {
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(
            processed.metadata.additional.get("source").unwrap(),
            &serde_json::json!("test")
        );
        assert_eq!(
            processed.metadata.additional.get("page").unwrap(),
            &serde_json::json!(1)
        );
    }

    #[tokio::test]
    async fn test_pipeline_preserves_tables() {
        use crate::types::Table;

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 0,
        };

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![table],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.tables.len(), 1);
        assert_eq!(processed.tables[0].cells.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_empty_content() {
        let result = ExtractionResult {
            content: String::new(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig::default();

        let processed = run_pipeline(result, &config).await.unwrap();
        assert_eq!(processed.content, "");
    }

    #[tokio::test]
    #[cfg(feature = "chunking")]
    async fn test_pipeline_with_all_features() {
        let result = ExtractionResult {
            content: "This is a comprehensive test document. ".repeat(50),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };
        let config = ExtractionConfig {
            enable_quality_processing: true,
            chunking: Some(crate::ChunkingConfig {
                max_chars: 500,
                max_overlap: 50,
            }),
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();
        assert!(processed.metadata.additional.contains_key("quality_score"));
        assert!(processed.metadata.additional.contains_key("chunk_count"));
    }

    #[tokio::test]
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    async fn test_pipeline_with_keyword_extraction() {
        let result = ExtractionResult {
            content: r#"
Machine learning is a branch of artificial intelligence that focuses on
building systems that can learn from data. Deep learning is a subset of
machine learning that uses neural networks with multiple layers.
Natural language processing enables computers to understand human language.
            "#
            .to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };

        #[cfg(feature = "keywords-yake")]
        let keyword_config = crate::keywords::KeywordConfig::yake();

        #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
        let keyword_config = crate::keywords::KeywordConfig::rake();

        let config = ExtractionConfig {
            keywords: Some(keyword_config),
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();

        assert!(processed.metadata.additional.contains_key("keywords"));

        let keywords_value = processed.metadata.additional.get("keywords").unwrap();
        assert!(keywords_value.is_array());

        let keywords = keywords_value.as_array().unwrap();
        assert!(!keywords.is_empty(), "Should have extracted keywords");

        let first_keyword = &keywords[0];
        assert!(first_keyword.is_object());
        assert!(first_keyword.get("text").is_some());
        assert!(first_keyword.get("score").is_some());
        assert!(first_keyword.get("algorithm").is_some());
    }

    #[tokio::test]
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    async fn test_pipeline_without_keyword_config() {
        let result = ExtractionResult {
            content: "Machine learning and artificial intelligence.".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };

        let config = ExtractionConfig {
            keywords: None,
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();

        assert!(!processed.metadata.additional.contains_key("keywords"));
    }

    #[tokio::test]
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    async fn test_pipeline_keyword_extraction_short_content() {
        let result = ExtractionResult {
            content: "Short text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        };

        #[cfg(feature = "keywords-yake")]
        let keyword_config = crate::keywords::KeywordConfig::yake();

        #[cfg(all(feature = "keywords-rake", not(feature = "keywords-yake")))]
        let keyword_config = crate::keywords::KeywordConfig::rake();

        let config = ExtractionConfig {
            keywords: Some(keyword_config),
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await.unwrap();

        assert!(!processed.metadata.additional.contains_key("keywords"));
    }
}
