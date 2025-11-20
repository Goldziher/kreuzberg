```rust
use kreuzberg::{
    extract_file_sync, ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, OcrConfig,
};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        // Enable OCR
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: Some("eng+deu".to_string()), // Multiple languages
            ..Default::default()
        }),

        // Enable chunking for LLM processing
        chunking: Some(ChunkingConfig {
            max_chunk_size: 1000,
            overlap: 100,
        }),

        // Enable language detection
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            detect_multiple: true,
            ..Default::default()
        }),

        // Enable caching
        use_cache: true,

        // Enable quality processing
        enable_quality_processing: true,

        ..Default::default()
    };

    let result = extract_file_sync("document.pdf", None, &config)?;

    // Access chunks
    if let Some(chunks) = result.chunks {
        for chunk in chunks {
            println!("Chunk: {}...", &chunk[..100.min(chunk.len())]);
        }
    }

    // Access detected languages
    if let Some(languages) = result.detected_languages {
        println!("Languages: {:?}", languages);
    }
    Ok(())
}
```
