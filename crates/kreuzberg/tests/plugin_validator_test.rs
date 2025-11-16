//! Comprehensive validator plugin system tests.
//!
//! Tests custom validator registration, execution, validation logic,
//! error handling, and cleanup with real file extraction.

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::registry::get_validator_registry;
use kreuzberg::plugins::{Plugin, Validator};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result, extract_file_sync};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct MinLengthValidator {
    name: String,
    min_length: usize,
    call_count: AtomicUsize,
}

impl Plugin for MinLengthValidator {
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
impl Validator for MinLengthValidator {
    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if result.content.len() < self.min_length {
            Err(KreuzbergError::validation(format!(
                "Content too short: {} < {} characters",
                result.content.len(),
                self.min_length
            )))
        } else {
            Ok(())
        }
    }

    fn priority(&self) -> i32 {
        50
    }
}

struct PassingValidator {
    name: String,
    initialized: AtomicBool,
}

impl Plugin for PassingValidator {
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
impl Validator for PassingValidator {
    async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        Ok(())
    }
}

struct MimeTypeValidator {
    name: String,
    allowed_mime: String,
}

impl Plugin for MimeTypeValidator {
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
impl Validator for MimeTypeValidator {
    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        if result.mime_type != self.allowed_mime {
            Err(KreuzbergError::validation(format!(
                "MIME type '{}' not allowed, expected '{}'",
                result.mime_type, self.allowed_mime
            )))
        } else {
            Ok(())
        }
    }

    fn should_validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> bool {
        !result.mime_type.is_empty()
    }
}

struct MetadataValidator {
    name: String,
    required_key: String,
}

impl Plugin for MetadataValidator {
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
impl Validator for MetadataValidator {
    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        if !result.metadata.additional.contains_key(&self.required_key) {
            Err(KreuzbergError::validation(format!(
                "Required metadata key '{}' missing",
                self.required_key
            )))
        } else {
            Ok(())
        }
    }

    fn priority(&self) -> i32 {
        100
    }
}

struct FailingValidator {
    name: String,
}

impl Plugin for FailingValidator {
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
impl Validator for FailingValidator {
    async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        Err(KreuzbergError::validation(
            "Validation intentionally failed".to_string(),
        ))
    }
}

struct TrackingValidator {
    name: String,
    called: AtomicBool,
}

impl Plugin for TrackingValidator {
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
impl Validator for TrackingValidator {
    async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        self.called.store(true, Ordering::Release);
        Ok(())
    }
}

#[test]
fn test_register_custom_validator() {
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(PassingValidator {
        name: "test-validator".to_string(),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        let result = reg.register(Arc::clone(&validator) as Arc<dyn Validator>);
        assert!(result.is_ok(), "Failed to register validator: {:?}", result.err());
    }

    assert!(
        validator.initialized.load(Ordering::Acquire),
        "Validator was not initialized"
    );

    let list = {
        let reg = registry.read().unwrap();
        reg.list()
    };

    assert!(list.contains(&"test-validator".to_string()));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_called_during_extraction() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(MinLengthValidator {
        name: "call-test-validator".to_string(),
        min_length: 1,
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&validator) as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    assert_eq!(
        validator.call_count.load(Ordering::SeqCst),
        1,
        "Validator was not called exactly once"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_can_reject_invalid_input() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(MinLengthValidator {
        name: "reject-validator".to_string(),
        min_length: 1_000_000,
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_err(), "Expected validation to fail");

    match result.err().unwrap() {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("Content too short"));
        }
        other => panic!("Expected Validation error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_can_pass_valid_input() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(MinLengthValidator {
        name: "pass-validator".to_string(),
        min_length: 10,
        call_count: AtomicUsize::new(0),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok(), "Validation should have passed: {:?}", result.err());

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_receives_correct_parameters() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(MimeTypeValidator {
        name: "mime-validator".to_string(),
        allowed_mime: "text/plain".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok(), "Validation failed: {:?}", result.err());

    let extraction_result = result.unwrap();
    assert_eq!(extraction_result.mime_type, "text/plain");

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_rejects_wrong_mime_type() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(MimeTypeValidator {
        name: "strict-mime-validator".to_string(),
        allowed_mime: "application/pdf".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_err(), "Expected MIME type validation to fail");

    match result.err().unwrap() {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("MIME type"));
            assert!(message.contains("not allowed"));
        }
        other => panic!("Expected Validation error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_unregister_validator() {
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(FailingValidator {
        name: "unregister-test".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator as Arc<dyn Validator>).unwrap();
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

    assert!(
        result.is_ok(),
        "Extraction should succeed after unregistering validator"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_clear_all_validators() {
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator1 = Arc::new(FailingValidator {
        name: "clear-test-1".to_string(),
    });

    let validator2 = Arc::new(FailingValidator {
        name: "clear-test-2".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator1 as Arc<dyn Validator>).unwrap();
        reg.register(validator2 as Arc<dyn Validator>).unwrap();
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

    let test_file = "../../test_documents/text/fake_text.txt";
    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok(), "Extraction should succeed after clearing validators");
}

#[test]
fn test_validator_invalid_name() {
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(PassingValidator {
        name: "invalid name".to_string(),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        let result = reg.register(validator);

        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), KreuzbergError::Validation { .. }));
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_initialization_lifecycle() {
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(PassingValidator {
        name: "lifecycle-test".to_string(),
        initialized: AtomicBool::new(false),
    });

    assert!(
        !validator.initialized.load(Ordering::Acquire),
        "Validator should not be initialized yet"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&validator) as Arc<dyn Validator>).unwrap();
    }

    assert!(
        validator.initialized.load(Ordering::Acquire),
        "Validator should be initialized after registration"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    assert!(
        !validator.initialized.load(Ordering::Acquire),
        "Validator should be shutdown"
    );
}

#[test]
fn test_multiple_validators_execution() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator1 = Arc::new(MinLengthValidator {
        name: "multi-validator-1".to_string(),
        min_length: 10,
        call_count: AtomicUsize::new(0),
    });

    let validator2 = Arc::new(MimeTypeValidator {
        name: "multi-validator-2".to_string(),
        allowed_mime: "text/plain".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&validator1) as Arc<dyn Validator>).unwrap();
        reg.register(validator2 as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_ok(), "Both validators should pass");
    assert_eq!(validator1.call_count.load(Ordering::SeqCst), 1);

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_priority_execution_order() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let high_priority = Arc::new(MetadataValidator {
        name: "high-priority-validator".to_string(),
        required_key: "nonexistent_key".to_string(),
    });

    let low_priority = Arc::new(PassingValidator {
        name: "low-priority-validator".to_string(),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(high_priority as Arc<dyn Validator>).unwrap();
        reg.register(low_priority as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_err(), "Expected high-priority validator to fail");

    match result.err().unwrap() {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("Required metadata key"));
        }
        other => panic!("Expected Validation error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_always_fails() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let validator = Arc::new(FailingValidator {
        name: "always-fails".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(validator as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_err(), "Validator should always fail");

    match result.err().unwrap() {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("intentionally failed"));
        }
        other => panic!("Expected Validation error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_validator_registration_order_preserved_for_same_priority() {
    let test_file = "../../test_documents/text/fake_text.txt";
    let registry = get_validator_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let tracker = Arc::new(TrackingValidator {
        name: "order-second".to_string(),
        called: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::new(FailingValidator {
            name: "order-first".to_string(),
        }) as Arc<dyn Validator>)
            .unwrap();
        reg.register(tracker.clone() as Arc<dyn Validator>).unwrap();
    }

    let config = ExtractionConfig::default();
    let result = extract_file_sync(test_file, None, &config);

    assert!(result.is_err(), "Expected first validator to fail");
    assert!(
        !tracker.called.load(Ordering::Acquire),
        "Second validator should not run once the first validator fails"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}
