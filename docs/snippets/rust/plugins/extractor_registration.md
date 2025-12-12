```rust title="Rust"
use kreuzberg::plugins::registry::get_document_extractor_registry;
use std::sync::Arc;

fn register_custom_extractor() -> Result<()> {
    let extractor = Arc::new(CustomJsonExtractor);
    let registry = get_document_extractor_registry();
    registry.register(extractor, 50)?;
    Ok(())
}
```
