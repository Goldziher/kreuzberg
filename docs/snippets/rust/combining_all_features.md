```rust
use kreuzberg::{
    extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig,
    LanguageDetectionConfig, TokenReductionConfig,
    KeywordConfig, KeywordAlgorithm
};

let config = ExtractionConfig {
    enable_quality_processing: true,

    language_detection: Some(LanguageDetectionConfig {
        enabled: true,
        detect_multiple: true,
        ..Default::default()
    }),

    token_reduction: Some(TokenReductionConfig {
        mode: "moderate".to_string(),
        preserve_markdown: true,
        ..Default::default()
    }),

    chunking: Some(ChunkingConfig {
        max_chars: 512,
        max_overlap: 50,
        embedding: Some(EmbeddingConfig {
            model: "balanced".to_string(),
            normalize: true,
            ..Default::default()
        }),
        ..Default::default()
    }),

    keywords: Some(KeywordConfig {
        algorithm: KeywordAlgorithm::Yake,
        max_keywords: 10,
        ..Default::default()
    }),

    ..Default::default()
};

let result = extract_file("document.pdf", None, &config).await?;

println!("Quality: {:.2}", result.metadata.additional.get("quality_score").unwrap());
println!("Languages: {:?}", result.detected_languages);
println!("Keywords: {:?}", result.metadata.additional.get("keywords"));
if let Some(chunks) = result.chunks {
    if let Some(first_chunk) = chunks.first() {
        if let Some(embedding) = &first_chunk.embedding {
            println!("Chunks: {} with {} dimensions", chunks.len(), embedding.len());
        }
    }
}
```
