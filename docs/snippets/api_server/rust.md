```rust
use kreuzberg::{ExtractionConfig, api::serve_with_config};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::discover()?;
    serve_with_config("0.0.0.0", 8000, config).await?;
    Ok(())
}

```
