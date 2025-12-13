```rust title="Rust"
use kreuzberg::{ExtractionConfig, TokenReductionConfig};

let config = ExtractionConfig {
    token_reduction: Some(TokenReductionConfig {
        mode: "moderate".to_string(),
        preserve_markdown: true,
        preserve_code: true,
        language_hint: Some("eng".to_string()),
        ..Default::default()
    }),
    ..Default::default()
};
```
