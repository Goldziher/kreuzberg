```rust title="Rust"
use kreuzberg::{ExtractionConfig, LanguageDetectionConfig};

let config = ExtractionConfig {
    language_detection: Some(LanguageDetectionConfig {
        enabled: true,
        min_confidence: 0.8,
        detect_multiple: false,
    }),
    ..Default::default()
};
```
