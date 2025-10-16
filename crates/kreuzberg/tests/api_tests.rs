//! Integration tests for the API module.

#![cfg(feature = "api")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt; // for `oneshot`

use kreuzberg::api::{HealthResponse, InfoResponse, create_router};

/// Test the health check endpoint.
#[tokio::test]
async fn test_health_endpoint() {
    let app = create_router();

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let health: HealthResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(health.status, "healthy");
    assert!(!health.version.is_empty());
}

/// Test the info endpoint.
#[tokio::test]
async fn test_info_endpoint() {
    let app = create_router();

    let response = app
        .oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let info: InfoResponse = serde_json::from_slice(&body).unwrap();

    assert!(!info.version.is_empty());
    assert!(info.rust_backend);
}

/// Test extract endpoint with no files returns 400.
#[tokio::test]
async fn test_extract_no_files() {
    let app = create_router();

    // Create an empty multipart body
    let boundary = "----boundary";
    let body_content = format!("--{}--\r\n", boundary);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test extract endpoint with a simple text file.
#[tokio::test]
async fn test_extract_text_file() {
    let app = create_router();

    let boundary = "----boundary";
    let file_content = "Hello, world!";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, file_content, boundary
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["mime_type"], "text/plain");
    assert!(results[0]["content"].as_str().unwrap().contains("Hello, world!"));
}

/// Test extract endpoint with JSON config.
#[tokio::test]
async fn test_extract_with_config() {
    let app = create_router();

    let boundary = "----boundary";
    let file_content = "Hello, world!";
    let config = json!({
        "force_ocr": false
    });

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}\r\n\
         Content-Disposition: form-data; name=\"config\"\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary,
        file_content,
        boundary,
        config.to_string(),
        boundary
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test extract endpoint with invalid config returns 400.
#[tokio::test]
async fn test_extract_invalid_config() {
    let app = create_router();

    let boundary = "----boundary";
    let file_content = "Hello, world!";
    let invalid_config = "not valid json";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}\r\n\
         Content-Disposition: form-data; name=\"config\"\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, file_content, boundary, invalid_config, boundary
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Test extract endpoint with multiple files.
#[tokio::test]
async fn test_extract_multiple_files() {
    let app = create_router();

    let boundary = "----boundary";
    let file1_content = "First file content";
    let file2_content = "Second file content";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test1.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test2.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, file1_content, boundary, file2_content, boundary
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                .body(Body::from(body_content))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0]["content"].as_str().unwrap().contains("First file"));
    assert!(results[1]["content"].as_str().unwrap().contains("Second file"));
}
