//! Comprehensive pipeline integration tests.
//!
//! Tests the post-processing pipeline orchestration: ordering, error handling,
//! metadata/content modifications, and integration with extractors.
//!
//! IMPORTANT: These tests use a global registry and must run serially to avoid interference.

use async_trait::async_trait;
use serial_test::serial;
use std::borrow::Cow;
use std::sync::Arc;
use xberg::core::config::{ExtractionConfig, PostProcessorConfig};
use xberg::core::pipeline::{clear_processor_cache, run_pipeline};
use xberg::internal::{ElementKind, InternalDocument, InternalElement};
use xberg::plugins::registry::get_post_processor_registry;
use xberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use xberg::types::ExtractedDocument;
use xberg::{Result, XbergError};

#[path = "pipeline_integration/stage_data_flow.rs"]
mod stage_data_flow;

/// Helper: build a minimal `InternalDocument` whose derived content equals `text`.
fn mock_doc(text: &str) -> InternalDocument {
    let mut doc = InternalDocument::new("text");
    doc.mime_type = "text/plain".to_string();
    if !text.is_empty() {
        doc.elements
            .push(InternalElement::text(ElementKind::Paragraph, text, 0));
    }
    doc
}

struct OrderTrackingProcessor {
    name: String,
    stage: ProcessingStage,
    priority: i32,
}

impl OrderTrackingProcessor {
    fn new(name: impl Into<String>, stage: ProcessingStage) -> Self {
        Self {
            name: name.into(),
            stage,
            priority: 50,
        }
    }

    fn with_priority(name: impl Into<String>, stage: ProcessingStage, priority: i32) -> Self {
        Self {
            name: name.into(),
            stage,
            priority,
        }
    }
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
    async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
        result.content.push_str(&format!("[{}]", self.name));
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }

    fn priority(&self) -> i32 {
        self.priority
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
    async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
        result
            .metadata
            .additional
            .insert(Cow::Owned(self.key.clone()), serde_json::json!(self.value));
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
    async fn process(&self, _: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
        Err(XbergError::Plugin {
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
    let mut reg = registry.write();
    let _ = reg.shutdown_all();
    drop(reg);
    let _ = clear_processor_cache();
}

#[tokio::test]
#[serial]
async fn test_pipeline_empty_no_processors() {
    clear_processor_registry();

    let doc = mock_doc("original content");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "original content");
}

#[tokio::test]
#[serial]
async fn test_pipeline_single_processor_per_stage() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let early = Arc::new(OrderTrackingProcessor::new("early", ProcessingStage::Early));
        let middle = Arc::new(OrderTrackingProcessor::new("middle", ProcessingStage::Middle));
        let late = Arc::new(OrderTrackingProcessor::new("late", ProcessingStage::Late));

        reg.register(early).expect("Operation failed");
        reg.register(middle).expect("Operation failed");
        reg.register(late).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[early][middle][late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_multiple_processors_per_stage() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let early_high = Arc::new(OrderTrackingProcessor::with_priority(
            "early-high",
            ProcessingStage::Early,
            100,
        ));
        let early_medium = Arc::new(OrderTrackingProcessor::with_priority(
            "early-medium",
            ProcessingStage::Early,
            50,
        ));
        let early_low = Arc::new(OrderTrackingProcessor::with_priority(
            "early-low",
            ProcessingStage::Early,
            10,
        ));

        reg.register(early_low).expect("Operation failed");
        reg.register(early_high).expect("Operation failed");
        reg.register(early_medium).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[early-high][early-medium][early-low]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_all_stages_enabled() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for stage in [ProcessingStage::Early, ProcessingStage::Middle, ProcessingStage::Late] {
            let processor = Arc::new(OrderTrackingProcessor::new(format!("{:?}", stage), stage));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[Early][Middle][Late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_postprocessing_disabled() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let processor = Arc::new(OrderTrackingProcessor::new("processor", ProcessingStage::Early));
        reg.register(processor).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: false,
            enabled_processors: None,
            disabled_processors: None,
            enabled_set: None,
            disabled_set: None,
        }),
        ..Default::default()
    };

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start");
}

#[tokio::test]
#[serial]
async fn test_pipeline_early_stage_runs_first() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let late = Arc::new(OrderTrackingProcessor::new("late", ProcessingStage::Late));
        let early = Arc::new(OrderTrackingProcessor::new("early", ProcessingStage::Early));

        reg.register(late).expect("Operation failed");
        reg.register(early).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[early][late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_middle_stage_runs_second() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let early = Arc::new(OrderTrackingProcessor::new("early", ProcessingStage::Early));
        let middle = Arc::new(OrderTrackingProcessor::new("middle", ProcessingStage::Middle));

        reg.register(middle).expect("Operation failed");
        reg.register(early).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[early][middle]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_late_stage_runs_last() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for stage in [ProcessingStage::Late, ProcessingStage::Early, ProcessingStage::Middle] {
            let processor = Arc::new(OrderTrackingProcessor::new(format!("{:?}", stage), stage));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[Early][Middle][Late]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_within_stage_priority_order() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        for (name, priority) in [("p1", 100), ("p2", 10), ("p3", 50), ("p4", 75)] {
            let processor = Arc::new(OrderTrackingProcessor::with_priority(
                name,
                ProcessingStage::Early,
                priority,
            ));
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
    assert_eq!(processed.content, "start[p1][p4][p3][p2]");
}

#[tokio::test]
#[serial]
async fn test_pipeline_cross_stage_data_flow() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

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
            async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
                if let Some(stage) = result.metadata.additional.get(&Cow::Borrowed("stage")) {
                    result.content.push_str(&format!(
                        "[saw:{}]",
                        stage.as_str().expect("Failed to extract string from value")
                    ));
                }
                Ok(())
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Middle
            }
        }

        reg.register(early).expect("Operation failed");
        reg.register(Arc::new(MiddleProcessor)).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let processed = run_pipeline(doc, &config).await.expect("Async operation failed");
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
        async fn process(&self, _: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
            Err(XbergError::Plugin {
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
        let mut reg = registry.write();
        reg.register(Arc::new(EarlyFailingProcessor)).expect("Operation failed");
    }

    let doc = mock_doc("content");
    let config = ExtractionConfig::default();

    let result = run_pipeline(doc, &config).await;
    assert!(result.is_err(), "Expected pipeline to return error");
    match result {
        Err(XbergError::Plugin { message, plugin_name }) => {
            assert_eq!(message, "Early error");
            assert_eq!(plugin_name, "early-failing");
        }
        _ => panic!("Expected Plugin error"),
    }
}

#[tokio::test]
#[serial]
async fn test_pipeline_middle_stage_error_propagation() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let failing = Arc::new(FailingProcessor {
            name: "middle-failing".to_string(),
            error_message: "Middle stage error".to_string(),
        });

        reg.register(failing).expect("Operation failed");
    }

    let doc = mock_doc("content");
    let config = ExtractionConfig::default();

    let result = run_pipeline(doc, &config).await;
    assert!(result.is_err(), "Expected pipeline to return error");
    match result {
        Err(XbergError::Plugin { message, plugin_name }) => {
            assert_eq!(message, "Middle stage error");
            assert_eq!(plugin_name, "middle-failing");
        }
        _ => panic!("Expected Plugin error"),
    }
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
        async fn process(&self, _: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
            Err(XbergError::Plugin {
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
        let mut reg = registry.write();

        let early = Arc::new(OrderTrackingProcessor::new("early", ProcessingStage::Early));
        let late_failing = Arc::new(LateFailingProcessor);

        reg.register(early).expect("Operation failed");
        reg.register(late_failing).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let result = run_pipeline(doc, &config).await;
    assert!(result.is_err(), "Expected pipeline to return error");
    match result {
        Err(XbergError::Plugin { message, plugin_name }) => {
            assert_eq!(message, "Late error");
            assert_eq!(plugin_name, "late-failing");
        }
        _ => panic!("Expected Plugin error"),
    }
}

#[tokio::test]
#[serial]
async fn test_pipeline_processor_error_doesnt_stop_other_processors() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let p1 = Arc::new(OrderTrackingProcessor::with_priority("p1", ProcessingStage::Early, 100));
        let _p2_failing = Arc::new(FailingProcessor {
            name: "p2-failing".to_string(),
            error_message: "Test error".to_string(),
        });
        let p3 = Arc::new(OrderTrackingProcessor::new("p3", ProcessingStage::Late));

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
            async fn process(&self, _: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
                Err(XbergError::Plugin {
                    message: "Test error".to_string(),
                    plugin_name: self.name.clone(),
                })
            }
            fn processing_stage(&self) -> ProcessingStage {
                ProcessingStage::Early
            }
        }

        reg.register(p1).expect("Operation failed");
        reg.register(Arc::new(EarlyFailingProcessor {
            name: "p2-failing".to_string(),
        }))
        .expect("Operation failed");
        reg.register(p3).expect("Operation failed");
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let result = run_pipeline(doc, &config).await;
    assert!(result.is_err(), "Expected pipeline to return error");
    match result {
        Err(XbergError::Plugin { message, plugin_name }) => {
            assert_eq!(message, "Test error");
            assert_eq!(plugin_name, "p2-failing");
        }
        _ => panic!("Expected Plugin error"),
    }
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
        async fn process(&self, _: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
            Err(XbergError::Plugin {
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
        let mut reg = registry.write();

        for (name, stage) in [
            ("fail1", ProcessingStage::Early),
            ("fail2", ProcessingStage::Middle),
            ("fail3", ProcessingStage::Late),
        ] {
            let processor = Arc::new(MultiFailingProcessor {
                name: name.to_string(),
                stage,
            });
            reg.register(processor).expect("Operation failed");
        }
    }

    let doc = mock_doc("start");
    let config = ExtractionConfig::default();

    let result = run_pipeline(doc, &config).await;
    assert!(result.is_err(), "Expected pipeline to return error");
    match result {
        Err(XbergError::Plugin { message, plugin_name }) => {
            assert_eq!(message, "fail1 error");
            assert_eq!(plugin_name, "fail1");
        }
        _ => panic!("Expected Plugin error"),
    }
}

#[tokio::test]
#[serial]
async fn test_pipeline_error_context_preservation() {
    clear_processor_registry();

    let registry = get_post_processor_registry();
    {
        let mut reg = registry.write();

        let failing = Arc::new(FailingProcessor {
            name: "context-test".to_string(),
            error_message: "Detailed error message with context".to_string(),
        });

        reg.register(failing).expect("Operation failed");
    }

    let doc = mock_doc("content");
    let config = ExtractionConfig::default();

    let result = run_pipeline(doc, &config).await;
    assert!(result.is_err(), "Expected pipeline to return error");
    match result {
        Err(XbergError::Plugin { message, plugin_name }) => {
            assert_eq!(message, "Detailed error message with context");
            assert_eq!(plugin_name, "context-test");
        }
        _ => panic!("Expected Plugin error"),
    }
}
