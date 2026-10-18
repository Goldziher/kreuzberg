use crate::core::config::{ExtractionConfig, OcrConfig};
use std::borrow::Cow;

pub(in crate::extractors::pdf) fn resolve_ocr_config(config: &ExtractionConfig) -> Cow<'_, OcrConfig> {
    let mut resolved = match config.ocr.as_ref() {
        Some(ocr) => Cow::Borrowed(ocr),
        None => Cow::Owned(OcrConfig::default()),
    };
    // Forward the outer opt-out only after routing: materializing config.ocr changes the
    // near-empty native-text fallback decision. Backend options also preserve automatic
    // Tesseract defaults, which an explicit tesseract_config would otherwise replace. ~keep
    if !config.use_cache {
        let options = &mut resolved.to_mut().backend_options;
        let value = options.get_or_insert_with(|| serde_json::json!({}));
        if !value.is_object() {
            *value = serde_json::json!({});
        }
        value["use_cache"] = serde_json::Value::Bool(false);
    }
    resolved
}
