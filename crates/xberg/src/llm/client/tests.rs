use super::*;
use crate::core::config::{BedrockConfig, CredentialProviderConfig, LlmConfig};

#[cfg(feature = "api")]
#[tokio::test]
async fn test_client_path_normalization_with_base_url() {
    use axum::{Router, routing::post};
    use liter_llm::LlmClient;
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let app = Router::new().fallback(post(
        move |_method: axum::http::Method, uri: axum::http::Uri, headers: axum::http::HeaderMap| async move {
            assert_eq!(uri.path(), "/v1/chat/completions");

            let auth = headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("none")
                .to_string();
            let _ = tx.send(auth);

            axum::response::Json(serde_json::json!({
                "id": "test",
                "object": "chat.completion",
                "created": 12345,
                "model": "test",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "{\"foo\": \"bar\"}" },
                    "finish_reason": "stop"
                }]
            }))
        },
    ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let base_url = format!("http://{}/v1/", addr);
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-api-key".to_string()),
        base_url: Some(base_url),
        ..LlmConfig::default()
    };

    let client = create_client(&config).unwrap();

    let request = liter_llm::ChatCompletionRequest {
        model: config.model.clone(),
        messages: vec![liter_llm::Message::User(liter_llm::UserMessage {
            content: liter_llm::UserContent::Text("test".to_string()),
            ..Default::default()
        })],
        ..Default::default()
    };

    let _ = client.chat(request).await.expect("Request failed");

    let auth_header = tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("Timeout waiting for header")
        .expect("No header received");

    assert_eq!(auth_header, "Bearer test-api-key");
}

#[test]
fn test_create_client_sanitizes_base_url() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        base_url: Some("https://api.openai.com/v1/".to_string()),
        ..LlmConfig::default()
    };

    let _ = create_client(&config).unwrap();
}

#[test]
fn test_create_client_applies_load_env_and_valid_headers() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Gateway-Key".to_string(), "secret123".to_string());
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        load_env: Some(true),
        headers: Some(headers),
        ..LlmConfig::default()
    };

    assert!(
        create_client(&config).is_ok(),
        "valid load_env + headers should build a client"
    );
}

#[test]
fn test_create_client_rejects_invalid_header() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Bad\r\nInjected".to_string(), "value".to_string());
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        headers: Some(headers),
        ..LlmConfig::default()
    };

    match create_client(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("Invalid LLM header"), "unexpected message: {message}");
        }
        Err(other) => panic!("expected a Validation error, got: {other}"),
        Ok(_) => panic!("expected create_client to reject the invalid header"),
    }
}

/// Regression test for GH#1465.
///
/// A per-client in-flight-request limit is only a real *provider* concurrency bound if
/// every caller resolving to the same [`LlmConfig`] shares one client (and therefore
/// one semaphore). Before this fix, `create_client` built a fresh `DefaultClient` (no
/// Tower stack at all) on every call, so two calls with an identical config produced
/// two independent instances — this asserts they now produce the *same* instance.
#[test]
fn test_create_client_shares_one_instance_for_identical_config() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key-for-sharing-test".to_string()),
        ..LlmConfig::default()
    };

    let first = create_client(&config).expect("build first client");
    let second = create_client(&config).expect("build second client");

    assert!(
        Arc::ptr_eq(&first, &second),
        "two create_client calls with an identical LlmConfig must return the same client \
         instance so an in-flight-request limit is enforced globally, not per call"
    );
}

/// Companion to [`test_create_client_shares_one_instance_for_identical_config`]: two
/// distinct configurations must never collide onto the same cached client — that would
/// leak one caller's client (and its credentials-derived provider auth) to a caller
/// with unrelated configuration.
#[test]
fn test_create_client_does_not_share_instance_across_different_configs() {
    let config_a = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key-a-for-sharing-test".to_string()),
        ..LlmConfig::default()
    };
    let config_b = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key-b-for-sharing-test".to_string()),
        ..LlmConfig::default()
    };

    let client_a = create_client(&config_a).expect("build client for config a");
    let client_b = create_client(&config_b).expect("build client for config b");

    assert!(
        !Arc::ptr_eq(&client_a, &client_b),
        "create_client must not share a client instance across different LlmConfigs"
    );
}

fn bedrock_model_config(bedrock: BedrockConfig) -> LlmConfig {
    LlmConfig {
        model: "bedrock/anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
        bedrock: Some(Box::new(bedrock)),
        ..LlmConfig::default()
    }
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// Region and cross-region prefix must reach the liter-llm client config —
/// before this they were unreachable because `LlmConfig` had no `bedrock` field.
#[test]
fn test_build_client_config_maps_bedrock_region_and_cross_region_prefix() {
    let config = bedrock_model_config(BedrockConfig {
        region: Some("eu-central-1".to_string()),
        cross_region_prefix: Some("eu".to_string()),
        ..BedrockConfig::default()
    });

    let client_config = build_client_config(&config).expect("build client config");

    assert_eq!(client_config.bedrock_region.as_deref(), Some("eu-central-1"));
    assert_eq!(client_config.bedrock_cross_region_prefix.as_deref(), Some("eu"));
    assert_eq!(client_config.bedrock_access_key_id, None);
    assert_eq!(client_config.bedrock_secret_access_key, None);
    assert_eq!(client_config.bedrock_session_token, None);
}

/// Explicit static credentials must reach the client config verbatim.
#[test]
fn test_build_client_config_maps_bedrock_credentials() {
    let config = bedrock_model_config(BedrockConfig {
        region: Some("us-east-1".to_string()),
        access_key_id: Some("AKIAEXAMPLE".to_string()),
        secret_access_key: Some("example-secret".to_string()),
        session_token: Some("example-token".to_string()),
        ..BedrockConfig::default()
    });

    let client_config = build_client_config(&config).expect("build client config");

    assert_eq!(client_config.bedrock_region.as_deref(), Some("us-east-1"));
    assert_eq!(client_config.bedrock_access_key_id.as_deref(), Some("AKIAEXAMPLE"));
    assert_eq!(
        client_config.bedrock_secret_access_key.as_deref(),
        Some("example-secret")
    );
    assert_eq!(client_config.bedrock_session_token.as_deref(), Some("example-token"));
}

/// Session token is optional: long-lived key pairs must map without one.
#[test]
fn test_build_client_config_maps_bedrock_credentials_without_session_token() {
    let config = bedrock_model_config(BedrockConfig {
        access_key_id: Some("AKIAEXAMPLE".to_string()),
        secret_access_key: Some("example-secret".to_string()),
        ..BedrockConfig::default()
    });

    let client_config = build_client_config(&config).expect("build client config");

    assert_eq!(client_config.bedrock_access_key_id.as_deref(), Some("AKIAEXAMPLE"));
    assert_eq!(
        client_config.bedrock_secret_access_key.as_deref(),
        Some("example-secret")
    );
    assert_eq!(client_config.bedrock_session_token, None);
}

/// With no `bedrock` block, every Bedrock slot must stay `None` so liter-llm
/// resolves region and credentials from the AWS environment / default chain.
#[test]
fn test_build_client_config_leaves_bedrock_unset_when_absent() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");

    assert_eq!(client_config.bedrock_region, None);
    assert_eq!(client_config.bedrock_cross_region_prefix, None);
    assert_eq!(client_config.bedrock_access_key_id, None);
    assert_eq!(client_config.bedrock_secret_access_key, None);
    assert_eq!(client_config.bedrock_session_token, None);
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// A `bedrock/`-prefixed model with no explicit `BedrockConfig` credentials
/// and no AWS credentials in the environment must fail fast at
/// `create_client` time with a clear, actionable error instead of a bare
/// "failed to build client" message or a confusing runtime request failure.
/// `#[serial]` because the test mutates process-wide AWS environment
/// variables, matching the save/restore convention used by the
/// other env-mutating tests in this crate (see
/// `core::server_config::tests::env_tests`).
///
/// `#[allow(unsafe_code)]` is scoped to this one function rather than the module:
/// the crate sets `#![deny(unsafe_code)]` (lib.rs:36), and `std::env::set_var` /
/// `remove_var` are `unsafe` as of edition 2024. `core::server_config::tests::env_tests`
/// takes the same exemption, but as a file-level `#![allow]` — it can, because it is a
/// dedicated test file. These tests sit inline in a production module, so a
/// module-scoped allow would also cover `create_client` itself. ~keep
#[allow(unsafe_code)]
#[serial_test::serial]
#[test]
fn test_create_client_reports_clear_error_for_unconfigured_bedrock() {
    // ~keep `AWS_BEARER_TOKEN_BEDROCK` has to be cleared as well, not just the access key:
    // liter-llm's `BedrockProvider::bearer_token` reads it straight from the environment and
    // `validate` returns `Ok(())` the moment it is set (bedrock.rs:381), so on a machine that
    // exports one this test would see a successful `create_client` and fail for a reason
    // unrelated to what it pins.
    let restore: Vec<(&str, Option<String>)> = ["AWS_ACCESS_KEY_ID", "AWS_BEARER_TOKEN_BEDROCK"]
        .into_iter()
        .map(|key| (key, std::env::var(key).ok()))
        .collect();
    // SAFETY: guarded by #[serial_test::serial] — no other test in this
    // process observes these variables concurrently while this one runs.
    unsafe {
        for (key, _) in &restore {
            std::env::remove_var(key);
        }
    }

    let config = bedrock_model_config(BedrockConfig::default());
    let result = create_client(&config);

    // SAFETY: see above.
    unsafe {
        for (key, original) in &restore {
            match original {
                Some(val) => std::env::set_var(key, val),
                None => std::env::remove_var(key),
            }
        }
    }

    match result {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(
                message.contains("bedrock/anthropic.claude-3-sonnet-20240229-v1:0"),
                "error should name the model (the input) that failed to build: {message}"
            );
            // ~keep Matches liter-llm 2.1.1's wording ("AWS Bedrock requires credentials.").
            // The literal "AWS credentials" this used to look for is from an older release and
            // stopped appearing when the dependency moved, which is GH#1910.
            assert!(
                message.contains("requires credentials"),
                "error should name the root cause (missing AWS credentials): {message}"
            );
            assert!(
                message.contains("AWS_ACCESS_KEY_ID") && message.contains("AWS_SECRET_ACCESS_KEY"),
                "error should suggest how to fix it (set explicit config or the AWS env vars): {message}"
            );
        }
        Err(other) => panic!("expected a Validation error, got: {other}"),
        Ok(_) => panic!("expected create_client to reject an unconfigured bedrock model"),
    }
}

/// Once Bedrock credentials actually reach liter-llm's own `ClientConfig`
/// (built by `build_client_config`), that type's `Debug` impl — not xberg's
/// `LlmConfig`/`BedrockConfig` impls — is the last line of defense against a
/// credential reaching a log via an accidental `tracing::debug!("{config:?}")`
/// or panic dump. This proves the credential survives the xberg -> liter-llm
/// boundary redacted, using real-looking (but fake) values.
#[test]
fn test_build_client_config_debug_never_leaks_bedrock_credentials() {
    let config = bedrock_model_config(BedrockConfig {
        region: Some("us-east-1".to_string()),
        access_key_id: Some("AKIAEXAMPLEFAKE".to_string()),
        secret_access_key: Some("example-fake-secret".to_string()),
        session_token: Some("example-fake-token".to_string()),
        ..BedrockConfig::default()
    });

    let client_config = build_client_config(&config).expect("build client config");
    let rendered = format!("{client_config:?}");

    for secret in ["AKIAEXAMPLEFAKE", "example-fake-secret", "example-fake-token"] {
        assert!(
            !rendered.contains(secret),
            "liter-llm ClientConfig Debug leaked {secret}: {rendered}"
        );
    }
    assert!(
        rendered.contains("us-east-1"),
        "non-secret region should stay visible for diagnosability: {rendered}"
    );
}

/// A misconfigured request (invalid header) on a model that also carries
/// live-looking Bedrock credentials must never leak those credentials into
/// the resulting error — the error's `Display` output is exactly the string
/// that reaches xberg's logs and callers, so this is the real "does a
/// credential reach a log" boundary (as opposed to the config-file
/// persistence `Serialize` impl, which legitimately carries a credential the
/// caller explicitly set, the same way `api_key` already does).
#[test]
fn test_create_client_error_never_leaks_bedrock_credentials() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Bad\r\nInjected".to_string(), "value".to_string());
    let mut config = bedrock_model_config(BedrockConfig {
        region: Some("us-east-1".to_string()),
        access_key_id: Some("AKIAEXAMPLEFAKE".to_string()),
        secret_access_key: Some("example-fake-secret".to_string()),
        session_token: Some("example-fake-token".to_string()),
        ..BedrockConfig::default()
    });
    config.headers = Some(headers);

    // Not `expect_err`: that needs `T: Debug` and liter-llm's `ManagedClient` (returned
    // wrapped in `Arc` by `create_client`) does not implement it, so the success arm has
    // to be discarded by hand.
    let Err(err) = create_client(&config) else {
        panic!("invalid header must reject the request");
    };
    let rendered = format!("{err}");

    for secret in ["AKIAEXAMPLEFAKE", "example-fake-secret", "example-fake-token"] {
        assert!(
            !rendered.contains(secret),
            "create_client error leaked {secret}: {rendered}"
        );
    }
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// The response cache configuration must reach liter-llm's `ClientConfig`
/// unchanged, via the delegated `into_client_builder` mapping.
#[test]
fn test_build_client_config_maps_cache_settings() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        cache: Some(Box::new(LlmCacheConfig {
            max_entries: Some(128),
            ttl_seconds: Some(90),
            backend: Some("memory".to_string()),
            backend_config: None,
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let cache = client_config.cache_config.expect("cache_config present");

    assert_eq!(cache.max_entries, 128);
    assert_eq!(cache.ttl, std::time::Duration::from_secs(90));
    assert!(matches!(cache.backend, liter_llm::tower::CacheBackend::Memory));
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// Without the `opendal-cache` liter-llm feature, a non-memory cache backend name must be
/// rejected up front instead of silently degrading to the in-memory backend inside
/// liter-llm's own `into_client_builder` — a user who configured `backend = "s3"` and got
/// silent in-memory caching would have no way to notice.
#[test]
fn test_build_client_config_rejects_unsupported_cache_backend_without_opendal_cache() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        cache: Some(Box::new(LlmCacheConfig {
            backend: Some("s3".to_string()),
            ..LlmCacheConfig::default()
        })),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("Unsupported LLM cache backend 's3'"), "{message}");
            assert!(message.contains("opendal-cache"), "{message}");
        }
        Err(other) => panic!("expected a Validation error, got: {other}"),
        Ok(_) => panic!("expected build_client_config to reject the unsupported cache backend"),
    }
}

/// A `cache.backend` of exactly `"memory"` must still build successfully — the only
/// backend name xberg accepts without the `opendal-cache` liter-llm feature.
#[test]
fn test_build_client_config_accepts_explicit_memory_cache_backend() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        cache: Some(Box::new(LlmCacheConfig {
            backend: Some("memory".to_string()),
            ..LlmCacheConfig::default()
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let cache = client_config.cache_config.expect("cache_config present");
    assert!(matches!(cache.backend, liter_llm::tower::CacheBackend::Memory));
}

/// An absent `cache.backend` must still build successfully, defaulting to the in-memory
/// backend exactly as liter-llm's own `into_client_builder` does.
#[test]
fn test_build_client_config_accepts_unset_cache_backend() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        cache: Some(Box::new(LlmCacheConfig::default())),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let cache = client_config.cache_config.expect("cache_config present");
    assert!(matches!(cache.backend, liter_llm::tower::CacheBackend::Memory));
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// Budget limits, per-model limits, and enforcement mode must all reach
/// liter-llm's `ClientConfig`.
#[test]
fn test_build_client_config_maps_budget_settings() {
    let mut model_limits = std::collections::HashMap::new();
    model_limits.insert("openai/gpt-4o".to_string(), 25.0);
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        budget: Some(Box::new(LlmBudgetConfig {
            global_limit: Some(100.0),
            model_limits: Some(model_limits),
            enforcement: Some("soft".to_string()),
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let budget = client_config.budget_config.expect("budget_config present");

    assert_eq!(budget.global_limit, Some(100.0));
    assert_eq!(budget.model_limits.get("openai/gpt-4o"), Some(&25.0));
    assert_eq!(budget.enforcement, liter_llm::tower::Enforcement::Soft);
}

/// An unrecognized enforcement string must fall back to liter-llm's own
/// default (`Hard`), matching `into_client_builder`'s `_ => Enforcement::Hard`.
#[test]
fn test_build_client_config_defaults_unknown_enforcement_to_hard() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        budget: Some(Box::new(LlmBudgetConfig {
            global_limit: Some(10.0),
            enforcement: Some("not-a-real-mode".to_string()),
            ..LlmBudgetConfig::default()
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let budget = client_config.budget_config.expect("budget_config present");

    assert_eq!(budget.enforcement, liter_llm::tower::Enforcement::Hard);
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// Rate limit RPM/TPM/window must reach liter-llm's `ClientConfig`.
#[test]
fn test_build_client_config_maps_rate_limit_settings() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        rate_limit: Some(Box::new(LlmRateLimitConfig {
            rpm: Some(60),
            tpm: Some(100_000),
            window_seconds: Some(30),
        })),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");
    let rate_limit = client_config.rate_limit_config.expect("rate_limit_config present");

    assert_eq!(rate_limit.rpm, Some(60));
    assert_eq!(rate_limit.tpm, Some(100_000));
    assert_eq!(rate_limit.window, std::time::Duration::from_secs(30));
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// `cost_tracking` and `tracing` flags must reach liter-llm's `ClientConfig`.
#[test]
fn test_build_client_config_maps_cost_tracking_and_tracing_flags() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        cost_tracking: Some(true),
        tracing: Some(true),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");

    assert!(client_config.enable_cost_tracking);
    assert!(client_config.enable_tracing);
}

/// Regression test for https://github.com/xberg-io/xberg/issues/1381
///
/// `cooldown_secs` and `health_check_secs` must reach liter-llm's
/// `ClientConfig` as `Duration`s.
#[test]
fn test_build_client_config_maps_cooldown_and_health_check_intervals() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        cooldown_secs: Some(15),
        health_check_secs: Some(45),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");

    assert_eq!(
        client_config.cooldown_duration,
        Some(std::time::Duration::from_secs(15))
    );
    assert_eq!(
        client_config.health_check_interval,
        Some(std::time::Duration::from_secs(45))
    );
}

/// With none of the new liter-llm passthrough fields set, every corresponding
/// `ClientConfig` slot must stay at liter-llm's own default/unset state.
#[test]
fn test_build_client_config_leaves_new_passthrough_fields_unset_when_absent() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        ..LlmConfig::default()
    };

    let client_config = build_client_config(&config).expect("build client config");

    assert!(client_config.cache_config.is_none());
    assert!(client_config.budget_config.is_none());
    assert!(client_config.rate_limit_config.is_none());
    assert!(client_config.cooldown_duration.is_none());
    assert!(client_config.health_check_interval.is_none());
    assert!(!client_config.enable_cost_tracking);
    assert!(!client_config.enable_tracing);
}

/// Header validation must still run even when the request also carries the
/// new tower-backed passthrough fields — `to_liter_llm_config` deliberately
/// leaves `headers` unset on the DTO handed to `into_client_builder`, so this
/// validation (not liter-llm's own silent-drop-on-parse-failure behavior) is
/// what actually runs.
#[test]
fn test_build_client_config_still_validates_headers_with_tower_fields_set() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Bad\r\nInjected".to_string(), "value".to_string());
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("test-key".to_string()),
        headers: Some(headers),
        cost_tracking: Some(true),
        ..LlmConfig::default()
    };

    match build_client_config(&config) {
        Err(crate::XbergError::Validation { message, .. }) => {
            assert!(message.contains("Invalid LLM header"), "unexpected message: {message}");
        }
        Err(other) => panic!("expected a Validation error, got: {other}"),
        Ok(_) => panic!("expected build_client_config to reject the invalid header"),
    }
}

/// Regression test for GH#1465.
///
/// `LlmConfig::max_concurrency` must map onto liter-llm's `in_flight_limit` — the field
/// `into_client_builder` wires onto a Tower in-flight-request-limit layer when the
/// client is built through `ManagedClient` (see `build_managed_client`'s docs). `None`
/// must stay `None` (unlimited), never default to an artificial limit.
#[test]
fn test_to_liter_llm_config_maps_max_concurrency_to_in_flight_limit() {
    let with_limit = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        max_concurrency: Some(4),
        ..LlmConfig::default()
    };
    assert_eq!(
        to_liter_llm_config(&with_limit).in_flight_limit,
        Some(LlmInFlightLimitConfig { max_in_flight: Some(4) })
    );

    let without_limit = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        max_concurrency: None,
        ..LlmConfig::default()
    };
    assert_eq!(to_liter_llm_config(&without_limit).in_flight_limit, None);
}

/// `providers` has no equivalent field on liter-llm's `ClientConfig` at all —
/// liter-llm's own `into_client_builder` does not map it either. This test proves
/// the DTO-to-DTO conversion in `to_liter_llm_config` is lossless for `providers`
/// on its own terms; it is *not* proof that a configured provider takes effect —
/// see `test_build_client_config_registers_configured_providers_in_the_liter_llm_registry`
/// below for that, which exercises the actual runtime registration in
/// `register_configured_providers`/`build_client_config`.
#[test]
fn test_to_liter_llm_config_forwards_providers_into_the_dto_boundary() {
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        providers: Some(vec![LlmProviderConfig {
            name: "my-provider".to_string(),
            base_url: "https://my-llm.example.com/v1".to_string(),
            auth_header: Some("X-Api-Key".to_string()),
            model_prefixes: vec!["my-provider/".to_string()],
        }]),
        ..LlmConfig::default()
    };

    let upstream = to_liter_llm_config(&config);
    let providers = upstream.providers.expect("providers present");

    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "my-provider");
    assert_eq!(providers[0].base_url, "https://my-llm.example.com/v1");
    assert_eq!(providers[0].auth_header.as_deref(), Some("X-Api-Key"));
    assert_eq!(providers[0].model_prefixes, vec!["my-provider/".to_string()]);
}

// Split for file-too-long (#1567): `include!` keeps these tests in one module scope. ~keep
include!("tests/part2.rs");
