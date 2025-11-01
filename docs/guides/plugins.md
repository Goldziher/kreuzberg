# Creating Plugins

Kreuzberg's plugin system allows you to extend functionality by creating custom extractors, post-processors, OCR backends, and validators. Plugins can be written in Rust or Python.

## Plugin Types

Kreuzberg supports four types of plugins:

| Plugin Type | Purpose | Use Cases |
|-------------|---------|-----------|
| **DocumentExtractor** | Extract content from file formats | Add support for new formats, override built-in extractors |
| **PostProcessor** | Transform extraction results | Add metadata, enrich content, apply custom processing |
| **OcrBackend** | Perform OCR on images | Integrate cloud OCR services, custom OCR engines |
| **Validator** | Validate extraction quality | Enforce minimum quality, check completeness |

## Plugin Architecture

All plugins must implement the base `Plugin` trait and a type-specific trait. Plugins are:

- **Thread-safe**: All plugins must be `Send + Sync` (Rust) or thread-safe (Python)
- **Registered globally**: Plugins are stored in singleton registries
- **Lifecycle-managed**: Plugins have `initialize()` and `shutdown()` methods
- **Immutable**: Methods take `&self`, not `&mut self` - use interior mutability for state

## Document Extractors

Extract content from custom file formats or override built-in extractors.

### Rust Implementation

=== "Rust"

    ```rust
    use kreuzberg::plugins::{Plugin, DocumentExtractor};
    use kreuzberg::{Result, ExtractionResult, ExtractionConfig, Metadata};
    use async_trait::async_trait;
    use std::path::Path;

    struct CustomJsonExtractor;

    impl Plugin for CustomJsonExtractor {
        fn name(&self) -> &str { "custom-json-extractor" }
        fn version(&self) -> String { "1.0.0".to_string() }
        fn initialize(&self) -> Result<()> { Ok(()) }
        fn shutdown(&self) -> Result<()> { Ok(()) }
    }

    #[async_trait]
    impl DocumentExtractor for CustomJsonExtractor {
        async fn extract_bytes(
            &self,
            content: &[u8],
            _mime_type: &str,
            _config: &ExtractionConfig,
        ) -> Result<ExtractionResult> {
            let json: serde_json::Value = serde_json::from_slice(content)?;
            let text = extract_text_from_json(&json);

            Ok(ExtractionResult {
                content: text,
                mime_type: "application/json".to_string(),
                metadata: Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
            })
        }

        fn supported_mime_types(&self) -> &[&str] {
            &["application/json", "text/json"]
        }

        fn priority(&self) -> i32 { 50 }
    }

    fn extract_text_from_json(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => format!("{}\n", s),
            serde_json::Value::Array(arr) => {
                arr.iter().map(extract_text_from_json).collect()
            }
            serde_json::Value::Object(obj) => {
                obj.values().map(extract_text_from_json).collect()
            }
            _ => String::new(),
        }
    }
    ```

### Python Implementation

=== "Python"

    ```python
    from kreuzberg import register_document_extractor
    import json

    class CustomJsonExtractor:
        def name(self) -> str:
            return "custom-json-extractor"

        def version(self) -> str:
            return "1.0.0"

        def supported_mime_types(self) -> list[str]:
            return ["application/json", "text/json"]

        def priority(self) -> int:
            return 50

        def extract_bytes(
            self,
            content: bytes,
            mime_type: str,
            config: dict
        ) -> dict:
            data = json.loads(content)
            text = self._extract_text(data)

            return {
                "content": text,
                "mime_type": "application/json",
                "metadata": {},
                "tables": [],
            }

        def _extract_text(self, obj) -> str:
            if isinstance(obj, str):
                return f"{obj}\n"
            elif isinstance(obj, list):
                return "".join(self._extract_text(item) for item in obj)
            elif isinstance(obj, dict):
                return "".join(self._extract_text(v) for v in obj.values())
            return ""

        def initialize(self) -> None:
            print("Extractor initialized")

        def shutdown(self) -> None:
            print("Extractor shutdown")

    # Register the extractor
    register_document_extractor(CustomJsonExtractor())
    ```

### Registration

=== "Rust"

    ```rust
    use kreuzberg::plugins::registry::get_document_extractor_registry;
    use std::sync::Arc;

    fn register_custom_extractor() -> Result<()> {
        let extractor = Arc::new(CustomJsonExtractor);

        let registry = get_document_extractor_registry();
        let mut registry = registry.write()?;
        registry.register(extractor)?;

        Ok(())
    }
    ```

=== "Python"

    ```python
    from kreuzberg import register_document_extractor

    # Registration happens automatically when you call the function
    register_document_extractor(CustomJsonExtractor())
    ```

### Priority System

When multiple extractors support the same MIME type, the highest priority wins:

- **0-25**: Fallback/low-quality extractors
- **26-49**: Alternative implementations
- **50**: Default (built-in extractors)
- **51-75**: Enhanced/premium extractors
- **76-100**: Specialized/high-priority extractors

## Post-Processors

Transform and enrich extraction results after initial extraction.

### Processing Stages

Post-processors execute in three stages:

- **Early**: Foundational processing (language detection, quality scoring, encoding normalization)
- **Middle**: Content transformation (keyword extraction, token reduction, summarization)
- **Late**: Final enrichment (custom hooks, analytics, output formatting)

### Rust Implementation

=== "Rust"

    ```rust
    use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    use async_trait::async_trait;

    struct WordCountProcessor;

    impl Plugin for WordCountProcessor {
        fn name(&self) -> &str { "word-count" }
        fn version(&self) -> String { "1.0.0".to_string() }
        fn initialize(&self) -> Result<()> { Ok(()) }
        fn shutdown(&self) -> Result<()> { Ok(()) }
    }

    #[async_trait]
    impl PostProcessor for WordCountProcessor {
        async fn process(
            &self,
            result: &mut ExtractionResult,
            _config: &ExtractionConfig
        ) -> Result<()> {
            let word_count = result.content.split_whitespace().count();

            result.metadata.additional.insert(
                "word_count".to_string(),
                serde_json::json!(word_count)
            );

            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }

        fn should_process(
            &self,
            result: &ExtractionResult,
            _config: &ExtractionConfig
        ) -> bool {
            // Only process if content is non-empty
            !result.content.is_empty()
        }
    }
    ```

### Python Implementation

=== "Python"

    ```python
    from kreuzberg import register_post_processor

    class WordCountProcessor:
        def name(self) -> str:
            return "word_count"

        def version(self) -> str:
            return "1.0.0"

        def processing_stage(self) -> str:
            return "early"  # or "middle", "late"

        def process(self, result: dict) -> dict:
            word_count = len(result["content"].split())
            result["metadata"]["word_count"] = word_count
            return result

        def should_process(self, result: dict) -> bool:
            return bool(result["content"])

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_post_processor(WordCountProcessor())
    ```

### Conditional Processing

=== "Rust"

    ```rust
    impl PostProcessor for PdfOnlyProcessor {
        async fn process(
            &self,
            result: &mut ExtractionResult,
            _config: &ExtractionConfig
        ) -> Result<()> {
            // PDF-specific processing
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }

        fn should_process(
            &self,
            result: &ExtractionResult,
            _config: &ExtractionConfig
        ) -> bool {
            result.mime_type == "application/pdf"
        }
    }
    ```

=== "Python"

    ```python
    class PdfOnlyProcessor:
        def process(self, result: dict) -> dict:
            # PDF-specific processing
            return result

        def should_process(self, result: dict) -> bool:
            return result["mime_type"] == "application/pdf"
    ```

## OCR Backends

Integrate custom OCR engines or cloud services.

### Rust Implementation

=== "Rust"

    ```rust
    use kreuzberg::plugins::{Plugin, OcrBackend, OcrBackendType};
    use kreuzberg::{Result, ExtractionResult, OcrConfig, Metadata};
    use async_trait::async_trait;
    use std::path::Path;

    struct CloudOcrBackend {
        api_key: String,
        supported_langs: Vec<String>,
    }

    impl Plugin for CloudOcrBackend {
        fn name(&self) -> &str { "cloud-ocr" }
        fn version(&self) -> String { "1.0.0".to_string() }
        fn initialize(&self) -> Result<()> { Ok(()) }
        fn shutdown(&self) -> Result<()> { Ok(()) }
    }

    #[async_trait]
    impl OcrBackend for CloudOcrBackend {
        async fn process_image(
            &self,
            image_bytes: &[u8],
            config: &OcrConfig,
        ) -> Result<ExtractionResult> {
            // Send image to cloud OCR service
            let text = self.call_cloud_api(image_bytes, &config.language).await?;

            Ok(ExtractionResult {
                content: text,
                mime_type: "text/plain".to_string(),
                metadata: Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
            })
        }

        fn supports_language(&self, lang: &str) -> bool {
            self.supported_langs.iter().any(|l| l == lang)
        }

        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }

        fn supported_languages(&self) -> Vec<String> {
            self.supported_langs.clone()
        }
    }

    impl CloudOcrBackend {
        async fn call_cloud_api(
            &self,
            image: &[u8],
            language: &str
        ) -> Result<String> {
            // API call implementation
            Ok("Extracted text".to_string())
        }
    }
    ```

### Python Implementation

=== "Python"

    ```python
    from kreuzberg import register_ocr_backend
    import requests

    class CloudOcrBackend:
        def __init__(self, api_key: str):
            self.api_key = api_key
            self.supported_langs = ["eng", "deu", "fra"]

        def name(self) -> str:
            return "cloud-ocr"

        def version(self) -> str:
            return "1.0.0"

        def backend_type(self) -> str:
            return "custom"

        def supported_languages(self) -> list[str]:
            return self.supported_langs

        def supports_language(self, language: str) -> bool:
            return language in self.supported_langs

        def process_image(self, image_bytes: bytes, config: dict) -> dict:
            # Send image to cloud OCR service
            response = requests.post(
                "https://api.example.com/ocr",
                files={"image": image_bytes},
                headers={"Authorization": f"Bearer {self.api_key}"},
                json={"language": config.get("language", "eng")}
            )

            text = response.json()["text"]

            return {
                "content": text,
                "mime_type": "text/plain",
                "metadata": {"confidence": response.json().get("confidence", 0.0)},
                "tables": [],
            }

        def initialize(self) -> None:
            print("Cloud OCR backend initialized")

        def shutdown(self) -> None:
            print("Cloud OCR backend shutdown")

    # Register the backend
    register_ocr_backend(CloudOcrBackend(api_key="your-api-key"))
    ```

## Validators

Enforce quality requirements on extraction results.

!!! warning "Validators are Fatal"
    Validation errors cause extraction to fail. Use validators for critical quality checks only.

### Rust Implementation

=== "Rust"

    ```rust
    use kreuzberg::plugins::{Plugin, Validator};
    use kreuzberg::{Result, ExtractionResult, ExtractionConfig, KreuzbergError};
    use async_trait::async_trait;

    struct MinLengthValidator {
        min_length: usize,
    }

    impl Plugin for MinLengthValidator {
        fn name(&self) -> &str { "min-length-validator" }
        fn version(&self) -> String { "1.0.0".to_string() }
        fn initialize(&self) -> Result<()> { Ok(()) }
        fn shutdown(&self) -> Result<()> { Ok(()) }
    }

    #[async_trait]
    impl Validator for MinLengthValidator {
        async fn validate(
            &self,
            result: &ExtractionResult,
            _config: &ExtractionConfig,
        ) -> Result<()> {
            if result.content.len() < self.min_length {
                return Err(KreuzbergError::validation(format!(
                    "Content too short: {} < {} characters",
                    result.content.len(),
                    self.min_length
                )));
            }
            Ok(())
        }

        fn priority(&self) -> i32 {
            100  // Run early - fast check
        }
    }
    ```

### Python Implementation

=== "Python"

    ```python
    from kreuzberg import register_validator
    from kreuzberg.exceptions import ValidationError

    class MinLengthValidator:
        def __init__(self, min_length: int = 100):
            self.min_length = min_length

        def name(self) -> str:
            return "min_length_validator"

        def version(self) -> str:
            return "1.0.0"

        def priority(self) -> int:
            return 100  # Run early

        def validate(self, result: dict) -> None:
            if len(result["content"]) < self.min_length:
                raise ValidationError(
                    f"Content too short: {len(result['content'])} < {self.min_length}"
                )

        def should_validate(self, result: dict) -> bool:
            return True  # Always validate

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_validator(MinLengthValidator(min_length=100))
    ```

### Quality Score Validator

=== "Rust"

    ```rust
    #[async_trait]
    impl Validator for QualityValidator {
        async fn validate(
            &self,
            result: &ExtractionResult,
            _config: &ExtractionConfig,
        ) -> Result<()> {
            let score = result.metadata
                .additional
                .get("quality_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            if score < 0.5 {
                return Err(KreuzbergError::validation(format!(
                    "Quality score too low: {:.2} < 0.50",
                    score
                )));
            }

            Ok(())
        }
    }
    ```

=== "Python"

    ```python
    class QualityValidator:
        def validate(self, result: dict) -> None:
            score = result["metadata"].get("quality_score", 0.0)

            if score < 0.5:
                raise ValidationError(
                    f"Quality score too low: {score:.2f} < 0.50"
                )
    ```

## Plugin Management

### Listing Plugins

=== "Rust"

    ```rust
    use kreuzberg::plugins::registry::*;

    // List document extractors
    let registry = get_document_extractor_registry();
    let registry = registry.read()?;
    let extractors = registry.list();
    println!("Registered extractors: {:?}", extractors);

    // List post-processors
    let registry = get_post_processor_registry();
    let registry = registry.read()?;
    let processors = registry.list();
    println!("Registered processors: {:?}", processors);

    // List OCR backends
    let registry = get_ocr_backend_registry();
    let registry = registry.read()?;
    let backends = registry.list();
    println!("Registered OCR backends: {:?}", backends);

    // List validators
    let registry = get_validator_registry();
    let registry = registry.read()?;
    let validators = registry.list();
    println!("Registered validators: {:?}", validators);
    ```

=== "Python"

    ```python
    from kreuzberg import (
        list_document_extractors,
        list_post_processors,
        list_ocr_backends,
        list_validators,
    )

    print("Extractors:", list_document_extractors())
    print("Processors:", list_post_processors())
    print("OCR backends:", list_ocr_backends())
    print("Validators:", list_validators())
    ```

### Unregistering Plugins

=== "Rust"

    ```rust
    use kreuzberg::plugins::registry::get_document_extractor_registry;

    // Unregister a specific plugin
    let registry = get_document_extractor_registry();
    let mut registry = registry.write()?;
    registry.remove("custom-json-extractor")?;
    ```

=== "Python"

    ```python
    from kreuzberg import (
        unregister_document_extractor,
        unregister_post_processor,
        unregister_ocr_backend,
        unregister_validator,
    )

    unregister_document_extractor("custom-json-extractor")
    unregister_post_processor("word_count")
    unregister_ocr_backend("cloud-ocr")
    unregister_validator("min_length_validator")
    ```

### Clearing All Plugins

=== "Python"

    ```python
    from kreuzberg import (
        clear_document_extractors,
        clear_post_processors,
        clear_ocr_backends,
        clear_validators,
    )

    # Clear all plugins of a specific type
    clear_post_processors()
    clear_validators()
    ```

## Thread Safety

All plugins must be thread-safe:

### Rust Thread Safety

=== "Rust"

    ```rust
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct StatefulPlugin {
        // Use atomic types for simple counters
        call_count: AtomicUsize,

        // Use Mutex for complex state
        cache: Mutex<HashMap<String, String>>,
    }

    impl Plugin for StatefulPlugin {
        fn name(&self) -> &str { "stateful-plugin" }
        fn version(&self) -> String { "1.0.0".to_string() }

        fn initialize(&self) -> Result<()> {
            // Safe to modify with interior mutability
            self.call_count.store(0, Ordering::Release);
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            let count = self.call_count.load(Ordering::Acquire);
            println!("Plugin called {} times", count);
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for StatefulPlugin {
        async fn process(
            &self,
            result: &mut ExtractionResult,
            _config: &ExtractionConfig
        ) -> Result<()> {
            // Increment counter atomically
            self.call_count.fetch_add(1, Ordering::AcqRel);

            // Access cache with lock
            let mut cache = self.cache.lock().unwrap();
            cache.insert("last_mime".to_string(), result.mime_type.clone());

            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Middle
        }
    }
    ```

### Python Thread Safety

=== "Python"

    ```python
    import threading

    class StatefulPlugin:
        def __init__(self):
            self.lock = threading.Lock()
            self.call_count = 0
            self.cache = {}

        def process(self, result: dict) -> dict:
            with self.lock:
                self.call_count += 1
                self.cache["last_mime"] = result["mime_type"]
            return result
    ```

## Best Practices

### Naming

- Use kebab-case for plugin names: `my-custom-plugin`
- Use lowercase only, no spaces or special characters
- Be descriptive but concise

### Error Handling

=== "Rust"

    ```rust
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Validate inputs
        if content.is_empty() {
            return Err(KreuzbergError::validation("Empty content"));
        }

        // Handle errors with context
        let parsed = parse_content(content)
            .map_err(|e| KreuzbergError::parsing(
                format!("Failed to parse {}: {}", mime_type, e)
            ))?;

        Ok(result)
    }
    ```

=== "Python"

    ```python
    def extract_bytes(
        self,
        content: bytes,
        mime_type: str,
        config: dict
    ) -> dict:
        if not content:
            raise ValueError("Empty content")

        try:
            data = parse_content(content)
        except Exception as e:
            raise ParsingError(
                f"Failed to parse {mime_type}: {e}"
            ) from e

        return result
    ```

### Logging

=== "Rust"

    ```rust
    use log::{info, warn, error};

    impl Plugin for MyPlugin {
        fn initialize(&self) -> Result<()> {
            info!("Initializing plugin: {}", self.name());
            Ok(())
        }

        fn shutdown(&self) -> Result<()> {
            info!("Shutting down plugin: {}", self.name());
            Ok(())
        }
    }

    #[async_trait]
    impl DocumentExtractor for MyPlugin {
        async fn extract_bytes(
            &self,
            content: &[u8],
            mime_type: &str,
            _config: &ExtractionConfig,
        ) -> Result<ExtractionResult> {
            info!("Extracting {} ({} bytes)", mime_type, content.len());

            // Processing...

            if result.content.is_empty() {
                warn!("Extraction resulted in empty content");
            }

            Ok(result)
        }
    }
    ```

=== "Python"

    ```python
    import logging

    logger = logging.getLogger(__name__)

    class MyPlugin:
        def initialize(self) -> None:
            logger.info(f"Initializing plugin: {self.name()}")

        def shutdown(self) -> None:
            logger.info(f"Shutting down plugin: {self.name()}")

        def extract_bytes(
            self,
            content: bytes,
            mime_type: str,
            config: dict
        ) -> dict:
            logger.info(f"Extracting {mime_type} ({len(content)} bytes)")

            # Processing...

            if not result["content"]:
                logger.warning("Extraction resulted in empty content")

            return result
    ```

### Testing

=== "Rust"

    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_custom_extractor() {
            let extractor = CustomJsonExtractor;

            let json_data = br#"{"message": "Hello, world!"}"#;
            let config = ExtractionConfig::default();

            let result = extractor
                .extract_bytes(json_data, "application/json", &config)
                .await
                .unwrap();

            assert!(result.content.contains("Hello, world!"));
            assert_eq!(result.mime_type, "application/json");
        }
    }
    ```

=== "Python"

    ```python
    import pytest

    def test_custom_extractor():
        extractor = CustomJsonExtractor()

        json_data = b'{"message": "Hello, world!"}'
        config = {}

        result = extractor.extract_bytes(json_data, "application/json", config)

        assert "Hello, world!" in result["content"]
        assert result["mime_type"] == "application/json"
    ```

## Complete Example: PDF Metadata Extractor

=== "Rust"

    ```rust
    use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
    use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct PdfMetadataExtractor {
        processed_count: AtomicUsize,
    }

    impl PdfMetadataExtractor {
        fn new() -> Self {
            Self {
                processed_count: AtomicUsize::new(0),
            }
        }
    }

    impl Plugin for PdfMetadataExtractor {
        fn name(&self) -> &str { "pdf-metadata-extractor" }
        fn version(&self) -> String { "1.0.0".to_string() }
        fn description(&self) -> &str {
            "Extracts and enriches PDF metadata"
        }
        fn initialize(&self) -> Result<()> {
            log::info!("PDF metadata extractor initialized");
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            let count = self.processed_count.load(Ordering::Acquire);
            log::info!("Processed {} PDFs", count);
            Ok(())
        }
    }

    #[async_trait]
    impl PostProcessor for PdfMetadataExtractor {
        async fn process(
            &self,
            result: &mut ExtractionResult,
            _config: &ExtractionConfig,
        ) -> Result<()> {
            self.processed_count.fetch_add(1, Ordering::AcqRel);

            // Extract PDF-specific metadata
            result.metadata.additional.insert(
                "pdf_processed".to_string(),
                serde_json::json!(true)
            );

            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Early
        }

        fn should_process(
            &self,
            result: &ExtractionResult,
            _config: &ExtractionConfig,
        ) -> bool {
            result.mime_type == "application/pdf"
        }

        fn estimated_duration_ms(&self, _result: &ExtractionResult) -> u64 {
            10  // Fast operation
        }
    }

    // Registration
    use kreuzberg::plugins::registry::get_post_processor_registry;
    use std::sync::Arc;

    fn register() -> Result<()> {
        let processor = Arc::new(PdfMetadataExtractor::new());
        let registry = get_post_processor_registry();
        let mut registry = registry.write()?;
        registry.register(processor, 50)?;  // Default priority
        Ok(())
    }
    ```

=== "Python"

    ```python
    from kreuzberg import register_post_processor
    import logging

    logger = logging.getLogger(__name__)

    class PdfMetadataExtractor:
        def __init__(self):
            self.processed_count = 0

        def name(self) -> str:
            return "pdf_metadata_extractor"

        def version(self) -> str:
            return "1.0.0"

        def description(self) -> str:
            return "Extracts and enriches PDF metadata"

        def processing_stage(self) -> str:
            return "early"

        def should_process(self, result: dict) -> bool:
            return result["mime_type"] == "application/pdf"

        def process(self, result: dict) -> dict:
            self.processed_count += 1

            # Extract PDF-specific metadata
            result["metadata"]["pdf_processed"] = True

            return result

        def initialize(self) -> None:
            logger.info("PDF metadata extractor initialized")

        def shutdown(self) -> None:
            logger.info(f"Processed {self.processed_count} PDFs")

    # Register the processor
    register_post_processor(PdfMetadataExtractor())
    ```
