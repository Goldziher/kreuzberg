```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: Some("tesseract".to_string()),
            language: Some("eng".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("scanned.pdf", None, &config)?;
    println!("{}", result.content);
    println!("Detected languages: {:?}", result.detected_languages);
    Ok(())
}
```
