//! ~keep Deterministic coverage of the public OCR backend document contract.
#![cfg(all(feature = "pdf", feature = "ocr-pipeline", feature = "tokio-runtime"))]

use std::sync::Arc;

use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend};
use xberg::types::ExtractedDocument;

const TABLE: &str =
    "<table><tr><td rowspan=\"2\">A</td><td colspan=\"2\">B</td></tr><tr><td>C</td><td>D</td></tr></table>";

struct StructuredBackend;

impl Plugin for StructuredBackend {
    fn name(&self) -> &str {
        "structured-handoff-test"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl OcrBackend for StructuredBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let node = |content| serde_json::json!({"content": content, "content_layer": "body"});
        let mut result = ExtractedDocument::default();
        result.content = "Before the table.\n\nA B C D\n\nTable caption.\n\nLiteral <table> is prose.".into();
        result.document = Some(
            serde_json::from_value(serde_json::json!({
                "nodes": [
                    node(serde_json::json!({"node_type": "paragraph", "text": "Before the table."})),
                    node(serde_json::json!({"node_type": "raw_block", "format": "html", "content": TABLE})),
                    node(serde_json::json!({"node_type": "paragraph", "text": "Table caption."})),
                    node(serde_json::json!({"node_type": "paragraph", "text": "Literal <table> is prose."}))
                ]
            }))
            .unwrap(),
        );
        match config
            .backend_options
            .as_ref()
            .and_then(|options| options.get("fixture"))
            .and_then(|value| value.as_str())
        {
            Some("review-backend-validation") => return Err(xberg::XbergError::Validation { message: "Backend fixture validation failure".into(), source: None }),
            Some("review-bbox-overflow") => {
                result.metadata.additional.insert("ocr_processed_image_width".into(), serde_json::json!(1));
                result.metadata.additional.insert("ocr_processed_image_height".into(), serde_json::json!(1));
                result.document.as_mut().unwrap().nodes[1].bbox = Some(xberg::types::BoundingBox { x0: 0.0, y0: 0.0, x1: 1e308, y1: 1e308 });
            }
            Some("recursive-metadata") => recursive_metadata_fixture(&mut result, config),
            Some("review-empty") => result.document.as_mut().unwrap().nodes.clear(),
            Some("review-metadata") => {
                result.llm_usage = Some(vec![serde_json::from_value(serde_json::json!({"model": "fixture-ocr", "source": "ocr", "input_tokens": 3, "output_tokens": 4, "total_tokens": 7})).unwrap()]);
                result.metadata.additional.insert("producer_note".into(), serde_json::json!("retained"));
                result.processing_warnings.push(serde_json::from_value(serde_json::json!({"source": "fixture", "message": "Backend fixture warning"})).unwrap());
            }
            Some("review-annotations") => {
                result.annotations = Some(vec![serde_json::from_value(serde_json::json!({"annotation_type": "text", "content": "unsupported backend note", "page_number": 1})).unwrap()]);
            }
            Some("review-work-limit") => {
                result.document.as_mut().unwrap().relationships = (0..33).map(|_| serde_json::from_value(serde_json::json!({"source": 2, "target": 1, "kind": "caption"})).unwrap()).collect();
            }
            Some("review-depth-limit") => {
                let value = (0..8).fold(serde_json::Value::Null, |value, _| serde_json::Value::Array(vec![value]));
                result.metadata.additional.insert("nested_fixture".into(), value);
            }
            Some("geometry" | "review-geometry" | "review-geometry-resized") => {
                let image = image::load_from_memory(image_bytes).unwrap();
                if config.backend_options.as_ref().and_then(|options| options.get("fixture")).and_then(|value| value.as_str()).is_some_and(|fixture| fixture.starts_with("review-geometry")) {
                    assert_eq!((image.width(), image.height()), (400, 800), "fixed geometry fixture raster");
                }
                let resized = config.backend_options.as_ref().and_then(|options| options.get("fixture")).and_then(|value| value.as_str()) == Some("review-geometry-resized");
                let scale = if resized { 2 } else { 1 };
                let width = f64::from(image.width() * scale);
                let height = f64::from(image.height() * scale);
                if resized {
                    result.metadata.additional.insert("ocr_processed_image_width".into(), serde_json::json!(image.width() * scale));
                    result.metadata.additional.insert("ocr_processed_image_height".into(), serde_json::json!(image.height() * scale));
                }
                for (index, left, top, right, bottom) in [
                    (0, 0.1, 0.0, 0.8, 0.05),
                    (1, 0.2, 0.25, 0.9, 0.5),
                    (3, 0.1, 0.95, 0.8, 1.0),
                ] {
                    result.document.as_mut().unwrap().nodes[index].bbox = Some(xberg::types::BoundingBox {
                        x0: left * width, y0: top * height, x1: right * width, y1: bottom * height,
                    });
                }
                if resized || config.backend_options.as_ref().and_then(|options| options.get("fixture")).and_then(|value| value.as_str()) == Some("review-geometry") {
                    let structure = result.document.as_mut().unwrap();
                    let mut note = structure.nodes[2].clone();
                    note.content = xberg::types::NodeContent::Paragraph { text: "Unboxed OCR note.".into() };
                    structure.nodes.push(note);
                    structure.nodes[2].bbox = Some(xberg::types::BoundingBox { x0: 0.2 * width, y0: 0.55 * height, x1: 0.9 * width, y1: 0.6 * height });
                    structure.relationships.push(serde_json::from_value(serde_json::json!({"source": 2, "target": 1, "kind": "caption"})).unwrap());
                }
            }
            Some("string") => result.document = None,
            Some("asset") => result.document.as_mut().unwrap().nodes[0].content = serde_json::from_value(serde_json::json!({"node_type": "image", "image_index": 99})).unwrap(),
            Some("table-span") => result.document.as_mut().unwrap().nodes[0].content = serde_json::from_value(serde_json::json!({"node_type": "table", "grid": {"rows": 1, "cols": 1, "cells": [{"content": "A", "row": 0, "col": 0, "row_span": 0, "col_span": 1, "is_header": false}]}})).unwrap(),
            Some("limit") => result.document.as_mut().unwrap().nodes[0].content = xberg::types::NodeContent::Paragraph { text: "x".repeat(1024 * 1024) },
            Some("parent") => result.document.as_mut().unwrap().nodes[0].parent = Some(xberg::types::NodeIndex(99)),
            Some("annotation") => result.document.as_mut().unwrap().nodes[0].annotations.push(
                serde_json::from_value(
                    serde_json::json!({"start": 0, "end": 999, "kind": {"annotation_type": "bold"}}),
                )
                .unwrap(),
            ),
            Some("relationship") => result.document.as_mut().unwrap().relationships.push(
                serde_json::from_value(serde_json::json!({"source": 0, "target": 99, "kind": "caption"})).unwrap(),
            ),
            Some("relationship-valid") => result.document.as_mut().unwrap().relationships.push(
                serde_json::from_value(serde_json::json!({"source": 2, "target": 1, "kind": "caption"})).unwrap(),
            ),
            Some("format") => {
                result.document.as_mut().unwrap().nodes[1].content = xberg::types::NodeContent::RawBlock {
                    format: "unsupported".into(),
                    content: TABLE.into(),
                }
            }
            _ => {}
        }
        Ok(result)
    }
}

fn png() -> Vec<u8> {
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 16, image::Rgb([255, 255, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    png.into_inner()
}

fn json_chain(depth: usize) -> serde_json::Value {
    (1..depth).fold(serde_json::Value::Null, |value, _| {
        serde_json::Value::Array(vec![value])
    })
}

fn json_width(children: usize) -> serde_json::Value {
    serde_json::Value::Array(vec![serde_json::Value::Null; children])
}

fn add_recursive_json(result: &mut ExtractedDocument, owner: &str, value: serde_json::Value) {
    match owner {
        "additional" => {
            result.metadata.additional.insert("recursive_probe".into(), value);
        }
        "schema" => result.metadata.json_schema = Some(value),
        "element" => result
            .ocr_elements
            .get_or_insert_with(Vec::new)
            .push(xberg::types::OcrElement {
                text: "Backend metadata sample".into(),
                page_number: 1,
                backend_metadata: std::collections::HashMap::from([("recursive_probe".into(), value)]),
                ..Default::default()
            }),
        #[cfg(feature = "office")]
        "docx" => {
            result.metadata.format = Some(xberg::types::FormatMetadata::Docx(Box::new(
                xberg::types::metadata::DocxMetadata {
                    custom_properties: Some(std::collections::HashMap::from([("recursive_probe".into(), value)])),
                    ..Default::default()
                },
            )))
        }
        _ => panic!("unknown recursive fixture owner: {owner}"),
    }
}

#[cfg(feature = "tree-sitter")]
fn code_node(children: Vec<xberg::types::metadata::CodeDataNode>) -> xberg::types::metadata::CodeDataNode {
    xberg::types::metadata::CodeDataNode {
        kind: xberg::types::metadata::CodeDataNodeKind::Sequence,
        key: None,
        value: children.is_empty().then(|| "leaf".into()),
        attributes: vec![],
        children,
        byte_start: 0,
        byte_end: 1,
    }
}

fn recursive_metadata_fixture(result: &mut ExtractedDocument, config: &OcrConfig) {
    let options = config.backend_options.as_ref().unwrap();
    let owner = options["owner"].as_str().unwrap();
    let shape = options["shape"].as_str().unwrap();
    result.processing_warnings.push(
        serde_json::from_value(
            serde_json::json!({"source": "recursive-fixture", "message": "Small deterministic metadata fixture"}),
        )
        .unwrap(),
    );
    result.llm_usage = Some(vec![
        serde_json::from_value(serde_json::json!({"model": "fixture-ocr", "source": "ocr", "total_tokens": 7}))
            .unwrap(),
    ]);
    #[cfg(feature = "tree-sitter")]
    if owner == "code" {
        let data = match shape {
            "depth" => (1..8).fold(code_node(vec![]), |child, _| code_node(vec![child])),
            "depth-boundary" => (1..4).fold(code_node(vec![]), |child, _| code_node(vec![child])),
            "width" => code_node((0..33).map(|_| code_node(vec![])).collect()),
            "work-boundary" => code_node((0..27).map(|_| code_node(vec![])).collect()),
            "cross-work" => {
                add_recursive_json(result, "additional", json_width(14));
                code_node((0..14).map(|_| code_node(vec![])).collect())
            }
            "flat" => code_node(vec![]),
            _ => panic!("unknown code fixture shape"),
        };
        result.metadata.format = Some(xberg::types::FormatMetadata::Code(
            xberg::types::metadata::CodeMetadata {
                chunks: vec![],
                data: Some(data),
            },
        ));
        return;
    }
    if shape == "cross-work" {
        for root in owner.split('+') {
            add_recursive_json(result, root, json_width(14));
        }
        return;
    }
    let value = match shape {
        "depth" => json_chain(8),
        "width" => json_width(33),
        "depth-boundary" => json_chain(4),
        // ~keep Four public nodes, one JSON root and, for element metadata, its OCR element.
        "work-boundary" => json_width(if owner == "element" { 26 } else { 27 }),
        "flat" => serde_json::json!("flat"),
        _ => panic!("unknown JSON fixture shape"),
    };
    add_recursive_json(result, owner, value);
}

fn recursive_metadata_config(pipeline: bool, owner: &str, shape: &str) -> ExtractionConfig {
    let mut config = if pipeline {
        config("recursive-metadata")
    } else {
        single_backend_config("recursive-metadata")
    };
    let options = serde_json::json!({"fixture": "recursive-metadata", "owner": owner, "shape": shape});
    let ocr = config.ocr.as_mut().unwrap();
    ocr.backend_options = Some(options.clone());
    ocr.element_config = Some(xberg::types::OcrElementConfig {
        include_elements: true,
        ..Default::default()
    });
    if let Some(pipeline) = ocr.pipeline.as_mut() {
        pipeline.stages[0].backend_options = Some(options);
    }
    config.security_limits =
        Some(serde_json::from_value(serde_json::json!({"max_nesting_depth": 4, "max_iterations": 32})).unwrap());
    config
}

#[tokio::test]
async fn recursive_metadata_json_matrix_rejects_unbounded_values() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    let mut rejected = 0;
    let mut accepted_invalid = Vec::new();
    for pipeline in [false, true] {
        for owner in ["additional", "element"] {
            for shape in ["depth", "width"] {
                let output = xberg::extract(
                    xberg::ExtractInput::from_bytes(png(), "image/png", None),
                    &recursive_metadata_config(pipeline, owner, shape),
                )
                .await;
                match output {
                    Err(error) => {
                        assert!(matches!(error, xberg::XbergError::Validation { .. }), "{error}");
                        assert!(error.to_string().contains("Invalid structured OCR document"), "{error}");
                        rejected += 1;
                    }
                    Ok(_) => accepted_invalid.push(format!("pipeline={pipeline}, owner={owner}, shape={shape}")),
                }
            }
        }
    }
    assert_eq!(rejected, 8, "invalid metadata accepted: {accepted_invalid:?}");
}

#[tokio::test]
async fn recursive_metadata_json_flat_and_boundary_controls() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for pipeline in [false, true] {
        for owner in ["additional", "element"] {
            for shape in ["flat", "depth-boundary", "work-boundary"] {
                let output = xberg::extract(
                    xberg::ExtractInput::from_bytes(png(), "image/png", None),
                    &recursive_metadata_config(pipeline, owner, shape),
                )
                .await
                .unwrap();
                assert!(
                    output.errors.is_empty(),
                    "{pipeline}/{owner}/{shape}: {:?}",
                    output.errors
                );
                let result = &output.results[0];
                assert_eq!(result.content.matches(TABLE).count(), 1);
                assert_eq!(result.llm_usage.as_ref().unwrap()[0].total_tokens, Some(7));
                assert!(
                    result
                        .processing_warnings
                        .iter()
                        .any(|warning| warning.source == "recursive-fixture")
                );
                let value = if owner == "additional" {
                    &result.metadata.additional["recursive_probe"]
                } else {
                    &result
                        .ocr_elements
                        .as_ref()
                        .unwrap()
                        .iter()
                        .find(|element| element.backend_metadata.contains_key("recursive_probe"))
                        .unwrap()
                        .backend_metadata["recursive_probe"]
                };
                let expected = match shape {
                    "flat" => serde_json::json!("flat"),
                    "depth-boundary" => json_chain(4),
                    _ => json_width(if owner == "element" { 26 } else { 27 }),
                };
                assert_eq!(value, &expected, "{pipeline}/{owner}/{shape}");
            }
        }
    }
}

#[tokio::test]
async fn recursive_metadata_work_budget_is_shared_across_roots() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for pipeline in [false, true] {
        for owner in [
            "additional+schema",
            "additional+element",
            "schema+element",
            "element+element",
            #[cfg(feature = "office")]
            "additional+docx",
            #[cfg(feature = "office")]
            "docx+element",
        ] {
            let error = xberg::extract(
                xberg::ExtractInput::from_bytes(png(), "image/png", None),
                &recursive_metadata_config(pipeline, owner, "cross-work"),
            )
            .await
            .unwrap_err();
            assert!(
                matches!(error, xberg::XbergError::Validation { .. }),
                "{pipeline}/{owner}: {error}"
            );
            assert!(error.to_string().contains("Invalid structured OCR document"));
        }
    }
}

#[cfg(feature = "tree-sitter")]
#[tokio::test]
async fn recursive_metadata_code_tree_limits_and_controls() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for pipeline in [false, true] {
        for shape in ["depth", "width", "cross-work"] {
            let error = xberg::extract(
                xberg::ExtractInput::from_bytes(png(), "image/png", None),
                &recursive_metadata_config(pipeline, "code", shape),
            )
            .await
            .unwrap_err();
            assert!(
                matches!(error, xberg::XbergError::Validation { .. }),
                "{pipeline}/{shape}: {error}"
            );
            assert!(error.to_string().contains("Invalid structured OCR document"));
        }
        for shape in ["flat", "depth-boundary", "work-boundary"] {
            let output = xberg::extract(
                xberg::ExtractInput::from_bytes(png(), "image/png", None),
                &recursive_metadata_config(pipeline, "code", shape),
            )
            .await
            .unwrap();
            assert!(output.errors.is_empty(), "{pipeline}/{shape}: {:?}", output.errors);
            assert_eq!(output.results[0].content.matches(TABLE).count(), 1);
            assert_eq!(output.results[0].llm_usage.as_ref().unwrap()[0].total_tokens, Some(7));
            assert!(
                output.results[0]
                    .processing_warnings
                    .iter()
                    .any(|warning| warning.source == "recursive-fixture")
            );
        }
    }
}

fn config(fixture: &str) -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "output_format": "markdown", "use_cache": false,
        "ocr": {
            "backend": "structured-handoff-test",
            "pipeline": {"stages": [{"backend": "structured-handoff-test", "backend_options": {"fixture": fixture}}]}
        }
    }))
    .unwrap()
}

#[tokio::test]
async fn structured_ocr_handoff_string_only_remains_literal() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    let output = xberg::extract(
        xberg::ExtractInput::from_bytes(png(), "image/png", None),
        &config("string"),
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    assert!(!output.results[0].content.contains("<table>"));
    assert!(output.results[0].content.contains("A B C D"));
}

#[tokio::test]
async fn structured_ocr_handoff_malformed_payloads_fail_explicitly() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for fixture in [
        "parent",
        "annotation",
        "relationship",
        "format",
        "limit",
        "asset",
        "table-span",
    ] {
        let mut config = config(fixture);
        if fixture == "limit" {
            config.security_limits =
                Some(serde_json::from_value(serde_json::json!({"max_content_size": 512 * 1024})).unwrap());
        }
        let error = xberg::extract(xberg::ExtractInput::from_bytes(png(), "image/png", None), &config)
            .await
            .unwrap_err();
        assert!(
            matches!(error, xberg::XbergError::Validation { .. })
                && error.to_string().contains("Invalid structured OCR document"),
            "{fixture}: {error}"
        );
    }
}

fn blank_pdf() -> Vec<u8> {
    blank_pdf_size(72, 72)
}

fn blank_pdf_size(width: i64, height: i64) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut document = Document::with_version("1.5");
    let pages = document.new_object_id();
    let content = document.add_object(Stream::new(dictionary! {}, Vec::new()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        "Contents" => content, "Resources" => dictionary! {}
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1
        }),
    );
    let catalog = document.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

fn single_backend_config(fixture: &str) -> ExtractionConfig {
    let mut config = config(fixture);
    let ocr = config.ocr.as_mut().unwrap();
    ocr.pipeline = None;
    ocr.backend_options = Some(serde_json::json!({"fixture": fixture}));
    config.include_document_structure = true;
    config
}

#[tokio::test]
async fn upstream_review_metadata_and_structured_payload_validation() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for pipeline in [false, true] {
        let config = if pipeline {
            config("review-metadata")
        } else {
            single_backend_config("review-metadata")
        };
        let output = xberg::extract(xberg::ExtractInput::from_bytes(png(), "image/png", None), &config)
            .await
            .unwrap();
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        let result = &output.results[0];
        assert_eq!(result.metadata.additional.get("producer_note").unwrap(), "retained");
        assert_eq!(result.llm_usage.as_ref().unwrap().len(), 1);
        assert_eq!(result.llm_usage.as_ref().unwrap()[0].total_tokens, Some(7));
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "fixture")
        );
        assert_eq!(result.content.matches(TABLE).count(), 1);
    }
    for fixture in [
        "review-annotations",
        "review-work-limit",
        "review-depth-limit",
        "review-empty",
        "review-bbox-overflow",
    ] {
        let mut config = single_backend_config(fixture);
        if fixture == "review-work-limit" {
            config.security_limits = Some(serde_json::from_value(serde_json::json!({"max_iterations": 32})).unwrap());
        }
        if fixture == "review-depth-limit" {
            config.security_limits = Some(serde_json::from_value(serde_json::json!({"max_nesting_depth": 4})).unwrap());
        }
        let error = xberg::extract(xberg::ExtractInput::from_bytes(png(), "image/png", None), &config)
            .await
            .unwrap_err();
        assert!(matches!(error, xberg::XbergError::Validation { .. }), "{error}");
        assert!(error.to_string().contains("Invalid structured OCR document"), "{error}");
        if fixture == "review-bbox-overflow" {
            assert!(error.to_string().contains("bounding box"));
        }
    }
}

#[tokio::test]
async fn upstream_review_single_backend_image_preserves_structure() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for pipeline in [false, true] {
        let mut config = if pipeline {
            config("relationship-valid")
        } else {
            single_backend_config("relationship-valid")
        };
        config.include_document_structure = true;
        let output = xberg::extract(xberg::ExtractInput::from_bytes(png(), "image/png", None), &config)
            .await
            .unwrap();
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        let result = &output.results[0];
        assert!(result.content.contains(TABLE), "{}", result.content);
        assert_eq!(result.document.as_ref().unwrap().relationships.len(), 1);
        assert!(result.content.find("Before the table.").unwrap() < result.content.find(TABLE).unwrap());
        assert!(result.content.find(TABLE).unwrap() < result.content.find("Table caption.").unwrap());
        assert!(
            result.content.contains("Literal \\<table\\> is prose.")
                || result.content.contains("Literal &lt;table&gt; is prose.")
        );
    }
}

#[tokio::test]
async fn upstream_review_fallback_distinguishes_backend_failure_from_invalid_structure() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for fixture in ["review-backend-validation", "review-annotations"] {
        let mut config = config("review-metadata");
        config.ocr.as_mut().unwrap().pipeline.as_mut().unwrap().stages.insert(0,
            serde_json::from_value(serde_json::json!({"backend": "structured-handoff-test", "priority": 200, "backend_options": {"fixture": fixture}})).unwrap());
        let output = xberg::extract(xberg::ExtractInput::from_bytes(png(), "image/png", None), &config).await;
        if fixture == "review-backend-validation" {
            let output = output.unwrap();
            assert!(output.errors.is_empty(), "{:?}", output.errors);
            assert_eq!(output.results[0].content.matches(TABLE).count(), 1);
        } else {
            let error = output.unwrap_err();
            assert!(matches!(error, xberg::XbergError::Validation { .. }), "{error}");
            assert!(error.to_string().contains("Invalid structured OCR document"));
        }
    }
}

#[tokio::test]
async fn upstream_review_image_envelope_survives_single_and_pipeline_handoff() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for pipeline in [false, true] {
        let mut config = if pipeline {
            config("relationship-valid")
        } else {
            single_backend_config("relationship-valid")
        };
        config.include_document_structure = true;
        config.images = Some(serde_json::from_value(serde_json::json!({"extract_images": true})).unwrap());
        let bytes = png();
        let output = xberg::extract(
            xberg::ExtractInput::from_bytes(bytes.clone(), "image/png", None),
            &config,
        )
        .await
        .unwrap();
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        let result = &output.results[0];
        let structure = result.document.as_ref().unwrap();
        assert_eq!(structure.source_format.as_deref(), Some("image"), "pipeline={pipeline}");
        let images = result.images.as_ref().unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].data.as_ref(), bytes);
        assert!(
            result.content.contains(&format!("image_0.{}", images[0].format)),
            "source link absent, pipeline={pipeline}: {}",
            result.content
        );
        assert!(
            structure.nodes.iter().any(|node| matches!(
                node.content,
                xberg::types::NodeContent::Image {
                    image_index: Some(0),
                    ..
                }
            )),
            "source image reference absent, pipeline={pipeline}"
        );
    }
}

#[tokio::test]
async fn upstream_review_detached_mixed_pipeline_preserves_pdf_frame() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    for (pipeline, forced) in [(true, false), (false, false), (true, true), (false, true)] {
        for fixture in ["review-geometry", "review-geometry-resized"] {
            let mut config = if pipeline {
                config(fixture)
            } else {
                single_backend_config(fixture)
            };
            if forced {
                config.force_ocr = true;
            } else {
                config.force_ocr_pages = Some(vec![1]);
            }
            config.include_document_structure = true;
            config.pdf_options = Some(
                serde_json::from_value(serde_json::json!({"top_margin_fraction": 0.1, "bottom_margin_fraction": 0.1}))
                    .unwrap(),
            );
            // A fixed test raster, not an OCR quality/DPI trial: 200x400 pt at 144 DPI.
            config.images = Some(serde_json::from_value(serde_json::json!({"extract_images": false, "target_dpi": 144, "auto_adjust_dpi": false, "min_dpi": 144, "max_dpi": 144})).unwrap());
            let output = xberg::extract(
                xberg::ExtractInput::from_bytes(blank_pdf_size(200, 400), "application/pdf", None),
                &config,
            )
            .await
            .unwrap();
            assert!(output.errors.is_empty(), "{:?}", output.errors);
            let result = &output.results[0];
            assert_eq!(
                result.extraction_method,
                Some(if forced {
                    xberg::types::ExtractionMethod::Ocr
                } else {
                    xberg::types::ExtractionMethod::Mixed
                })
            );
            assert_eq!(result.content.matches(TABLE).count(), 1);
            let structure = result.document.as_ref().unwrap();
            let table = structure
                .nodes
                .iter()
                .find(|node| matches!(node.content, xberg::types::NodeContent::RawBlock { .. }))
                .unwrap();
            let bbox = table.bbox.unwrap();
            for (actual, expected) in [(bbox.x0, 40.0), (bbox.y0, 200.0), (bbox.x1, 180.0), (bbox.y1, 300.0)] {
                assert!((actual - expected).abs() < 0.001, "{bbox:?}");
            }
            assert!(!result.content.contains("Before the table.") && !result.content.contains("Literal"));
            let caption = structure.nodes.iter().find(|node| matches!(&node.content, xberg::types::NodeContent::Paragraph { text } if text == "Table caption.")).unwrap();
            let bbox = caption.bbox.unwrap();
            for (actual, expected) in [(bbox.x0, 40.0), (bbox.y0, 160.0), (bbox.x1, 180.0), (bbox.y1, 180.0)] {
                assert!(
                    (actual - expected).abs() < 0.001,
                    "pipeline={pipeline}, forced={forced}: {bbox:?}"
                );
            }
            let note = structure.nodes.iter().find(|node| matches!(&node.content, xberg::types::NodeContent::Paragraph { text } if text == "Unboxed OCR note.")).unwrap();
            assert!(note.bbox.is_none());
            assert_eq!(structure.relationships.len(), 1);
            assert!(matches!(
                structure.nodes[structure.relationships[0].target.0 as usize].content,
                xberg::types::NodeContent::RawBlock { .. }
            ));
        }
    }
}

#[tokio::test]
async fn structured_ocr_handoff_pdf_supplied_geometry_and_missing_caption_geometry_obey_margins() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    let mut config = config("geometry");
    config.force_ocr = true;
    config.include_document_structure = true;
    config.pdf_options = Some(
        serde_json::from_value(serde_json::json!({
            "top_margin_fraction": 0.1, "bottom_margin_fraction": 0.1
        }))
        .unwrap(),
    );
    let output = xberg::extract(
        xberg::ExtractInput::from_bytes(blank_pdf(), "application/pdf", None),
        &config,
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let result = &output.results[0];
    assert_eq!(result.content.matches(TABLE).count(), 1, "{}", result.content);
    assert!(!result.content.contains("Before the table.") && !result.content.contains("Literal"));
    assert!(result.content.contains("Table caption."));
    let nodes = &result.document.as_ref().unwrap().nodes;
    let table = nodes
        .iter()
        .find(|node| matches!(node.content, xberg::types::NodeContent::RawBlock { .. }))
        .unwrap();
    let bbox = table.bbox.unwrap();
    for (actual, expected) in [(bbox.x0, 14.4), (bbox.y0, 36.0), (bbox.x1, 64.8), (bbox.y1, 54.0)] {
        assert!((actual - expected).abs() < 0.000001, "{bbox:?}");
    }
    let caption = nodes
        .iter()
        .find(|node| matches!(&node.content, xberg::types::NodeContent::Paragraph { text } if text == "Table caption."))
        .unwrap();
    assert!(caption.bbox.is_none(), "the pipeline must not invent caption geometry");
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("margin"))
    );
}

#[tokio::test]
async fn structured_ocr_handoff_pdf_preserves_raw_table_once() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    let mut config = config("structured");
    config.force_ocr = true;
    let output = xberg::extract(
        xberg::ExtractInput::from_bytes(blank_pdf(), "application/pdf", None),
        &config,
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let content = &output.results[0].content;
    assert!(content.contains(TABLE), "{content}");
    assert_eq!(content.matches("<table>").count(), 1, "{content}");
    assert!(content.find("Before the table.").unwrap() < content.find(TABLE).unwrap());
    assert!(content.find(TABLE).unwrap() < content.find("Table caption.").unwrap());
}

#[tokio::test]
async fn structured_ocr_handoff_image_preserves_raw_table_and_literal_prose() {
    register_ocr_backend(Arc::new(StructuredBackend)).unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 16, image::Rgb([255, 255, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "output_format": "markdown",
        "include_document_structure": true,
        "use_cache": false,
        "ocr": {
            "backend": "structured-handoff-test",
            "pipeline": {"stages": [{"backend": "structured-handoff-test", "backend_options": {"fixture": "relationship-valid"}}]}
        }
    }))
    .unwrap();
    assert_eq!(config.output_format, OutputFormat::Markdown);
    let output = xberg::extract(
        xberg::ExtractInput::from_bytes(png.into_inner(), "image/png", None),
        &config,
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let result = &output.results[0];
    let structure = result.document.as_ref().unwrap();
    assert_eq!(structure.relationships.len(), 1);
    let relationship = &structure.relationships[0];
    assert!(matches!(
        structure.nodes[relationship.target.0 as usize].content,
        xberg::types::NodeContent::RawBlock { .. }
    ));
    assert!(result.content.contains(TABLE), "{}", result.content);
    assert_eq!(result.content.matches("<table>").count(), 1, "{}", result.content);
    let before = result.content.find("Before the table.").unwrap();
    let table = result.content.find(TABLE).unwrap();
    let caption = result.content.find("Table caption.").unwrap();
    let literal = result.content.find("Literal").unwrap();
    assert!(before < table && table < caption && caption < literal);
    assert!(
        result.content.contains("Literal \\<table\\> is prose.")
            || result.content.contains("Literal &lt;table&gt; is prose."),
        "{}",
        result.content
    );
}
