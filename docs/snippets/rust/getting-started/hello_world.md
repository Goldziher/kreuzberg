```rust
use kreuzberg::extract_file_sync;

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &Default::default())?;
    println!("{}", result.content);
    Ok(())
}
```
