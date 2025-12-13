```rust title="Rust"
use kreuzberg::{batch_extract_bytes_sync, ExtractionConfig};
use std::fs;

fn main() -> kreuzberg::Result<()> {
    let files = vec!["doc1.pdf", "doc2.docx"];

    let data_list: Vec<Vec<u8>> = files.iter()
        .map(|f| fs::read(f).expect("read file"))
        .collect();

    let mime_types: Vec<&str> = files.iter()
        .map(|f| if f.ends_with(".pdf") {
            "application/pdf"
        } else {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        })
        .collect();

    let results = batch_extract_bytes_sync(
        &data_list,
        &mime_types,
        &ExtractionConfig::default()
    )?;

    for (i, result) in results.iter().enumerate() {
        println!("Document {}: {} characters", i + 1, result.content.len());
    }
    Ok(())
}
```
