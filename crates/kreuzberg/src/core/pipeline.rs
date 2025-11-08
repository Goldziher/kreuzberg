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
/// 1. Post-Processors - Execute by stage (Early, Middle, Late) to modify/enhance the result
/// 2. Quality Processing - Text cleaning and quality scoring
/// 3. Chunking - Text splitting if enabled
/// 4. Validators - Run validation hooks on the processed result (can fail fast)
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

                #[cfg(feature = "embeddings")]
                if let Some(ref embedding_config) = chunking_config.embedding
                    && let Some(ref mut chunks) = result.chunks
                {
                    match crate::embeddings::generate_embeddings_for_chunks(chunks, embedding_config) {
                        Ok(()) => {
                            result
                                .metadata
                                .additional
                                .insert("embeddings_generated".to_string(), serde_json::Value::Bool(true));
                        }
                        Err(e) => {
                            result
                                .metadata
                                .additional
                                .insert("embedding_error".to_string(), serde_json::Value::String(e.to_string()));
                        }
                    }
                }

                #[cfg(not(feature = "embeddings"))]
                if chunking_config.embedding.is_some() {
                    result.metadata.additional.insert(
                        "embedding_error".to_string(),
                        serde_json::Value::String("Embeddings feature not enabled".to_string()),
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

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Metadata;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref REGISTRY_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
    }

    #[tokio::test]
    async fn test_run_pipeline_basic() {
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
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
            images: None,
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
            images: None,
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
            images: None,
        };
        let config = ExtractionConfig {
            chunking: Some(crate::ChunkingConfig {
                max_chars: 500,
                max_overlap: 50,
                embedding: None,
                preset: None,
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
            images: None,
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
            images: None,
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
            images: None,
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
            images: None,
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
            images: None,
        };
        let config = ExtractionConfig {
            enable_quality_processing: true,
            chunking: Some(crate::ChunkingConfig {
                max_chars: 500,
                max_overlap: 50,
                embedding: None,
                preset: None,
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
            images: None,
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
            images: None,
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
            images: None,
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

    #[tokio::test]
    async fn test_postprocessor_runs_before_validator() {
        let _guard = REGISTRY_TEST_GUARD.lock().unwrap();
        use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
        use async_trait::async_trait;
        use std::sync::Arc;

        struct TestPostProcessor;
        impl Plugin for TestPostProcessor {
            fn name(&self) -> &str {
                "test-processor"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl PostProcessor for TestPostProcessor {
            async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                result
                    .metadata
                    .additional
                    .insert("processed".to_string(), serde_json::json!(true));
                Ok(())
            }

            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }
        }

        struct TestValidator;
        impl Plugin for TestValidator {
            fn name(&self) -> &str {
                "test-validator"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl Validator for TestValidator {
            async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                let processed = result
                    .metadata
                    .additional
                    .get("processed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if !processed {
                    return Err(crate::KreuzbergError::Validation {
                        message: "Post-processor did not run before validator".to_string(),
                        source: None,
                    });
                }
                Ok(())
            }
        }

        let pp_registry = crate::plugins::registry::get_post_processor_registry();
        {
            let mut registry = pp_registry.write().unwrap();
            registry.register(Arc::new(TestPostProcessor), 0).unwrap();
        }

        let val_registry = crate::plugins::registry::get_validator_registry();
        {
            let mut registry = val_registry.write().unwrap();
            registry.register(Arc::new(TestValidator)).unwrap();
        }

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();
        let processed = run_pipeline(result, &config).await;

        {
            let mut registry = pp_registry.write().unwrap();
            registry.remove("test-processor").unwrap();
        }
        {
            let mut registry = val_registry.write().unwrap();
            registry.remove("test-validator").unwrap();
        }

        assert!(processed.is_ok(), "Validator should have seen post-processor metadata");
        let processed = processed.unwrap();
        assert_eq!(
            processed.metadata.additional.get("processed"),
            Some(&serde_json::json!(true)),
            "Post-processor metadata should be present"
        );
    }

    #[tokio::test]
    #[cfg(feature = "quality")]
    async fn test_quality_processing_runs_before_validator() {
        let _guard = REGISTRY_TEST_GUARD.lock().unwrap();
        use crate::plugins::{Plugin, Validator};
        use async_trait::async_trait;
        use std::sync::Arc;

        struct QualityValidator;
        impl Plugin for QualityValidator {
            fn name(&self) -> &str {
                "quality-validator"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl Validator for QualityValidator {
            async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                if !result.metadata.additional.contains_key("quality_score") {
                    return Err(crate::KreuzbergError::Validation {
                        message: "Quality processing did not run before validator".to_string(),
                        source: None,
                    });
                }
                Ok(())
            }
        }

        let val_registry = crate::plugins::registry::get_validator_registry();
        {
            let mut registry = val_registry.write().unwrap();
            registry.register(Arc::new(QualityValidator)).unwrap();
        }

        let result = ExtractionResult {
            content: "This is meaningful test content for quality scoring.".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig {
            enable_quality_processing: true,
            ..Default::default()
        };

        let processed = run_pipeline(result, &config).await;

        {
            let mut registry = val_registry.write().unwrap();
            registry.remove("quality-validator").unwrap();
        }

        assert!(processed.is_ok(), "Validator should have seen quality_score");
    }

    #[tokio::test]
    async fn test_multiple_postprocessors_run_before_validator() {
        let _guard = REGISTRY_TEST_GUARD.lock().unwrap();
        use crate::plugins::{Plugin, PostProcessor, ProcessingStage, Validator};
        use async_trait::async_trait;
        use std::sync::Arc;

        struct EarlyProcessor;
        impl Plugin for EarlyProcessor {
            fn name(&self) -> &str {
                "early-proc"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl PostProcessor for EarlyProcessor {
            async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                let mut order = result
                    .metadata
                    .additional
                    .get("execution_order")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                order.push(serde_json::json!("early"));
                result
                    .metadata
                    .additional
                    .insert("execution_order".to_string(), serde_json::json!(order));
                Ok(())
            }

            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Early
            }
        }

        struct LateProcessor;
        impl Plugin for LateProcessor {
            fn name(&self) -> &str {
                "late-proc"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl PostProcessor for LateProcessor {
            async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                let mut order = result
                    .metadata
                    .additional
                    .get("execution_order")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                order.push(serde_json::json!("late"));
                result
                    .metadata
                    .additional
                    .insert("execution_order".to_string(), serde_json::json!(order));
                Ok(())
            }

            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Late
            }
        }

        struct OrderValidator;
        impl Plugin for OrderValidator {
            fn name(&self) -> &str {
                "order-validator"
            }
            fn version(&self) -> String {
                "1.0.0".to_string()
            }
            fn initialize(&self) -> Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl Validator for OrderValidator {
            async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
                let order = result
                    .metadata
                    .additional
                    .get("execution_order")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| crate::KreuzbergError::Validation {
                        message: "No execution order found".to_string(),
                        source: None,
                    })?;

                if order.len() != 2 {
                    return Err(crate::KreuzbergError::Validation {
                        message: format!("Expected 2 processors to run, got {}", order.len()),
                        source: None,
                    });
                }

                if order[0] != "early" || order[1] != "late" {
                    return Err(crate::KreuzbergError::Validation {
                        message: format!("Wrong execution order: {:?}", order),
                        source: None,
                    });
                }

                Ok(())
            }
        }

        let pp_registry = crate::plugins::registry::get_post_processor_registry();
        {
            let mut registry = pp_registry.write().unwrap();
            registry.register(Arc::new(EarlyProcessor), 0).unwrap();
            registry.register(Arc::new(LateProcessor), 0).unwrap();
        }

        let val_registry = crate::plugins::registry::get_validator_registry();
        {
            let mut registry = val_registry.write().unwrap();
            registry.register(Arc::new(OrderValidator)).unwrap();
        }

        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let config = ExtractionConfig::default();
        let processed = run_pipeline(result, &config).await;

        {
            let mut registry = pp_registry.write().unwrap();
            registry.remove("early-proc").unwrap();
            registry.remove("late-proc").unwrap();
        }
        {
            let mut registry = val_registry.write().unwrap();
            registry.remove("order-validator").unwrap();
        }

        assert!(processed.is_ok(), "All processors should run before validator");
    }
}
