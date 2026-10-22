//! Strict structured extraction validates actual provider output locally. ~keep
#![cfg(all(feature = "liter-llm", feature = "tokio-runtime", not(target_arch = "wasm32")))]

use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use xberg::{ExtractInput, ExtractionConfig, LlmConfig, StructuredExtractionConfig, extract};

const DOCUMENT: &str = "Synthetic invoice INV-17. Vendor Example. Total amount 42.50. Payment is due next month.";
const PRIVATE_VALUE: &str = "SYNTHETIC_PRIVATE_RESPONSE_MARKER";

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "invoice_number": {"type": "string"},
            "total_amount": {"type": ["number", "null"]}
        },
        "required": ["invoice_number", "total_amount"],
        "additionalProperties": false
    })
}

fn llm(server: &MockServer) -> LlmConfig {
    LlmConfig {
        model: "openai/gpt-4o-mini".into(),
        api_key: Some("synthetic-local-key".into()),
        base_url: Some(server.uri()),
        max_retries: Some(0),
        timeout_secs: Some(5),
        ..Default::default()
    }
}

async fn mount_response(server: &MockServer, text: &str) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer synthetic-local-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "synthetic", "object": "chat.completion", "created": 0,
            "model": "gpt-4o-mini",
            "choices": [{"index":0, "message":{"role":"assistant", "content":text}, "finish_reason":"stop"}],
            "usage": {"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}
        })))
        .expect(1)
        .mount(server)
        .await;
}

fn structured_config(server: &MockServer, schema: Value, strict: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        structured_extraction: Some(StructuredExtractionConfig {
            schema,
            schema_name: "invoice".into(),
            schema_description: None,
            strict,
            prompt: None,
            llm: llm(server),
        }),
        ..Default::default()
    }
}

async fn run_case(schema: Value, text: &str, strict: bool) -> xberg::ExtractionResult {
    let server = MockServer::builder().start().await;
    mount_response(&server, text).await;
    let config = structured_config(&server, schema.clone(), strict);
    let output = extract(
        ExtractInput::from_bytes(DOCUMENT.as_bytes().to_vec(), "text/plain", None),
        &config,
    )
    .await
    .expect("text extraction succeeds even when structured extraction warns");
    assert_eq!(
        (output.summary.inputs, output.summary.results, output.summary.errors),
        (1, 1, 0)
    );
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content, DOCUMENT);
    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    let request: Value = serde_json::from_slice(&requests[0].body).expect("request JSON");
    assert_eq!(request["model"], "gpt-4o-mini");
    assert_eq!(request["response_format"]["json_schema"]["schema"], schema);
    assert_eq!(request["response_format"]["json_schema"]["strict"], strict);
    assert!(
        request["messages"][0]["content"]
            .as_str()
            .expect("prompt")
            .contains(DOCUMENT)
    );
    server.verify().await;
    output
}

fn assert_rejected(output: &xberg::ExtractionResult) {
    let result = &output.results[0];
    assert!(
        result.structured_output.is_none(),
        "schema-invalid output must not be published"
    );
    let warnings: Vec<_> = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "structured_extraction")
        .collect();
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].message.contains("schema validation failed"),
        "unexpected structured warning: {}",
        warnings[0]
            .message
            .replace(PRIVATE_VALUE, "[redacted]")
            .replace("synthetic-local-key", "[redacted]")
    );
    assert!(!warnings[0].message.contains(PRIVATE_VALUE));
    assert!(!warnings[0].message.contains("synthetic-local-key"));
}

#[tokio::test]
async fn strict_output_rejects_wrong_scalar_types() {
    let value = json!({"invoice_number":17,"total_amount":PRIVATE_VALUE});
    assert_rejected(&run_case(schema(), &value.to_string(), true).await);
}

#[tokio::test]
async fn strict_output_rejects_missing_required_fields() {
    assert_rejected(&run_case(schema(), r#"{"invoice_number":"INV-17"}"#, true).await);
}

#[tokio::test]
async fn strict_output_rejects_unexpected_fields() {
    let value = json!({"invoice_number":"INV-17","total_amount":42.5,"unexpected":PRIVATE_VALUE});
    assert_rejected(&run_case(schema(), &value.to_string(), true).await);
}

#[tokio::test]
async fn strict_output_accepts_valid_values_and_null() {
    for total in [json!(42.5), Value::Null] {
        let value = json!({"invoice_number":"INV-17","total_amount":total});
        let output = run_case(schema(), &format!("```json\n{value}\n```"), true).await;
        assert_eq!(output.results[0].structured_output, Some(value));
        assert!(output.results[0].processing_warnings.is_empty());
        assert_eq!(
            output.results[0].llm_usage.as_ref().expect("usage")[0].total_tokens,
            Some(15)
        );
    }
}

#[tokio::test]
async fn loose_output_preserves_existing_nonconforming_json_behavior() {
    let value = json!({"invoice_number":17,"total_amount":PRIVATE_VALUE,"extra":true});
    let output = run_case(schema(), &value.to_string(), false).await;
    assert_eq!(output.results[0].structured_output, Some(value));
    assert!(output.results[0].processing_warnings.is_empty());
}

#[tokio::test]
async fn strict_output_honors_declared_draft_and_local_references() {
    let draft7 = json!({"$schema":"http://json-schema.org/draft-07/schema#", "definitions":{"amount":{"type":"number"}},
        "type":"object", "properties":{"total":{"$ref":"#/definitions/amount"}}, "required":["total"]});
    let value = json!({"total":42.5});
    assert_eq!(
        run_case(draft7, &value.to_string(), true).await.results[0].structured_output,
        Some(value)
    );
    let modern = json!({"$schema":"https://json-schema.org/draft/2020-12/schema", "type":"array",
        "prefixItems":[{"type":"string"}], "items":false});
    assert_eq!(
        run_case(modern.clone(), r#"["INV-17"]"#, true).await.results[0].structured_output,
        Some(json!(["INV-17"]))
    );
    assert_rejected(&run_case(modern, "[17]", true).await);
    let old =
        json!({"$schema":"http://json-schema.org/draft-07/schema#", "type":"array", "prefixItems":[{"type":"string"}]});
    assert_eq!(
        run_case(old, "[17]", true).await.results[0].structured_output,
        Some(json!([17]))
    );
}

async fn assert_schema_rejected_without_requests(schema: Value, server: &MockServer) {
    let config = ExtractionConfig {
        use_cache: false,
        structured_extraction: Some(StructuredExtractionConfig {
            schema,
            schema_name: "offline".into(),
            schema_description: None,
            strict: true,
            prompt: None,
            llm: llm(server),
        }),
        ..Default::default()
    };
    let output = extract(
        ExtractInput::from_bytes(DOCUMENT.as_bytes().to_vec(), "text/plain", None),
        &config,
    )
    .await
    .expect("text survives");
    assert_eq!(
        (output.summary.inputs, output.summary.results, output.summary.errors),
        (1, 1, 0)
    );
    assert_eq!(output.results.len(), 1);
    assert!(output.results[0].structured_output.is_none());
    let warnings = &output.results[0].processing_warnings;
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("invalid structured extraction schema"));
    assert!(!warnings[0].message.contains("/private/"));
    assert!(!warnings[0].message.contains("127.0.0.1"));
    assert!(!warnings[0].message.contains(PRIVATE_VALUE));
    assert_eq!(server.received_requests().await.expect("requests").len(), 0);
}

#[tokio::test]
async fn strict_schema_rejects_remote_references_without_network_access() {
    let server = MockServer::start().await;
    let target = tempfile::NamedTempFile::new().expect("create referenced schema");
    std::fs::write(target.path(), br#"{"type":"object"}"#).expect("write valid referenced schema");
    let file_url = url::Url::from_file_path(target.path()).expect("file schema URL");
    for reference in [format!("{}/private-schema", server.uri()), file_url.to_string()] {
        assert_schema_rejected_without_requests(json!({"$ref":reference}), &server).await;
    }
}

#[tokio::test]
async fn strict_schema_rejects_malformed_schema_before_provider_request() {
    let server = MockServer::start().await;
    assert_schema_rejected_without_requests(json!({"type":PRIVATE_VALUE}), &server).await;
}

#[tokio::test]
async fn invalid_json_warning_does_not_disclose_response_content() {
    let output = run_case(schema(), &format!("not-json {PRIVATE_VALUE}"), true).await;
    assert!(output.results[0].structured_output.is_none());
    let warnings = &output.results[0].processing_warnings;
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("invalid JSON"));
    assert!(!warnings[0].message.contains(PRIVATE_VALUE));
}

#[tokio::test]
async fn shared_loose_json_helper_retains_existing_behavior() {
    let server = MockServer::start().await;
    let value = json!({"invoice_number":17});
    mount_response(&server, &value.to_string()).await;
    let (actual, _) =
        xberg::llm::structured::complete_with_json_schema(&llm(&server), DOCUMENT, "invoice", &schema(), "control")
            .await
            .expect("helper deliberately uses provider strict=false");
    assert_eq!(actual, value);
    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
    let request: Value = serde_json::from_slice(&requests[0].body).expect("JSON");
    assert_eq!(request["response_format"]["json_schema"]["strict"], false);
}

#[tokio::test]
async fn gemini_strict_output_validates_original_schema_after_provider_sanitization() {
    for extra_field in [false, true] {
        let server = MockServer::start().await;
        let mut value = json!({"invoice_number":"INV-17","total_amount":42.5});
        if extra_field {
            value["unexpected"] = json!(PRIVATE_VALUE);
        }
        Mock::given(method("POST"))
            .and(path("/models/gemini-2.5-flash:generateContent"))
            .and(header("x-goog-api-key", "synthetic-local-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "candidates":[{"content":{"role":"model","parts":[{"text":value.to_string()}]},"finishReason":"STOP"}],
                "usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let mut config = structured_config(&server, schema(), true);
        config
            .structured_extraction
            .as_mut()
            .expect("structured configuration")
            .llm
            .model = "gemini/gemini-2.5-flash".into();
        let output = extract(
            ExtractInput::from_bytes(DOCUMENT.as_bytes().to_vec(), "text/plain", None),
            &config,
        )
        .await
        .expect("text extraction survives structured validation");
        assert_eq!(
            (output.summary.inputs, output.summary.results, output.summary.errors),
            (1, 1, 0)
        );
        assert_eq!(output.results.len(), 1);
        assert_gemini_request(&server).await;
        if extra_field {
            assert_rejected(&output);
        } else {
            assert_eq!(output.results[0].structured_output, Some(value));
            assert!(output.results[0].processing_warnings.is_empty());
        }
        server.verify().await;
    }
}

async fn assert_gemini_request(server: &MockServer) {
    let requests = server.received_requests().await.expect("native Gemini requests");
    assert_eq!(requests.len(), 1);
    let request: Value = serde_json::from_slice(&requests[0].body).expect("native request JSON");
    assert!(request.get("messages").is_none());
    assert!(
        request["contents"][0]["parts"][0]["text"]
            .as_str()
            .expect("native prompt")
            .contains(DOCUMENT)
    );
    let generation = &request["generationConfig"];
    assert_eq!(generation["responseMimeType"], "application/json");
    let outgoing_schema = generation
        .get("responseJsonSchema")
        .or_else(|| generation.get("responseSchema"))
        .expect("native structured response schema");
    let mut expected = schema();
    expected
        .as_object_mut()
        .expect("object schema")
        .remove("additionalProperties");
    assert_eq!(outgoing_schema, &expected);
}
