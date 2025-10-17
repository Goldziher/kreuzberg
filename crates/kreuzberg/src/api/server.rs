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
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{ExtractionConfig, Result};

use super::{
    handlers::{cache_clear_handler, cache_stats_handler, extract_handler, health_handler, info_handler},
    types::ApiState,
};

/// Create the API router with all routes configured.
///
/// This is public to allow users to embed the router in their own applications.
///
/// # Arguments
///
/// * `config` - Default extraction configuration. Per-request configs override these defaults.
pub fn create_router(config: ExtractionConfig) -> Router {
    let state = ApiState {
        default_config: Arc::new(config),
    };

    Router::new()
        .route("/extract", post(extract_handler))
        .route("/health", get(health_handler))
        .route("/info", get(info_handler))
        .route("/cache/stats", get(cache_stats_handler))
        .route("/cache/clear", delete(cache_clear_handler))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
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
/// python -m kreuzberg.api
/// ```
pub async fn serve(host: impl AsRef<str>, port: u16) -> Result<()> {
    // Discover config file in current/parent directories
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

    serve_with_config(host, port, config).await
}

/// Start the API server with explicit config.
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
    let ip: IpAddr = host
        .as_ref()
        .parse()
        .map_err(|e| crate::error::KreuzbergError::validation(format!("Invalid host address: {}", e)))?;

    let addr = SocketAddr::new(ip, port);
    let app = create_router(config);

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
        // Router creation should not panic
    }

    #[test]
    fn test_router_has_routes() {
        let config = ExtractionConfig::default();
        let router = create_router(config);
        // Test that router is created with expected structure
        // Actual route testing is done in integration tests
        assert!(std::mem::size_of_val(&router) > 0);
    }
}
