//! MCP server implementation for Kreuzberg.
//!
//! This module provides the core MCP server that exposes document extraction
//! as tools for AI assistants via the Model Context Protocol.

use base64::prelude::*;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::{
    ExtractionConfig, ExtractionResult as KreuzbergResult, batch_extract_file, batch_extract_file_sync, cache,
    detect_mime_type, extract_bytes, extract_bytes_sync, extract_file, extract_file_sync,
};

/// Request parameters for file extraction.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExtractFileParams {
    /// Path to the file to extract
    pub path: String,
    /// Optional MIME type hint (auto-detected if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Enable OCR for scanned documents
    #[serde(default)]
    pub enable_ocr: bool,
    /// Force OCR even if text extraction succeeds
    #[serde(default)]
    pub force_ocr: bool,
    /// Use async extraction (default: false for sync)
    #[serde(default)]
    pub r#async: bool,
}

/// Request parameters for bytes extraction.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExtractBytesParams {
    /// Base64-encoded file content
    pub data: String,
    /// Optional MIME type hint (auto-detected if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Enable OCR for scanned documents
    #[serde(default)]
    pub enable_ocr: bool,
    /// Force OCR even if text extraction succeeds
    #[serde(default)]
    pub force_ocr: bool,
    /// Use async extraction (default: false for sync)
    #[serde(default)]
    pub r#async: bool,
}

/// Request parameters for batch file extraction.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BatchExtractFilesParams {
    /// Paths to files to extract
    pub paths: Vec<String>,
    /// Enable OCR for scanned documents
    #[serde(default)]
    pub enable_ocr: bool,
    /// Force OCR even if text extraction succeeds
    #[serde(default)]
    pub force_ocr: bool,
    /// Use async extraction (default: false for sync)
    #[serde(default)]
    pub r#async: bool,
}

/// Request parameters for MIME type detection.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DetectMimeTypeParams {
    /// Path to the file
    pub path: String,
    /// Use content-based detection (default: true)
    #[serde(default = "default_use_content")]
    pub use_content: bool,
}

fn default_use_content() -> bool {
    true
}

/// Kreuzberg MCP server.
///
/// Provides document extraction capabilities via MCP tools.
///
/// The server loads a default extraction configuration from kreuzberg.toml/yaml/json
/// via discovery. Per-request OCR settings override the defaults.
#[derive(Clone)]
pub struct KreuzbergMcp {
    tool_router: ToolRouter<KreuzbergMcp>,
    /// Default extraction configuration loaded from config file via discovery
    default_config: std::sync::Arc<ExtractionConfig>,
}

#[tool_router]
impl KreuzbergMcp {
    /// Create a new Kreuzberg MCP server instance with default config.
    ///
    /// Uses `ExtractionConfig::discover()` to search for kreuzberg.toml/yaml/json
    /// in current and parent directories. Falls back to default configuration if
    /// no config file is found.
    pub fn new() -> crate::Result<Self> {
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

        Ok(Self::with_config(config))
    }

    /// Create a new Kreuzberg MCP server instance with explicit config.
    ///
    /// # Arguments
    ///
    /// * `config` - Default extraction configuration for all tool calls
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self {
            tool_router: Self::tool_router(),
            default_config: std::sync::Arc::new(config),
        }
    }

    /// Extract content from a file.
    ///
    /// This tool extracts text, metadata, and tables from documents in various formats
    /// including PDFs, Word documents, Excel spreadsheets, images (with OCR), and more.
    #[tool(
        description = "Extract content from a file by path. Supports PDFs, Word, Excel, images (with OCR), HTML, and more."
    )]
    async fn extract_file(
        &self,
        Parameters(params): Parameters<ExtractFileParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = build_config(&self.default_config, params.enable_ocr, params.force_ocr);

        let result = if params.r#async {
            extract_file(&params.path, params.mime_type.as_deref(), &config)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            extract_file_sync(&params.path, params.mime_type.as_deref(), &config)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        let response = format_extraction_result(&result);
        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Extract content from base64-encoded bytes.
    ///
    /// This tool extracts text, metadata, and tables from base64-encoded document data.
    #[tool(
        description = "Extract content from base64-encoded file data. Returns extracted text, metadata, and tables."
    )]
    async fn extract_bytes(
        &self,
        Parameters(params): Parameters<ExtractBytesParams>,
    ) -> Result<CallToolResult, McpError> {
        let bytes = BASE64_STANDARD
            .decode(&params.data)
            .map_err(|e| McpError::invalid_params(format!("Invalid base64: {}", e), None))?;

        let config = build_config(&self.default_config, params.enable_ocr, params.force_ocr);

        let mime_type = params.mime_type.as_deref().unwrap_or("");

        let result = if params.r#async {
            extract_bytes(&bytes, mime_type, &config)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            extract_bytes_sync(&bytes, mime_type, &config).map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        let response = format_extraction_result(&result);
        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Extract content from multiple files in parallel.
    ///
    /// This tool efficiently processes multiple documents simultaneously, useful for batch operations.
    #[tool(description = "Extract content from multiple files in parallel. Returns results for all files.")]
    async fn batch_extract_files(
        &self,
        Parameters(params): Parameters<BatchExtractFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = build_config(&self.default_config, params.enable_ocr, params.force_ocr);

        let results = if params.r#async {
            batch_extract_file(params.paths.clone(), &config)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            batch_extract_file_sync(params.paths.clone(), &config)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        let mut response = String::new();
        for (i, result) in results.iter().enumerate() {
            response.push_str(&format!("=== Document {}: {} ===\n", i + 1, params.paths[i]));
            response.push_str(&format_extraction_result(result));
            response.push_str("\n\n");
        }

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Detect the MIME type of a file.
    ///
    /// This tool identifies the file format, useful for determining which extractor to use.
    #[tool(description = "Detect the MIME type of a file. Returns the detected MIME type string.")]
    fn detect_mime_type(
        &self,
        Parameters(params): Parameters<DetectMimeTypeParams>,
    ) -> Result<CallToolResult, McpError> {
        let mime_type = detect_mime_type(&params.path, params.use_content)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(mime_type)]))
    }

    /// Get cache statistics.
    ///
    /// This tool returns statistics about the cache including total files, size, and disk space.
    #[tool(description = "Get cache statistics including total files, size, and available disk space.")]
    fn cache_stats(&self, Parameters(_): Parameters<()>) -> Result<CallToolResult, McpError> {
        let cache_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".kreuzberg");

        let stats = cache::get_cache_metadata(cache_dir.to_str().unwrap_or("."))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = format!(
            "Cache Statistics\n\
             ================\n\
             Directory: {}\n\
             Total files: {}\n\
             Total size: {:.2} MB\n\
             Available space: {:.2} MB\n\
             Oldest file age: {:.2} days\n\
             Newest file age: {:.2} days",
            cache_dir.to_string_lossy(),
            stats.total_files,
            stats.total_size_mb,
            stats.available_space_mb,
            stats.oldest_file_age_days,
            stats.newest_file_age_days
        );

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    /// Clear the cache.
    ///
    /// This tool removes all cached files and returns the number of files removed and space freed.
    #[tool(description = "Clear all cached files. Returns the number of files removed and space freed in MB.")]
    fn cache_clear(&self, Parameters(_): Parameters<()>) -> Result<CallToolResult, McpError> {
        let cache_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".kreuzberg");

        let (removed_files, freed_mb) = cache::clear_cache_directory(cache_dir.to_str().unwrap_or("."))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = format!(
            "Cache cleared successfully\n\
             Directory: {}\n\
             Removed files: {}\n\
             Freed space: {:.2} MB",
            cache_dir.to_string_lossy(),
            removed_files,
            freed_mb
        );

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }
}

#[tool_handler]
impl ServerHandler for KreuzbergMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "kreuzberg-mcp".to_string(),
                title: Some("Kreuzberg Document Intelligence MCP Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://goldziher.github.io/kreuzberg/".to_string()),
            },
            instructions: Some(
                "Extract content from documents in various formats. Supports PDFs, Word documents, \
                 Excel spreadsheets, images (with OCR), HTML, emails, and more. Use enable_ocr=true \
                 for scanned documents, force_ocr=true to always use OCR even if text extraction \
                 succeeds."
                    .to_string(),
            ),
        }
    }
}

impl Default for KreuzbergMcp {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::warn!("Failed to discover config, using default: {}", e);
            Self::with_config(ExtractionConfig::default())
        })
    }
}

/// Start the Kreuzberg MCP server.
///
/// This function initializes and runs the MCP server using stdio transport.
/// It will block until the server is shut down.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a fatal error.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::mcp::start_mcp_server;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     start_mcp_server().await?;
///     Ok(())
/// }
/// ```
pub async fn start_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let service = KreuzbergMcp::new()?.serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

/// Build extraction config from MCP parameters.
///
/// Starts with the default config and overlays OCR settings from request parameters.
fn build_config(default_config: &ExtractionConfig, enable_ocr: bool, force_ocr: bool) -> ExtractionConfig {
    let mut config = default_config.clone();

    config.ocr = if enable_ocr {
        Some(crate::OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        })
    } else {
        None
    };
    config.force_ocr = force_ocr;

    config
}

/// Format extraction result as human-readable text.
fn format_extraction_result(result: &KreuzbergResult) -> String {
    let mut response = String::new();

    response.push_str(&format!("Content ({} characters):\n", result.content.len()));
    response.push_str(&result.content);
    response.push_str("\n\n");

    response.push_str("Metadata:\n");
    response.push_str(&serde_json::to_string_pretty(&result.metadata).unwrap_or_default());
    response.push_str("\n\n");

    if !result.tables.is_empty() {
        response.push_str(&format!("Tables ({}):\n", result.tables.len()));
        for (i, table) in result.tables.iter().enumerate() {
            response.push_str(&format!("\nTable {} (page {}):\n", i + 1, table.page_number));
            response.push_str(&table.markdown);
            response.push('\n');
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_router_has_routes() {
        let router = KreuzbergMcp::tool_router();
        assert!(router.has_route("extract_file"));
        assert!(router.has_route("extract_bytes"));
        assert!(router.has_route("batch_extract_files"));
        assert!(router.has_route("detect_mime_type"));
        assert!(router.has_route("cache_stats"));
        assert!(router.has_route("cache_clear"));

        let tools = router.list_all();
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn test_server_info() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_build_config() {
        let default_config = ExtractionConfig::default();

        let config = build_config(&default_config, false, false);
        assert!(config.ocr.is_none());
        assert!(!config.force_ocr);

        let config = build_config(&default_config, true, false);
        assert!(config.ocr.is_some());
        assert!(!config.force_ocr);

        let config = build_config(&default_config, true, true);
        assert!(config.ocr.is_some());
        assert!(config.force_ocr);
    }
}
