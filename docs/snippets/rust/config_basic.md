```rust
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract_file("document.pdf", None, &config).await?;
    println!("{}", result.content);
    Ok(())
}
```
