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
- **Lifecycle-managed**: Plugins have `initialize()` and `shutdown()` methods
- **Registered globally**: Use registry functions to register your plugins

## Document Extractors

Extract content from custom file formats or override built-in extractors.

### Rust Implementation

=== "Rust"

    --8<-- "snippets/rust/plugin_extractor.md"

### Python Implementation

=== "Python"

    --8<-- "snippets/python/plugin_extractor.md"

### Registration

=== "C#"

    --8<-- "snippets/csharp/extractor_registration.md"

=== "Go"

    --8<-- "snippets/go/extractor_registration.md"

=== "Java"

    --8<-- "snippets/java/extractor_registration.md"

=== "Python"

    --8<-- "snippets/python/extractor_registration.md"

=== "Ruby"

    --8<-- "snippets/ruby/extractor_registration.md"

=== "Rust"

    --8<-- "snippets/rust/extractor_registration.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extractor_registration.md"

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

- **Early**: Run first, use for foundational operations like language detection, quality scoring, or text normalization that other processors may depend on
- **Middle**: Run second, use for content transformation like keyword extraction, token reduction, or summarization
- **Late**: Run last, use for final enrichment like custom metadata, analytics tracking, or output formatting

### Rust Implementation

=== "Rust"

    --8<-- "snippets/rust/word_count_processor.md"

### Python Implementation

=== "Python"

    --8<-- "snippets/python/word_count_processor.md"

### Conditional Processing

=== "C#"

    --8<-- "snippets/csharp/pdf_only_processor.md"

=== "Java"

    --8<-- "snippets/java/pdf_only_processor.md"

=== "Python"

    --8<-- "snippets/python/pdf_only_processor.md"

=== "Rust"

    --8<-- "snippets/rust/pdf_only_processor.md"

## OCR Backends

Integrate custom OCR engines or cloud services.

### Rust Implementation

=== "Rust"

    --8<-- "snippets/rust/cloud_ocr_backend.md"

### Python Implementation

=== "C#"

    --8<-- "snippets/csharp/cloud_ocr_backend.md"

=== "Java"

    --8<-- "snippets/java/cloud_ocr_backend.md"

=== "Python"

    --8<-- "snippets/python/cloud_ocr_backend.md"

## Validators

Enforce quality requirements on extraction results.

!!! warning "Validators are Fatal"
    Validation errors cause extraction to fail. Use validators for critical quality checks only.

### Rust Implementation

=== "Rust"

    --8<-- "snippets/rust/min_length_validator.md"

### Python Implementation

=== "C#"

    --8<-- "snippets/csharp/min_length_validator.md"

=== "Java"

    --8<-- "snippets/java/min_length_validator.md"

=== "Python"

    --8<-- "snippets/python/min_length_validator.md"

### Quality Score Validator

=== "C#"

    --8<-- "snippets/csharp/quality_score_validator.md"

=== "Java"

    --8<-- "snippets/java/quality_score_validator.md"

=== "Python"

    --8<-- "snippets/python/quality_score_validator.md"

=== "Rust"

    --8<-- "snippets/rust/quality_score_validator.md"

## Plugin Management

### Listing Plugins

=== "C#"

    --8<-- "snippets/csharp/list_plugins.md"

=== "Java"

    --8<-- "snippets/java/list_plugins.md"

=== "Python"

    --8<-- "snippets/python/list_plugins.md"

=== "Rust"

    --8<-- "snippets/rust/list_plugins.md"

### Unregistering Plugins

=== "C#"

    --8<-- "snippets/csharp/unregister_plugins.md"

=== "Java"

    --8<-- "snippets/java/unregister_plugins.md"

=== "Python"

    --8<-- "snippets/python/unregister_plugins.md"

=== "Rust"

    --8<-- "snippets/rust/unregister_plugins.md"

### Clearing All Plugins

=== "C#"

    --8<-- "snippets/csharp/clear_plugins.md"

=== "Java"

    --8<-- "snippets/java/clear_plugins.md"

=== "Python"

    --8<-- "snippets/python/clear_plugins.md"

=== "Rust"

    --8<-- "snippets/rust/clear_plugins.md"

## Thread Safety

All plugins must be thread-safe:

### Rust Thread Safety

=== "Rust"

    --8<-- "snippets/rust/stateful_plugin.md"

### Python Thread Safety

=== "C#"

    --8<-- "snippets/csharp/stateful_plugin.md"

=== "Java"

    --8<-- "snippets/java/stateful_plugin.md"

=== "Python"

    --8<-- "snippets/python/stateful_plugin.md"

## Best Practices

### Naming

- Use kebab-case for plugin names: `my-custom-plugin`
- Use lowercase only, no spaces or special characters
- Be descriptive but concise

### Error Handling

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

=== "Java"

    --8<-- "snippets/java/error_handling.md"

=== "Python"

    --8<-- "snippets/python/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/error_handling.md"

### Logging

=== "C#"

    --8<-- "snippets/csharp/plugin_logging.md"

=== "Java"

    --8<-- "snippets/java/plugin_logging.md"

=== "Python"

    --8<-- "snippets/python/plugin_logging.md"

=== "Rust"

    --8<-- "snippets/rust/plugin_logging.md"

### Testing

=== "C#"

    --8<-- "snippets/csharp/plugin_testing.md"

=== "Java"

    --8<-- "snippets/java/plugin_testing.md"

=== "Python"

    --8<-- "snippets/python/plugin_testing.md"

=== "Rust"

    --8<-- "snippets/rust/plugin_testing.md"

## Complete Example: PDF Metadata Extractor

=== "C#"

    --8<-- "snippets/csharp/pdf_metadata_extractor.md"

=== "Java"

    --8<-- "snippets/java/pdf_metadata_extractor.md"

=== "Python"

    --8<-- "snippets/python/pdf_metadata_extractor.md"

=== "Rust"

    --8<-- "snippets/rust/pdf_metadata_extractor.md"
