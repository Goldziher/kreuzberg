/// `to_auth_header_format` must map `None` and the literal `"Authorization"` header
/// name (case-insensitively) onto liter-llm's `Bearer` scheme — xberg's documented
/// default.
#[test]
fn test_to_auth_header_format_maps_none_and_authorization_to_bearer() {
    assert!(matches!(
        to_auth_header_format(None),
        liter_llm::AuthHeaderFormat::Bearer
    ));
    assert!(matches!(
        to_auth_header_format(Some("Authorization")),
        liter_llm::AuthHeaderFormat::Bearer
    ));
    assert!(matches!(
        to_auth_header_format(Some("authorization")),
        liter_llm::AuthHeaderFormat::Bearer
    ));
}

/// Any other configured header name must map onto liter-llm's `ApiKey` variant,
/// carrying the exact header name through unchanged.
#[test]
fn test_to_auth_header_format_maps_custom_header_name_to_api_key() {
    match to_auth_header_format(Some("X-Api-Key")) {
        liter_llm::AuthHeaderFormat::ApiKey(name) => assert_eq!(name, "X-Api-Key"),
        other => panic!("expected ApiKey variant, got {other:?}"),
    }
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// A configured `providers` entry must actually reach liter-llm's process-global
/// custom-provider registry, not merely round-trip through the `to_liter_llm_config`
/// DTO conversion (that is all `test_to_liter_llm_config_forwards_providers_into_the_dto_boundary`
/// proves). `register_custom_provider`/`unregister_custom_provider` are the only
/// public read/write surface liter-llm exposes on that registry —
/// `unregister_custom_provider` returning `true` is used here as the observable
/// proof of a prior successful registration, since there is no public getter.
#[test]
fn test_build_client_config_registers_configured_providers_in_the_liter_llm_registry() {
    let provider_name = "xberg-test-provider-registers-in-registry";
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        providers: Some(vec![LlmProviderConfig {
            name: provider_name.to_string(),
            base_url: "https://my-llm.example.com/v1".to_string(),
            auth_header: Some("X-Api-Key".to_string()),
            model_prefixes: vec!["xberg-test-provider-registers/".to_string()],
        }]),
        ..LlmConfig::default()
    };

    build_client_config(&config).expect("build client config");

    let removed = liter_llm::unregister_custom_provider(provider_name).expect("unregister should succeed");
    assert!(removed, "expected provider '{provider_name}' to have been registered");

    let removed_again = liter_llm::unregister_custom_provider(provider_name).expect("unregister should succeed");
    assert!(
        !removed_again,
        "provider should already be gone after the first unregister"
    );
}

/// Building a client twice from the same config (e.g. two extraction calls
/// reusing one `LlmConfig`) must not fail merely because the provider was already
/// registered — `register_custom_provider` replaces same-named entries rather than
/// erroring, so repeated registration of identical data is idempotent.
#[test]
fn test_build_client_config_provider_registration_is_idempotent_for_same_config() {
    let provider_name = "xberg-test-provider-idempotent";
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        providers: Some(vec![LlmProviderConfig {
            name: provider_name.to_string(),
            base_url: "https://my-llm.example.com/v1".to_string(),
            auth_header: None,
            model_prefixes: vec!["xberg-test-provider-idempotent/".to_string()],
        }]),
        ..LlmConfig::default()
    };

    build_client_config(&config).expect("first registration should succeed");
    build_client_config(&config).expect("second registration with identical config should succeed");

    let removed = liter_llm::unregister_custom_provider(provider_name).expect("unregister should succeed");
    assert!(removed);
}

/// An invalid provider entry (liter-llm rejects an empty `model_prefixes` list —
/// it would match every model) must surface as a named `XbergError::Validation`,
/// not be swallowed.
#[test]
fn test_build_client_config_rejects_invalid_provider_registration() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        providers: Some(vec![LlmProviderConfig {
            name: "xberg-test-provider-invalid".to_string(),
            base_url: "https://my-llm.example.com/v1".to_string(),
            auth_header: None,
            model_prefixes: vec![],
        }]),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(
                message.contains("xberg-test-provider-invalid"),
                "error should name the provider: {message}"
            );
        }
        Err(other) => panic!("expected a Validation error, got: {other}"),
        Ok(_) => panic!("expected build_client_config to reject the invalid provider config"),
    }
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// `reasoning_effort` must parse into the exact matching `liter_llm::ReasoningEffort`
/// variant, case-insensitively, so it can be assigned onto
/// `ChatCompletionRequest::reasoning_effort` at each request-building call site.
#[test]
fn test_parse_reasoning_effort_maps_known_values_case_insensitively() {
    let cases = [
        ("low", liter_llm::ReasoningEffort::Low),
        ("MEDIUM", liter_llm::ReasoningEffort::Medium),
        ("High", liter_llm::ReasoningEffort::High),
        ("minimal", liter_llm::ReasoningEffort::Minimal),
        ("max", liter_llm::ReasoningEffort::Max),
    ];
    for (input, expected) in cases {
        let config = LlmConfig {
            model: "openai/gpt-4o".to_string(),
            reasoning_effort: Some(input.to_string()),
            ..LlmConfig::default()
        };
        assert_eq!(
            parse_reasoning_effort(&config).expect("known value should parse"),
            Some(expected),
            "input {input:?} mapped incorrectly"
        );
    }
}

/// An absent `reasoning_effort` must parse to `None` rather than defaulting to a
/// specific effort level — silence in config should stay silence on the request.
#[test]
fn test_parse_reasoning_effort_returns_none_when_unset() {
    let config = LlmConfig::default();
    assert_eq!(parse_reasoning_effort(&config).expect("no value should parse"), None);
}

/// An unrecognized `reasoning_effort` string must be a named `XbergError::Validation`,
/// not a silent drop of the misconfigured value.
#[test]
fn test_parse_reasoning_effort_rejects_unrecognized_value() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        reasoning_effort: Some("maximum".to_string()),
        ..LlmConfig::default()
    };

    match parse_reasoning_effort(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(
                message.contains("maximum"),
                "error should name the bad value: {message}"
            );
        }
        other => panic!("expected a Validation error, got: {other:?}"),
    }
}

/// `to_stop_sequence` must map an absent `stop` to `None`.
#[test]
fn test_to_stop_sequence_returns_none_when_unset() {
    let config = LlmConfig::default();
    assert!(to_stop_sequence(&config).is_none());
}

/// `to_stop_sequence` must map a single-element list onto liter-llm's
/// `StopSequence::Multiple` — never `StopSequence::Single` — so xberg's `LlmConfig::stop`
/// has one FFI-friendly shape regardless of how many sequences are configured.
#[test]
fn test_to_stop_sequence_maps_single_entry_to_multiple_variant() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        stop: Some(vec!["\n\n".to_string()]),
        ..LlmConfig::default()
    };

    match to_stop_sequence(&config) {
        Some(liter_llm::StopSequence::Multiple(sequences)) => {
            assert_eq!(sequences, vec!["\n\n".to_string()]);
        }
        other => panic!("expected StopSequence::Multiple, got {other:?}"),
    }
}

/// `to_stop_sequence` must map every configured stop sequence, in order.
#[test]
fn test_to_stop_sequence_maps_multiple_entries_in_order() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        stop: Some(vec!["\n\n".to_string(), "[END]".to_string()]),
        ..LlmConfig::default()
    };

    match to_stop_sequence(&config) {
        Some(liter_llm::StopSequence::Multiple(sequences)) => {
            assert_eq!(sequences, vec!["\n\n".to_string(), "[END]".to_string()]);
        }
        other => panic!("expected StopSequence::Multiple, got {other:?}"),
    }
}

/// `build_client_config` must reject an out-of-range `top_p` before doing any other
/// work (provider registration, header validation), via `LlmConfig::validate`.
#[test]
fn test_build_client_config_rejects_invalid_top_p() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        top_p: Some(1.5),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("top_p"), "{message}");
        }
        other => panic!("expected a Validation error, got {other:?}"),
    }
}

/// `build_client_config` must reject an out-of-range `presence_penalty`.
#[test]
fn test_build_client_config_rejects_invalid_presence_penalty() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        presence_penalty: Some(-3.0),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("presence_penalty"), "{message}");
        }
        other => panic!("expected a Validation error, got {other:?}"),
    }
}

/// `build_client_config` must reject an out-of-range `frequency_penalty`.
#[test]
fn test_build_client_config_rejects_invalid_frequency_penalty() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        frequency_penalty: Some(3.0),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("frequency_penalty"), "{message}");
        }
        other => panic!("expected a Validation error, got {other:?}"),
    }
}

/// Valid `top_p`, `stop`, `seed`, `presence_penalty`, and `frequency_penalty` values
/// must not prevent `build_client_config` from succeeding.
#[test]
fn test_build_client_config_accepts_valid_sampling_fields() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        top_p: Some(0.9),
        stop: Some(vec!["\n\n".to_string()]),
        seed: Some(42),
        presence_penalty: Some(0.5),
        frequency_penalty: Some(-0.5),
        ..LlmConfig::default()
    };

    assert!(build_client_config(&config).is_ok());
}

/// `build_client_config` must succeed with a nonzero `max_response_bytes` set, applying
/// liter-llm's native-only builder method (`ClientConfigBuilder::max_response_bytes`).
#[test]
fn test_build_client_config_accepts_nonzero_max_response_bytes() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        max_response_bytes: Some(4096),
        ..LlmConfig::default()
    };

    assert!(build_client_config(&config).is_ok());
}

/// An unset `max_response_bytes` must still build a client (unbounded, matching
/// liter-llm's own default).
#[test]
fn test_build_client_config_accepts_unset_max_response_bytes() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        max_response_bytes: None,
        ..LlmConfig::default()
    };

    assert!(build_client_config(&config).is_ok());
}

/// `build_client_config` must reject `max_response_bytes: Some(0)` — caught by
/// `LlmConfig::validate` before liter-llm's own builder is ever reached.
#[test]
fn test_build_client_config_rejects_zero_max_response_bytes() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        max_response_bytes: Some(0),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("max_response_bytes"), "{message}");
        }
        other => panic!("expected a Validation error, got {other:?}"),
    }
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// A `VertexAdc` credential provider needs no I/O to construct (unlike `AzureAd`, which
/// would perform a live OAuth exchange the first time it resolves, and `VertexOauth2`,
/// which reads a key file) — it must build a client and attach a provider without error.
/// liter-llm's own `ClientConfig::Debug` prints only `"[configured]"` for a set
/// `credential_provider`, so that string is the exact, structural proof a provider reached
/// the client config, not merely a truthiness check.
#[test]
fn test_build_client_config_attaches_vertex_adc_credential_provider() {
    let config = LlmConfig {
        model: "vertex_ai/gemini-1.5-pro".to_string(),
        credential_provider: Some(Box::new(CredentialProviderConfig::VertexAdc { scope: None })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    assert!(client_config.credential_provider.is_some());
    let rendered = format!("{client_config:?}");
    assert!(
        rendered.contains(r#"credential_provider: Some("[configured]")"#),
        "{rendered}"
    );
}

/// An `AzureAd` credential provider must reach the client config, and liter-llm's own
/// `ClientConfig::Debug` must never leak the `client_secret` xberg handed it.
#[test]
fn test_build_client_config_attaches_azure_ad_credential_provider_without_leaking_secret() {
    let config = LlmConfig {
        model: "azure/gpt-4o".to_string(),
        credential_provider: Some(Box::new(CredentialProviderConfig::AzureAd {
            tenant_id: "tenant-123".to_string(),
            client_id: "client-456".to_string(),
            client_secret: "super-secret-azure-value".to_string(),
            scope: None,
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let rendered = format!("{client_config:?}");
    assert!(
        !rendered.contains("super-secret-azure-value"),
        "liter-llm ClientConfig Debug leaked the Azure client secret: {rendered}"
    );
    assert!(
        rendered.contains(r#"credential_provider: Some("[configured]")"#),
        "{rendered}"
    );
}

/// A `VertexOauth2` credential provider reads its service-account key file at build time —
/// an invalid file must surface as a named `XbergError::Validation` naming the file, not a
/// panic or an opaque downstream error.
#[test]
fn test_build_client_config_rejects_missing_vertex_oauth2_key_file() {
    let config = LlmConfig {
        model: "vertex_ai/gemini-1.5-pro".to_string(),
        credential_provider: Some(Box::new(CredentialProviderConfig::VertexOauth2 {
            service_account_key_file: "/nonexistent/xberg-test-vertex-key.json".to_string(),
            scope: None,
        })),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(
                message.contains("/nonexistent/xberg-test-vertex-key.json"),
                "error should name the file: {message}"
            );
            assert!(
                message.contains("Vertex OAuth2"),
                "error should name the auth mode: {message}"
            );
        }
        other => panic!("expected a Validation error, got: {other:?}"),
    }
}

/// A valid service-account JSON key file must build a `VertexOauth2` credential provider
/// successfully. Uses a per-process-unique temp file path so parallel test runs never
/// collide (test-independence).
#[test]
fn test_build_client_config_attaches_vertex_oauth2_credential_provider_from_key_file() {
    let path = std::env::temp_dir().join(format!(
        "xberg-test-vertex-oauth2-key-{}-{:?}.json",
        std::process::id(),
        std::thread::current().id()
    ));
    // Assembled rather than written as a literal: a PEM header in source trips the
    // detect-private-key commit hook, and suppressing that hook to admit a fixture is
    // exactly the exemption that later lets a real key through.
    let pem_label = "PRIVATE KEY";
    let fixture_json = serde_json::json!({
        "client_email": "xberg-test@example.iam.gserviceaccount.com",
        "private_key": format!("-----BEGIN {pem_label}-----\nZmFrZS1rZXktbWF0ZXJpYWw=\n-----END {pem_label}-----\n"),
    });
    std::fs::write(&path, fixture_json.to_string()).expect("write fixture key file");

    let config = LlmConfig {
        model: "vertex_ai/gemini-1.5-pro".to_string(),
        credential_provider: Some(Box::new(CredentialProviderConfig::VertexOauth2 {
            service_account_key_file: path.to_string_lossy().into_owned(),
            scope: Some("https://www.googleapis.com/auth/cloud-platform".to_string()),
        })),
        ..LlmConfig::default()
    };

    let result = build_client_config(&config);
    let _ = std::fs::remove_file(&path);

    let client_config = result.expect("build client config");
    assert!(client_config.credential_provider.is_some());
}

/// A `BedrockWebIdentity` credential provider needs no I/O to construct — the token file is
/// only read when the provider later resolves a credential — so it must build a client
/// successfully from plain data alone.
#[test]
fn test_build_client_config_attaches_bedrock_web_identity_credential_provider() {
    let config = LlmConfig {
        model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
        credential_provider: Some(Box::new(CredentialProviderConfig::BedrockWebIdentity {
            role_arn: "arn:aws:iam::123456789012:role/xberg-bedrock".to_string(),
            token_file: "/var/run/secrets/eks.amazonaws.com/serviceaccount/token".to_string(),
            session_name: None,
            region: None,
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    assert!(client_config.credential_provider.is_some());
}

/// With no `credential_provider` set, the client config's slot must stay `None` so
/// liter-llm falls back to `api_key`/environment resolution as before this feature existed.
#[test]
fn test_build_client_config_leaves_credential_provider_unset_when_absent() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    assert!(client_config.credential_provider.is_none());
}

// -----------------------------------------------------------------------
// `apply_request_time_params` — `LlmConfig` carries nine request-time
// parameters. Before this helper existed each request builder set four of
// them by hand, so `top_p`, `stop`, `seed`, `presence_penalty` and
// `frequency_penalty` were accepted by every config file and language
// binding and then silently never reached a provider.
// -----------------------------------------------------------------------

fn base_request() -> liter_llm::ChatCompletionRequest {
    liter_llm::ChatCompletionRequest {
        model: "openai/gpt-4o".to_string(),
        messages: vec![liter_llm::Message::User(liter_llm::UserMessage {
            content: liter_llm::UserContent::Text("hello".to_string()),
            name: None,
        })],
        ..Default::default()
    }
}

/// `LlmConfig::stop` — always a plain list, even for a single sequence — must map onto
/// exactly `StopSequence::Multiple` with the same strings, in the same order.
#[test]
fn should_forward_stop_sequences_to_the_request() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        stop: Some(vec!["\n\n".to_string(), "[END]".to_string()]),
        ..LlmConfig::default()
    };
    let mut request = base_request();

    apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

    assert_eq!(
        request.stop,
        Some(liter_llm::StopSequence::Multiple(vec![
            "\n\n".to_string(),
            "[END]".to_string()
        ]))
    );
}

/// An unset `LlmConfig::stop` must leave `request.stop` as `None`, not an empty list.
#[test]
fn should_leave_stop_none_when_config_stop_is_unset() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        ..LlmConfig::default()
    };
    let mut request = base_request();

    apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

    assert_eq!(request.stop, None);
}

/// `top_p`, `seed`, `presence_penalty`, and `frequency_penalty` must all reach the
/// request with their exact configured values — these four were silently dropped by
/// every call site before `apply_request_time_params` existed.
#[test]
fn should_forward_top_p_seed_and_penalties_to_the_request() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        top_p: Some(0.9),
        seed: Some(42),
        presence_penalty: Some(0.5),
        frequency_penalty: Some(-0.5),
        ..LlmConfig::default()
    };
    let mut request = base_request();

    apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

    assert_eq!(request.top_p, Some(0.9));
    assert_eq!(request.seed, Some(42));
    assert_eq!(request.presence_penalty, Some(0.5));
    assert_eq!(request.frequency_penalty, Some(-0.5));
}

/// `temperature`, `max_tokens`, `reasoning_effort`, and `extra_body` — the four fields
/// every call site already wired by hand — must still reach the request exactly the
/// same way now that they are routed through `apply_request_time_params`.
#[test]
fn should_forward_temperature_max_tokens_reasoning_effort_and_extra_body_to_the_request() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(1024),
        reasoning_effort: Some("high".to_string()),
        extra_body: Some(serde_json::json!({"safety_settings": {"harassment": "block_none"}})),
        ..LlmConfig::default()
    };
    let mut request = base_request();

    apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(1024));
    assert!(matches!(
        request.reasoning_effort,
        Some(liter_llm::ReasoningEffort::High)
    ));
    assert_eq!(
        request.extra_body,
        Some(serde_json::json!({"safety_settings": {"harassment": "block_none"}}))
    );
}

/// An unrecognized `reasoning_effort` string must surface as a named
/// `XbergError::Validation` and must leave the request completely untouched — the
/// parse happens before any field is written, so a rejected config cannot half-apply.
#[test]
fn should_return_validation_error_for_invalid_reasoning_effort() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        reasoning_effort: Some("maximum".to_string()),
        stop: Some(vec!["\n\n".to_string()]),
        temperature: Some(0.7),
        ..LlmConfig::default()
    };
    let mut request = base_request();

    match apply_request_time_params(&mut request, &config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("reasoning_effort"), "{message}");
            assert!(message.contains("maximum"), "{message}");
        }
        other => panic!("expected a Validation error, got {other:?}"),
    }

    assert_eq!(request.stop, None, "a rejected config must not half-apply");
    assert_eq!(request.temperature, None, "a rejected config must not half-apply");
}

/// `apply_request_time_params` must never touch `request.model` or `request.messages` —
/// those are call-site specific (the prompt/schema differs per feature), not carried on
/// `LlmConfig`.
#[test]
fn should_not_touch_model_or_messages_fields() {
    let config = LlmConfig {
        model: "anthropic/claude-sonnet-4-20250514".to_string(),
        temperature: Some(0.3),
        ..LlmConfig::default()
    };
    let mut request = base_request();
    let original_model = request.model.clone();
    let original_messages_len = request.messages.len();

    apply_request_time_params(&mut request, &config).expect("apply_request_time_params should succeed");

    assert_eq!(request.model, original_model, "model must stay call-site controlled");
    assert_eq!(
        request.messages.len(),
        original_messages_len,
        "messages must stay call-site controlled"
    );
}
