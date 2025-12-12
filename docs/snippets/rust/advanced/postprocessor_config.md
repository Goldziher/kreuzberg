```rust title="Rust"
use kreuzberg::{ExtractionConfig, PostProcessorConfig};

fn main() {
    let config = ExtractionConfig {
        postprocessor: Some(PostProcessorConfig {
            enabled: Some(true),
            enabled_processors: Some(vec![
                "deduplication".to_string(),
                "whitespace_normalization".to_string(),
            ]),
            disabled_processors: Some(vec!["mojibake_fix".to_string()]),
        }),
        ..Default::default()
    };
    println!("{:?}", config.postprocessor);
}
```
