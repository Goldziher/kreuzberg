```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

    for table in &result.tables {
        println!("Table with {} rows", table.cells.len());
        println!("{}", table.markdown);

        for row in &table.cells {
            println!("{:?}", row);
        }
    }
    Ok(())
}
```
