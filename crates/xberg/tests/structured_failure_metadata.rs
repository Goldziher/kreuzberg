//! Structured failures retain safe attempt metadata. ~keep
#![cfg(all(feature = "liter-llm", feature = "tokio-runtime", not(target_arch = "wasm32")))]

use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use xberg::{ExtractInput, ExtractionConfig, LlmConfig, StructuredExtractionConfig, extract};

const DOCUMENT: &str = "Synthetic invoice INV-17. Total amount 42.50.";
const RESPONSE_SECRET: &str = "PRIVATE_RESPONSE_SENTINEL";
const PROMPT_SECRET: &str = "PRIVATE_PROMPT_SENTINEL";
const KEY: &str = "synthetic-2139-credential";

fn schema() -> Value {
    json!({"type":"object", "properties":{"total":{"type":"number"}}, "required":["total"],
        "additionalProperties":false})
}

fn llm(server: &MockServer) -> LlmConfig {
    LlmConfig {
        model: "openai/gpt-4o-mini".into(),
        api_key: Some(KEY.into()),
        base_url: Some(server.uri()),
        max_retries: Some(0),
        timeout_secs: Some(5),
        ..Default::default()
    }
}

fn config(server: &MockServer) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        structured_extraction: Some(StructuredExtractionConfig {
            schema: schema(),
            schema_name: "invoice".into(),
            schema_description: None,
            strict: true,
            prompt: Some(format!("{PROMPT_SECRET} {{{{content}}}} {{{{schema}}}}")),
            llm: llm(server),
        }),
        ..Default::default()
    }
}

fn response(content: Value) -> Value {
    json!({"id":"synthetic", "object":"chat.completion", "created":0, "model":"gpt-4o-mini",
        "choices":[{"index":0, "message":{"role":"assistant","content":content}, "finish_reason":"length"}],
        "usage":{"prompt_tokens":21,"completion_tokens":8,"total_tokens":29}})
}

async fn mount(server: &MockServer, body: Value) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", format!("Bearer {KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

async fn assert_requests(server: &MockServer) {
    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(requests.len(), 1);
    let body: Value = serde_json::from_slice(&requests[0].body).expect("request JSON");
    assert_eq!(body["model"], "gpt-4o-mini");
    let prompt = body["messages"][0]["content"].as_str().expect("prompt");
    assert!(prompt.contains(DOCUMENT));
    assert!(prompt.contains(PROMPT_SECRET));
    server.verify().await;
}

async fn run(server: &MockServer, config: &ExtractionConfig) -> xberg::ExtractionResult {
    let output = extract(
        ExtractInput::from_bytes(DOCUMENT.as_bytes().to_vec(), "text/plain", None),
        config,
    )
    .await
    .expect("base extraction survives a structured failure");
    assert_eq!(
        (output.summary.inputs, output.summary.results, output.summary.errors),
        (1, 1, 0)
    );
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content, DOCUMENT);
    assert_requests(server).await;
    output
}

fn assert_safe(text: &str, server: &MockServer) {
    let endpoint = server.uri();
    for excluded in [RESPONSE_SECRET, PROMPT_SECRET, KEY, endpoint.as_str(), "127.0.0.1"] {
        assert!(!text.contains(excluded), "sensitive marker leaked");
    }
}

fn assert_usage(output: &xberg::ExtractionResult, has_choices: bool, has_tokens: bool) {
    let entries = output.results[0].llm_usage.as_ref().expect("attempt usage survives");
    assert_eq!(entries.len(), 1);
    assert_usage_fields(&entries[0], has_choices, has_tokens);
}

fn assert_usage_fields(usage: &xberg::LlmUsage, has_choices: bool, has_tokens: bool) {
    assert_eq!(usage.model, "gpt-4o-mini");
    assert_eq!(usage.source, "structured_extraction");
    assert_eq!(usage.input_tokens, has_tokens.then_some(21));
    assert_eq!(usage.output_tokens, has_tokens.then_some(8));
    assert_eq!(usage.total_tokens, has_tokens.then_some(29));
    assert_eq!(usage.finish_reason.as_deref(), has_choices.then_some("length"));
}

fn assert_safe_error_chain(error: &xberg::XbergError, server: &MockServer) {
    let mut current: &(dyn std::error::Error + 'static) = error;
    let mut depth = 0;
    loop {
        assert_safe(&current.to_string(), server);
        assert_safe(&format!("{current:?}"), server);
        depth += 1;
        assert!(depth <= 16, "error source chain must terminate");
        match current.source() {
            Some(source) => current = source,
            None => break,
        }
    }
    assert!(depth >= 2, "safe attempt metadata source must survive");
}

fn assert_failed(output: &xberg::ExtractionResult, server: &MockServer, message: &str) {
    let document = &output.results[0];
    assert!(document.structured_output.is_none());
    assert_eq!(document.processing_warnings.len(), 1);
    assert_eq!(document.processing_warnings[0].source, "structured_extraction");
    assert!(document.processing_warnings[0].message.contains(message));
    assert_safe(&serde_json::to_string(output).expect("serialized result"), server);
}

async fn assert_malformed_output(text: String) {
    let server = MockServer::start().await;
    mount(&server, response(json!(text))).await;
    let output = run(&server, &config(&server)).await;
    assert_failed(&output, &server, "invalid JSON");
    assert_usage(&output, true, true);
}

#[tokio::test]
async fn truncated_json_retains_exact_attempt_usage() {
    assert_malformed_output(format!("{{\"total\":\"{RESPONSE_SECRET}")).await;
}

#[tokio::test]
async fn invalid_json_token_retains_exact_attempt_usage() {
    assert_malformed_output(format!("! {RESPONSE_SECRET}")).await;
}

async fn assert_absent_output(body: Value, has_choices: bool) {
    let server = MockServer::start().await;
    mount(&server, body).await;
    let output = run(&server, &config(&server)).await;
    assert_failed(&output, &server, "no content");
    assert_usage(&output, has_choices, true);
}

#[tokio::test]
async fn null_content_retains_attempt_metadata() {
    assert_absent_output(response(Value::Null), true).await;
}

#[tokio::test]
async fn missing_content_retains_attempt_metadata() {
    let mut body = response(Value::Null);
    body["choices"][0]["message"]
        .as_object_mut()
        .expect("message")
        .remove("content");
    assert_absent_output(body, true).await;
}

#[tokio::test]
async fn zero_choices_retains_usage_without_inventing_finish_reason() {
    let mut body = response(Value::Null);
    body["choices"] = json!([]);
    assert_absent_output(body, false).await;
}

#[tokio::test]
async fn schema_invalid_json_retains_attempt_usage() {
    let server = MockServer::start().await;
    mount(&server, response(json!(format!("{{\"total\":\"{RESPONSE_SECRET}\"}}")))).await;
    let output = run(&server, &config(&server)).await;
    assert_failed(&output, &server, "schema validation failed");
    assert_usage(&output, true, true);
}

#[tokio::test]
async fn valid_output_retains_existing_success_and_one_usage_entry() {
    let server = MockServer::start().await;
    mount(&server, response(json!("{\"total\":42.5}"))).await;
    let output = run(&server, &config(&server)).await;
    assert_eq!(output.results[0].structured_output, Some(json!({"total":42.5})));
    assert!(output.results[0].processing_warnings.is_empty());
    assert_usage(&output, true, true);
}

#[tokio::test]
async fn missing_provider_token_usage_stays_absent_on_failure() {
    let server = MockServer::start().await;
    let mut body = response(json!(format!("! {RESPONSE_SECRET}")));
    body.as_object_mut().expect("response").remove("usage");
    mount(&server, body).await;
    let output = run(&server, &config(&server)).await;
    assert_failed(&output, &server, "invalid JSON");
    assert_usage(&output, true, false);
}

#[tokio::test]
async fn invalid_schema_has_no_attempt_metadata_and_no_requests() {
    let server = MockServer::start().await;
    let mut config = config(&server);
    config.structured_extraction.as_mut().expect("structured config").schema = json!({"type":RESPONSE_SECRET});
    let output = extract(
        ExtractInput::from_bytes(DOCUMENT.as_bytes().to_vec(), "text/plain", None),
        &config,
    )
    .await
    .expect("base extraction survives");
    assert_eq!(output.results.len(), 1);
    assert!(output.results[0].llm_usage.is_none());
    assert_failed(&output, &server, "invalid structured extraction schema");
    assert_eq!(server.received_requests().await.expect("requests").len(), 0);
}

#[tokio::test]
async fn malformed_direct_helper_is_error_with_safe_source() {
    let server = MockServer::start().await;
    mount(&server, response(json!(format!("{{\"total\":\"{RESPONSE_SECRET}")))).await;
    let error = xberg::llm::structured::complete_with_json_schema(
        &llm(&server),
        &format!("{PROMPT_SECRET} {DOCUMENT}"),
        "invoice",
        &schema(),
        "structured_extraction",
    )
    .await
    .expect_err("malformed output must still fail");
    assert_requests(&server).await;
    assert_safe_error_chain(&error, &server);
    let usage = xberg::llm::usage::usage_from_error(&error).expect("typed attempt metadata");
    assert_usage_fields(&usage, true, true);
}

#[tokio::test]
async fn absent_direct_helper_is_error_with_safe_source() {
    let server = MockServer::start().await;
    mount(&server, response(Value::Null)).await;
    let error = xberg::llm::structured::complete_with_json_schema(
        &llm(&server),
        &format!("{PROMPT_SECRET} {DOCUMENT}"),
        "invoice",
        &schema(),
        "structured_extraction",
    )
    .await
    .expect_err("absent output must still fail");
    assert_requests(&server).await;
    assert_safe_error_chain(&error, &server);
    let usage = xberg::llm::usage::usage_from_error(&error).expect("typed attempt metadata");
    assert_usage_fields(&usage, true, true);
}

#[tokio::test]
async fn direct_helper_valid_output_still_succeeds() {
    let server = MockServer::start().await;
    mount(&server, response(json!("{\"total\":42.5}"))).await;
    let (value, usage) = xberg::llm::structured::complete_with_json_schema(
        &llm(&server),
        &format!("{PROMPT_SECRET} {DOCUMENT}"),
        "invoice",
        &schema(),
        "structured_extraction",
    )
    .await
    .expect("valid output succeeds");
    assert_eq!(value, json!({"total":42.5}));
    assert_eq!(usage.expect("successful usage").total_tokens, Some(29));
    assert_requests(&server).await;
}

#[test]
fn unrelated_errors_do_not_report_attempt_usage() {
    let plain = xberg::XbergError::parsing("unrelated parsing failure");
    let sourced =
        xberg::XbergError::parsing_with_source("unrelated source", std::io::Error::other("synthetic IO failure"));
    for error in [plain, sourced] {
        assert!(xberg::llm::usage::usage_from_error(&error).is_none());
    }
}
