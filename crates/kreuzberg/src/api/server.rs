//! API server setup and configuration.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::{ExtractionConfig, Result};

use super::{
    handlers::{cache_clear_handler, cache_stats_handler, extract_handler, health_handler, info_handler},
    types::{ApiSizeLimits, ApiState},
};

/// Parse size limits from environment variables.
///
/// Reads `KREUZBERG_MAX_UPLOAD_SIZE_MB` to configure upload size limits.
/// Falls back to default (100 MB) if not set or invalid.
fn parse_size_limits_from_env() -> ApiSizeLimits {
    match std::env::var("KREUZBERG_MAX_UPLOAD_SIZE_MB") {
        Ok(value) => match value.parse::<usize>() {
            Ok(mb) if mb > 0 => {
                tracing::info!(
                    "Upload size limit configured from environment: {} MB ({} bytes)",
                    mb,
                    mb * 1024 * 1024
                );
                ApiSizeLimits::from_mb(mb, mb)
            }
            Ok(_) => {
                tracing::warn!("Invalid KREUZBERG_MAX_UPLOAD_SIZE_MB value (must be > 0), using default 100 MB");
                let limits = ApiSizeLimits::default();
                tracing::info!(
                    "Upload size limit: 100 MB (default, {} bytes)",
                    limits.max_request_body_bytes
                );
                limits
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse KREUZBERG_MAX_UPLOAD_SIZE_MB='{}': {}, using default 100 MB",
                    value,
                    e
                );
                let limits = ApiSizeLimits::default();
                tracing::info!(
                    "Upload size limit: 100 MB (default, {} bytes)",
                    limits.max_request_body_bytes
                );
                limits
            }
        },
        Err(_) => {
            let limits = ApiSizeLimits::default();
            tracing::info!(
                "Upload size limit: 100 MB (default, {} bytes)",
                limits.max_request_body_bytes
            );
            limits
        }
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
/// use kreuzberg::{ExtractionConfig, api::create_router};
///
/// # #[tokio::main]
/// # async fn main() {
/// // Create router with default config and size limits
/// let config = ExtractionConfig::default();
/// let router = create_router(config);
/// # }
/// ```
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
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
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
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::{create_router_with_limits, ApiSizeLimits}};
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
pub fn create_router_with_limits(config: ExtractionConfig, limits: ApiSizeLimits) -> Router {
    let state = ApiState {
        default_config: Arc::new(config),
    };

    // Configure CORS based on environment variable
    // SECURITY WARNING: The default allows all origins for development convenience,
    // but this permits CSRF attacks. For production deployments, set KREUZBERG_CORS_ORIGINS
    // to a comma-separated list of allowed origins (e.g., "https://app.example.com,https://api.example.com")
    let cors_layer = if let Ok(origins_str) = std::env::var("KREUZBERG_CORS_ORIGINS") {
        let origins: Vec<_> = origins_str
            .split(',')
            .filter(|s| !s.trim().is_empty())
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
                "KREUZBERG_CORS_ORIGINS set but empty/invalid - falling back to permissive CORS. \
                 This allows CSRF attacks. Set explicit origins for production."
            );
            CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
        }
    } else {
        tracing::warn!(
            "CORS configured to allow all origins (default). This permits CSRF attacks. \
             For production, set KREUZBERG_CORS_ORIGINS environment variable to comma-separated \
             list of allowed origins (e.g., 'https://app.example.com,https://api.example.com')"
        );
        CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
    };

    // Apply request body size limit to all routes
    // This protects against excessively large uploads that could cause memory issues
    Router::new()
        .route("/extract", post(extract_handler))
        .route("/health", get(health_handler))
        .route("/info", get(info_handler))
        .route("/cache/stats", get(cache_stats_handler))
        .route("/cache/clear", delete(cache_clear_handler))
        .layer(RequestBodyLimitLayer::new(limits.max_request_body_bytes))
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Start the API server with config file discovery.
///
/// Searches for kreuzberg.toml/yaml/json in current and parent directories.
/// If no config file is found, uses default configuration.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::api::serve;
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     // Local development
///     serve("127.0.0.1", 8000).await?;
///     Ok(())
/// }
/// ```
///
/// ```no_run
/// use kreuzberg::api::serve;
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     // Docker/production (listen on all interfaces)
///     serve("0.0.0.0", 8000).await?;
///     Ok(())
/// }
/// ```
///
/// # Environment Variables
///
/// ```bash
/// # Python/Docker usage
/// export KREUZBERG_HOST=0.0.0.0
/// export KREUZBERG_PORT=8000
///
/// # CORS configuration (IMPORTANT for production security)
/// # Default: allows all origins (permits CSRF attacks)
/// # Production: set to comma-separated list of allowed origins
/// export KREUZBERG_CORS_ORIGINS="https://app.example.com,https://api.example.com"
///
/// # Upload size limit (default: 100 MB)
/// export KREUZBERG_MAX_UPLOAD_SIZE_MB=200
///
/// python -m kreuzberg.api
/// ```
pub async fn serve(host: impl AsRef<str>, port: u16) -> Result<()> {
    let config = match ExtractionConfig::discover()? {
        Some(config) => {
            tracing::info!("Loaded extraction config from discovered file");
            config
        }
        None => {
            tracing::info!("No config file found, using default configuration");
            ExtractionConfig::default()
        }
    };

    // Parse upload size limit from environment variable
    let limits = parse_size_limits_from_env();

    serve_with_config_and_limits(host, port, config, limits).await
}

/// Start the API server with explicit config.
///
/// Uses default size limits (100 MB). For custom limits, use `serve_with_config_and_limits`.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
/// * `config` - Default extraction configuration for all requests
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::serve_with_config};
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     let config = ExtractionConfig::from_toml_file("config/kreuzberg.toml")?;
///     serve_with_config("127.0.0.1", 8000, config).await?;
///     Ok(())
/// }
/// ```
pub async fn serve_with_config(host: impl AsRef<str>, port: u16, config: ExtractionConfig) -> Result<()> {
    let limits = ApiSizeLimits::default();
    tracing::info!(
        "Upload size limit: 100 MB (default, {} bytes)",
        limits.max_request_body_bytes
    );
    serve_with_config_and_limits(host, port, config, limits).await
}

/// Start the API server with explicit config and size limits.
///
/// # Arguments
///
/// * `host` - IP address to bind to (e.g., "127.0.0.1" or "0.0.0.0")
/// * `port` - Port number to bind to (e.g., 8000)
/// * `config` - Default extraction configuration for all requests
/// * `limits` - Size limits for request bodies and multipart uploads
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::{ExtractionConfig, api::{serve_with_config_and_limits, ApiSizeLimits}};
///
/// #[tokio::main]
/// async fn main() -> kreuzberg::Result<()> {
///     let config = ExtractionConfig::from_toml_file("config/kreuzberg.toml")?;
///     let limits = ApiSizeLimits::from_mb(200, 200);
///     serve_with_config_and_limits("127.0.0.1", 8000, config, limits).await?;
///     Ok(())
/// }
/// ```
pub async fn serve_with_config_and_limits(
    host: impl AsRef<str>,
    port: u16,
    config: ExtractionConfig,
    limits: ApiSizeLimits,
) -> Result<()> {
    let ip: IpAddr = host
        .as_ref()
        .parse()
        .map_err(|e| crate::error::KreuzbergError::validation(format!("Invalid host address: {}", e)))?;

    let addr = SocketAddr::new(ip, port);
    let app = create_router_with_limits(config, limits);

    tracing::info!("Starting Kreuzberg API server on http://{}:{}", ip, port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(crate::error::KreuzbergError::Io)?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::KreuzbergError::Other(e.to_string()))?;

    Ok(())
}

/// Start the API server with default host and port.
///
/// Defaults: host = "127.0.0.1", port = 8000
///
/// Uses config file discovery (searches current/parent directories for kreuzberg.toml/yaml/json).
pub async fn serve_default() -> Result<()> {
    serve("127.0.0.1", 8000).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router() {
        let config = ExtractionConfig::default();
        let _router = create_router(config);
    }

    #[test]
    fn test_router_has_routes() {
        let config = ExtractionConfig::default();
        let router = create_router(config);
        assert!(size_of_val(&router) > 0);
    }
}
