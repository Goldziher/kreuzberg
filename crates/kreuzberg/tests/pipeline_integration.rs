//! Comprehensive pipeline integration tests.
//!
//! Tests the post-processing pipeline orchestration: ordering, error handling,
//! metadata/content modifications, and integration with extractors.
//!
//! IMPORTANT: These tests use a global registry and must run serially to avoid interference.

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, PostProcessorConfig};
use kreuzberg::core::pipeline::run_pipeline;
use kreuzberg::plugins::registry::get_post_processor_registry;
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::types::{ExtractionResult, Metadata};
use kreuzberg::{KreuzbergError, Result};
use serial_test::serial;
use std::sync::Arc;

struct OrderTrackingProcessor {
    name: String,
    stage: ProcessingStage,
}

impl Plugin for OrderTrackingProcessor {
    fn name(&self) -> &str {
        &self.name
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
impl PostProcessor for OrderTrackingProcessor {
    async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
        result.content.push_str(&format!("[{}]", self.name));
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }
}

struct MetadataAddingProcessor {
    name: String,
    key: String,
    value: String,
}

impl Plugin for MetadataAddingProcessor {
    fn name(&self) -> &str {
        &self.name
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
impl PostProcessor for MetadataAddingProcessor {
    async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
        result
            .metadata
            .additional
            .insert(self.key.clone(), serde_json::json!(self.value));
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }
}

struct FailingProcessor {
    name: String,
    error_message: String,
}

impl Plugin for FailingProcessor {
    fn name(&self) -> &str {
        &self.name
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
impl PostProcessor for FailingProcessor {
    async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
        Err(KreuzbergError::Plugin {
            message: self.error_message.clone(),
            plugin_name: self.name.clone(),
        })
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

fn clear_processor_registry() {
    let registry = get_post_processor_registry();
    let mut reg = registry
        .write()
        .expect("Failed to acquire write lock on registry in test");
    let _ = reg.shutdown_all();
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_no_processors() {
    clear_processor_registry();

    let result = ExtractionResult {
        content: "original content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "original content");
}

#[tokio::test]
#[serial]
async fn test_pipeline_single_processor_per_stage() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let early = Arc::new(OrderTrackingProcessor {
            name: "early".to_string(),
            stage: ProcessingStage::Early,
        });
        let middle = Arc::new(OrderTrackingProcessor {
            name: "middle".to_string(),
            stage: ProcessingStage::Middle,
        });
        let late = Arc::new(OrderTrackingProcessor {
            name: "late".to_string(),
            stage: ProcessingStage::Late,
        });

        reg.register(early, 50).unwrap();
        reg.register(middle, 50).unwrap();
        reg.register(late, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[early][middle][late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_multiple_processors_per_stage() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let early_high = Arc::new(OrderTrackingProcessor {
            name: "early-high".to_string(),
            stage: ProcessingStage::Early,
        });
        let early_medium = Arc::new(OrderTrackingProcessor {
            name: "early-medium".to_string(),
            stage: ProcessingStage::Early,
        });
        let early_low = Arc::new(OrderTrackingProcessor {
            name: "early-low".to_string(),
            stage: ProcessingStage::Early,
        });

        reg.register(early_low, 10).unwrap();
        reg.register(early_high, 100).unwrap();
        reg.register(early_medium, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[early-high][early-medium][early-low]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_all_stages_enabled() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: format!("{:?}", stage),
                stage,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[Early][Middle][Late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_postprocessing_disabled() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let processor = Arc::new(OrderTrackingProcessor {
            name: "processor".to_string(),
            stage: ProcessingStage::Early,
        });
        reg.register(processor, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: false,
            enabled_processors: None,
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start");
}

#[tokio::test]
#[serial]
async fn test_pipeline_early_stage_runs_first() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let late = Arc::new(OrderTrackingProcessor {
            name: "late".to_string(),
            stage: ProcessingStage::Late,
        });
        let early = Arc::new(OrderTrackingProcessor {
            name: "early".to_string(),
            stage: ProcessingStage::Early,
        });

        reg.register(late, 50).unwrap();
        reg.register(early, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[early][late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_middle_stage_runs_second() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let early = Arc::new(OrderTrackingProcessor {
            name: "early".to_string(),
            stage: ProcessingStage::Early,
        });
        let middle = Arc::new(OrderTrackingProcessor {
            name: "middle".to_string(),
            stage: ProcessingStage::Middle,
        });

        reg.register(middle, 50).unwrap();
        reg.register(early, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[early][middle]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_late_stage_runs_last() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for stage in [ProcessingStage::Late, ProcessingStage::Early, ProcessingStage::Middle] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: format!("{:?}", stage),
                stage,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[Early][Middle][Late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_within_stage_priority_order() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for (name, priority) in [("p1", 100), ("p2", 10), ("p3", 50), ("p4", 75)] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: name.to_string(),
                stage: ProcessingStage::Early,
            });
            reg.register(processor, priority).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[p1][p4][p3][p2]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_cross_stage_data_flow() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let early = Arc::new(MetadataAddingProcessor {
            name: "early".to_string(),
            key: "stage".to_string(),
            value: "early".to_string(),
        });

        struct MiddleProcessor;
        impl Plugin for MiddleProcessor {
            fn name(&self) -> &str {
                "middle"
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
        impl PostProcessor for MiddleProcessor {
            async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                if let Some(stage) = result.metadata.additional.get("stage") {
                    result.content.push_str(&format!("[saw:{}]", stage.as_str().unwrap()));
                }
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }
        }

        reg.register(early, 50).unwrap();
        reg.register(Arc::new(MiddleProcessor), 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.contains("[saw:early]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_early_stage_error_recorded() {
    clear_processor_registry();

    struct EarlyFailingProcessor;
    impl Plugin for EarlyFailingProcessor {
        fn name(&self) -> &str {
            "early-failing"
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
    impl PostProcessor for EarlyFailingProcessor {
        async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Err(KreuzbergError::Plugin {
                message: "Early error".to_string(),
                plugin_name: "early-failing".to_string(),
            })
        }
        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");
        reg.register(Arc::new(EarlyFailingProcessor), 50).unwrap();
    }

    let result = ExtractionResult {
        content: "content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(
        processed
            .metadata
            .additional
            .contains_key("processing_error_early-failing")
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_middle_stage_error_propagation() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let failing = Arc::new(FailingProcessor {
            name: "middle-failing".to_string(),
            error_message: "Middle stage error".to_string(),
        });

        reg.register(failing, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(
        processed
            .metadata
            .additional
            .contains_key("processing_error_middle-failing")
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_late_stage_error_doesnt_affect_earlier_stages() {
    clear_processor_registry();

    struct LateFailingProcessor;
    impl Plugin for LateFailingProcessor {
        fn name(&self) -> &str {
            "late-failing"
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
    impl PostProcessor for LateFailingProcessor {
        async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Err(KreuzbergError::Plugin {
                message: "Late error".to_string(),
                plugin_name: "late-failing".to_string(),
            })
        }
        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let early = Arc::new(OrderTrackingProcessor {
            name: "early".to_string(),
            stage: ProcessingStage::Early,
        });
        let late_failing = Arc::new(LateFailingProcessor);

        reg.register(early, 50).unwrap();
        reg.register(late_failing, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start[early]");
    assert!(
        processed
            .metadata
            .additional
            .contains_key("processing_error_late-failing")
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_processor_error_doesnt_stop_other_processors() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let p1 = Arc::new(OrderTrackingProcessor {
            name: "p1".to_string(),
            stage: ProcessingStage::Early,
        });
        let _p2_failing = Arc::new(FailingProcessor {
            name: "p2-failing".to_string(),
            error_message: "Test error".to_string(),
        });
        let p3 = Arc::new(OrderTrackingProcessor {
            name: "p3".to_string(),
            stage: ProcessingStage::Late,
        });

        struct EarlyFailingProcessor {
            name: String,
        }
        impl Plugin for EarlyFailingProcessor {
            fn name(&self) -> &str {
                &self.name
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
        impl PostProcessor for EarlyFailingProcessor {
            async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                Err(KreuzbergError::Plugin {
                    message: "Test error".to_string(),
                    plugin_name: self.name.clone(),
                })
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Early
            }
        }

        reg.register(p1, 100).unwrap();
        reg.register(
            Arc::new(EarlyFailingProcessor {
                name: "p2-failing".to_string(),
            }),
            50,
        )
        .unwrap();
        reg.register(p3, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.contains("[p1]"));
    assert!(processed.content.contains("[p3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_multiple_processor_errors() {
    clear_processor_registry();

    struct MultiFailingProcessor {
        name: String,
        stage: ProcessingStage,
    }
    impl Plugin for MultiFailingProcessor {
        fn name(&self) -> &str {
            &self.name
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
    impl PostProcessor for MultiFailingProcessor {
        async fn process(&self, _: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Err(KreuzbergError::Plugin {
                message: format!("{} error", self.name),
                plugin_name: self.name.clone(),
            })
        }
        fn processing_stage(&self) -> ProcessingStage {
            self.stage
        }
    }

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for (name, stage) in [
            ("fail1", ProcessingStage::Early),
            ("fail2", ProcessingStage::Middle),
            ("fail3", ProcessingStage::Late),
        ] {
            let processor = Arc::new(MultiFailingProcessor {
                name: name.to_string(),
                stage,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.metadata.additional.contains_key("processing_error_fail1"));
    assert!(processed.metadata.additional.contains_key("processing_error_fail2"));
    assert!(processed.metadata.additional.contains_key("processing_error_fail3"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_error_context_preservation() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let failing = Arc::new(FailingProcessor {
            name: "context-test".to_string(),
            error_message: "Detailed error message with context".to_string(),
        });

        reg.register(failing, 50).unwrap();
    }

    let result = ExtractionResult {
        content: "content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();

    let error_value = processed
        .metadata
        .additional
        .get("processing_error_context-test")
        .unwrap();
    let error_str = error_value.as_str().unwrap();
    assert!(error_str.contains("Detailed error message with context"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_metadata_added_in_early_visible_in_middle() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let early = Arc::new(MetadataAddingProcessor {
            name: "early".to_string(),
            key: "early_key".to_string(),
            value: "early_value".to_string(),
        });

        struct MiddleReadingProcessor;
        impl Plugin for MiddleReadingProcessor {
            fn name(&self) -> &str {
                "middle"
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
        impl PostProcessor for MiddleReadingProcessor {
            async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                if let Some(val) = result.metadata.additional.get("early_key") {
                    result.metadata.additional.insert("middle_saw".to_string(), val.clone());
                }
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }
        }

        reg.register(early, 50).unwrap();
        reg.register(Arc::new(MiddleReadingProcessor), 50).unwrap();
    }

    let result = ExtractionResult {
        content: "content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(
        processed
            .metadata
            .additional
            .get("middle_saw")
            .unwrap()
            .as_str()
            .unwrap(),
        "early_value"
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_content_modified_in_middle_visible_in_late() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        let middle = Arc::new(OrderTrackingProcessor {
            name: "middle-content".to_string(),
            stage: ProcessingStage::Middle,
        });

        struct LateReadingProcessor;
        impl Plugin for LateReadingProcessor {
            fn name(&self) -> &str {
                "late"
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
        impl PostProcessor for LateReadingProcessor {
            async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                result.content.push_str("[late-saw-middle]");
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Late
            }
        }

        reg.register(middle, 50).unwrap();
        reg.register(Arc::new(LateReadingProcessor), 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.contains("[middle-content]"));
    assert!(processed.content.contains("[late-saw-middle]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_multiple_processors_modifying_same_metadata() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for i in 1..=3 {
            struct MetadataOverwritingProcessor {
                name: String,
                value: String,
            }
            impl Plugin for MetadataOverwritingProcessor {
                fn name(&self) -> &str {
                    &self.name
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
            impl PostProcessor for MetadataOverwritingProcessor {
                async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                    result
                        .metadata
                        .additional
                        .insert("shared_key".to_string(), serde_json::json!(self.value));
                    Ok(())
                }
                fn processing_stage(&self) -> ProcessingStage {
                    ProcessingStage::Early
                }
            }

            let processor = Arc::new(MetadataOverwritingProcessor {
                name: format!("proc{}", i),
                value: format!("value{}", i),
            });
            reg.register(processor, 100 - i * 10).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(
        processed
            .metadata
            .additional
            .get("shared_key")
            .unwrap()
            .as_str()
            .unwrap(),
        "value3"
    );
}

#[tokio::test]
#[serial]
async fn test_pipeline_processors_reading_previous_output() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        struct CountingProcessor {
            name: String,
            stage: ProcessingStage,
        }
        impl Plugin for CountingProcessor {
            fn name(&self) -> &str {
                &self.name
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
        impl PostProcessor for CountingProcessor {
            async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                let current_count = result
                    .metadata
                    .additional
                    .get("count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                result
                    .metadata
                    .additional
                    .insert("count".to_string(), serde_json::json!(current_count + 1));
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                self.stage
            }
        }

        for (name, stage) in [
            ("early1", ProcessingStage::Early),
            ("early2", ProcessingStage::Early),
            ("middle1", ProcessingStage::Middle),
            ("late1", ProcessingStage::Late),
        ] {
            let processor = Arc::new(CountingProcessor {
                name: name.to_string(),
                stage,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "content".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.metadata.additional.get("count").unwrap().as_i64().unwrap(), 4);
}

#[tokio::test]
#[serial]
async fn test_pipeline_large_content_modification() {
    clear_processor_registry();

    struct LargeContentProcessor;
    impl Plugin for LargeContentProcessor {
        fn name(&self) -> &str {
            "large"
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
    impl PostProcessor for LargeContentProcessor {
        async fn process(&self, result: &mut ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            result.content.push_str(&"x".repeat(10000));
            Ok(())
        }
        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }
    }

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");
        reg.register(Arc::new(LargeContentProcessor), 50).unwrap();
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.len() > 10000);
}

#[tokio::test]
#[serial]
async fn test_pipeline_enabled_processors_whitelist() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for name in ["proc1", "proc2", "proc3"] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: name.to_string(),
                stage: ProcessingStage::Early,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec!["proc1".to_string(), "proc3".to_string()]),
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.contains("[proc1]"));
    assert!(!processed.content.contains("[proc2]"));
    assert!(processed.content.contains("[proc3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_disabled_processors_blacklist() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for name in ["proc1", "proc2", "proc3"] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: name.to_string(),
                stage: ProcessingStage::Early,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: None,
            disabled_processors: Some(vec!["proc2".to_string()]),
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.contains("[proc1]"));
    assert!(!processed.content.contains("[proc2]"));
    assert!(processed.content.contains("[proc3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_no_filtering_runs_all() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for name in ["proc1", "proc2", "proc3"] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: name.to_string(),
                stage: ProcessingStage::Early,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig::default();

    let processed = run_pipeline(result, &config).await.unwrap();
    assert!(processed.content.contains("[proc1]"));
    assert!(processed.content.contains("[proc2]"));
    assert!(processed.content.contains("[proc3]"));
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_whitelist_runs_none() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry
            .write()
            .expect("Failed to acquire write lock on registry in test");

        for name in ["proc1", "proc2"] {
            let processor = Arc::new(OrderTrackingProcessor {
                name: name.to_string(),
                stage: ProcessingStage::Early,
            });
            reg.register(processor, 50).unwrap();
        }
    }

    let result = ExtractionResult {
        content: "start".to_string(),
        mime_type: "text/plain".to_string(),
        metadata: Metadata::default(),
        tables: vec![],
        detected_languages: None,
        chunks: None,
    };
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: Some(vec![]),
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(result, &config).await.unwrap();
    assert_eq!(processed.content, "start");
}
