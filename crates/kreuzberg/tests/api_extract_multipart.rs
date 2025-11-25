#![cfg(feature = "api")]
//! Integration test for the `/extract` API handler using multipart uploads.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use kreuzberg::{
    ExtractionConfig,
    api::{ApiSizeLimits, create_router_with_limits},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn test_extract_accepts_single_file_multipart() {
    let router = create_router_with_limits(ExtractionConfig::default(), ApiSizeLimits::from_mb(5, 5));

    let boundary = "X-BOUNDARY";
    let body = format!(
        "--{boundary}\r\n\
Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
Hello world\r\n\
--{boundary}--\r\n"
    );
    let body_bytes = body.into_bytes();

    let request = Request::builder()
        .method("POST")
        .uri("/extract")
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .header("content-length", body_bytes.len())
        .body(Body::from(body_bytes))
        .expect("Failed to build request");

    let response = router.oneshot(request).await.expect("Request failed");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 1_000_000)
        .await
        .expect("Failed to read body");
    let value: Value = serde_json::from_slice(&bytes).expect("Response JSON parse failed");
    let content = value
        .get(0)
        .and_then(|v| v.get("content"))
        .and_then(Value::as_str)
        .expect("Response should include extracted content");

    assert_eq!(content, "Hello world");
}
