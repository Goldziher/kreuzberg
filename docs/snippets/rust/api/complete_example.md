```rust
use kreuzberg::{
    extract_file, ExtractionConfig, OcrConfig, TesseractConfig, ImagePreprocessingConfig,
    PdfConfig, ImageExtractionConfig, ChunkingConfig, TokenReductionConfig,
    LanguageDetectionConfig, PostProcessorConfig,
};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        force_ocr: false,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng+fra".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: 3,
                oem: 3,
                min_confidence: 0.8,
                preprocessing: Some(ImagePreprocessingConfig {
                    target_dpi: 300,
                    denoise: true,
                    deskew: true,
                    contrast_enhance: true,
                    ..Default::default()
                }),
                enable_table_detection: true,
                ..Default::default()
            }),
        }),
        pdf_options: Some(PdfConfig {
            extract_images: true,
            extract_metadata: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            target_dpi: 150,
            max_image_dimension: 4096,
            ..Default::default()
        }),
        chunking: Some(ChunkingConfig {
            max_chars: 1000,
            max_overlap: 200,
            ..Default::default()
        }),
        token_reduction: Some(TokenReductionConfig {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        }),
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        }),
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file("document.pdf", None, &config).await?;
    println!("Extracted content length: {}", result.content.len());
    Ok(())
}
```
