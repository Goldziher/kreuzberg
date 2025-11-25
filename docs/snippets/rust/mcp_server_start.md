```rust
use kreuzberg::{ExtractionConfig, mcp::start_mcp_server_with_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ExtractionConfig::discover()?;
    start_mcp_server_with_config(config).await?;
    Ok(())
}
```
