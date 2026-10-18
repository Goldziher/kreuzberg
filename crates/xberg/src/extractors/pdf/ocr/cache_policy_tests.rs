use super::pipeline::extract_with_ocr;
use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use std::sync::{Arc, Mutex};

struct CapturingBackend {
    seen: Mutex<Vec<OcrConfig>>,
}

impl Plugin for CapturingBackend {
    fn name(&self) -> &str {
        "pdf-cache-policy-capture"
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for CapturingBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.seen.lock().unwrap().push(config.clone());
        Ok(ExtractedDocument::default())
    }
}

#[tokio::test]
#[serial_test::serial]
async fn pdf_cache_policy_should_forward_opt_out_to_the_executing_backend_only() {
    let backend = Arc::new(CapturingBackend {
        seen: Mutex::new(Vec::new()),
    });
    crate::plugins::register_ocr_backend(backend.clone()).unwrap();
    let images = [image::DynamicImage::new_rgb8(4, 4)];
    for (use_cache, explicit_pipeline) in [(true, false), (false, false), (true, true), (false, true)] {
        let config = ExtractionConfig {
            use_cache,
            ocr: Some(OcrConfig {
                backend: backend.name().to_string(),
                backend_options: Some(serde_json::json!({"custom": 7})),
                pipeline: explicit_pipeline.then(|| {
                    serde_json::from_value(serde_json::json!({
                        "stages": [{"backend": backend.name(), "backend_options": {"custom": 7}}]
                    }))
                    .unwrap()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        extract_with_ocr(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            &config,
            None,
        )
        .await
        .unwrap();
    }
    crate::plugins::unregister_ocr_backend(backend.name()).unwrap();
    let seen = backend.seen.lock().unwrap();
    assert_eq!(seen.len(), 4);
    assert_eq!(seen[2].backend_options, seen[0].backend_options);
    assert_eq!(seen[3].backend_options, seen[1].backend_options);
    assert_eq!(seen[0].backend_options, Some(serde_json::json!({"custom": 7})));
    assert_eq!(
        seen[1].backend_options,
        Some(serde_json::json!({"custom": 7, "use_cache": false}))
    );
}

#[test]
fn pdf_cache_policy_should_preserve_implicit_ocr_routing_and_defaults() {
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let resolved = super::resolve_ocr_config(&config);
    assert!(config.ocr.is_none());
    assert!(resolved.tesseract_config.is_none());
    assert_eq!(resolved.backend_options, Some(serde_json::json!({"use_cache": false})));
    assert!(!super::super::near_empty_ocr_fallback_applies(&config, "Page 3 of 412"));
}
