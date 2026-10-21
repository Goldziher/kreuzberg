#![cfg(feature = "api")]

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use xberg::plugins::{DocumentExtractor, Plugin};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, Result, api::create_router};

const MIME: &str = "text/prs.fallenstein.rst";
const NAME: &str = "issue-2187-image-response";
const IMAGE: &[u8] = b"fixture-image-bytes";

struct ImageFixture;

impl Plugin for ImageFixture {
    fn name(&self) -> &str {
        NAME
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for ImageFixture {
    async fn extract(&self, _input: ExtractInput, _config: &ExtractionConfig) -> Result<ExtractedDocument> {
        let mut document = ExtractedDocument::default();
        document.content = "image response fixture".to_owned();
        document.mime_type = MIME.into();
        document.images = Some(vec![
            serde_json::from_value(json!({
                "data": IMAGE, "format": "png", "image_index": 0, "description": "kept metadata"
            }))
            .expect("image fixture must deserialize"),
        ]);
        Ok(document)
    }
    fn supported_mime_types(&self) -> &[&str] {
        &[MIME]
    }
    fn priority(&self) -> i32 {
        i32::MAX
    }
}

struct FixtureGuard;
impl FixtureGuard {
    fn register() -> Self {
        xberg::plugins::register_document_extractor(Arc::new(ImageFixture)).expect("fixture registration");
        Self
    }
}
impl Drop for FixtureGuard {
    fn drop(&mut self) {
        xberg::plugins::unregister_document_extractor(NAME).expect("fixture cleanup");
    }
}

fn config(base64: bool) -> Value {
    json!({"use_cache": false, "images": {
        "extract_images": true, "run_ocr_on_images": false, "include_data_base64": base64
    }})
}

fn json_request(path: &str, include: Option<bool>, base64: bool) -> Request<Body> {
    let mut body = json!({"inputs": [{"text": "fixture", "mime_type": MIME}], "config": config(base64)});
    if let Some(include) = include {
        body["include_image_data"] = json!(include);
    }
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request")
}

async fn response_json(app: Router, request: Request<Body>, status: StatusCode) -> Value {
    let response = app.oneshot(request).await.expect("router response");
    let actual_status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response bytes");
    assert_eq!(actual_status, status, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice(&body).expect("response JSON")
}

fn assert_images(value: &Value, include: Option<bool>, base64: bool) {
    use base64::Engine;
    let document = &value["results"][0];
    assert!(
        document["content"]
            .as_str()
            .expect("content")
            .contains("image response fixture")
    );
    assert_eq!(value["results"].as_array().expect("results").len(), 1);
    let image = &document["images"][0];
    assert_eq!(image["description"], "kept metadata");
    if include == Some(false) {
        assert!(image.get("data").is_none(), "{image}");
    } else {
        assert_eq!(image["data"], json!(IMAGE));
    }
    if base64 {
        assert_eq!(
            image["data_base64"],
            base64::engine::general_purpose::STANDARD.encode(IMAGE)
        );
    } else {
        assert!(image.get("data_base64").is_none());
    }
}

#[tokio::test]
#[serial_test::serial]
async fn should_omit_image_data_only_when_requested_in_sync_json() {
    let _guard = FixtureGuard::register();
    let app = create_router(ExtractionConfig::default());
    let mut cases = 0;
    for include in [None, Some(true), Some(false)] {
        for base64 in [false, true] {
            let value = response_json(app.clone(), json_request("/extract", include, base64), StatusCode::OK).await;
            assert_images(&value, include, base64);
            cases += 1;
        }
    }
    assert_eq!(cases, 6);
}

#[tokio::test]
#[serial_test::serial]
async fn should_omit_image_data_only_when_requested_in_multipart() {
    let _guard = FixtureGuard::register();
    let app = create_router(ExtractionConfig::default());
    let mut cases = 0;
    for include in [None, Some(true), Some(false)] {
        for base64 in [false, true] {
            let mut body = format!(
                "--boundary\r\nContent-Disposition: form-data; name=\"files\"; filename=\"fixture.rst\"\r\nContent-Type: {MIME}\r\n\r\nfixture\r\n--boundary\r\nContent-Disposition: form-data; name=\"config\"\r\n\r\n{}\r\n",
                config(base64)
            );
            if let Some(include) = include {
                body.push_str(&format!(
                    "--boundary\r\nContent-Disposition: form-data; name=\"include_image_data\"\r\n\r\n{include}\r\n"
                ));
            }
            body.push_str("--boundary--\r\n");
            let request = Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", "multipart/form-data; boundary=boundary")
                .body(Body::from(body))
                .expect("multipart request");
            let value = response_json(app.clone(), request, StatusCode::OK).await;
            assert_images(&value, include, base64);
            cases += 1;
        }
    }
    assert_eq!(cases, 6);
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_image_omission_with_async_job_results() {
    let _guard = FixtureGuard::register();
    let app = create_router(ExtractionConfig::default());
    let mut cases = 0;
    for include in [None, Some(true), Some(false)] {
        for base64 in [false, true] {
            let queued = response_json(
                app.clone(),
                json_request("/extract-async", include, base64),
                StatusCode::ACCEPTED,
            )
            .await;
            let id = queued["job_id"].as_str().expect("job id");
            let completed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let request = Request::builder()
                        .uri(format!("/jobs/{id}"))
                        .body(Body::empty())
                        .expect("poll request");
                    let value = response_json(app.clone(), request, StatusCode::OK).await;
                    match value["state"].as_str() {
                        Some("completed") => break value,
                        Some("failed") => panic!("job failed: {value}"),
                        _ => tokio::time::sleep(std::time::Duration::from_millis(5)).await,
                    }
                }
            })
            .await
            .expect("job completion");
            assert_images(&completed["result"], include, base64);
            cases += 1;
        }
    }
    assert_eq!(cases, 6);
}

#[tokio::test]
#[serial_test::serial]
async fn should_reject_non_boolean_image_data_options() {
    let _guard = FixtureGuard::register();
    let app = create_router(ExtractionConfig::default());
    for invalid in [json!("false"), json!(0), Value::Null] {
        let mut body = json!({"inputs": [{"text": "fixture", "mime_type": MIME}], "config": config(false)});
        body["include_image_data"] = invalid;
        let request = Request::builder()
            .method("POST")
            .uri("/extract")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("invalid option request");
        let response = response_json(app.clone(), request, StatusCode::BAD_REQUEST).await;
        assert!(
            response["message"]
                .as_str()
                .expect("validation message")
                .contains("boolean")
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn should_omit_image_data_from_toon_responses() {
    let _guard = FixtureGuard::register();
    let app = create_router(ExtractionConfig::default());
    let mut request = json_request("/extract", Some(false), true);
    request
        .headers_mut()
        .insert("accept", "application/toon".parse().expect("accept header"));
    let response = app.oneshot(request).await.expect("TOON response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/toon");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("TOON bytes");
    let value: Value = serde_toon::from_str(std::str::from_utf8(&body).expect("TOON UTF-8")).expect("TOON document");
    assert_images(&value, Some(false), true);
}

#[tokio::test]
async fn should_document_conditional_image_data_omission_in_openapi() {
    let app = create_router(ExtractionConfig::default());
    let request = Request::builder()
        .uri("/openapi.json")
        .body(Body::empty())
        .expect("schema request");
    let value = response_json(app, request, StatusCode::OK).await;
    for path in ["/extract", "/extract-async"] {
        let description = value["paths"][path]["post"]["description"]
            .as_str()
            .expect("endpoint documentation");
        assert!(
            description.contains("include_image_data"),
            "missing request option documentation: {path}"
        );
        assert!(
            description.contains("default `true`"),
            "missing compatibility default: {path}"
        );
    }
    let image = &value["components"]["schemas"]["ExtractedImage"];
    assert!(image["properties"].get("data").is_some(), "image schema missing");
    let required = image["required"].as_array().expect("required image fields");
    assert!(
        !required.contains(&json!("data")),
        "response data is conditionally omitted"
    );
    assert!(
        required.contains(&json!("format")),
        "other required fields stay required"
    );
}

#[tokio::test]
async fn should_reject_invalid_multipart_image_data_options() {
    let app = create_router(ExtractionConfig::default());
    for invalid in ["yes", "0", ""] {
        let body = format!(
            "--boundary\r\nContent-Disposition: form-data; name=\"include_image_data\"\r\n\r\n{invalid}\r\n--boundary--\r\n"
        );
        let request = Request::builder()
            .method("POST")
            .uri("/extract")
            .header("content-type", "multipart/form-data; boundary=boundary")
            .body(Body::from(body))
            .expect("invalid multipart option");
        let response = response_json(app.clone(), request, StatusCode::BAD_REQUEST).await;
        assert!(
            response["message"]
                .as_str()
                .expect("validation message")
                .contains("include_image_data must be a boolean")
        );
    }
}
