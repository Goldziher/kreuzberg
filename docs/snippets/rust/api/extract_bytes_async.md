```rust
use kreuzberg::{extract_bytes, ExtractionConfig};
use tokio::fs;

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let data = fs::read("document.pdf").await?;

    let result = extract_bytes(
        &data,
        "application/pdf",
        &ExtractionConfig::default()
    ).await?;
    println!("{}", result.content);
    Ok(())
}
```
