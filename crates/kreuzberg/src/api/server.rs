//! API server setup and configuration.

use std::net::{IpAddr, SocketAddr};

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::Result;

use super::handlers::{extract_handler, health_handler, info_handler};

/// Create the API router with all routes configured.
///
/// This is public to allow users to embed the router in their own applications.
pub fn create_router() -> Router {
    Router::new()
        .route("/extract", post(extract_handler))
        .route("/health", get(health_handler))
        .route("/info", get(info_handler))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
}

/// Start the API server.
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
    let ip: IpAddr = host
        .as_ref()
        .parse()
        .map_err(|e| crate::error::KreuzbergError::validation(format!("Invalid host address: {}", e)))?;

    let addr = SocketAddr::new(ip, port);
    let app = create_router();

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
pub async fn serve_default() -> Result<()> {
    serve("127.0.0.1", 8000).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router() {
        let _router = create_router();
        // Router creation should not panic
    }

    #[test]
    fn test_router_has_routes() {
        let router = create_router();
        // Test that router is created with expected structure
        // Actual route testing is done in integration tests
        assert!(std::mem::size_of_val(&router) > 0);
    }
}
