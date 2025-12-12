```rust title="Rust"
use kreuzberg::{ExtractionConfig, TokenReductionConfig};

fn main() {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionConfig {
            mode: Some("moderate".to_string()),
            preserve_important_words: Some(true),
        }),
        ..Default::default()
    };
    println!("{:?}", config.token_reduction);
}
```
