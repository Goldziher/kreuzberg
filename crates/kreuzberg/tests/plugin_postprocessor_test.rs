//! Comprehensive post-processor plugin system tests.
//!
//! Tests custom post-processor registration, execution, modifications,
//! error handling, and cleanup with real file extraction.

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::registry::get_post_processor_registry;
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result, extract_file_sync};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// Simple post-processor that appends text to content
struct AppendTextProcessor {
    name: String,
    text_to_append: String,
    call_count: AtomicUsize,
}

impl Plugin for AppendTextProcessor {
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
impl PostProcessor for AppendTextProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        result.content.push_str(&self.text_to_append);
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Late
    }
}

// Post-processor that adds metadata
struct MetadataAddingProcessor {
    name: String,
    initialized: AtomicBool,
}

impl Plugin for MetadataAddingProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        self.initialized.store(false, Ordering::Release);
        Ok(())
    }
}

#[async_trait]
impl PostProcessor for MetadataAddingProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        result
            .metadata
            .additional
            .insert("processed_by".to_string(), serde_json::json!(self.name()));
        result.metadata.additional.insert(
            "word_count".to_string(),
            serde_json::json!(result.content.split_whitespace().count()),
        );
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }
}

// Post-processor that modifies content
struct UppercaseProcessor {
    name: String,
}

impl Plugin for UppercaseProcessor {
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
impl PostProcessor for UppercaseProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        result.content = result.content.to_uppercase();
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

// Post-processor that fails
struct FailingProcessor {
    name: String,
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
    async fn process(&self, _result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        Err(KreuzbergError::Plugin {
            message: "Processor intentionally failed".to_string(),
            plugin_name: self.name.clone(),
        })
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }
}

#[test]
fn test_register_custom_postprocessor() {
    let registry = get_post_processor_registry();

    let processor = Arc::new(AppendTextProcessor {
        name: "test-appender".to_string(),
        text_to_append: " [PROCESSED]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    {
        let mut reg = registry.write().unwrap();
        let result = reg.register(Arc::clone(&processor) as Arc<dyn PostProcessor>, 100);
        assert!(result.is_ok(), "Failed to register processor: {:?}", result.err());
    }

    let list = {
        let reg = registry.read().unwrap();
        reg.list()
    };

    assert!(list.contains(&"test-appender".to_string()));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_postprocessor_called_during_extraction() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_post_processor_registry();

    let processor = Arc::new(AppendTextProcessor {
        name: "call-test-appender".to_string(),
        text_to_append: "\n[APPENDED BY PROCESSOR]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&processor) as Arc<dyn PostProcessor>, 100)
            .unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let extraction_result = result.unwrap();
    assert!(
        extraction_result.content.contains("[APPENDED BY PROCESSOR]"),
        "Processor did not modify content. Content: {}",
        extraction_result.content
    );

    assert_eq!(
        processor.call_count.load(Ordering::SeqCst),
        1,
        "Processor was not called exactly once"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_postprocessor_modifies_content() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_post_processor_registry();

    let processor = Arc::new(UppercaseProcessor {
        name: "uppercase-processor".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(processor as Arc<dyn PostProcessor>, 100).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.unwrap();
    let has_lowercase = extraction_result.content.chars().any(|c| c.is_lowercase());

    assert!(!has_lowercase, "Content was not fully uppercased");

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_postprocessor_adds_metadata() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_post_processor_registry();

    let processor = Arc::new(MetadataAddingProcessor {
        name: "metadata-adder".to_string(),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&processor) as Arc<dyn PostProcessor>, 100)
            .unwrap();
    }

    assert!(
        processor.initialized.load(Ordering::Acquire),
        "Processor was not initialized"
    );

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.unwrap();

    assert!(
        extraction_result.metadata.additional.contains_key("processed_by"),
        "Metadata 'processed_by' not added"
    );
    assert!(
        extraction_result.metadata.additional.contains_key("word_count"),
        "Metadata 'word_count' not added"
    );

    let processed_by = extraction_result.metadata.additional.get("processed_by").unwrap();
    assert_eq!(processed_by.as_str().unwrap(), "metadata-adder");

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    assert!(
        !processor.initialized.load(Ordering::Acquire),
        "Processor was not shutdown"
    );
}

#[test]
fn test_unregister_postprocessor() {
    let registry = get_post_processor_registry();

    let processor = Arc::new(AppendTextProcessor {
        name: "unregister-test".to_string(),
        text_to_append: " [SHOULD NOT APPEAR]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&processor) as Arc<dyn PostProcessor>, 100)
            .unwrap();
    }

    {
        let mut reg = registry.write().unwrap();
        reg.remove("unregister-test").unwrap();
    }

    let list = {
        let reg = registry.read().unwrap();
        reg.list()
    };

    assert!(!list.contains(&"unregister-test".to_string()));

    let test_file = "../../test_documents/text/fake_text.txt";
    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.unwrap();
    assert!(
        !extraction_result.content.contains("[SHOULD NOT APPEAR]"),
        "Unregistered processor still modified content"
    );

    assert_eq!(processor.call_count.load(Ordering::SeqCst), 0);

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_clear_all_postprocessors() {
    let registry = get_post_processor_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let processor1 = Arc::new(AppendTextProcessor {
        name: "clear-test-1".to_string(),
        text_to_append: " [ONE]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    let processor2 = Arc::new(AppendTextProcessor {
        name: "clear-test-2".to_string(),
        text_to_append: " [TWO]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(processor1 as Arc<dyn PostProcessor>, 100).unwrap();
        reg.register(processor2 as Arc<dyn PostProcessor>, 100).unwrap();
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let list = {
        let reg = registry.read().unwrap();
        reg.list()
    };

    assert!(list.is_empty(), "Registry was not cleared");
}

#[test]
fn test_postprocessor_error_handling() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_post_processor_registry();

    let failing_processor = Arc::new(FailingProcessor {
        name: "failing-processor".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(failing_processor as Arc<dyn PostProcessor>, 100).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    // NOTE: In the current implementation, processor errors are caught and handled
    // gracefully, so extraction still succeeds. This is by design to make the pipeline
    // resilient to processor failures.
    assert!(result.is_ok(), "Extraction should succeed even with failing processor");

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_postprocessor_invalid_name() {
    let registry = get_post_processor_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let processor = Arc::new(AppendTextProcessor {
        name: "invalid name".to_string(), // Contains space - invalid
        text_to_append: " [TEST]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        let result = reg.register(processor, 100);

        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), KreuzbergError::Validation { .. }));
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_multiple_postprocessors_execution_order() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_post_processor_registry();

    // Early stage - adds metadata
    let early_processor = Arc::new(MetadataAddingProcessor {
        name: "early-processor".to_string(),
        initialized: AtomicBool::new(false),
    });

    // Middle stage - uppercases content
    let middle_processor = Arc::new(UppercaseProcessor {
        name: "middle-processor".to_string(),
    });

    // Late stage - appends text
    let late_processor = Arc::new(AppendTextProcessor {
        name: "late-processor".to_string(),
        text_to_append: " [LATE]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(early_processor as Arc<dyn PostProcessor>, 100).unwrap();
        reg.register(middle_processor as Arc<dyn PostProcessor>, 100).unwrap();
        reg.register(late_processor as Arc<dyn PostProcessor>, 100).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.unwrap();

    // Verify all processors ran
    assert!(extraction_result.metadata.additional.contains_key("processed_by"));
    assert!(!extraction_result.content.chars().any(|c| c.is_lowercase()));
    assert!(extraction_result.content.contains("[LATE]"));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_postprocessor_preserves_mime_type() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_post_processor_registry();

    let processor = Arc::new(AppendTextProcessor {
        name: "mime-test".to_string(),
        text_to_append: " [TEST]".to_string(),
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&processor) as Arc<dyn PostProcessor>, 100)
            .unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.unwrap();
    assert_eq!(extraction_result.mime_type, "text/plain");

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}
