```rust title="Rust"
use kreuzberg::{batch_extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let files = vec!["doc1.pdf", "doc2.docx", "doc3.pptx"];
    let config = ExtractionConfig::default();

    let results = batch_extract_file_sync(&files, None, &config)?;

    for (i, result) in results.iter().enumerate() {
        println!("File {}: {} characters", i + 1, result.content.len());
    }
    Ok(())
}
```
