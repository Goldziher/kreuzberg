```rust
use kreuzberg::{ExtractionConfig, LanguageDetectionConfig};

fn main() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: Some(true),
            min_confidence: Some(0.9),
            detect_multiple: Some(true),
        }),
        ..Default::default()
    };
    println!("{:?}", config.language_detection);
}
```
