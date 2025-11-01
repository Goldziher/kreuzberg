//! Pandoc server mode for persistent process conversions
//!
//! Manages a long-running pandoc-server process to amortize startup costs
//! across multiple document conversions. Suitable for high-throughput scenarios.
//!
//! # Overview
//!
//! This module provides low-level control over pandoc-server lifecycle. For most use cases,
//! prefer [`BatchExtractor`](super::batch::BatchExtractor) which automatically handles
//! server detection, startup, and fallback.
//!
//! # Server Activation
//!
//! Pandoc only runs in server mode when invoked as "pandoc-server" (checks argv[0]).
//! This module handles activation automatically:
//!
//! 1. Detects pandoc binary location via `which pandoc`
//! 2. Creates temporary symlink: `/tmp/pandoc-server-{pid}`
//! 3. Invokes symlink with `--port` and `--timeout` flags
//!
//! # Example
//!
//! ```no_run
//! use kreuzberg::extraction::pandoc::server::PandocServer;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     // Create and start server
//!     let server = PandocServer::new(Some(3030), Some(120)).await?;
//!     server.start().await?;
//!
//!     // Health check
//!     let version = server.health_check().await?;
//!     println!("Pandoc version: {}", version);
//!
//!     // Convert document
//!     let markdown = "# Hello World\n\nTest content.";
//!     let html = server.convert(markdown, "markdown", "html").await?;
//!
//!     // Extract with metadata
//!     let (content, metadata) = server.extract_with_server(markdown, "markdown").await?;
//!
//!     // Cleanup
//!     server.stop().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Thread Safety
//!
//! `PandocServer` uses Arc<RwLock<Option<Child>>> for thread-safe process management.
//! It can be safely shared across async tasks.

use crate::error::{KreuzbergError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep, timeout};

/// Default port for pandoc-server
const DEFAULT_PORT: u16 = 3030;

/// Default timeout for conversions (seconds)
const DEFAULT_TIMEOUT: u64 = 120;

/// Health check interval (seconds)
#[allow(dead_code)]
const HEALTH_CHECK_INTERVAL: u64 = 30;

/// Maximum retries for failed requests
const MAX_RETRIES: usize = 3;

/// Pandoc server request payload
#[derive(Debug, Serialize)]
struct PandocRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    standalone: Option<bool>,
}

/// Pandoc server response payload
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PandocResponse {
    Success {
        output: String,
        #[serde(default)]
        base64: bool,
        #[serde(default)]
        #[allow(dead_code)]
        messages: Vec<Value>,
    },
    Error {
        error: String,
    },
}

/// Pandoc server instance manager
pub struct PandocServer {
    port: u16,
    timeout_secs: u64,
    process: Arc<RwLock<Option<Child>>>,
    pandoc_path: PathBuf,
    server_symlink: Option<PathBuf>,
}

impl PandocServer {
    /// Create a new Pandoc server manager
    ///
    /// # Arguments
    /// * `port` - HTTP port for the server (default: 3030)
    /// * `timeout_secs` - Conversion timeout in seconds (default: 120)
    pub async fn new(port: Option<u16>, timeout_secs: Option<u64>) -> Result<Self> {
        let port = port.unwrap_or(DEFAULT_PORT);
        let timeout_secs = timeout_secs.unwrap_or(DEFAULT_TIMEOUT);

        // Locate pandoc binary
        let pandoc_path = Self::find_pandoc().await?;

        // Check if we need to create a symlink for server mode
        let server_symlink = if pandoc_path.file_name() == Some(std::ffi::OsStr::new("pandoc")) {
            Some(Self::create_server_symlink(&pandoc_path).await?)
        } else {
            None
        };

        Ok(Self {
            port,
            timeout_secs,
            process: Arc::new(RwLock::new(None)),
            pandoc_path,
            server_symlink,
        })
    }

    /// Check if pandoc-server is available (pandoc 3.8+)
    pub async fn is_server_available() -> bool {
        // Check for pandoc-server binary directly
        if let Ok(output) = Command::new("which").arg("pandoc-server").output().await
            && output.status.success()
        {
            tracing::debug!("Found pandoc-server binary in PATH");
            return true;
        }

        // Check if pandoc supports server mode by looking for version 3.8+
        if let Ok(output) = Command::new("pandoc").arg("--version").output().await
            && let Ok(version_str) = String::from_utf8(output.stdout)
            && let Some(version_line) = version_str.lines().next()
            && let Some(version) = version_line.split_whitespace().nth(1)
            && let Some(major_minor) = version.split('.').take(2).collect::<Vec<_>>().get(0..2)
            && let (Ok(major), Ok(minor)) = (major_minor[0].parse::<u32>(), major_minor[1].parse::<u32>())
        {
            let supports_server = major > 3 || (major == 3 && minor >= 8);
            if supports_server {
                tracing::debug!("Pandoc version {}.{} supports server mode", major, minor);
            } else {
                tracing::debug!(
                    "Pandoc version {}.{} does not support server mode (requires 3.8+)",
                    major,
                    minor
                );
            }
            return supports_server;
        }

        tracing::debug!("Could not determine pandoc server availability");
        false
    }

    /// Find pandoc binary in PATH
    async fn find_pandoc() -> Result<PathBuf> {
        let output = Command::new("which")
            .arg("pandoc")
            .output()
            .await
            .map_err(|e| KreuzbergError::MissingDependency(format!("Failed to locate pandoc: {}", e)))?;

        if !output.status.success() {
            return Err(KreuzbergError::MissingDependency(
                "pandoc not found in PATH".to_string(),
            ));
        }

        let path_str = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(path_str.trim());

        if !path.exists() {
            return Err(KreuzbergError::MissingDependency(
                "pandoc binary does not exist".to_string(),
            ));
        }

        Ok(path)
    }

    /// Create a temporary symlink to pandoc named "pandoc-server"
    ///
    /// This is required because pandoc only runs in server mode when invoked
    /// as "pandoc-server" (checks argv[0]).
    async fn create_server_symlink(pandoc_path: &Path) -> Result<PathBuf> {
        #[cfg(unix)]
        {
            let temp_dir = std::env::temp_dir();
            let symlink_path = temp_dir.join(format!("pandoc-server-{}", std::process::id()));

            // Remove existing symlink if present
            let _ = tokio::fs::remove_file(&symlink_path).await;

            // Create symlink
            tokio::fs::symlink(pandoc_path, &symlink_path).await.map_err(|e| {
                KreuzbergError::Io(std::io::Error::other(format!(
                    "Failed to create pandoc-server symlink: {}",
                    e
                )))
            })?;

            Ok(symlink_path)
        }

        #[cfg(not(unix))]
        {
            let _ = pandoc_path; // Suppress unused variable warning
            Err(KreuzbergError::validation(
                "Pandoc server mode requires Unix-like system for symlinks",
            ))
        }
    }

    /// Start the pandoc server process
    pub async fn start(&self) -> Result<()> {
        let mut process_lock = self.process.write().await;

        // Stop existing process if running
        if let Some(mut child) = process_lock.take() {
            let _ = child.kill().await;
        }

        // Determine which command to use
        let command_path = self.server_symlink.as_ref().unwrap_or(&self.pandoc_path);

        // Start server process
        let mut child = Command::new(command_path)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--timeout")
            .arg(self.timeout_secs.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| KreuzbergError::Io(std::io::Error::other(format!("Failed to start pandoc-server: {}", e))))?;

        // Wait for server to be ready
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| KreuzbergError::Io(std::io::Error::other("Failed to capture server stdout")))?;

        let mut reader = BufReader::new(stdout).lines();

        // Read startup message with timeout
        match timeout(Duration::from_secs(5), reader.next_line()).await {
            Ok(Ok(Some(line))) if line.contains("Starting server") => {
                *process_lock = Some(child);
                Ok(())
            }
            _ => {
                let _ = child.kill().await;
                Err(KreuzbergError::Io(std::io::Error::other(
                    "Pandoc server failed to start within 5 seconds",
                )))
            }
        }
    }

    /// Stop the pandoc server process
    pub async fn stop(&self) -> Result<()> {
        let mut process_lock = self.process.write().await;

        if let Some(mut child) = process_lock.take() {
            child.kill().await.map_err(|e| {
                KreuzbergError::Io(std::io::Error::other(format!("Failed to kill pandoc-server: {}", e)))
            })?;
        }

        // Cleanup symlink if we created one
        if let Some(symlink) = &self.server_symlink {
            let _ = tokio::fs::remove_file(symlink).await;
        }

        Ok(())
    }

    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        let process_lock = self.process.read().await;
        process_lock.is_some()
    }

    /// Perform health check by querying /version endpoint
    pub async fn health_check(&self) -> Result<String> {
        let url = format!("http://localhost:{}/version", self.port);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| KreuzbergError::Io(std::io::Error::other(format!("Failed to create HTTP client: {}", e))))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| KreuzbergError::Io(std::io::Error::other(format!("Health check failed: {}", e))))?;

        if !response.status().is_success() {
            return Err(KreuzbergError::Io(std::io::Error::other(format!(
                "Health check returned status: {}",
                response.status()
            ))));
        }

        let version = response.text().await.map_err(|e| {
            KreuzbergError::Io(std::io::Error::other(format!(
                "Failed to read health check response: {}",
                e
            )))
        })?;

        Ok(version.trim().to_string())
    }

    /// Convert document using the pandoc server
    ///
    /// # Arguments
    /// * `content` - Document content as string
    /// * `from_format` - Input format (e.g., "markdown", "docx", "rst")
    /// * `to_format` - Output format (e.g., "json", "markdown", "html")
    pub async fn convert(&self, content: &str, from_format: &str, to_format: &str) -> Result<String> {
        let url = format!("http://localhost:{}/", self.port);

        let request_body = PandocRequest {
            text: content.to_string(),
            from: Some(from_format.to_string()),
            to: Some(to_format.to_string()),
            standalone: Some(true),
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| KreuzbergError::Io(std::io::Error::other(format!("Failed to create HTTP client: {}", e))))?;

        // Retry logic for transient failures
        for attempt in 0..MAX_RETRIES {
            let response = client
                .post(&url)
                .json(&request_body)
                .header("Accept", "application/json")
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        if attempt < MAX_RETRIES - 1 {
                            sleep(Duration::from_millis(100 * (attempt as u64 + 1))).await;
                            continue;
                        }
                        return Err(KreuzbergError::parsing(format!(
                            "Pandoc server returned status: {}",
                            resp.status()
                        )));
                    }

                    let pandoc_response: PandocResponse = resp
                        .json()
                        .await
                        .map_err(|e| KreuzbergError::parsing(format!("Failed to parse server response: {}", e)))?;

                    match pandoc_response {
                        PandocResponse::Success {
                            output,
                            base64,
                            messages: _,
                        } => {
                            if base64 {
                                // Decode base64 if needed
                                let decoded = base64_simd::STANDARD.decode_to_vec(output.as_bytes()).map_err(|e| {
                                    KreuzbergError::parsing(format!("Failed to decode base64 output: {}", e))
                                })?;
                                return String::from_utf8(decoded).map_err(|e| {
                                    KreuzbergError::parsing(format!("Failed to decode UTF-8 output: {}", e))
                                });
                            }
                            return Ok(output);
                        }
                        PandocResponse::Error { error } => {
                            return Err(KreuzbergError::parsing(format!("Pandoc server error: {}", error)));
                        }
                    }
                }
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        // Transient network error, retry
                        sleep(Duration::from_millis(100 * (attempt as u64 + 1))).await;
                        continue;
                    }
                    return Err(KreuzbergError::Io(std::io::Error::other(format!(
                        "HTTP request failed after {} retries: {}",
                        MAX_RETRIES, e
                    ))));
                }
            }
        }

        Err(KreuzbergError::parsing(
            "Max retries exceeded for pandoc server conversion".to_string(),
        ))
    }

    /// Extract both content and metadata using server mode
    ///
    /// This is the optimized version that matches our combined extraction approach.
    pub async fn extract_with_server(
        &self,
        content: &str,
        from_format: &str,
    ) -> Result<(String, HashMap<String, Value>)> {
        // Use the existing JSON extraction + parsing logic
        let json_output = self.convert(content, from_format, "json").await?;

        let json_data: Value = serde_json::from_str(&json_output)
            .map_err(|e| KreuzbergError::parsing(format!("Failed to parse JSON from server: {}", e)))?;

        // Reuse existing extraction functions from subprocess.rs
        let content = super::subprocess::extract_content_from_json(&json_data)?;
        let metadata = super::subprocess::extract_metadata_from_json(&json_data)?;

        Ok((content, metadata))
    }
}

impl Drop for PandocServer {
    fn drop(&mut self) {
        // Best-effort cleanup on drop
        if let Some(symlink) = &self.server_symlink {
            let _ = std::fs::remove_file(symlink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_pandoc() {
        let result = PandocServer::find_pandoc().await;
        if result.is_ok() {
            let path = result.unwrap();
            assert!(path.exists());
            assert!(path.ends_with("pandoc"));
        }
    }

    #[tokio::test]
    async fn test_server_lifecycle() {
        let server = PandocServer::new(Some(3032), Some(120)).await;
        if server.is_err() {
            // Pandoc not installed, skip test
            return;
        }

        let server = server.unwrap();

        // Start server
        let start_result = server.start().await;
        if start_result.is_err() {
            // Server mode not available, skip
            return;
        }

        assert!(server.is_running().await);

        // Health check
        let version = server.health_check().await;
        assert!(version.is_ok());

        // Stop server
        let stop_result = server.stop().await;
        assert!(stop_result.is_ok());
        assert!(!server.is_running().await);
    }

    #[tokio::test]
    async fn test_server_conversion() {
        let server = PandocServer::new(Some(3033), Some(120)).await;
        if server.is_err() {
            return;
        }

        let server = server.unwrap();

        if server.start().await.is_err() {
            return;
        }

        // Test conversion
        let result = server
            .convert("# Hello World\n\nTest paragraph.", "markdown", "html")
            .await;

        // Cleanup
        let _ = server.stop().await;

        if let Ok(html) = result {
            assert!(html.contains("Hello World"));
            assert!(html.contains("Test paragraph"));
        }
    }
}
