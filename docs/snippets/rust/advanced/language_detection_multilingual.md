```rust
use kreuzberg::{extract_file, ExtractionConfig, LanguageDetectionConfig};

let config = ExtractionConfig {
    language_detection: Some(LanguageDetectionConfig {
        enabled: true,
        min_confidence: 0.8,
        detect_multiple: true,
    }),
    ..Default::default()
};

let result = extract_file("multilingual_document.pdf", None, &config).await?;

println!("Detected languages: {:?}", result.detected_languages);
```
