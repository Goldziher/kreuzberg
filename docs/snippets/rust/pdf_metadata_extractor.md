```rust
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

struct PdfMetadataExtractor {
    processed_count: AtomicUsize,
}

impl PdfMetadataExtractor {
    fn new() -> Self {
        Self {
            processed_count: AtomicUsize::new(0),
        }
    }
}

impl Plugin for PdfMetadataExtractor {
    fn name(&self) -> &str { "pdf-metadata-extractor" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn description(&self) -> &str {
        "Extracts and enriches PDF metadata"
    }
    fn initialize(&self) -> Result<()> {
        log::info!("PDF metadata extractor initialized");
        Ok(())
    }
    fn shutdown(&self) -> Result<()> {
        let count = self.processed_count.load(Ordering::Acquire);
        log::info!("Processed {} PDFs", count);
        Ok(())
    }
}

#[async_trait]
impl PostProcessor for PdfMetadataExtractor {
    async fn process(
        &self,
        result: &mut ExtractionResult,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        self.processed_count.fetch_add(1, Ordering::AcqRel);

        // Extract PDF-specific metadata
        result.metadata.additional.insert(
            "pdf_processed".to_string(),
            serde_json::json!(true)
        );

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }

    fn should_process(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig,
    ) -> bool {
        result.mime_type == "application/pdf"
    }

    fn estimated_duration_ms(&self, _result: &ExtractionResult) -> u64 {
        10  // Fast operation
    }
}

// Registration
use kreuzberg::plugins::registry::get_post_processor_registry;
use std::sync::Arc;

fn register() -> Result<()> {
    let processor = Arc::new(PdfMetadataExtractor::new());
    let registry = get_post_processor_registry();
    registry.register(processor, 50)?;  // Default priority
    Ok(())
}
```
