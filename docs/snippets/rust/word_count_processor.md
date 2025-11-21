```rust
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
use async_trait::async_trait;

struct WordCountProcessor;

impl Plugin for WordCountProcessor {
    fn name(&self) -> &str { "word-count" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl PostProcessor for WordCountProcessor {
    async fn process(
        &self,
        result: &mut ExtractionResult,
        _config: &ExtractionConfig
    ) -> Result<()> {
        let word_count = result.content.split_whitespace().count();

        result.metadata.additional.insert(
            "word_count".to_string(),
            serde_json::json!(word_count)
        );

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }

    fn should_process(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig
    ) -> bool {
        // Only process if content is non-empty
        !result.content.is_empty()
    }
}
```
