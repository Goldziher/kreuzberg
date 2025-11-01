//! Integration tests for the API module.

#![cfg(feature = "api")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use kreuzberg::{
    ExtractionConfig,
    api::{HealthResponse, InfoResponse, create_router},
};

/// Test the health check endpoint.
#[tokio::test]
async fn test_health_endpoint() {
    let app = create_router(ExtractionConfig::default());

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
    let app = create_router(ExtractionConfig::default());

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
    let app = create_router(ExtractionConfig::default());

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
    let app = create_router(ExtractionConfig::default());

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

    assert!(
        results[0]["chunks"].is_null(),
        "Chunks should be null without chunking config"
    );
    assert!(
        results[0]["detected_languages"].is_null(),
        "Language detection not enabled"
    );
    assert!(results[0]["tables"].is_array(), "Tables field should be present");
    assert!(results[0]["metadata"].is_object(), "Metadata field should be present");
}

/// Test extract endpoint with JSON config.
#[tokio::test]
async fn test_extract_with_config() {
    let app = create_router(ExtractionConfig::default());

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
        boundary, file_content, boundary, config, boundary
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

    assert!(
        results[0]["chunks"].is_null(),
        "Chunks should be null without chunking config"
    );
    assert!(
        results[0]["detected_languages"].is_null(),
        "Language detection not enabled"
    );
    assert!(results[0]["tables"].is_array(), "Tables field should be present");
    assert!(results[0]["metadata"].is_object(), "Metadata field should be present");
}

/// Test extract endpoint with invalid config returns 400.
#[tokio::test]
async fn test_extract_invalid_config() {
    let app = create_router(ExtractionConfig::default());

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
    let app = create_router(ExtractionConfig::default());

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

    for result in &results {
        assert!(
            result["chunks"].is_null(),
            "Chunks should be null without chunking config"
        );
        assert!(result["detected_languages"].is_null(), "Language detection not enabled");
        assert!(result["tables"].is_array(), "Tables field should be present");
        assert!(result["metadata"].is_object(), "Metadata field should be present");
        assert_eq!(result["mime_type"], "text/plain");
    }
}

/// Test extract endpoint with markdown content.
#[tokio::test]
async fn test_extract_markdown_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "# Heading\n\nSome **bold** text.";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.md\"\r\n\
         Content-Type: text/markdown\r\n\
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
    assert_eq!(results[0]["mime_type"], "text/markdown");
    assert!(results[0]["content"].as_str().unwrap().contains("Heading"));
}

/// Test extract endpoint with JSON content.
#[tokio::test]
async fn test_extract_json_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = r#"{"key": "value", "number": 42}"#;

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.json\"\r\n\
         Content-Type: application/json\r\n\
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
    assert_eq!(results[0]["mime_type"], "application/json");
}

/// Test extract endpoint with XML content.
#[tokio::test]
#[cfg(feature = "xml")]
async fn test_extract_xml_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = r#"<?xml version="1.0"?><root><item>test</item></root>"#;

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.xml\"\r\n\
         Content-Type: application/xml\r\n\
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
    assert_eq!(results[0]["mime_type"], "application/xml");
    assert!(results[0]["content"].as_str().unwrap().contains("test"));
}

/// Test extract endpoint with HTML content.
#[tokio::test]
#[cfg(feature = "html")]
async fn test_extract_html_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = r#"<html><body><h1>Title</h1><p>Content</p></body></html>"#;

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.html\"\r\n\
         Content-Type: text/html\r\n\
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
    assert_eq!(results[0]["mime_type"], "text/html");
    assert!(results[0]["content"].as_str().unwrap().contains("Title"));
}

/// Test extract endpoint with missing Content-Type header.
#[tokio::test]
async fn test_extract_missing_content_type() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .body(Body::from("some data"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail - no multipart/form-data content-type
    assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

/// Test extract endpoint with empty file.
#[tokio::test]
async fn test_extract_empty_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"empty.txt\"\r\n\
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
}

/// Test extract endpoint with unsupported MIME type.
#[tokio::test]
async fn test_extract_unsupported_mime_type() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "Binary data";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.bin\"\r\n\
         Content-Type: application/x-unknown-binary\r\n\
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

    // Should return an error response (422 or 500)
    assert!(
        response.status() == StatusCode::UNPROCESSABLE_ENTITY || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

/// Test extract endpoint without filename in multipart field.
#[tokio::test]
async fn test_extract_without_filename() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "Test content";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"\r\n\
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

    // Should still succeed - filename is optional
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test extract endpoint with malformed multipart data.
#[tokio::test]
async fn test_extract_malformed_multipart() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let body_content = format!("--{}\r\nmalformed data\r\n", boundary);

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

    // Should return an error
    assert!(response.status().is_client_error() || response.status().is_server_error());
}

/// Test CORS headers are present.
#[tokio::test]
async fn test_cors_headers() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // CORS headers should be present (tower-http CORS layer)
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin") || headers.contains_key("Access-Control-Allow-Origin"));
}

/// Test OPTIONS preflight request for CORS.
#[tokio::test]
async fn test_cors_preflight() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/extract")
                .header("origin", "http://example.com")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Preflight should succeed
    assert!(response.status().is_success() || response.status() == StatusCode::NO_CONTENT);
}

/// Test error response format for validation errors.
#[tokio::test]
async fn test_error_response_format_validation() {
    let app = create_router(ExtractionConfig::default());

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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(error["error_type"].is_string());
    assert!(error["message"].is_string());
    assert_eq!(error["status_code"], 400);
    assert_eq!(error["error_type"], "ValidationError");
}

/// Test error response format for parsing errors.
#[tokio::test]
async fn test_error_response_format_parsing() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let invalid_config = "not valid json";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         content\r\n\
         --{}\r\n\
         Content-Disposition: form-data; name=\"config\"\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, boundary, invalid_config, boundary
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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(error["error_type"], "ValidationError");
    assert!(error["message"].as_str().unwrap().contains("configuration"));
}

/// Test 404 error for non-existent endpoint.
#[tokio::test]
async fn test_not_found_endpoint() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(Request::builder().uri("/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test extract endpoint with very large text content.
#[tokio::test]
async fn test_extract_large_file() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    // Create a 1MB text file
    let large_content = "A".repeat(1024 * 1024);

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"large.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, large_content, boundary
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
}

/// Test concurrent requests to extract endpoint.
#[tokio::test]
async fn test_concurrent_requests() {
    use tower::ServiceExt;

    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "Concurrent test";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, file_content, boundary
    );

    let mut handles = vec![];

    for _ in 0..5 {
        let app_clone = app.clone();
        let body_clone = body_content.clone();

        let handle = tokio::spawn(async move {
            app_clone
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/extract")
                        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                        .body(Body::from(body_clone))
                        .unwrap(),
                )
                .await
        });

        handles.push(handle);
    }

    for handle in handles {
        let response = handle.await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Test cache stats endpoint.
#[tokio::test]
async fn test_cache_stats_endpoint() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(Request::builder().uri("/cache/stats").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let stats: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(stats["directory"].is_string());
    assert!(stats["total_files"].is_number());
    assert!(stats["total_size_mb"].is_number());
}

/// Test cache clear endpoint.
#[tokio::test]
async fn test_cache_clear_endpoint() {
    let app = create_router(ExtractionConfig::default());

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/cache/clear")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let clear_result: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(clear_result["directory"].is_string());
    assert!(clear_result["removed_files"].is_number());
    assert!(clear_result["freed_mb"].is_number());
}

/// Test extract endpoint with mixed content types.
#[tokio::test]
async fn test_extract_mixed_content_types() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let text_content = "Text file";
    let json_content = r#"{"test": "data"}"#;

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.json\"\r\n\
         Content-Type: application/json\r\n\
         \r\n\
         {}\r\n\
         --{}--\r\n",
        boundary, text_content, boundary, json_content, boundary
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
    assert_eq!(results[0]["mime_type"], "text/plain");
    assert_eq!(results[1]["mime_type"], "application/json");
}

/// Test extract endpoint with unknown field in multipart.
#[tokio::test]
async fn test_extract_unknown_multipart_field() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "Test content";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         {}\r\n\
         --{}\r\n\
         Content-Disposition: form-data; name=\"unknown_field\"\r\n\
         \r\n\
         some value\r\n\
         --{}--\r\n",
        boundary, file_content, boundary, boundary
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

    // Should succeed - unknown fields are ignored
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test extract endpoint with default MIME type (application/octet-stream).
#[tokio::test]
async fn test_extract_default_mime_type() {
    let app = create_router(ExtractionConfig::default());

    let boundary = "----boundary";
    let file_content = "Test content";

    let body_content = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"test.txt\"\r\n\
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

    // Should handle default MIME type
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}
