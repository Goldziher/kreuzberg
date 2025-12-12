//! Plugin registry integration tests.
//!
//! Tests the core registry APIs for all plugin types:
//! - Validator registration/unregistration
//! - Extractor registration/unregistration
//! - Registry clearing and listing
//! - Error handling and edge cases

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::registry::{DocumentExtractorRegistry, ValidatorRegistry};
use kreuzberg::plugins::{DocumentExtractor, Plugin, Validator};
use kreuzberg::types::{ExtractionResult, Metadata};
use kreuzberg::{KreuzbergError, Result};
use std::path::Path;
use std::sync::Arc;

struct MockValidator {
    name: String,
    should_fail: bool,
}

impl Plugin for MockValidator {
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
impl Validator for MockValidator {
    async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        if self.should_fail {
            Err(KreuzbergError::validation("Mock validation failed"))
        } else {
            Ok(())
        }
    }

    fn priority(&self) -> i32 {
        50
    }
}

struct FailingInitValidator {
    name: String,
}

impl Plugin for FailingInitValidator {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Err(KreuzbergError::Plugin {
            message: "Initialization failed".to_string(),
            plugin_name: self.name.clone(),
        })
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl Validator for FailingInitValidator {
    async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        Ok(())
    }
}

struct MockExtractor {
    name: String,
    mime_types: Vec<&'static str>,
    priority: i32,
}

impl Plugin for MockExtractor {
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
impl DocumentExtractor for MockExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        Ok(ExtractionResult {
            content: format!("Extracted by {}: {}", self.name, String::from_utf8_lossy(content)),
            mime_type: mime_type.to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            pages: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let content = std::fs::read(path)?;
        self.extract_bytes(&content, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &self.mime_types
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Test validator registration and listing.
#[test]
fn test_validator_registration_succeeds() {
    let mut registry = ValidatorRegistry::new();

    let validator = Arc::new(MockValidator {
        name: "test-validator".to_string(),
        should_fail: false,
    });

    let result = registry.register(validator);
    assert!(result.is_ok(), "Validator registration should succeed");

    let list = registry.list();
    assert_eq!(list.len(), 1, "Should have one validator");
    assert!(
        list.contains(&"test-validator".to_string()),
        "Should contain registered validator"
    );
}

/// Test registering multiple validators.
#[test]
fn test_register_multiple_validators_succeeds() {
    let mut registry = ValidatorRegistry::new();

    let v1 = Arc::new(MockValidator {
        name: "validator-1".to_string(),
        should_fail: false,
    });
    let v2 = Arc::new(MockValidator {
        name: "validator-2".to_string(),
        should_fail: false,
    });
    let v3 = Arc::new(MockValidator {
        name: "validator-3".to_string(),
        should_fail: true,
    });

    registry.register(v1).unwrap();
    registry.register(v2).unwrap();
    registry.register(v3).unwrap();

    let list = registry.list();
    assert_eq!(list.len(), 3, "Should have three validators");
    assert!(list.contains(&"validator-1".to_string()));
    assert!(list.contains(&"validator-2".to_string()));
    assert!(list.contains(&"validator-3".to_string()));
}

/// Test validator unregistration.
#[test]
fn test_validator_unregistration_succeeds() {
    let mut registry = ValidatorRegistry::new();

    let validator = Arc::new(MockValidator {
        name: "temp-validator".to_string(),
        should_fail: false,
    });

    registry.register(validator).unwrap();
    assert_eq!(registry.list().len(), 1);

    let result = registry.remove("temp-validator");
    assert!(result.is_ok(), "Unregistration should succeed");
    assert_eq!(registry.list().len(), 0, "Registry should be empty after removal");
}

/// Test unregistering non-existent validator.
#[test]
fn test_unregister_nonexistent_validator_succeeds() {
    let mut registry = ValidatorRegistry::new();

    let result = registry.remove("nonexistent-validator");
    assert!(result.is_ok(), "Removing non-existent validator should succeed (no-op)");
}

/// Test validator registration with empty name fails.
#[test]
fn test_validator_registration_with_empty_name_fails() {
    let mut registry = ValidatorRegistry::new();

    let validator = Arc::new(MockValidator {
        name: "".to_string(),
        should_fail: false,
    });

    let result = registry.register(validator);
    assert!(result.is_err(), "Registration with empty name should fail");

    match result {
        Err(KreuzbergError::Validation { message, .. }) => {
            assert!(message.contains("empty"), "Error should mention empty name");
        }
        _ => panic!("Expected Validation error"),
    }
}

/// Test validator registration with whitespace in name fails.
#[test]
fn test_validator_registration_with_whitespace_fails() {
    let mut registry = ValidatorRegistry::new();

    let validator = Arc::new(MockValidator {
        name: "validator with spaces".to_string(),
        should_fail: false,
    });

    let result = registry.register(validator);
    assert!(result.is_err(), "Registration with whitespace should fail");

    match result {
        Err(KreuzbergError::Validation { message, .. }) => {
            assert!(message.contains("whitespace"), "Error should mention whitespace");
        }
        _ => panic!("Expected Validation error"),
    }
}

/// Test validator registration with failed initialization.
#[test]
fn test_validator_registration_with_failed_init_fails() {
    let mut registry = ValidatorRegistry::new();

    let validator = Arc::new(FailingInitValidator {
        name: "failing-validator".to_string(),
    });

    let result = registry.register(validator);
    assert!(result.is_err(), "Registration with failed init should fail");

    match result {
        Err(KreuzbergError::Plugin { .. }) => {}
        _ => panic!("Expected Plugin error"),
    }

    assert_eq!(registry.list().len(), 0, "Failed validator should not be registered");
}

/// Test clearing all validators.
#[test]
fn test_clear_validators_succeeds() {
    let mut registry = ValidatorRegistry::new();

    let v1 = Arc::new(MockValidator {
        name: "validator-1".to_string(),
        should_fail: false,
    });
    let v2 = Arc::new(MockValidator {
        name: "validator-2".to_string(),
        should_fail: false,
    });

    registry.register(v1).unwrap();
    registry.register(v2).unwrap();
    assert_eq!(registry.list().len(), 2);

    let result = registry.shutdown_all();
    assert!(result.is_ok(), "Clear should succeed");
    assert_eq!(registry.list().len(), 0, "Registry should be empty after clear");
}

/// Test getting all validators in priority order.
#[test]
fn test_get_all_validators_respects_priority() {
    let mut registry = ValidatorRegistry::new();

    struct PriorityValidator {
        name: String,
        priority: i32,
    }

    impl Plugin for PriorityValidator {
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
    impl Validator for PriorityValidator {
        async fn validate(&self, _result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
            Ok(())
        }
        fn priority(&self) -> i32 {
            self.priority
        }
    }

    let low = Arc::new(PriorityValidator {
        name: "low-priority".to_string(),
        priority: 10,
    });
    let medium = Arc::new(PriorityValidator {
        name: "medium-priority".to_string(),
        priority: 50,
    });
    let high = Arc::new(PriorityValidator {
        name: "high-priority".to_string(),
        priority: 100,
    });

    registry.register(medium).unwrap();
    registry.register(low).unwrap();
    registry.register(high).unwrap();

    let all = registry.get_all();
    assert_eq!(all.len(), 3, "Should have three validators");

    assert_eq!(all[0].name(), "high-priority");
    assert_eq!(all[1].name(), "medium-priority");
    assert_eq!(all[2].name(), "low-priority");
}

/// Test extractor registration and retrieval.
#[test]
fn test_extractor_registration_succeeds() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "test-extractor".to_string(),
        mime_types: vec!["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(result.is_ok(), "Extractor registration should succeed");

    let list = registry.list();
    assert_eq!(list.len(), 1, "Should have one extractor");
    assert!(list.contains(&"test-extractor".to_string()));
}

/// Test extractor retrieval by MIME type.
#[test]
fn test_get_extractor_by_mime_type_succeeds() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "pdf-extractor".to_string(),
        mime_types: vec!["application/pdf"],
        priority: 50,
    });

    registry.register(extractor).unwrap();

    let result = registry.get("application/pdf");
    assert!(result.is_ok(), "Should find extractor for PDF");
    assert_eq!(result.unwrap().name(), "pdf-extractor");
}

/// Test extractor not found for unsupported MIME type.
#[test]
fn test_get_extractor_for_unsupported_mime_fails() {
    let registry = DocumentExtractorRegistry::new();

    let result = registry.get("application/nonexistent");
    assert!(result.is_err(), "Should not find extractor for unsupported MIME type");

    match result {
        Err(KreuzbergError::UnsupportedFormat(mime)) => {
            assert_eq!(mime, "application/nonexistent");
        }
        _ => panic!("Expected UnsupportedFormat error"),
    }
}

/// Test extractor priority selection.
#[test]
fn test_extractor_priority_selection() {
    let mut registry = DocumentExtractorRegistry::new();

    let low_priority = Arc::new(MockExtractor {
        name: "low-priority-extractor".to_string(),
        mime_types: vec!["text/plain"],
        priority: 10,
    });

    let high_priority = Arc::new(MockExtractor {
        name: "high-priority-extractor".to_string(),
        mime_types: vec!["text/plain"],
        priority: 100,
    });

    registry.register(low_priority).unwrap();
    registry.register(high_priority).unwrap();

    let result = registry.get("text/plain").unwrap();
    assert_eq!(
        result.name(),
        "high-priority-extractor",
        "Should select highest priority extractor"
    );
}

/// Test extractor wildcard MIME type matching.
#[test]
fn test_extractor_wildcard_mime_matching() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "text-extractor".to_string(),
        mime_types: vec!["text/*"],
        priority: 50,
    });

    registry.register(extractor).unwrap();

    let result = registry.get("text/plain");
    assert!(result.is_ok(), "Should match text/plain with text/*");
    assert_eq!(result.unwrap().name(), "text-extractor");

    let result = registry.get("text/html");
    assert!(result.is_ok(), "Should match text/html with text/*");
    assert_eq!(result.unwrap().name(), "text-extractor");

    let result = registry.get("application/pdf");
    assert!(result.is_err(), "Should not match application/pdf with text/*");
}

/// Test extractor unregistration.
#[test]
fn test_extractor_unregistration_succeeds() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "temp-extractor".to_string(),
        mime_types: vec!["text/plain"],
        priority: 50,
    });

    registry.register(extractor).unwrap();
    assert_eq!(registry.list().len(), 1);

    let result = registry.remove("temp-extractor");
    assert!(result.is_ok(), "Unregistration should succeed");
    assert_eq!(registry.list().len(), 0, "Registry should be empty after removal");

    let lookup_result = registry.get("text/plain");
    assert!(lookup_result.is_err(), "Should not find extractor after removal");
}

/// Test extractor registration with multiple MIME types.
#[test]
fn test_extractor_multiple_mime_types() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "multi-format-extractor".to_string(),
        mime_types: vec!["application/pdf", "application/vnd.ms-excel", "text/csv"],
        priority: 50,
    });

    registry.register(extractor).unwrap();

    assert!(registry.get("application/pdf").is_ok());
    assert!(registry.get("application/vnd.ms-excel").is_ok());
    assert!(registry.get("text/csv").is_ok());

    assert_eq!(
        registry.get("application/pdf").unwrap().name(),
        "multi-format-extractor"
    );
    assert_eq!(registry.get("text/csv").unwrap().name(), "multi-format-extractor");
}

/// Test clearing all extractors.
#[test]
fn test_clear_extractors_succeeds() {
    let mut registry = DocumentExtractorRegistry::new();

    let e1 = Arc::new(MockExtractor {
        name: "extractor-1".to_string(),
        mime_types: vec!["text/plain"],
        priority: 50,
    });
    let e2 = Arc::new(MockExtractor {
        name: "extractor-2".to_string(),
        mime_types: vec!["application/pdf"],
        priority: 50,
    });

    registry.register(e1).unwrap();
    registry.register(e2).unwrap();
    assert_eq!(registry.list().len(), 2);

    let result = registry.shutdown_all();
    assert!(result.is_ok(), "Clear should succeed");
    assert_eq!(registry.list().len(), 0, "Registry should be empty after clear");
}

/// Test extractor registration with empty name fails.
#[test]
fn test_extractor_registration_with_empty_name_fails() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "".to_string(),
        mime_types: vec!["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(result.is_err(), "Registration with empty name should fail");

    match result {
        Err(KreuzbergError::Validation { message, .. }) => {
            assert!(message.contains("empty"), "Error should mention empty name");
        }
        _ => panic!("Expected Validation error"),
    }
}

/// Test extractor registration with whitespace fails.
#[test]
fn test_extractor_registration_with_whitespace_fails() {
    let mut registry = DocumentExtractorRegistry::new();

    let extractor = Arc::new(MockExtractor {
        name: "extractor with spaces".to_string(),
        mime_types: vec!["text/plain"],
        priority: 50,
    });

    let result = registry.register(extractor);
    assert!(result.is_err(), "Registration with whitespace should fail");

    match result {
        Err(KreuzbergError::Validation { message, .. }) => {
            assert!(message.contains("whitespace"), "Error should mention whitespace");
        }
        _ => panic!("Expected Validation error"),
    }
}
