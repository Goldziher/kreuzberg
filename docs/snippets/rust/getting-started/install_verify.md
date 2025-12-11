```rust
use kreuzberg::extract_file_sync;

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &Default::default())?;
    println!("Extraction successful: {}", !result.content.is_empty());
    println!("Content length: {} characters", result.content.len());
    Ok(())
}
```
