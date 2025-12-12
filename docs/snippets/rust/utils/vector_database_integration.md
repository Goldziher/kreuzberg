```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_chars: 512,
        max_overlap: 50,
        embedding: Some(EmbeddingConfig {
            model: kreuzberg::EmbeddingModelType::Preset { name: "balanced".to_string() },
            normalize: true,
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file("document.pdf", None, &config).await?;

if let Some(chunks) = result.chunks {
    for (i, chunk) in chunks.iter().enumerate() {
        if let Some(embedding) = &chunk.embedding {
            println!("Chunk {}: {} dimensions", i, embedding.len());
        }
    }
}
```
