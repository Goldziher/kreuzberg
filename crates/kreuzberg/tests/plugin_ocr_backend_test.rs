//! Comprehensive OCR backend plugin system tests.
//!
//! Tests custom OCR backend registration, execution, parameter passing,
//! error handling, and backend switching with real image extraction.

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend, OcrBackendType, Plugin};
use kreuzberg::types::{ExtractionResult, Metadata};
use kreuzberg::{KreuzbergError, Result, extract_file_sync};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// Simple mock OCR backend that returns fixed text
struct MockOcrBackend {
    name: String,
    return_text: String,
    call_count: AtomicUsize,
    last_language: Mutex<String>,
    initialized: AtomicBool,
}

impl Plugin for MockOcrBackend {
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
impl OcrBackend for MockOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Store the language for verification
        *self.last_language.lock().unwrap() = config.language.clone();

        // Verify we received image data
        if image_bytes.is_empty() {
            return Err(KreuzbergError::validation("Empty image data".to_string()));
        }

        Ok(ExtractionResult {
            content: format!("{} (lang: {})", self.return_text, config.language),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supports_language(&self, lang: &str) -> bool {
        matches!(lang, "eng" | "deu" | "fra")
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["eng".to_string(), "deu".to_string(), "fra".to_string()]
    }
}

// OCR backend that fails during processing
struct FailingOcrBackend {
    name: String,
}

impl Plugin for FailingOcrBackend {
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
impl OcrBackend for FailingOcrBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
        Err(KreuzbergError::ocr("OCR processing intentionally failed".to_string()))
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

// OCR backend that validates image size
struct ValidatingOcrBackend {
    name: String,
    min_size: usize,
}

impl Plugin for ValidatingOcrBackend {
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
impl OcrBackend for ValidatingOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], _config: &OcrConfig) -> Result<ExtractionResult> {
        if image_bytes.len() < self.min_size {
            return Err(KreuzbergError::validation(format!(
                "Image too small: {} < {} bytes",
                image_bytes.len(),
                self.min_size
            )));
        }

        Ok(ExtractionResult {
            content: format!("Processed {} bytes", image_bytes.len()),
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

// OCR backend that adds metadata
struct MetadataOcrBackend {
    name: String,
}

impl Plugin for MetadataOcrBackend {
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
impl OcrBackend for MetadataOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        let mut metadata = Metadata::default();
        metadata
            .additional
            .insert("ocr_backend".to_string(), serde_json::json!(self.name()));
        metadata
            .additional
            .insert("image_size".to_string(), serde_json::json!(image_bytes.len()));
        metadata
            .additional
            .insert("ocr_language".to_string(), serde_json::json!(config.language));

        Ok(ExtractionResult {
            content: "OCR processed text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

#[test]
fn test_register_custom_ocr_backend() {
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "test-ocr".to_string(),
        return_text: "Mocked OCR Result".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        let result = reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>);
        assert!(result.is_ok(), "Failed to register OCR backend: {:?}", result.err());
    }

    assert!(
        backend.initialized.load(Ordering::Acquire),
        "OCR backend was not initialized"
    );

    let list = {
        let reg = registry.read().unwrap();
        reg.list()
    };

    assert!(list.contains(&"test-ocr".to_string()));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_used_for_image_extraction() {
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "extraction-test-ocr".to_string(),
        return_text: "CUSTOM OCR TEXT".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>).unwrap();
    }

    let ocr_config = OcrConfig {
        backend: "extraction-test-ocr".to_string(),
        language: "eng".to_string(),
        tesseract_config: None,
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let extraction_result = result.unwrap();
    assert!(
        extraction_result.content.contains("CUSTOM OCR TEXT"),
        "Custom OCR backend was not used. Content: {}",
        extraction_result.content
    );

    assert_eq!(
        backend.call_count.load(Ordering::SeqCst),
        1,
        "OCR backend was not called exactly once"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_receives_correct_parameters() {
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "param-test-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>).unwrap();
    }

    let ocr_config = OcrConfig {
        backend: "param-test-ocr".to_string(),
        language: "deu".to_string(), // German language
        tesseract_config: None,
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_ok());

    // Verify language parameter was passed correctly
    let last_lang = backend.last_language.lock().unwrap();
    assert_eq!(*last_lang, "deu", "Language parameter not passed correctly");

    let extraction_result = result.unwrap();
    assert!(extraction_result.content.contains("(lang: deu)"));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_returns_correct_format() {
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MetadataOcrBackend {
        name: "format-test-ocr".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(backend as Arc<dyn OcrBackend>).unwrap();
    }

    let ocr_config = OcrConfig {
        backend: "format-test-ocr".to_string(),
        language: "eng".to_string(),
        tesseract_config: None,
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_ok());

    let extraction_result = result.unwrap();

    // Verify result format
    assert!(!extraction_result.content.is_empty());
    assert_eq!(extraction_result.mime_type, "text/plain");
    assert!(extraction_result.metadata.additional.contains_key("ocr_backend"));
    assert!(extraction_result.metadata.additional.contains_key("image_size"));
    assert!(extraction_result.metadata.additional.contains_key("ocr_language"));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_error_handling() {
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(FailingOcrBackend {
        name: "failing-ocr".to_string(),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(backend as Arc<dyn OcrBackend>).unwrap();
    }

    let ocr_config = OcrConfig {
        backend: "failing-ocr".to_string(),
        language: "eng".to_string(),
        tesseract_config: None,
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_err(), "Expected OCR to fail");

    match result.err().unwrap() {
        KreuzbergError::Ocr { message, .. } => {
            assert!(message.contains("intentionally failed"));
        }
        other => panic!("Expected Ocr error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_validation_error() {
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(ValidatingOcrBackend {
        name: "validating-ocr".to_string(),
        min_size: 1_000_000, // 1MB - will fail for our test image
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(backend as Arc<dyn OcrBackend>).unwrap();
    }

    let ocr_config = OcrConfig {
        backend: "validating-ocr".to_string(),
        language: "eng".to_string(),
        tesseract_config: None,
    };

    let config = ExtractionConfig {
        ocr: Some(ocr_config),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(test_image, None, &config);

    assert!(result.is_err(), "Expected validation to fail");

    match result.err().unwrap() {
        KreuzbergError::Validation { message, .. } => {
            assert!(message.contains("Image too small"));
        }
        other => panic!("Expected Validation error, got: {:?}", other),
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_switching_between_ocr_backends() {
    let test_image = "../../test_documents/images/test_hello_world.png";
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend1 = Arc::new(MockOcrBackend {
        name: "backend-1".to_string(),
        return_text: "BACKEND ONE OUTPUT".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    let backend2 = Arc::new(MockOcrBackend {
        name: "backend-2".to_string(),
        return_text: "BACKEND TWO OUTPUT".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&backend1) as Arc<dyn OcrBackend>).unwrap();
        reg.register(Arc::clone(&backend2) as Arc<dyn OcrBackend>).unwrap();
    }

    // Test with backend 1
    let ocr_config1 = OcrConfig {
        backend: "backend-1".to_string(),
        language: "eng".to_string(),
        tesseract_config: None,
    };

    let config1 = ExtractionConfig {
        ocr: Some(ocr_config1),
        force_ocr: false,
        ..Default::default()
    };

    let result1 = extract_file_sync(test_image, None, &config1);
    assert!(result1.is_ok());
    assert!(result1.unwrap().content.contains("BACKEND ONE OUTPUT"));
    assert_eq!(backend1.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(backend2.call_count.load(Ordering::SeqCst), 0);

    // Test with backend 2
    let ocr_config2 = OcrConfig {
        backend: "backend-2".to_string(),
        language: "eng".to_string(),
        tesseract_config: None,
    };

    let config2 = ExtractionConfig {
        ocr: Some(ocr_config2),
        force_ocr: false,
        ..Default::default()
    };

    let result2 = extract_file_sync(test_image, None, &config2);
    assert!(result2.is_ok());
    assert!(result2.unwrap().content.contains("BACKEND TWO OUTPUT"));
    assert_eq!(backend1.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(backend2.call_count.load(Ordering::SeqCst), 1);

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_language_support() {
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "lang-test-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>).unwrap();
    }

    // Test supported languages
    assert!(backend.supports_language("eng"));
    assert!(backend.supports_language("deu"));
    assert!(backend.supports_language("fra"));
    assert!(!backend.supports_language("jpn"));

    let supported = backend.supported_languages();
    assert_eq!(supported.len(), 3);
    assert!(supported.contains(&"eng".to_string()));
    assert!(supported.contains(&"deu".to_string()));
    assert!(supported.contains(&"fra".to_string()));

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_type() {
    let backend = MockOcrBackend {
        name: "type-test".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    };

    assert_eq!(backend.backend_type(), OcrBackendType::Custom);
}

#[test]
fn test_ocr_backend_invalid_name() {
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "invalid name".to_string(), // Contains space - invalid
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        let result = reg.register(backend);

        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), KreuzbergError::Validation { .. }));
    }

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }
}

#[test]
fn test_ocr_backend_initialization_lifecycle() {
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "lifecycle-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    assert!(
        !backend.initialized.load(Ordering::Acquire),
        "Backend should not be initialized yet"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>).unwrap();
    }

    assert!(
        backend.initialized.load(Ordering::Acquire),
        "Backend should be initialized after registration"
    );

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    assert!(
        !backend.initialized.load(Ordering::Acquire),
        "Backend should be shutdown"
    );
}

#[test]
fn test_unregister_ocr_backend() {
    let registry = get_ocr_backend_registry();

    {
        let mut reg = registry.write().unwrap();
        reg.shutdown_all().unwrap();
    }

    let backend = Arc::new(MockOcrBackend {
        name: "unregister-ocr".to_string(),
        return_text: "Test".to_string(),
        call_count: AtomicUsize::new(0),
        last_language: Mutex::new(String::new()),
        initialized: AtomicBool::new(false),
    });

    {
        let mut reg = registry.write().unwrap();
        reg.register(Arc::clone(&backend) as Arc<dyn OcrBackend>).unwrap();
    }

    {
        let mut reg = registry.write().unwrap();
        reg.remove("unregister-ocr").unwrap();
    }

    let list = {
        let reg = registry.read().unwrap();
        reg.list()
    };

    assert!(!list.contains(&"unregister-ocr".to_string()));
    assert!(
        !backend.initialized.load(Ordering::Acquire),
        "Backend should be shutdown after unregistration"
    );
}
