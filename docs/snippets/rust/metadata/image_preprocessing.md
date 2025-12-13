```rust title="Rust"
use kreuzberg::{ExtractionConfig, ImagePreprocessingConfig, OcrConfig, TesseractConfig};

fn main() {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                preprocessing: Some(ImagePreprocessingConfig {
                    target_dpi: Some(300),
                    denoise: Some(true),
                    deskew: Some(true),
                    contrast_enhance: Some(true),
                    binarization_method: Some("otsu".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    println!("{:?}", config.ocr);
}
```
