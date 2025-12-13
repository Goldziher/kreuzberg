```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_chars: 500,
        max_overlap: 50,
        embedding: Some(EmbeddingConfig {
            model: "balanced".to_string(),
            normalize: true,
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file("research_paper.pdf", None, &config).await?;

if let Some(chunks) = result.chunks {
    for chunk in chunks {
        println!("Chunk {}/{}",
            chunk.metadata.chunk_index + 1,
            chunk.metadata.total_chunks
        );
        println!("Position: {}-{}",
            chunk.metadata.char_start,
            chunk.metadata.char_end
        );
        println!("Content: {}...", &chunk.content[..100.min(chunk.content.len())]);
        if let Some(embedding) = chunk.embedding {
            println!("Embedding: {} dimensions", embedding.len());
        }
    }
}
```
