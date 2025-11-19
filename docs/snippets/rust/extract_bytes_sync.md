```rust
use kreuzberg::{extract_bytes_sync, ExtractionConfig};
use std::fs;

fn main() -> kreuzberg::Result<()> {
    let data = fs::read("document.pdf")?;

    let result = extract_bytes_sync(
        &data,
        "application/pdf",
        &ExtractionConfig::default()
    )?;
    println!("{}", result.content);
    Ok(())
}
```
