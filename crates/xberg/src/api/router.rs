//! API router setup and configuration.

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use tower::limit::GlobalConcurrencyLimitLayer;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::SetSensitiveHeadersLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};

use crate::{ExtractionConfig, core::ServerConfig, service::ExtractionServiceBuilder};

#[cfg(feature = "prometheus")]
use super::handlers::metrics_handler;
use super::{
    handlers::{
        cache_clear_handler, cache_manifest_handler, cache_stats_handler, cache_warm_handler, cancel_job_handler,
        detect_handler, extract_async_handler, extract_handler, formats_handler, health_handler, info_handler,
        job_status_handler, not_found_handler, version_handler,
    },
    openweb::{openweb_docling_handler, openweb_external_handler},
    types::{ApiSizeLimits, ApiState},
};

/// Environment variable controlling the server's global concurrency limit.
/// Set to `0` to disable the limit (unbounded concurrent requests).
const MAX_CONCURRENT_REQUESTS_ENV: &str = "XBERG_MAX_CONCURRENT_REQUESTS";

/// Resolve the maximum number of requests processed concurrently by the server.
///
/// Without a bound, every in-flight request buffers its body and extraction
/// working set in memory, so a burst of large requests can exhaust RAM and OOM
/// the process — especially in memory-limited containers. The default caps
/// concurrency at twice the CPU count (clamped to `[4, 32]`), applying
/// backpressure instead of unbounded growth. `XBERG_MAX_CONCURRENT_REQUESTS`
/// overrides it; `0` disables the limit for deployments that bound concurrency
/// upstream (e.g. a reverse proxy or queue).
fn resolve_max_concurrent_requests() -> usize {
    const MIN_DEFAULT: usize = 4;
    const MAX_DEFAULT: usize = 32;

    match std::env::var(MAX_CONCURRENT_REQUESTS_ENV) {
        Ok(value) => value.trim().parse::<usize>().unwrap_or_else(|_| {
            tracing::warn!("{MAX_CONCURRENT_REQUESTS_ENV}={value:?} is not a valid usize; using the computed default");
            (num_cpus::get() * 2).clamp(MIN_DEFAULT, MAX_DEFAULT)
        }),
        Err(_) => (num_cpus::get() * 2).clamp(MIN_DEFAULT, MAX_DEFAULT),
    }
}

/// Create the API router with all routes configured.
///
/// This is public to allow users to embed the router in their own applications.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
///
/// # Examples
///
/// ```no_run
/// use xberg::{ExtractionConfig, api::create_router};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with default config and size limits
/// let config = ExtractionConfig::default();
/// let router = create_router(config);
/// # }
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn create_router(config: ExtractionConfig) -> Router {
    create_router_with_limits(config, ApiSizeLimits::default())
}

/// Create the API router with custom size limits.
///
/// This allows fine-grained control over request body and multipart field size limits.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
/// * `limits` - Size limits for request bodies and multipart uploads.
///
/// # Examples
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with 50 MB limits
/// let config = ExtractionConfig::default();
/// let limits = ApiSizeLimits::from_mb(50, 50);
/// let router = create_router_with_limits(config, limits);
/// # }
/// ```
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
/// use tower_http::limit::RequestBodyLimitLayer;
///
/// # #[tokio::main]
/// # async fn main() {
/// // Custom limits for very large documents (500 MB)
/// let config = ExtractionConfig::default();
/// let limits = ApiSizeLimits::from_mb(500, 500);
/// let router = create_router_with_limits(config, limits);
/// # }
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn create_router_with_limits(config: ExtractionConfig, limits: ApiSizeLimits) -> Router {
    create_router_with_limits_and_server_config(config, limits, ServerConfig::default())
}

/// Build the [`ApiState`] shared by every route, including a fresh extraction service and
/// (behind `feature = "api"`) a fresh job store.
///
/// Extracted from [`create_router_with_limits_and_server_config`] so the wiring from
/// `server_config` into `ApiState` — in particular `job_timeout_secs` — is unit-testable
/// without building a full router.
fn build_api_state(config: ExtractionConfig, server_config: &ServerConfig) -> ApiState {
    // Fallback for embedders that only ever use this built-in router: install the
    // Prometheus-backed meter provider before the extraction service (and therefore the
    // `telemetry::metrics::get_metrics()` `OnceLock`) is built below. Callers who construct
    // their own extraction pipeline outside this router must call
    // `xberg::telemetry::init_prometheus()` themselves, before their first extraction —
    // see that function's doc comment. This call is idempotent, so calling it again here
    // after an embedder's own explicit call is harmless. ~keep
    #[cfg(feature = "prometheus")]
    let prometheus_registry = crate::telemetry::init_prometheus();

    let extraction_service_builder = ExtractionServiceBuilder::new().with_tracing();
    #[cfg(feature = "otel")]
    let extraction_service_builder = extraction_service_builder.with_metrics();
    let extraction_service = extraction_service_builder
        .build()
        .expect("the built-in extraction service uses a valid concurrency limit");

    ApiState {
        default_config: Arc::new(config),
        extraction_service: Arc::new(std::sync::Mutex::new(extraction_service)),
        #[cfg(feature = "api")]
        job_store: Arc::new(super::jobs::JobStore::new()),
        #[cfg(feature = "api")]
        job_timeout_secs: server_config.job_timeout_secs,
        #[cfg(feature = "prometheus")]
        prometheus_registry,
    }
}

/// Build the CORS layer for `server_config`, warning when the resulting policy is permissive.
///
/// An explicitly configured origin list that parses to nothing falls back to the permissive
/// policy rather than locking the server out of every browser client. ~keep
fn build_cors_layer(server_config: &ServerConfig) -> CorsLayer {
    if server_config.cors_allows_all() {
        tracing::warn!(
            "CORS configured to allow all origins (default). This permits CSRF attacks. \
             For production, set XBERG_CORS_ORIGINS environment variable to comma-separated \
             list of allowed origins (e.g., 'https://app.example.com,https://api.example.com')"
        );
        return CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    }

    let origins: Vec<_> = server_config
        .cors_origins
        .iter()
        .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
        .collect();

    if !origins.is_empty() {
        tracing::info!("CORS configured with {} explicit allowed origin(s)", origins.len());
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        tracing::warn!(
            "CORS origins configured but empty/invalid - falling back to permissive CORS. \
             This allows CSRF attacks. Set explicit origins for production."
        );
        CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
    }
}

/// Cap how many extraction requests process concurrently so a burst of large
/// uploads applies backpressure instead of exhausting memory (#1368). Applied to the
/// extraction routes only: `/health`, `/info` and `/version` must keep answering while
/// every slot is busy, or orchestrator probes fail on a saturated but healthy pod. ~keep
fn apply_concurrency_limit(router: Router<ApiState>, max_concurrent_requests: usize) -> Router<ApiState> {
    if max_concurrent_requests > 0 {
        tracing::info!(
            max_concurrent_requests,
            "API global concurrency limit active (set {MAX_CONCURRENT_REQUESTS_ENV}=0 to disable)"
        );
        router.layer(GlobalConcurrencyLimitLayer::new(max_concurrent_requests))
    } else {
        tracing::warn!(
            "API global concurrency limit DISABLED ({MAX_CONCURRENT_REQUESTS_ENV}=0); \
             concurrent requests are unbounded and may exhaust memory under load"
        );
        router
    }
}

/// Create the API router with custom size limits and server configuration.
///
/// This function provides full control over request limits, CORS, and server settings via ServerConfig.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
/// * `limits` - Size limits for request bodies and multipart uploads.
/// * `server_config` - Server configuration including host, port, and CORS settings.
///
/// # Examples
///
/// Not run as a doctest: `pub(crate)`, so it is unreachable from a downstream crate.
/// [`create_router_with_limits`] is the public entry point with the same shape
/// minus the server config.
///
/// ```ignore
/// use xberg::{ExtractionConfig, api::{create_router_with_limits_and_server_config, ApiSizeLimits}, core::ServerConfig};
///
/// # #[tokio::main]
/// # async fn main() -> xberg::Result<()> {
/// let extraction_config = ExtractionConfig::default();
/// let mut server_config = ServerConfig::default();
/// server_config.cors_origins = vec!["https://example.com".to_string()];
/// let router = create_router_with_limits_and_server_config(
///     extraction_config,
///     ApiSizeLimits::default(),
///     server_config
/// );
/// # Ok(())
/// # }
/// ```
pub(crate) fn create_router_with_limits_and_server_config(
    config: ExtractionConfig,
    limits: ApiSizeLimits,
    server_config: ServerConfig,
) -> Router {
    build_router(config, limits, server_config, resolve_max_concurrent_requests())
}

fn build_router(
    config: ExtractionConfig,
    limits: ApiSizeLimits,
    server_config: ServerConfig,
    max_concurrent_requests: usize,
) -> Router {
    let state = build_api_state(config, &server_config);

    let cors_layer = build_cors_layer(&server_config);

    let limited_routes = apply_concurrency_limit(
        Router::new()
            .route("/extract", post(extract_handler))
            .route("/extract-async", post(extract_async_handler))
            .route("/detect", post(detect_handler))
            .route("/process", put(openweb_external_handler))
            .route("/v1/convert/file", post(openweb_docling_handler)),
        max_concurrent_requests,
    );

    let mut router = Router::new()
        .merge(limited_routes)
        .route("/jobs/{job_id}", get(job_status_handler).delete(cancel_job_handler))
        .route("/formats", get(formats_handler))
        .route("/health", get(health_handler))
        .route("/info", get(info_handler))
        .route("/version", get(version_handler))
        .route("/cache/stats", get(cache_stats_handler))
        .route("/cache/clear", delete(cache_clear_handler))
        .route("/cache/manifest", get(cache_manifest_handler))
        .route("/cache/warm", post(cache_warm_handler))
        .fallback(not_found_handler);

    #[cfg(feature = "api")]
    {
        router = router.route("/openapi.json", get(openapi_schema_handler));
    }

    #[cfg(feature = "prometheus")]
    {
        router = router.route("/metrics", get(metrics_handler));
    }

    let router = router
        .layer(DefaultBodyLimit::max(limits.max_request_body_bytes))
        .layer(RequestBodyLimitLayer::new(limits.max_request_body_bytes))
        .layer(cors_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
        .layer(SetSensitiveHeadersLayer::new([axum::http::header::AUTHORIZATION]))
        // Per-request and per-response events are demoted to DEBUG so the default
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
                .on_request(DefaultOnRequest::new().level(tracing::Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG))
                .on_failure(DefaultOnFailure::new().level(tracing::Level::WARN)),
        );

    router.with_state(state)
}

/// OpenAPI schema handler.
///
/// Returns the OpenAPI 3.1 JSON schema for all documented endpoints.
#[cfg(feature = "api")]
async fn openapi_schema_handler() -> axum::Json<serde_json::Value> {
    use crate::api::openapi::openapi_json;

    let schema_str = openapi_json();
    let schema: serde_json::Value = serde_json::from_str(&schema_str)
        .unwrap_or_else(|_| serde_json::json!({"error": "Failed to generate OpenAPI schema"}));

    axum::Json(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probes_answer_while_every_extraction_slot_is_busy() {
        use axum::{body::Body, http::Request};
        use std::{convert::Infallible, time::Duration};
        use tower::ServiceExt;

        let router = build_router(
            ExtractionConfig::default(),
            ApiSizeLimits::default(),
            ServerConfig::default(),
            1,
        );

        let stalled_body = Body::from_stream(futures::stream::pending::<Result<Vec<u8>, Infallible>>());
        let holder = router.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/extract")
                .header("content-type", "multipart/form-data; boundary=x")
                .body(stalled_body)
                .unwrap(),
        );
        let holder = tokio::spawn(holder);
        tokio::time::sleep(Duration::from_millis(100)).await;

        for uri in ["/health", "/info", "/version"] {
            let response = tokio::time::timeout(
                Duration::from_secs(2),
                router
                    .clone()
                    .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()),
            )
            .await
            .unwrap_or_else(|_| panic!("{uri} queued behind the concurrency limit"))
            .unwrap();
            assert!(response.status().is_success(), "{uri}: {}", response.status());
        }

        let queued = tokio::time::timeout(
            Duration::from_millis(300),
            router.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/detect")
                    .body(Body::empty())
                    .unwrap(),
            ),
        )
        .await;
        assert!(queued.is_err(), "extraction routes must still queue behind the limit");
        holder.abort();
    }

    #[test]
    fn test_create_router() {
        let config = ExtractionConfig::default();
        let _router = create_router(config);
    }

    #[test]
    fn test_router_has_routes() {
        use std::mem::size_of_val;
        let config = ExtractionConfig::default();
        let router = create_router(config);
        assert!(size_of_val(&router) > 0);
    }

    #[test]
    fn test_create_router_with_limits() {
        let config = ExtractionConfig::default();
        let limits = ApiSizeLimits::from_mb(50, 50);
        let _router = create_router_with_limits(config, limits);
    }

    #[test]
    fn test_create_router_with_server_config() {
        let extraction_config = ExtractionConfig::default();
        let limits = ApiSizeLimits::from_mb(100, 100);
        let server_config = ServerConfig::default();
        let _router = create_router_with_limits_and_server_config(extraction_config, limits, server_config);
    }

    #[cfg(feature = "api")]
    #[test]
    fn build_api_state_copies_job_timeout_secs_from_server_config() {
        let server_config = ServerConfig {
            job_timeout_secs: 42,
            ..ServerConfig::default()
        };
        let state = build_api_state(ExtractionConfig::default(), &server_config);
        assert_eq!(state.job_timeout_secs, 42);
    }

    #[test]
    fn test_server_config_cors_handling() {
        let extraction_config = ExtractionConfig::default();
        let limits = ApiSizeLimits::default();
        let server_config = ServerConfig {
            cors_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        let _router = create_router_with_limits_and_server_config(extraction_config, limits, server_config);
    }

    #[test]
    #[allow(unsafe_code)]
    fn resolve_max_concurrent_requests_honors_env_and_defaults() {
        let original = std::env::var(MAX_CONCURRENT_REQUESTS_ENV).ok();

        unsafe {
            std::env::set_var(MAX_CONCURRENT_REQUESTS_ENV, "7");
            assert_eq!(resolve_max_concurrent_requests(), 7, "explicit value must be honored");

            std::env::set_var(MAX_CONCURRENT_REQUESTS_ENV, "0");
            assert_eq!(resolve_max_concurrent_requests(), 0, "0 must disable the limit");

            std::env::set_var(MAX_CONCURRENT_REQUESTS_ENV, "not-a-number");
            let fallback = resolve_max_concurrent_requests();
            assert!(
                (4..=32).contains(&fallback),
                "invalid value must fall back to the bounded default"
            );

            std::env::remove_var(MAX_CONCURRENT_REQUESTS_ENV);
            let default = resolve_max_concurrent_requests();
            assert!(
                (4..=32).contains(&default),
                "unset default must be bounded (not unlimited), got {default}"
            );
        }

        unsafe {
            match original {
                Some(value) => std::env::set_var(MAX_CONCURRENT_REQUESTS_ENV, value),
                None => std::env::remove_var(MAX_CONCURRENT_REQUESTS_ENV),
            }
        }
    }
}
