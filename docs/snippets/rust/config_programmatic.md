```rust
use kreuzberg::{extract_file_sync, ChunkingConfig, ExtractionConfig, OcrConfig, TesseractConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ExtractionConfig {
        use_cache: Some(true),
        ocr: Some(OcrConfig {
            backend: Some("tesseract".into()),
            language: Some("eng+deu".into()),
            tesseract: Some(TesseractConfig {
                psm: Some(6),
                ..Default::default()
            }),
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_chars: Some(1000),
            max_overlap: Some(200),
            ..Default::default()
        }),
        enable_quality_processing: Some(true),
        ..Default::default()
    };

    let result = extract_file_sync("document.pdf", None::<&str>, Some(config))?;
    println!("Content length: {}", result.content.len());
    Ok(())
}
```
