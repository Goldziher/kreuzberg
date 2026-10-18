//! LLM client factory — converts xberg's LlmConfig to a liter-llm ManagedClient.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::LazyLock;

use ahash::AHashMap;
use liter_llm::client::ClientConfig;
use liter_llm::{LlmInFlightLimitConfig, ManagedClient};
use parking_lot::RwLock;

#[cfg(not(target_arch = "wasm32"))]
use crate::core::config::CredentialProviderConfig;
use crate::core::config::{
    BedrockConfig, LlmBudgetConfig, LlmCacheConfig, LlmConfig, LlmProviderConfig, LlmRateLimitConfig,
};

/// Translate xberg's [`BedrockConfig`] into liter-llm's own (unboxed) `client::BedrockConfig`.
fn to_liter_llm_bedrock(bedrock: &BedrockConfig) -> liter_llm::client::BedrockConfig {
    liter_llm::client::BedrockConfig {
        region: bedrock.region.clone(),
        cross_region_prefix: bedrock.cross_region_prefix.clone(),
        access_key_id: bedrock.access_key_id.clone(),
        secret_access_key: bedrock.secret_access_key.clone(),
        session_token: bedrock.session_token.clone(),
    }
}

/// Translate xberg's [`LlmProviderConfig`] into liter-llm's own `client::LlmProviderConfig`.
fn to_liter_llm_provider(provider: &LlmProviderConfig) -> liter_llm::client::LlmProviderConfig {
    liter_llm::client::LlmProviderConfig {
        name: provider.name.clone(),
        base_url: provider.base_url.clone(),
        auth_header: provider.auth_header.clone(),
        model_prefixes: provider.model_prefixes.clone(),
    }
}

/// Convert xberg's flat `auth_header` header-name-or-default into liter-llm's richer
/// [`liter_llm::AuthHeaderFormat`].
///
/// xberg's [`LlmProviderConfig::auth_header`] is a single optional header *name*,
/// documented as "defaults to `Authorization` when unset" — it can only express
/// liter-llm's `Bearer` scheme or a raw named header, never the explicit
/// "no authentication" mode `AuthHeaderFormat::None` supports; that mode is
/// unreachable through this mapping (a future field would be needed to expose it).
/// The honest mapping for what xberg *can* express:
/// - `None`, or `Some("Authorization")` (case-insensitive) -> [`liter_llm::AuthHeaderFormat::Bearer`]
///   (`Authorization: Bearer <key>`), matching xberg's documented default.
/// - `Some(name)` for any other header name -> [`liter_llm::AuthHeaderFormat::ApiKey`]
///   (`<name>: <key>`, the raw key with no scheme prefix).
fn to_auth_header_format(auth_header: Option<&str>) -> liter_llm::AuthHeaderFormat {
    match auth_header {
        None => liter_llm::AuthHeaderFormat::Bearer,
        Some(name) if name.eq_ignore_ascii_case("authorization") => liter_llm::AuthHeaderFormat::Bearer,
        Some(name) => liter_llm::AuthHeaderFormat::ApiKey(name.to_string()),
    }
}

/// Register every configured custom provider ([`LlmConfig::providers`]) in liter-llm's
/// process-global custom-provider registry ([`liter_llm::register_custom_provider`]).
///
/// liter-llm's `into_client_builder` never reads `providers` — there is no equivalent
/// field on `ClientConfig` at all (see [`to_liter_llm_config`]). Custom providers only
/// take effect once registered here; without this call, a `providers` entry in config
/// is silently a no-op — shipping a config key that does nothing is worse than a clear
/// error, so registration failures are surfaced as [`crate::XbergError::Validation`]
/// rather than swallowed.
///
/// # Idempotency and cross-config collisions
///
/// The registry is process-global and keyed by provider `name`:
/// `register_custom_provider` replaces an existing entry with the same name rather than
/// erroring. Calling this repeatedly with the *same* [`LlmConfig`] is therefore
/// idempotent — it re-registers identical data every time a client is built from that
/// config. Calling it with *different* [`LlmConfig`]s that both define a provider under
/// the same `name` is **not** conflict-free: the most recently built client's
/// registration wins for every model lookup process-wide, including requests already in
/// flight on a client built from the other config. liter-llm exposes no read API to
/// detect this collision before overwriting (`detect_custom_provider` is crate-private
/// to liter-llm), so it cannot be caught here — use distinct provider names across every
/// `LlmConfig` used within one process to avoid it.
fn register_configured_providers(config: &LlmConfig) -> crate::Result<()> {
    let Some(providers) = config.providers.as_ref() else {
        return Ok(());
    };

    for provider in providers {
        let custom = liter_llm::CustomProviderConfig {
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            auth_header: to_auth_header_format(provider.auth_header.as_deref()),
            model_prefixes: provider.model_prefixes.clone(),
        };

        liter_llm::register_custom_provider(custom).map_err(|e| {
            let msg = format!("Failed to register custom LLM provider '{}': {e}", provider.name);
            crate::XbergError::Validation {
                message: msg,
                source: Some(Box::new(e)),
            }
        })?;
    }

    Ok(())
}

/// Parse xberg's plain-string [`LlmConfig::reasoning_effort`] into liter-llm's
/// [`liter_llm::ReasoningEffort`] request-time enum.
///
/// liter-llm serializes [`liter_llm::ReasoningEffort`] with
/// `#[serde(rename_all = "lowercase")]`; this matches the same five spellings
/// (`"low"`, `"medium"`, `"high"`, `"minimal"`, `"max"`) case-insensitively. An
/// unrecognized value is a [`crate::XbergError::Validation`] rather than a silent
/// drop, matching the "a clear error is better than silence" principle used for
/// `providers` above — a request built from a mistyped value (e.g. `"maximum"`)
/// should fail loudly instead of quietly going out without the hint.
///
/// Called from every site that builds a [`liter_llm::ChatCompletionRequest`] from an
/// [`LlmConfig`], alongside the `temperature`/`max_tokens` lines: `llm::text_completion`,
/// `llm::structured` (both request builders), `llm::vlm_ocr` and `text::ner::llm`. A new
/// request builder must wire this and `config.extra_body` too, or those two config fields
/// silently do nothing on that path.
pub(crate) fn parse_reasoning_effort(config: &LlmConfig) -> crate::Result<Option<liter_llm::ReasoningEffort>> {
    let Some(value) = config.reasoning_effort.as_deref() else {
        return Ok(None);
    };

    match value.to_ascii_lowercase().as_str() {
        "low" => Ok(Some(liter_llm::ReasoningEffort::Low)),
        "medium" => Ok(Some(liter_llm::ReasoningEffort::Medium)),
        "high" => Ok(Some(liter_llm::ReasoningEffort::High)),
        "minimal" => Ok(Some(liter_llm::ReasoningEffort::Minimal)),
        "max" => Ok(Some(liter_llm::ReasoningEffort::Max)),
        _ => Err(crate::XbergError::Validation {
            message: format!(
                "Invalid LLM reasoning_effort '{value}': expected one of \"low\", \"medium\", \"high\", \
                 \"minimal\", \"max\""
            ),
            source: None,
        }),
    }
}

/// Convert xberg's list-shaped [`LlmConfig::stop`] into liter-llm's untagged
/// [`liter_llm::StopSequence`] request-time enum.
///
/// liter-llm represents `stop` as either a single string or a list of strings
/// (`types::common::StopSequence`, `#[serde(untagged)]`); xberg's `LlmConfig::stop` is
/// always a plain list — even a single stop sequence is `["..."]` — so this always maps
/// onto [`liter_llm::StopSequence::Multiple`], which is wire-compatible with every
/// provider that also accepts a single-element array for `stop`.
///
/// Called only from [`apply_request_time_params`], which is the single place every
/// request builder goes through.
pub(crate) fn to_stop_sequence(config: &LlmConfig) -> Option<liter_llm::StopSequence> {
    config
        .stop
        .as_ref()
        .map(|sequences| liter_llm::StopSequence::Multiple(sequences.clone()))
}

/// Apply every request-time parameter carried on an [`LlmConfig`] onto a liter-llm
/// [`liter_llm::ChatCompletionRequest`] in one place, so a new (or migrated) request
/// builder cannot silently drop one of them the way `top_p`, `stop`, `seed`,
/// `presence_penalty`, and `frequency_penalty` previously were — every existing call site
/// wired `temperature`, `max_tokens`, `reasoning_effort` (via [`parse_reasoning_effort`]),
/// and `extra_body` individually, but none of them set those other five
/// [`LlmConfig`] fields, so they were accepted by every config file and language binding
/// and then silently never reached a provider.
///
/// Deliberately leaves `request.model` and `request.messages` untouched — those come from
/// the call site's own prompt/schema/messages, not from `config`.
///
/// Every request builder in the crate calls this: `llm::text_completion::complete_text`,
/// `llm::structured` (both builders), `llm::vlm_ocr` and `text::ner::llm`. Keep it that
/// way — a builder that sets these fields by hand is how the five above went missing.
///
/// # Errors
///
/// Propagates [`parse_reasoning_effort`]'s error for an unrecognized
/// [`LlmConfig::reasoning_effort`] string.
///
/// `#[cfg_attr(alef, alef(skip))]`: `&mut liter_llm::ChatCompletionRequest` is a foreign,
/// non-`repr(C)` type with no FFI-representable equivalent — alef's own
/// `lossy_sanitized_surface` check rejects it outright, matching every other
/// binding-unreachable LLM entry point in this crate (e.g.
/// `create_client_with_credential_provider` above, `llm::text_completion::complete_text`,
/// `llm::structured::complete_with_json_schema`).
#[cfg_attr(alef, alef(skip))]
pub(crate) fn apply_request_time_params(
    request: &mut liter_llm::ChatCompletionRequest,
    config: &LlmConfig,
) -> crate::Result<()> {
    // Parse before mutating anything: a rejected `reasoning_effort` must leave the
    // request untouched rather than half-applied.
    let reasoning_effort = parse_reasoning_effort(config)?;

    request.temperature = config.temperature;
    request.max_tokens = config.max_tokens;
    request.top_p = config.top_p;
    request.stop = to_stop_sequence(config);
    request.seed = config.seed;
    request.presence_penalty = config.presence_penalty;
    request.frequency_penalty = config.frequency_penalty;
    request.reasoning_effort = reasoning_effort;
    request.extra_body = config.extra_body.clone();
    Ok(())
}

/// Translate xberg's [`LlmCacheConfig`] into liter-llm's own `client::LlmCacheConfig`.
fn to_liter_llm_cache(cache: &LlmCacheConfig) -> liter_llm::client::LlmCacheConfig {
    liter_llm::client::LlmCacheConfig {
        max_entries: cache.max_entries,
        ttl_seconds: cache.ttl_seconds,
        backend: cache.backend.clone(),
        backend_config: cache.backend_config.clone(),
    }
}

/// Translate xberg's [`LlmBudgetConfig`] into liter-llm's own `client::LlmBudgetConfig`.
fn to_liter_llm_budget(budget: &LlmBudgetConfig) -> liter_llm::client::LlmBudgetConfig {
    liter_llm::client::LlmBudgetConfig {
        global_limit: budget.global_limit,
        model_limits: budget.model_limits.clone(),
        enforcement: budget.enforcement.clone(),
    }
}

/// Translate xberg's [`LlmRateLimitConfig`] into liter-llm's own `client::LlmRateLimitConfig`.
fn to_liter_llm_rate_limit(rate_limit: &LlmRateLimitConfig) -> liter_llm::client::LlmRateLimitConfig {
    liter_llm::client::LlmRateLimitConfig {
        rpm: rate_limit.rpm,
        tpm: rate_limit.tpm,
        window_seconds: rate_limit.window_seconds,
    }
}

/// Build a live liter-llm credential provider from xberg's [`CredentialProviderConfig`].
///
/// Every variant needs one of liter-llm's `native-http`-backed auth modules
/// (`azure-auth`, `vertex-auth`, `bedrock-auth` — `vertex-adc` rides plain `native-http`),
/// which xberg's Cargo.toml enables unconditionally on every non-wasm32 target (see the
/// `liter-llm` dependency comment there). This function is therefore only compiled on
/// non-wasm32 — the same reason it exists at all: the whole `crate::llm` module (this file
/// included) is compiled out on `wasm32` at the crate root
/// (`#[cfg(all(feature = "liter-llm", not(target_arch = "wasm32")))]` on `pub mod llm;` in
/// `lib.rs`), so there is nothing here for a wasm32 build to reject or fall back from.
#[cfg(not(target_arch = "wasm32"))]
fn build_credential_provider(
    config: &CredentialProviderConfig,
) -> crate::Result<Arc<dyn liter_llm::auth::CredentialProvider>> {
    match config {
        CredentialProviderConfig::AzureAd {
            tenant_id,
            client_id,
            client_secret,
            scope,
        } => Ok(build_azure_ad_provider(
            tenant_id,
            client_id,
            client_secret,
            scope.as_deref(),
        )),
        CredentialProviderConfig::VertexOauth2 {
            service_account_key_file,
            scope,
        } => build_vertex_oauth2_provider(service_account_key_file, scope.as_deref()),
        CredentialProviderConfig::VertexAdc { scope } => Ok(build_vertex_adc_provider(scope.as_deref())),
        CredentialProviderConfig::BedrockWebIdentity {
            role_arn,
            token_file,
            session_name,
            region,
        } => Ok(build_bedrock_web_identity_provider(
            role_arn,
            token_file,
            session_name.as_deref(),
            region.as_deref(),
        )),
    }
}

/// Build an Azure AD OAuth2 client-credentials provider. See [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_azure_ad_provider(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    scope: Option<&str>,
) -> Arc<dyn liter_llm::auth::CredentialProvider> {
    let mut provider = liter_llm::auth::azure_ad::AzureAdCredentialProvider::new(
        tenant_id.to_string(),
        client_id.to_string(),
        secrecy::SecretString::from(client_secret.to_string()),
    );
    if let Some(scope) = scope {
        provider = provider.with_scope(scope.to_string());
    }
    Arc::new(provider)
}

/// Build a Vertex AI OAuth2 provider from a service-account JSON key file. See
/// [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_vertex_oauth2_provider(
    service_account_key_file: &str,
    scope: Option<&str>,
) -> crate::Result<Arc<dyn liter_llm::auth::CredentialProvider>> {
    let mut provider = liter_llm::auth::vertex_oauth::VertexOAuthCredentialProvider::from_key_file(
        std::path::Path::new(service_account_key_file),
    )
    .map_err(|e| crate::XbergError::Validation {
        message: format!("Failed to load Vertex OAuth2 service account key file '{service_account_key_file}': {e}"),
        source: Some(Box::new(e)),
    })?;
    if let Some(scope) = scope {
        provider = provider.with_scope(scope.to_string());
    }
    Ok(Arc::new(provider))
}

/// Build a Vertex AI Application Default Credentials provider. See [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_vertex_adc_provider(scope: Option<&str>) -> Arc<dyn liter_llm::auth::CredentialProvider> {
    let mut provider = liter_llm::auth::vertex_adc::VertexAdcCredentialProvider::new();
    if let Some(scope) = scope {
        provider = provider.with_scope(scope.to_string());
    }
    Arc::new(provider)
}

/// Build an AWS STS `AssumeRoleWithWebIdentity` provider for Bedrock. See
/// [`build_credential_provider`].
#[cfg(not(target_arch = "wasm32"))]
fn build_bedrock_web_identity_provider(
    role_arn: &str,
    token_file: &str,
    session_name: Option<&str>,
    region: Option<&str>,
) -> Arc<dyn liter_llm::auth::CredentialProvider> {
    Arc::new(liter_llm::auth::bedrock_sts::WebIdentityCredentialProvider::new(
        role_arn.to_string(),
        token_file.to_string(),
        session_name.unwrap_or("liter-llm-session").to_string(),
        region.unwrap_or("us-east-1").to_string(),
    ))
}

/// Cache backend name accepted when liter-llm's `opendal-cache` feature is not compiled in.
///
/// xberg does not enable `opendal-cache` (see the `liter-llm` dependency comment in
/// Cargo.toml): it pulls opendal's `services-memory`/`services-redis`/`services-fs`/
/// `services-s3` clients, a heavy dependency for a passthrough cache config field. Without
/// this check, liter-llm's own `into_client_builder` would silently downgrade any other
/// backend name to `Memory` — see [`validate_cache_backend`].
const SUPPORTED_CACHE_BACKEND: &str = "memory";

/// Reject an `LlmConfig::cache` backend name xberg cannot honor, instead of letting it
/// silently degrade to the in-memory backend inside liter-llm's own `into_client_builder`.
fn validate_cache_backend(cache: &LlmCacheConfig) -> crate::Result<()> {
    match cache.backend.as_deref() {
        None | Some(SUPPORTED_CACHE_BACKEND) => Ok(()),
        Some(other) => Err(crate::XbergError::Validation {
            message: format!(
                "Unsupported LLM cache backend '{other}': xberg only supports \"{SUPPORTED_CACHE_BACKEND}\" \
                 (liter-llm's `opendal-cache` feature, which unlocks Redis/S3/filesystem-backed caches, is not \
                 compiled in). Use \"{SUPPORTED_CACHE_BACKEND}\" or omit `backend`."
            ),
            source: None,
        }),
    }
}

/// Translate xberg's [`LlmConfig`] into liter-llm's own canonical `client::LlmConfig`,
/// field for field, so the two never drift silently: every field the two types share
/// (by name and type, by construction — see the `LlmConfig` module doc) is copied
/// here without any per-field re-derivation of liter-llm's own mapping semantics.
///
/// Two deliberate exceptions:
/// - `headers` is left `None`. liter-llm's `into_client_builder` silently drops any
///   header that fails to parse as an HTTP header name/value pair, while xberg's
///   [`build_client_config`] validates headers explicitly and returns a
///   [`crate::XbergError::Validation`] naming the offending header — so headers are
///   applied separately, after conversion, on the builder this function's caller
///   gets back from `into_client_builder()`.
/// - `base_url` is trimmed of a trailing slash before being handed to liter-llm,
///   which does not perform this normalization itself; without it a base URL like
///   `"https://api.openai.com/v1/"` would join with a request path into a doubled
///   `//`.
///
/// [`LlmConfig::max_concurrency`] is the one field that is not a straight copy: it maps
/// onto liter-llm's [`LlmInFlightLimitConfig`] (`Some(max)` becomes
/// `Some(LlmInFlightLimitConfig { max_in_flight: Some(max) })`, `None` stays `None`).
/// liter-llm only installs the corresponding Tower in-flight-limit layer when the client
/// is built as a [`ManagedClient`] — a plain `DefaultClient` has no Tower stack at all, so
/// this field is silently inert unless the client is built through [`ManagedClient`] (see
/// [`create_client`]).
///
/// `providers` is copied below purely for a lossless DTO-to-DTO conversion (see the
/// dedicated test); liter-llm's own `into_client_builder` never reads it back off the
/// value returned here — [`build_client_config`] instead registers each entry directly
/// from xberg's [`LlmConfig::providers`] via [`register_configured_providers`], which is
/// the mapping that actually takes effect.
fn to_liter_llm_config(config: &LlmConfig) -> liter_llm::client::LlmConfig {
    liter_llm::client::LlmConfig {
        model: config.model.clone(),
        api_key: config.api_key.clone(),
        base_url: config
            .base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/').to_string()),
        timeout_secs: config.timeout_secs,
        max_retries: config.max_retries,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        load_env: config.load_env,
        headers: None,
        providers: config
            .providers
            .as_ref()
            .map(|providers| providers.iter().map(to_liter_llm_provider).collect()),
        cache: config.cache.as_deref().map(to_liter_llm_cache),
        budget: config.budget.as_deref().map(to_liter_llm_budget),
        rate_limit: config.rate_limit.as_deref().map(to_liter_llm_rate_limit),
        in_flight_limit: config.max_concurrency.map(|max| LlmInFlightLimitConfig {
            max_in_flight: Some(max),
        }),
        cost_tracking: config.cost_tracking,
        tracing: config.tracing,
        cooldown_secs: config.cooldown_secs,
        health_check_secs: config.health_check_secs,
        bedrock: config.bedrock.as_deref().map(to_liter_llm_bedrock),
    }
}

/// Translate xberg's [`LlmConfig`] into a liter-llm [`ClientConfig`].
///
/// `model`, `temperature`, and `max_tokens` are request-time parameters, not
/// client-level settings; they are deliberately carried on [`LlmConfig`] without
/// being mapped here, matching liter-llm's own `LlmConfig::into_client_builder`.
///
/// The actual field-by-field wiring (`base_url`, `timeout_secs`, `max_retries`,
/// `load_env`, `cache`, `budget`, `rate_limit`, `cost_tracking`, `tracing`,
/// `cooldown_secs`, `health_check_secs`, `bedrock`) is delegated to liter-llm's
/// own [`liter_llm::client::LlmConfig::into_client_builder`] via
/// [`to_liter_llm_config`], rather than re-implemented here, so this function
/// cannot drift from liter-llm's own mapping as new fields are added upstream.
/// `headers` and `max_response_bytes` are applied separately (see
/// [`to_liter_llm_config`]): neither has an equivalent on
/// [`liter_llm::client::LlmConfig`], only on the builder returned by
/// `into_client_builder()`. `providers` has no equivalent on [`ClientConfig`] at all;
/// it is instead registered as a side effect via [`register_configured_providers`] —
/// see that function's doc for the process-global registry semantics this implies.
///
/// Split out of [`create_client`] so the mapping is observable in tests without
/// constructing a live HTTP client.
///
/// [`LlmConfig::validate`] runs first, rejecting an out-of-range `top_p`,
/// `presence_penalty`, `frequency_penalty`, or a zero `max_response_bytes` before any
/// provider registration or header validation below — the same "fail before doing
/// anything else" position as the existing `validate_cache_backend` check.
fn build_client_config(config: &LlmConfig) -> crate::Result<ClientConfig> {
    config.validate()?;
    register_configured_providers(config)?;
    if let Some(ref cache) = config.cache {
        validate_cache_backend(cache)?;
    }

    let mut builder = to_liter_llm_config(config).into_client_builder();
    #[cfg(not(target_arch = "wasm32"))]
    {
        builder = apply_credential_provider(config, builder)?;
    }

    if let Some(ref headers) = config.headers {
        for (key, value) in headers {
            builder = builder.header(key.as_str(), value.as_str()).map_err(|e| {
                let msg = format!("Invalid LLM header '{key}': {e}");
                crate::XbergError::Validation {
                    message: msg,
                    source: Some(Box::new(e)),
                }
            })?;
        }
    }

    // liter-llm's `max_response_bytes` builder method is native-only (`native-http`,
    // non-wasm32) — see the doc comment on `LlmConfig::max_response_bytes`.
    // `LlmConfig::validate` above already rejects `Some(0)`, so this only ever forwards
    // a value liter-llm accepts, but the fallible builder call is still mapped for
    // defense in depth.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(limit) = config.max_response_bytes {
        builder = builder.max_response_bytes(limit).map_err(|e| {
            let msg = format!("Invalid LLM max_response_bytes {limit}: {e}");
            crate::XbergError::Validation {
                message: msg,
                source: Some(Box::new(e)),
            }
        })?;
    }

    Ok(builder.build())
}

/// Resolve [`LlmConfig::credential_provider`], if set, into a live liter-llm provider and
/// attach it to `builder`. A no-op when unset.
#[cfg(not(target_arch = "wasm32"))]
fn apply_credential_provider(
    config: &LlmConfig,
    builder: liter_llm::client::ClientConfigBuilder,
) -> crate::Result<liter_llm::client::ClientConfigBuilder> {
    let Some(provider_config) = config.credential_provider.as_deref() else {
        return Ok(builder);
    };
    let provider = build_credential_provider(provider_config)?;
    Ok(builder.credential_provider(provider))
}

/// Build a fresh liter-llm [`ManagedClient`] from an already-resolved [`ClientConfig`].
///
/// Shared by [`cached_managed_client`] (the cache-miss path behind [`create_client`]) and
/// [`create_client_with_credential_provider`] (which is never cached — see that function's
/// docs). Building through [`ManagedClient`] rather than the plain `DefaultClient` is what
/// makes [`LlmConfig::max_concurrency`] (mapped to `in_flight_limit` in
/// [`to_liter_llm_config`]) actually take effect: `DefaultClient` has no Tower stack at
/// all, so the same config field would be silently inert built the other way — see
/// GH#1465.
fn build_managed_client(config: &LlmConfig, client_config: ClientConfig) -> crate::Result<ManagedClient> {
    ManagedClient::new(client_config, Some(&config.model)).map_err(|e| {
        let msg = format!("Failed to build LLM client for model '{}': {e}", config.model);
        crate::XbergError::Validation {
            message: msg,
            source: Some(Box::new(e)),
        }
    })
}

/// Process-wide cache of shared [`ManagedClient`]s, keyed by a digest of the [`LlmConfig`]
/// that built them.
///
/// # Why a shared client (GH#1465)
///
/// [`LlmConfig::max_concurrency`] is meant to bound *provider* concurrency — e.g. "never
/// have more than N requests in flight against this OpenAI account at once". A limiter
/// attached to a client that is rebuilt on every call is not that: it is a per-call
/// allowance, so N concurrent extractions each mint their own instance and their own
/// semaphore, and aggregate provider concurrency stays unbounded — the opposite of what a
/// provider limit is for (see the GH#1465 issue and #1453, which this replaces). Caching
/// by configuration identity means every caller that resolves to the same [`LlmConfig`]
/// shares one [`ManagedClient`], and therefore one in-flight-request semaphore, making the
/// bound global instead of per-extraction.
///
/// # The key is a digest, never the config
///
/// [`LlmConfig::api_key`] (and, via [`LlmConfig::credential_provider`], secrets nested in
/// [`CredentialProviderConfig`]) must never become a map key, a log field, or anything
/// `Debug`-printed (see this crate's secrets-handling conventions). [`config_digest`]
/// stores only a 32-byte BLAKE3 digest of the JSON-serialized config here — a fixed-size,
/// non-reversible fingerprint. Two configs that serialize identically collide onto the
/// same cached client (the desired cache-hit behavior); nothing about the original secret
/// is recoverable from the stored digest.
///
/// # Growth
///
/// A cache keyed on arbitrary caller-supplied config content has unbounded key
/// cardinality in principle — e.g. a long-running server accepting a distinct per-request
/// [`LlmConfig`] from untrusted callers. [`MAX_CACHED_LLM_CLIENTS`] caps the number of
/// distinct clients held at once. Once full, a config that would need a new entry still
/// gets a correctly-built, fully-functional client from [`cached_managed_client`] — it is
/// simply not pooled, so that one call loses the cross-call sharing benefit instead of the
/// process growing without bound. There is no eviction (no LRU) of existing entries: the
/// cap is a ceiling on distinct configurations, not a size-managed cache, on the
/// expectation that in-process usage exercises a small, mostly-static set of
/// configurations (one per configured pipeline stage/feature), not one per request.
///
/// Shape matches the engine pools already established in this crate for other expensive,
/// process-wide resources — see `candle_ocr::trocr_backend::ENGINE_POOL`.
#[cfg(not(target_arch = "wasm32"))]
static LLM_CLIENT_POOL: LazyLock<RwLock<AHashMap<[u8; 32], Arc<ManagedClient>>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Ceiling on distinct cached LLM client configurations — see [`LLM_CLIENT_POOL`].
#[cfg(not(target_arch = "wasm32"))]
const MAX_CACHED_LLM_CLIENTS: usize = 256;

/// Compute the [`LLM_CLIENT_POOL`] cache key for `config`: a BLAKE3 digest of its
/// JSON serialization. Never log or otherwise expose this value's input — see
/// [`LLM_CLIENT_POOL`]'s docs on why only the digest is stored.
#[cfg(not(target_arch = "wasm32"))]
fn config_digest(config: &LlmConfig) -> crate::Result<[u8; 32]> {
    let serialized = serde_json::to_vec(config).map_err(|e| crate::XbergError::Validation {
        message: format!("Failed to serialize LLM config for client-cache lookup: {e}"),
        source: Some(Box::new(e)),
    })?;
    Ok(*blake3::hash(&serialized).as_bytes())
}

/// Return a cached [`ManagedClient`] for `config`, building and pooling one on first use.
///
/// Uses a read -> miss -> build -> write -> double-check pattern (matching
/// `candle_ocr::trocr_backend::get_or_init_engine`) so that two racing callers with the
/// same config do not both pay the client-construction cost, and so that only one of the
/// two ends up in the pool. See [`LLM_CLIENT_POOL`] for the key derivation, sharing
/// rationale, and growth bound.
#[cfg(not(target_arch = "wasm32"))]
fn cached_managed_client(config: &LlmConfig) -> crate::Result<Arc<ManagedClient>> {
    let digest = config_digest(config)?;

    {
        let pool = LLM_CLIENT_POOL.read();
        if let Some(client) = pool.get(&digest) {
            return Ok(Arc::clone(client));
        }
    }

    let client_config = build_client_config(config)?;
    let client = Arc::new(build_managed_client(config, client_config)?);

    let mut pool = LLM_CLIENT_POOL.write();
    if let Some(existing) = pool.get(&digest) {
        return Ok(Arc::clone(existing));
    }
    if pool.len() < MAX_CACHED_LLM_CLIENTS {
        pool.insert(digest, Arc::clone(&client));
    } else {
        tracing::debug!(
            cached = pool.len(),
            limit = MAX_CACHED_LLM_CLIENTS,
            "LLM client cache at capacity; built an unpooled client for this call"
        );
    }
    Ok(client)
}

/// Create a shared, process-wide-cached liter-llm [`ManagedClient`] from xberg's
/// [`LlmConfig`].
///
/// The `model` field from the config is passed as a model hint so that
/// liter-llm can resolve the correct provider automatically.
///
/// When `api_key` is `None`, liter-llm falls back to the provider's standard
/// environment variable (e.g., `OPENAI_API_KEY`).
///
/// For a `bedrock/`-prefixed model, liter-llm's own provider validation runs
/// here and rejects the request up front — with a message naming the required
/// AWS credentials and how to supply them — when none of an explicit
/// [`BedrockConfig`](crate::core::config::BedrockConfig) credential pair, an
/// `AWS_BEARER_TOKEN_BEDROCK` API key, or an `AWS_ACCESS_KEY_ID` /
/// `AWS_SECRET_ACCESS_KEY` pair in the environment is available. That upstream message
/// is wrapped with the model that failed to build, so the resulting error names
/// the operation (building the LLM client), the input (the model string), and
/// (via the wrapped source) a concrete suggestion.
///
/// Returns an `Arc` shared with every other caller that resolves to an identical
/// [`LlmConfig`] (see [`cached_managed_client`]/[`LLM_CLIENT_POOL`]) — this, plus building
/// through [`ManagedClient`] rather than `DefaultClient`, is what makes
/// [`LlmConfig::max_concurrency`] a real, global provider-request bound instead of a
/// per-call allowance (GH#1465).
pub(crate) fn create_client(config: &LlmConfig) -> crate::Result<Arc<ManagedClient>> {
    cached_managed_client(config)
}

/// Create a liter-llm [`ManagedClient`] from xberg's [`LlmConfig`], overriding any
/// [`LlmConfig::credential_provider`] with an explicit, caller-supplied provider.
///
/// This is the escape hatch for authentication modes [`CredentialProviderConfig`] cannot
/// express as data — GitHub Copilot's interactive device-flow provider
/// (`liter_llm::auth::github_copilot::GithubCopilotCredentialProvider`), or any fully custom
/// [`liter_llm::auth::CredentialProvider`] implementation. Building `provider` requires the
/// caller to depend on `liter-llm` directly; xberg does not re-export its auth types.
///
/// Deliberately **not** cached through [`LLM_CLIENT_POOL`]: the cache key is a digest of
/// the serializable [`LlmConfig`], and `provider` (an `Arc<dyn CredentialProvider>`) is not
/// part of that — nor of any other serializable identity this function has access to. Two
/// callers passing the same `config` with two different `provider`s must never be handed
/// the same client, so every call here builds a fresh, unpooled [`ManagedClient`] instead.
/// It is still built through [`ManagedClient`] (via [`build_managed_client`]), so
/// [`LlmConfig::max_concurrency`] still takes effect per client built this way — it simply
/// is not shared across calls the way [`create_client`]'s cached client is.
///
/// Not available on wasm32: this function lives in `crate::llm::client`, which — like the rest
/// of `crate::llm` — is compiled out on that target (see the crate-root `pub mod llm;` gate in
/// `lib.rs`), because there is no native-http-backed `CredentialProvider` implementation to
/// construct `provider` from there in the first place. [`LlmConfig::credential_provider`]
/// itself still exists as a field on wasm32 (see that field's docs) — it is simply never read.
///
/// This Rust-only function accepts a liter-llm trait object and is not exposed
/// through language bindings.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(alef, alef(skip))]
pub fn create_client_with_credential_provider(
    config: &LlmConfig,
    provider: Arc<dyn liter_llm::auth::CredentialProvider>,
) -> crate::Result<ManagedClient> {
    let mut client_config = build_client_config(config)?;
    client_config.credential_provider = Some(provider);

    build_managed_client(config, client_config)
}

#[cfg(test)]
mod tests;
