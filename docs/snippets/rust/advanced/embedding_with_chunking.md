```rust title="Rust"
use kreuzberg::{ExtractionConfig, ChunkingConfig, EmbeddingConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_chars: 1024,
        max_overlap: 100,
        embedding: Some(EmbeddingConfig {
            model: "balanced".to_string(),
            normalize: true,
            batch_size: 32,
            show_download_progress: false,
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};
```
