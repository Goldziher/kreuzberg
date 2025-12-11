```rust
use kreuzberg::extract_file_sync;

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &Default::default())?;

    println!("Extracted content: {}", result.content);
    println!("Tables found: {}", result.tables.len());
    println!("Format: {:?}", result.metadata.map(|m| m.format_type));
    Ok(())
}
```
