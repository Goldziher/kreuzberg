```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, OcrConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "paddleocr".to_string(),
            language: Some("en".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file("document.pdf", None, &config).await?;
    println!("Extracted text: {}", result.content);
    Ok(())
}
```
