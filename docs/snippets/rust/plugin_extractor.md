```rust
use kreuzberg::plugins::{DocumentExtractor, Plugin};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig, Metadata};
use async_trait::async_trait;
use std::path::Path;

struct CustomJsonExtractor;

impl Plugin for CustomJsonExtractor {
    fn name(&self) -> &str { "custom-json-extractor" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl DocumentExtractor for CustomJsonExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let json: serde_json::Value = serde_json::from_slice(content)?;
        let text = extract_text_from_json(&json);

        Ok(ExtractionResult {
            content: text,
            mime_type: "application/json".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/json", "text/json"]
    }

    fn priority(&self) -> i32 { 50 }
}

fn extract_text_from_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("{}\n", s),
        serde_json::Value::Array(arr) => arr.iter().map(extract_text_from_json).collect(),
        serde_json::Value::Object(obj) => obj.values().map(extract_text_from_json).collect(),
        _ => String::new(),
    }
}
```
