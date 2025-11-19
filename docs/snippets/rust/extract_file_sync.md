```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

    println!("{}", result.content);
    println!("Tables: {}", result.tables.len());
    println!("Metadata: {:?}", result.metadata);
    Ok(())
}
```
