```rust
use std::fs;
use kreuzberg::extract_bytes_sync;

fn main() -> kreuzberg::Result<()> {
    let data = fs::read("document.pdf")?;
    let result = extract_bytes_sync(&data, "application/pdf", &Default::default())?;

    println!("{}", result.content);
    println!("Success: true");
    println!("Content length: {} characters", result.content.len());
    Ok(())
}
```
