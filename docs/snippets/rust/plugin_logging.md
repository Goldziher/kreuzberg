```rust
use log::{info, warn, error};

impl Plugin for MyPlugin {
    fn initialize(&self) -> Result<()> {
        info!("Initializing plugin: {}", self.name());
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        info!("Shutting down plugin: {}", self.name());
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for MyPlugin {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        info!("Extracting {} ({} bytes)", mime_type, content.len());

        let result = ExtractionResult::default();

        if result.content.is_empty() {
            warn!("Extraction resulted in empty content");
        }

        Ok(result)
    }
}
```
