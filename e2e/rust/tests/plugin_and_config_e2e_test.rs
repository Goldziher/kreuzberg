//! End-to-end integration test for plugin registration and config loading.
//!
//! This test demonstrates:
//! 1. Programmatic validator registration
//! 2. Programmatic extractor registration
//! 3. Config loading from file
//! 4. Config discovery
//! 5. Using everything together in a real extraction workflow

use async_trait::async_trait;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::registry::{get_document_extractor_registry, get_validator_registry};
use kreuzberg::plugins::{DocumentExtractor, Plugin, Validator};
use kreuzberg::types::{ExtractionResult, Metadata};
use kreuzberg::{KreuzbergError, Result, extract_file_sync};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

// ===== Custom Validator =====

/// Custom validator that checks content length
struct ContentLengthValidator {
    min_length: usize,
}

impl Plugin for ContentLengthValidator {
    fn name(&self) -> &str {
        "content-length-validator"
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        println!("[ContentLengthValidator] Initializing");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        println!("[ContentLengthValidator] Shutting down");
        Ok(())
    }
}

#[async_trait]
impl Validator for ContentLengthValidator {
    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        println!(
            "[ContentLengthValidator] Validating content length: {} chars (min: {})",
            result.content.len(),
            self.min_length
        );

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
        100 // High priority
    }
}

// ===== Custom Extractor =====

/// Custom extractor for a fictional format
struct CustomFormatExtractor;

impl Plugin for CustomFormatExtractor {
    fn name(&self) -> &str {
        "custom-format-extractor"
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        println!("[CustomFormatExtractor] Initializing");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        println!("[CustomFormatExtractor] Shutting down");
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for CustomFormatExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        println!(
            "[CustomFormatExtractor] Extracting {} bytes of {}",
            content.len(),
            mime_type
        );

        // Custom extraction logic
        let text = String::from_utf8_lossy(content);
        let lines: Vec<&str> = text.lines().collect();

        let mut metadata = Metadata::default();
        metadata
            .additional
            .insert("line_count".to_string(), serde_json::json!(lines.len()));
        metadata
            .additional
            .insert("extractor".to_string(), serde_json::json!("custom-format-extractor"));

        Ok(ExtractionResult {
            content: text.to_string(),
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let content = std::fs::read(path)?;
        self.extract_bytes(&content, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["image/x-custom"] // Use image/* to bypass MIME validation
    }

    fn priority(&self) -> i32 {
        80 // High priority
    }
}

/// Test registering custom validator and using it in extraction.
#[test]
fn test_custom_validator_registration_and_usage() {
    println!("\n=== Test: Custom Validator Registration and Usage ===\n");

    // Create temp file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(
        &file_path,
        "This is a test document with sufficient content for validation.",
    )
    .unwrap();

    // Register custom validator
    {
        let registry = get_validator_registry();
        let mut registry = registry.write().unwrap();

        let validator = Arc::new(ContentLengthValidator { min_length: 20 });

        registry
            .register(validator)
            .expect("Validator registration should succeed");
        println!("Registered custom validator: content-length-validator");
    }

    // Extract file (should pass validation)
    let config = ExtractionConfig::default();
    let result = extract_file_sync(&file_path, None, &config);

    assert!(result.is_ok(), "Extraction should succeed with valid content");
    println!("Extraction passed validation!");

    // Test with content that's too short
    let short_file = temp_dir.path().join("short.txt");
    fs::write(&short_file, "Short").unwrap();

    let result = extract_file_sync(&short_file, None, &config);
    assert!(result.is_err(), "Extraction should fail validation for short content");

    match result.unwrap_err() {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("too short"), "Error should mention content length");
            println!("Validation correctly rejected short content: {}", message);
        }
        _ => panic!("Expected Validation error"),
    }

    // Cleanup validator
    {
        let registry = get_validator_registry();
        let mut registry = registry.write().unwrap();
        registry.remove("content-length-validator").ok();
        println!("Cleaned up validator");
    }

    println!("\n=== Test Passed ===\n");
}

/// Test registering custom extractor and using it.
#[tokio::test]
async fn test_custom_extractor_registration_and_usage_async() {
    println!("\n=== Test: Custom Extractor Registration and Usage ===\n");

    // Register custom extractor
    {
        let registry = get_document_extractor_registry();
        let mut registry = registry.write().unwrap();

        let extractor = Arc::new(CustomFormatExtractor);
        registry
            .register(extractor)
            .expect("Extractor registration should succeed");
        println!("Registered custom extractor: custom-format-extractor");
    }

    // Create temp file with enough content to pass any validators
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.custom");
    let content_text = "Line 1: This is a test document\nLine 2: With multiple lines of text\nLine 3: To demonstrate custom extraction\nLine 4: And pass validation requirements\n";
    fs::write(&file_path, content_text).unwrap();

    // Extract using custom extractor (need to manually specify MIME type)
    use kreuzberg::core::extractor::extract_bytes;

    let content = fs::read(&file_path).unwrap();
    let config = ExtractionConfig::default();
    let result = extract_bytes(&content, "image/x-custom", &config).await;

    assert!(result.is_ok(), "Extraction with custom extractor should succeed");

    let result = result.unwrap();
    assert!(result.content.contains("Line 1:"), "Should contain extracted content");
    assert!(result.content.contains("Line 2:"), "Should contain extracted content");
    assert!(result.content.contains("Line 3:"), "Should contain extracted content");
    assert!(result.content.contains("Line 4:"), "Should contain extracted content");

    // Check metadata
    let line_count = result.metadata.additional.get("line_count").unwrap().as_u64().unwrap();
    assert!(line_count >= 4, "Should have at least 4 lines, got {}", line_count);
    assert_eq!(
        result.metadata.additional.get("extractor").unwrap().as_str().unwrap(),
        "custom-format-extractor",
        "Should have correct extractor name"
    );

    println!("Custom extractor successfully processed file!");

    // Cleanup extractor
    {
        let registry = get_document_extractor_registry();
        let mut registry = registry.write().unwrap();
        registry.remove("custom-format-extractor").ok();
        println!("Cleaned up extractor");
    }

    println!("\n=== Test Passed ===\n");
}

/// Test config loading from file and using it in extraction.
#[test]
fn test_config_loading_from_file_and_usage() {
    println!("\n=== Test: Config Loading from File and Usage ===\n");

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("kreuzberg.toml");

    // Create config file
    let toml_content = r#"
[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
"#;

    fs::write(&config_path, toml_content).unwrap();
    println!("Created config file at: {}", config_path.display());

    // Load config
    let config = ExtractionConfig::from_file(&config_path);
    assert!(config.is_ok(), "Config loading should succeed");

    let config = config.unwrap();
    assert!(config.chunking.is_some(), "Should have chunking config");
    assert!(
        config.language_detection.is_some(),
        "Should have language detection config"
    );

    let chunking = config.chunking.as_ref().unwrap();
    assert_eq!(chunking.max_chars, 100, "Should have correct max_chars");
    assert_eq!(chunking.max_overlap, 20, "Should have correct max_overlap");

    println!("Config loaded successfully:");
    println!(
        "  - Chunking: max_chars={}, max_overlap={}",
        chunking.max_chars, chunking.max_overlap
    );

    // Create test file and extract with config
    let test_file = temp_dir.path().join("test.txt");
    let long_text = "This is a test. ".repeat(20); // ~320 chars
    fs::write(&test_file, &long_text).unwrap();

    let result = extract_file_sync(&test_file, None, &config);
    assert!(result.is_ok(), "Extraction with config should succeed");

    let result = result.unwrap();
    if let Some(chunks) = result.chunks {
        println!("Chunking applied: {} chunks created", chunks.len());
        assert!(chunks.len() > 1, "Should have multiple chunks for long text");

        for (i, chunk) in chunks.iter().enumerate() {
            println!("  Chunk {}: {} chars", i + 1, chunk.content.len());
            assert!(chunk.content.len() <= 120, "Chunk should respect max_chars + overlap");
        }
    } else {
        panic!("Chunking should be applied");
    }

    println!("\n=== Test Passed ===\n");
}

/// Test config discovery and usage.
#[test]
fn test_config_discovery_and_usage() {
    println!("\n=== Test: Config Discovery and Usage ===\n");

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("kreuzberg.toml");

    // Create config in root
    let toml_content = r#"
[chunking]
max_chars = 50
max_overlap = 10
"#;

    fs::write(&config_path, toml_content).unwrap();
    println!("Created config at: {}", config_path.display());

    // Create subdirectory
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    // Change to subdirectory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&sub_dir).unwrap();

    // Discover config
    let config_result = ExtractionConfig::discover();

    // Restore directory
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(config_result.is_ok(), "Config discovery should succeed");
    let config_opt = config_result.unwrap();
    assert!(config_opt.is_some(), "Should discover config in parent directory");

    let config = config_opt.unwrap();
    assert!(config.chunking.is_some(), "Should have chunking config");

    let chunking = config.chunking.as_ref().unwrap();
    assert_eq!(
        chunking.max_chars, 50,
        "Should have correct max_chars from discovered config"
    );

    println!("Config discovered successfully from parent directory!");
    println!("  - Chunking: max_chars={}", chunking.max_chars);

    println!("\n=== Test Passed ===\n");
}

/// Test complete workflow: custom plugins + config from file + extraction.
#[test]
fn test_complete_workflow_with_plugins_and_config() {
    println!("\n=== Test: Complete Workflow (Plugins + Config + Extraction) ===\n");

    let temp_dir = TempDir::new().unwrap();

    // 1. Create config file
    let config_path = temp_dir.path().join("kreuzberg.toml");
    let toml_content = r#"
[chunking]
max_chars = 80
max_overlap = 15
"#;
    fs::write(&config_path, toml_content).unwrap();
    println!("1. Created config file");

    // 2. Load config
    let config = ExtractionConfig::from_file(&config_path).expect("Config should load");
    println!("2. Loaded config from file");

    // 3. Register custom validator
    {
        let registry = get_validator_registry();
        let mut registry = registry.write().unwrap();
        let validator = Arc::new(ContentLengthValidator { min_length: 30 });
        registry
            .register(validator)
            .expect("Validator registration should succeed");
        println!("3. Registered custom validator");
    }

    // 4. Create test file
    let test_file = temp_dir.path().join("test.txt");
    let content = "This is a comprehensive test document that contains enough content to pass validation and be chunked properly. ".repeat(3);
    fs::write(&test_file, &content).unwrap();
    println!("4. Created test file ({} bytes)", content.len());

    // 5. Extract with everything configured
    let result = extract_file_sync(&test_file, None, &config);
    assert!(result.is_ok(), "Complete workflow extraction should succeed");

    let result = result.unwrap();
    println!("5. Extraction completed successfully");
    println!("   - Content length: {} chars", result.content.len());
    println!("   - Validation: PASSED");

    // 6. Verify chunking was applied
    if let Some(chunks) = result.chunks {
        println!("   - Chunks created: {}", chunks.len());
        assert!(chunks.len() > 1, "Should have multiple chunks");

        for (i, chunk) in chunks.iter().enumerate() {
            println!("     Chunk {}: {} chars", i + 1, chunk.content.len());
            assert!(chunk.content.len() <= 95, "Chunk should respect max_chars + overlap");
        }
    } else {
        panic!("Chunking should be applied");
    }

    // 7. Cleanup
    {
        let registry = get_validator_registry();
        let mut registry = registry.write().unwrap();
        registry.remove("content-length-validator").ok();
        println!("6. Cleaned up validator");
    }

    println!("\n=== Complete Workflow Test Passed ===\n");
}

/// Test error handling with plugins and config.
#[test]
fn test_error_handling_with_plugins_and_config() {
    println!("\n=== Test: Error Handling with Plugins and Config ===\n");

    let temp_dir = TempDir::new().unwrap();

    // Test 1: Invalid config file
    let invalid_config = temp_dir.path().join("invalid.toml");
    fs::write(&invalid_config, "[invalid syntax").unwrap();

    let result = ExtractionConfig::from_file(&invalid_config);
    assert!(result.is_err(), "Should fail with invalid config");
    println!("1. Invalid config correctly rejected");

    // Test 2: Missing config file
    let missing_config = temp_dir.path().join("missing.toml");
    let result = ExtractionConfig::from_file(&missing_config);
    assert!(result.is_err(), "Should fail with missing config");
    println!("2. Missing config correctly handled");

    // Test 3: Validator with empty name
    {
        let registry = get_validator_registry();
        let mut registry = registry.write().unwrap();

        struct EmptyNameValidator;
        impl Plugin for EmptyNameValidator {
            fn name(&self) -> &str {
                ""
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
        impl Validator for EmptyNameValidator {
            async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                Ok(())
            }
        }

        let validator = Arc::new(EmptyNameValidator);
        let result = registry.register(validator);
        assert!(result.is_err(), "Should reject validator with empty name");
        println!("3. Empty name validator correctly rejected");
    }

    // Test 4: Validator with whitespace in name
    {
        let registry = get_validator_registry();
        let mut registry = registry.write().unwrap();

        struct WhitespaceNameValidator;
        impl Plugin for WhitespaceNameValidator {
            fn name(&self) -> &str {
                "validator with spaces"
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
        impl Validator for WhitespaceNameValidator {
            async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
                Ok(())
            }
        }

        let validator = Arc::new(WhitespaceNameValidator);
        let result = registry.register(validator);
        assert!(result.is_err(), "Should reject validator with whitespace in name");
        println!("4. Whitespace name validator correctly rejected");
    }

    println!("\n=== Error Handling Test Passed ===\n");
}
