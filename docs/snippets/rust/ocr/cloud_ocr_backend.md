```rust title="Rust"
use kreuzberg::plugins::{Plugin, OcrBackend, OcrBackendType};
use kreuzberg::{Result, ExtractionResult, OcrConfig, Metadata};
use async_trait::async_trait;
use std::path::Path;

struct CloudOcrBackend {
    api_key: String,
    supported_langs: Vec<String>,
}

impl Plugin for CloudOcrBackend {
    fn name(&self) -> &str { "cloud-ocr" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl OcrBackend for CloudOcrBackend {
    async fn process_image(
        &self,
        image_bytes: &[u8],
        config: &OcrConfig,
    ) -> Result<ExtractionResult> {
        let text = self.call_cloud_api(image_bytes, &config.language).await?;

        Ok(ExtractionResult {
            content: text,
            mime_type: "text/plain".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supports_language(&self, lang: &str) -> bool {
        self.supported_langs.iter().any(|l| l == lang)
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        self.supported_langs.clone()
    }
}

impl CloudOcrBackend {
    async fn call_cloud_api(
        &self,
        image: &[u8],
        language: &str
    ) -> Result<String> {
        Ok("Extracted text".to_string())
    }
}
```
