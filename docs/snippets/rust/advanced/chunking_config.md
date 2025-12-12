```rust title="Rust"
use kreuzberg::{ExtractionConfig, ChunkingConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_chars: 1000,
        max_overlap: 200,
        embedding: None,
    }),
    ..Default::default()
};
```
