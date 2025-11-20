```rust
use kreuzberg::{ExtractionConfig, OcrConfig, TesseractConfig};

fn main() {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            language: Some("eng+fra+deu".to_string()),
            tesseract_config: Some(TesseractConfig {
                psm: Some(6),
                oem: Some(1),
                min_confidence: Some(0.8),
                tessedit_char_whitelist: Some("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?".to_string()),
                enable_table_detection: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    println!("{:?}", config.ocr);
}
```
