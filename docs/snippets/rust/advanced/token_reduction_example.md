```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, TokenReductionConfig};

let config = ExtractionConfig {
    token_reduction: Some(TokenReductionConfig {
        mode: "moderate".to_string(),
        preserve_markdown: true,
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file("verbose_document.pdf", None, &config).await?;

if let Some(original) = result.metadata.additional.get("original_token_count") {
    println!("Original tokens: {}", original);
}
if let Some(reduced) = result.metadata.additional.get("token_count") {
    println!("Reduced tokens: {}", reduced);
}
```
