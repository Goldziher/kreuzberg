```rust
use kreuzberg::{
    extract_file_sync, ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, OcrConfig,
};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: Some("eng+deu".to_string()),
            ..Default::default()
        }),

        chunking: Some(ChunkingConfig {
            max_chars: 1000,
            max_overlap: 100,
            ..Default::default()
        }),

        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            detect_multiple: true,
            ..Default::default()
        }),

        use_cache: true,
        enable_quality_processing: true,

        ..Default::default()
    };

    let result = extract_file_sync("document.pdf", None, &config)?;

    if let Some(chunks) = result.chunks {
        for chunk in chunks {
            println!("Chunk: {}...", &chunk[..100.min(chunk.len())]);
        }
    }

    if let Some(languages) = result.detected_languages {
        println!("Languages: {:?}", languages);
    }
    Ok(())
}
```
