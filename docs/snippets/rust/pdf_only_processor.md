```rust
impl PostProcessor for PdfOnlyProcessor {
    async fn process(
        &self,
        result: &mut ExtractionResult,
        _config: &ExtractionConfig
    ) -> Result<()> {
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }

    fn should_process(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig
    ) -> bool {
        result.mime_type == "application/pdf"
    }
}
```
