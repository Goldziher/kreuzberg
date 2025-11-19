```rust
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let result = extract_file("document.pdf", None, &ExtractionConfig::default()).await?;
    println!("{}", result.content);
    Ok(())
}
```
