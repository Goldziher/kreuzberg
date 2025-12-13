```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig, PdfConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            dpi: Some(300),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("scanned.pdf", None, &config)?;
    Ok(())
}
```
