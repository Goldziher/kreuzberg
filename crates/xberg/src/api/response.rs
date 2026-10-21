use crate::core::config::ExtractionResult;
use crate::types::ExtractedDocument;

pub(super) fn serialize_results(
    mut results: ExtractionResult,
    include_image_data: bool,
) -> Result<serde_json::Value, serde_json::Error> {
    if !include_image_data {
        for document in &mut results.results {
            clear_response_image_bytes(document);
        }
    }
    let mut value = serde_json::to_value(results)?;
    if !include_image_data && let Some(documents) = value.get_mut("results").and_then(serde_json::Value::as_array_mut) {
        for document in documents {
            omit_image_data_fields(document);
        }
    }
    Ok(value)
}

fn clear_response_image_bytes(document: &mut ExtractedDocument) {
    if let Some(images) = document.images.as_mut() {
        for image in images {
            // Only the completed response owns this handle; shared cache/OCR bytes stay intact.
            // Clear before creating Value so omitted byte arrays are never materialized. ~keep
            image.data = bytes::Bytes::new();
            if let Some(ocr) = image.ocr_result.as_mut() {
                clear_response_image_bytes(ocr);
            }
        }
    }
    if let Some(children) = document.children.as_mut() {
        for child in children {
            clear_response_image_bytes(&mut child.result);
        }
    }
}

fn omit_image_data_fields(document: &mut serde_json::Value) {
    if let Some(images) = document.get_mut("images").and_then(serde_json::Value::as_array_mut) {
        for image in images {
            if let Some(image) = image.as_object_mut() {
                image.remove("data");
                if let Some(ocr) = image.get_mut("ocr_result") {
                    omit_image_data_fields(ocr);
                }
            }
        }
    }
    if let Some(children) = document.get_mut("children").and_then(serde_json::Value::as_array_mut) {
        for child in children {
            if let Some(result) = child.get_mut("result") {
                omit_image_data_fields(result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ArchiveEntry, ExtractedImage};
    use serde_json::json;

    fn document_with_image() -> ExtractedDocument {
        let mut document = ExtractedDocument {
            content: "retained content".to_owned(),
            ..Default::default()
        };
        document
            .metadata
            .additional
            .insert("images".into(), json!([{"data": "unrelated metadata"}]));
        document.images = Some(vec![
            serde_json::from_value::<ExtractedImage>(json!({
                "data": [1, 2, 3], "format": "png", "image_index": 0,
                "data_base64": "AQID", "caption": "retained caption"
            }))
            .expect("image fixture"),
        ]);
        document
    }

    #[test]
    fn should_keep_default_serialization_identical() {
        let results = ExtractionResult {
            results: vec![document_with_image()],
            ..Default::default()
        };
        assert_eq!(
            serialize_results(results.clone(), true).expect("serialized"),
            serde_json::to_value(&results).expect("baseline")
        );
    }

    #[test]
    fn should_remove_only_image_data_from_nested_documents() {
        let mut document = document_with_image();
        document.images.as_mut().expect("images")[0].ocr_result = Some(Box::new(document_with_image()));
        document.children = Some(vec![ArchiveEntry {
            path: "child.rst".to_owned(),
            mime_type: "text/plain".to_owned(),
            result: Box::new(document_with_image()),
        }]);
        let results = ExtractionResult {
            results: vec![document],
            crawl_final_urls: vec!["https://example.test/final".to_owned()],
            crawl_redirect_count: 2,
            ..Default::default()
        };
        let mut expected = serde_json::to_value(&results).expect("baseline");
        for path in [
            "/results/0/images/0",
            "/results/0/images/0/ocr_result/images/0",
            "/results/0/children/0/result/images/0",
        ] {
            let image = expected
                .pointer_mut(path)
                .expect("image path")
                .as_object_mut()
                .expect("image object");
            assert_eq!(image.remove("data"), Some(json!([1, 2, 3])));
        }
        assert_eq!(serialize_results(results, false).expect("serialized"), expected);
    }

    #[test]
    fn should_preserve_shared_source_bytes_when_omitting_response_data() {
        let document = document_with_image();
        let source = document.images.as_ref().expect("images")[0].data.clone();
        let results = ExtractionResult {
            results: vec![document],
            ..Default::default()
        };
        let cached = results.clone();
        let output = serialize_results(results, false).expect("serialized");
        assert!(output["results"][0]["images"][0].get("data").is_none());
        assert_eq!(source.as_ref(), &[1, 2, 3]);
        assert_eq!(
            cached.results[0].images.as_ref().expect("cached images")[0]
                .data
                .as_ref(),
            &[1, 2, 3]
        );
        assert_eq!(
            serde_json::to_value(cached).expect("cached JSON")["results"][0]["images"][0]["data"],
            json!([1, 2, 3])
        );
    }
}
