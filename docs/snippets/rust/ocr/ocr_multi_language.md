```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: Some("eng+deu+fra".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("multilingual.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```
