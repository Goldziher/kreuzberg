```rust
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use kreuzberg::KreuzbergError;

struct StatefulPlugin {
    // Use atomic types for simple counters
    call_count: AtomicUsize,

    // Use Mutex for complex state
    cache: Mutex<HashMap<String, String>>,
}

impl Plugin for StatefulPlugin {
    fn name(&self) -> &str { "stateful-plugin" }
    fn version(&self) -> String { "1.0.0".to_string() }

    fn initialize(&self) -> Result<()> {
        self.call_count.store(0, Ordering::Release);
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        let count = self.call_count.load(Ordering::Acquire);
        println!("Plugin called {} times", count);
        Ok(())
    }
}

#[async_trait]
impl PostProcessor for StatefulPlugin {
    async fn process(
        &self,
        result: &mut ExtractionResult,
        _config: &ExtractionConfig
    ) -> Result<()> {
        // Increment counter atomically
        self.call_count.fetch_add(1, Ordering::AcqRel);

        // Access cache with proper error handling
        let mut cache = self.cache.lock()
            .map_err(|_| KreuzbergError::plugin("Cache lock poisoned"))?;
        cache.insert("last_mime".to_string(), result.mime_type.clone());

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}
```
