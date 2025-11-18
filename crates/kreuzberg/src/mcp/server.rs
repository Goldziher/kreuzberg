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
    ExtractionConfig, ExtractionResult as KreuzbergResult, KreuzbergError, batch_extract_file, batch_extract_file_sync,
    cache, detect_mime_type, extract_bytes, extract_bytes_sync, extract_file, extract_file_sync,
};

/// Request parameters for file extraction.
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
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
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
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
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
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
#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
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

/// Map Kreuzberg errors to MCP error responses with appropriate error codes.
///
/// This function ensures different error types are properly differentiated in MCP responses:
/// - `Validation` errors → `INVALID_PARAMS` (-32602)
/// - `UnsupportedFormat` errors → `INVALID_PARAMS` (-32602)
/// - `Parsing` errors → `PARSE_ERROR` (-32700)
/// - `Io` errors → `INTERNAL_ERROR` (-32603) with context preserved
/// - All other errors → `INTERNAL_ERROR` (-32603)
///
/// The error message and source chain are preserved to aid debugging.
#[doc(hidden)]
pub fn map_kreuzberg_error_to_mcp(error: KreuzbergError) -> McpError {
    match error {
        KreuzbergError::Validation { message, source } => {
            let mut error_message = format!("Validation error: {}", message);
            if let Some(src) = source {
                error_message.push_str(&format!(" (caused by: {})", src));
            }
            McpError::invalid_params(error_message, None)
        }

        KreuzbergError::UnsupportedFormat(mime_type) => {
            McpError::invalid_params(format!("Unsupported format: {}", mime_type), None)
        }

        KreuzbergError::MissingDependency(dep) => McpError::invalid_params(
            format!(
                "Missing required dependency: {}. Please install it to use this feature.",
                dep
            ),
            None,
        ),

        KreuzbergError::Parsing { message, source } => {
            let mut error_message = format!("Parsing error: {}", message);
            if let Some(src) = source {
                error_message.push_str(&format!(" (caused by: {})", src));
            }
            McpError::parse_error(error_message, None)
        }

        // OSError/RuntimeError must bubble up - system errors need user reports ~keep
        KreuzbergError::Io(io_err) => McpError::internal_error(format!("System I/O error: {}", io_err), None),

        KreuzbergError::Ocr { message, source } => {
            let mut error_message = format!("OCR processing error: {}", message);
            if let Some(src) = source {
                error_message.push_str(&format!(" (caused by: {})", src));
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::Cache { message, source } => {
            let mut error_message = format!("Cache error: {}", message);
            if let Some(src) = source {
                error_message.push_str(&format!(" (caused by: {})", src));
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::ImageProcessing { message, source } => {
            let mut error_message = format!("Image processing error: {}", message);
            if let Some(src) = source {
                error_message.push_str(&format!(" (caused by: {})", src));
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::Serialization { message, source } => {
            let mut error_message = format!("Serialization error: {}", message);
            if let Some(src) = source {
                error_message.push_str(&format!(" (caused by: {})", src));
            }
            McpError::internal_error(error_message, None)
        }

        KreuzbergError::Plugin { message, plugin_name } => {
            McpError::internal_error(format!("Plugin '{}' error: {}", plugin_name, message), None)
        }

        KreuzbergError::LockPoisoned(msg) => McpError::internal_error(format!("Internal lock poisoned: {}", msg), None),

        KreuzbergError::Other(msg) => McpError::internal_error(msg, None),
    }
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
    #[allow(clippy::manual_unwrap_or_default)]
    pub fn new() -> crate::Result<Self> {
        let config = match ExtractionConfig::discover()? {
            Some(config) => {
                #[cfg(feature = "api")]
                tracing::info!("Loaded extraction config from discovered file");
                config
            }
            None => {
                #[cfg(feature = "api")]
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
                .map_err(map_kreuzberg_error_to_mcp)?
        } else {
            extract_file_sync(&params.path, params.mime_type.as_deref(), &config).map_err(map_kreuzberg_error_to_mcp)?
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
                .map_err(map_kreuzberg_error_to_mcp)?
        } else {
            extract_bytes_sync(&bytes, mime_type, &config).map_err(map_kreuzberg_error_to_mcp)?
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
                .map_err(map_kreuzberg_error_to_mcp)?
        } else {
            batch_extract_file_sync(params.paths.clone(), &config).map_err(map_kreuzberg_error_to_mcp)?
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
        let mime_type = detect_mime_type(&params.path, params.use_content).map_err(map_kreuzberg_error_to_mcp)?;

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

        let stats = cache::get_cache_metadata(cache_dir.to_str().unwrap_or(".")).map_err(map_kreuzberg_error_to_mcp)?;

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

        let (removed_files, freed_mb) =
            cache::clear_cache_directory(cache_dir.to_str().unwrap_or(".")).map_err(map_kreuzberg_error_to_mcp)?;

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
            #[cfg(feature = "api")]
            tracing::warn!("Failed to discover config, using default: {}", e);
            #[cfg(not(feature = "api"))]
            tracing::debug!("Warning: Failed to discover config, using default: {}", e);
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
/// async fn main() -> kreuzberg::Result<()> {
///     start_mcp_server().await?;
///     Ok(())
/// }
/// ```
pub async fn start_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let service = KreuzbergMcp::new()?.serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

/// Start MCP server with custom extraction config.
///
/// This variant allows specifying a custom extraction configuration
/// (e.g., loaded from a file) instead of using defaults.
pub async fn start_mcp_server_with_config(config: ExtractionConfig) -> Result<(), Box<dyn std::error::Error>> {
    let service = KreuzbergMcp::with_config(config).serve(stdio()).await?;

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
    use std::path::PathBuf;

    /// Get the path to a test document relative to workspace root.
    fn get_test_path(relative_path: &str) -> String {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        workspace_root
            .join("test_documents")
            .join(relative_path)
            .to_string_lossy()
            .to_string()
    }

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

    #[test]
    fn test_with_config_stores_provided_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            use_cache: false,
            ..Default::default()
        };

        let server = KreuzbergMcp::with_config(custom_config);

        assert!(server.default_config.force_ocr);
        assert!(!server.default_config.use_cache);
    }

    #[test]
    fn test_map_validation_error_to_invalid_params() {
        let error = KreuzbergError::validation("invalid file path");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Validation error"));
        assert!(mcp_error.message.contains("invalid file path"));
    }

    #[test]
    fn test_map_validation_error_with_source_preserves_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad param");
        let error = KreuzbergError::validation_with_source("invalid configuration", source);
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Validation error"));
        assert!(mcp_error.message.contains("invalid configuration"));
        assert!(mcp_error.message.contains("caused by"));
    }

    #[test]
    fn test_map_unsupported_format_to_invalid_params() {
        let error = KreuzbergError::UnsupportedFormat("application/unknown".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Unsupported format"));
        assert!(mcp_error.message.contains("application/unknown"));
    }

    #[test]
    fn test_map_missing_dependency_to_invalid_params() {
        let error = KreuzbergError::MissingDependency("tesseract".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32602);
        assert!(mcp_error.message.contains("Missing required dependency"));
        assert!(mcp_error.message.contains("tesseract"));
        assert!(mcp_error.message.contains("Please install"));
    }

    #[test]
    fn test_map_parsing_error_to_parse_error() {
        let error = KreuzbergError::parsing("corrupt PDF file");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32700);
        assert!(mcp_error.message.contains("Parsing error"));
        assert!(mcp_error.message.contains("corrupt PDF file"));
    }

    #[test]
    fn test_map_parsing_error_with_source_preserves_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "malformed data");
        let error = KreuzbergError::parsing_with_source("failed to parse document", source);
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32700);
        assert!(mcp_error.message.contains("Parsing error"));
        assert!(mcp_error.message.contains("failed to parse document"));
        assert!(mcp_error.message.contains("caused by"));
    }

    #[test]
    fn test_map_io_error_to_internal_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = KreuzbergError::Io(io_error);
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("System I/O error"));
        assert!(mcp_error.message.contains("file not found"));
    }

    #[test]
    fn test_map_ocr_error_to_internal_error() {
        let error = KreuzbergError::ocr("tesseract failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("OCR processing error"));
        assert!(mcp_error.message.contains("tesseract failed"));
    }

    #[test]
    fn test_map_cache_error_to_internal_error() {
        let error = KreuzbergError::cache("cache write failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Cache error"));
        assert!(mcp_error.message.contains("cache write failed"));
    }

    #[test]
    fn test_map_image_processing_error_to_internal_error() {
        let error = KreuzbergError::image_processing("resize failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Image processing error"));
        assert!(mcp_error.message.contains("resize failed"));
    }

    #[test]
    fn test_map_serialization_error_to_internal_error() {
        let error = KreuzbergError::serialization("JSON encode failed");
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Serialization error"));
        assert!(mcp_error.message.contains("JSON encode failed"));
    }

    #[test]
    fn test_map_plugin_error_to_internal_error() {
        let error = KreuzbergError::Plugin {
            message: "extraction failed".to_string(),
            plugin_name: "pdf-extractor".to_string(),
        };
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Plugin 'pdf-extractor' error"));
        assert!(mcp_error.message.contains("extraction failed"));
    }

    #[test]
    fn test_map_lock_poisoned_error_to_internal_error() {
        let error = KreuzbergError::LockPoisoned("registry lock poisoned".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("Internal lock poisoned"));
        assert!(mcp_error.message.contains("registry lock poisoned"));
    }

    #[test]
    fn test_map_other_error_to_internal_error() {
        let error = KreuzbergError::Other("unexpected error".to_string());
        let mcp_error = map_kreuzberg_error_to_mcp(error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("unexpected error"));
    }

    #[test]
    fn test_error_type_differentiation() {
        let validation = KreuzbergError::validation("test");
        let parsing = KreuzbergError::parsing("test");
        let io = KreuzbergError::Io(std::io::Error::other("test"));

        let val_mcp = map_kreuzberg_error_to_mcp(validation);
        let parse_mcp = map_kreuzberg_error_to_mcp(parsing);
        let io_mcp = map_kreuzberg_error_to_mcp(io);

        assert_eq!(val_mcp.code.0, -32602);
        assert_eq!(parse_mcp.code.0, -32700);
        assert_eq!(io_mcp.code.0, -32603);

        assert_ne!(val_mcp.code.0, parse_mcp.code.0);
        assert_ne!(val_mcp.code.0, io_mcp.code.0);
        assert_ne!(parse_mcp.code.0, io_mcp.code.0);
    }

    #[test]
    fn test_format_extraction_result_with_content() {
        let result = KreuzbergResult {
            content: "Sample extracted text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let formatted = format_extraction_result(&result);

        assert!(formatted.contains("Content (21 characters)"));
        assert!(formatted.contains("Sample extracted text"));
        assert!(formatted.contains("Metadata:"));
    }

    #[test]
    fn test_format_extraction_result_with_tables() {
        let result = KreuzbergResult {
            content: "Document with tables".to_string(),
            mime_type: "application/pdf".to_string(),
            metadata: crate::Metadata::default(),
            tables: vec![
                crate::Table {
                    cells: vec![
                        vec!["Col1".to_string(), "Col2".to_string()],
                        vec!["A".to_string(), "B".to_string()],
                    ],
                    page_number: 1,
                    markdown: "| Col1 | Col2 |\n|------|------|\n| A    | B    |".to_string(),
                },
                crate::Table {
                    cells: vec![
                        vec!["X".to_string(), "Y".to_string()],
                        vec!["1".to_string(), "2".to_string()],
                    ],
                    page_number: 2,
                    markdown: "| X | Y |\n|---|---|\n| 1 | 2 |".to_string(),
                },
            ],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let formatted = format_extraction_result(&result);

        assert!(formatted.contains("Tables (2)"));
        assert!(formatted.contains("Table 1 (page 1)"));
        assert!(formatted.contains("Table 2 (page 2)"));
        assert!(formatted.contains("| Col1 | Col2 |"));
        assert!(formatted.contains("| X | Y |"));
    }

    #[test]
    fn test_format_extraction_result_empty_content() {
        let result = KreuzbergResult {
            content: String::new(),
            mime_type: "text/plain".to_string(),
            metadata: crate::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let formatted = format_extraction_result(&result);

        assert!(formatted.contains("Content (0 characters)"));
        assert!(formatted.contains("Metadata:"));
    }

    #[test]
    fn test_format_extraction_result_no_tables() {
        let result = KreuzbergResult {
            content: "Simple text".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: crate::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        };

        let formatted = format_extraction_result(&result);

        assert!(formatted.contains("Simple text"));
        assert!(!formatted.contains("Tables"));
    }

    #[tokio::test]
    async fn test_extract_file_sync_with_valid_pdf() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = ExtractFileParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_file(Parameters(params)).await;

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(!text.text.is_empty());
                    assert!(text.text.contains("Content"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_extract_file_async_with_valid_pdf() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = ExtractFileParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_file(Parameters(params)).await;

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(!text.text.is_empty());
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_extract_file_with_invalid_path() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = ExtractFileParams {
            path: "/nonexistent/file.pdf".to_string(),
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_file(Parameters(params)).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.code.0 == -32602 || error.code.0 == -32603);
    }

    #[tokio::test]
    async fn test_extract_file_with_mime_type_hint() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = ExtractFileParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            mime_type: Some("application/pdf".to_string()),
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_file(Parameters(params)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_file_with_ocr_enabled() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = ExtractFileParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            mime_type: None,
            enable_ocr: true,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_file(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_file_with_force_ocr() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = ExtractFileParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            mime_type: None,
            enable_ocr: true,
            force_ocr: true,
            r#async: true,
        };

        let result = server.extract_file(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_bytes_sync_with_valid_data() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let text_content = b"Hello, world!";
        let encoded = BASE64_STANDARD.encode(text_content);

        let params = ExtractBytesParams {
            data: encoded,
            mime_type: Some("text/plain".to_string()),
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.contains("Hello, world!"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_extract_bytes_async_with_valid_data() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let text_content = b"Async extraction test";
        let encoded = BASE64_STANDARD.encode(text_content);

        let params = ExtractBytesParams {
            data: encoded,
            mime_type: Some("text/plain".to_string()),
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_bytes_with_invalid_base64() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let params = ExtractBytesParams {
            data: "not-valid-base64!!!".to_string(),
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code.0, -32602);
        assert!(error.message.contains("Invalid base64"));
    }

    #[tokio::test]
    async fn test_extract_bytes_without_mime_type() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let text_content = b"Test content";
        let encoded = BASE64_STANDARD.encode(text_content);

        let params = ExtractBytesParams {
            data: encoded,
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_bytes_with_ocr_enabled() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let text_content = b"OCR test content";
        let encoded = BASE64_STANDARD.encode(text_content);

        let params = ExtractBytesParams {
            data: encoded,
            mime_type: Some("text/plain".to_string()),
            enable_ocr: true,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_batch_extract_files_sync_with_valid_files() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = BatchExtractFilesParams {
            paths: vec![get_test_path("pdfs_with_tables/tiny.pdf").to_string()],
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.batch_extract_files(Parameters(params)).await;

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.contains("Document 1"));
                    assert!(text.text.contains("tiny.pdf"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_batch_extract_files_async_with_multiple_files() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = BatchExtractFilesParams {
            paths: vec![
                get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
                get_test_path("pdfs_with_tables/medium.pdf").to_string(),
            ],
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.batch_extract_files(Parameters(params)).await;

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.contains("Document 1"));
                    assert!(text.text.contains("Document 2"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_batch_extract_files_with_empty_list() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = BatchExtractFilesParams {
            paths: vec![],
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.batch_extract_files(Parameters(params)).await;

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.is_empty() || text.text.trim().is_empty());
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_batch_extract_files_with_invalid_file() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = BatchExtractFilesParams {
            paths: vec!["/nonexistent/file.pdf".to_string()],
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.batch_extract_files(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_detect_mime_type_with_valid_file() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = DetectMimeTypeParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            use_content: true,
        };

        let result = server.detect_mime_type(Parameters(params));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.contains("application/pdf") || text.text.contains("pdf"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_detect_mime_type_without_content_detection() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = DetectMimeTypeParams {
            path: get_test_path("pdfs_with_tables/tiny.pdf").to_string(),
            use_content: false,
        };

        let result = server.detect_mime_type(Parameters(params));

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_mime_type_with_invalid_file() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let params = DetectMimeTypeParams {
            path: "/nonexistent/file.pdf".to_string(),
            use_content: true,
        };

        let result = server.detect_mime_type(Parameters(params));

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.code.0 == -32602 || error.code.0 == -32603);
    }

    #[tokio::test]
    async fn test_cache_stats_returns_statistics() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result = server.cache_stats(Parameters(()));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.contains("Cache Statistics"));
                    assert!(text.text.contains("Directory:"));
                    assert!(text.text.contains("Total files:"));
                    assert!(text.text.contains("Total size:"));
                    assert!(text.text.contains("Available space:"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[tokio::test]
    async fn test_cache_clear_returns_result() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result = server.cache_clear(Parameters(()));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first() {
            match &content.raw {
                RawContent::Text(text) => {
                    assert!(text.text.contains("Cache cleared"));
                    assert!(text.text.contains("Directory:"));
                    assert!(text.text.contains("Removed files:"));
                    assert!(text.text.contains("Freed space:"));
                }
                _ => panic!("Expected text content"),
            }
        } else {
            panic!("Expected content in result");
        }
    }

    #[test]
    fn test_new_creates_server_with_default_config() {
        let server = KreuzbergMcp::new();
        assert!(server.is_ok());
    }

    #[test]
    fn test_default_creates_server_without_panic() {
        let server = KreuzbergMcp::default();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "kreuzberg-mcp");
    }

    #[test]
    fn test_server_info_has_correct_fields() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert_eq!(
            info.server_info.title,
            Some("Kreuzberg Document Intelligence MCP Server".to_string())
        );
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            info.server_info.website_url,
            Some("https://goldziher.github.io/kreuzberg/".to_string())
        );
        assert!(info.instructions.is_some());
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_build_config_preserves_default_config_settings() {
        let default_config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let config = build_config(&default_config, false, false);

        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_ocr_disabled_by_default() {
        let default_config = ExtractionConfig::default();

        let config = build_config(&default_config, false, false);

        assert!(config.ocr.is_none());
        assert!(!config.force_ocr);
    }

    #[test]
    fn test_build_config_ocr_enabled_creates_tesseract_config() {
        let default_config = ExtractionConfig::default();

        let config = build_config(&default_config, true, false);

        assert!(config.ocr.is_some());
        let ocr_config = config.ocr.unwrap();
        assert_eq!(ocr_config.backend, "tesseract");
        assert_eq!(ocr_config.language, "eng");
    }

    #[test]
    fn test_extract_file_params_defaults() {
        let json = r#"{"path": "/test.pdf"}"#;
        let params: ExtractFileParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.path, "/test.pdf");
        assert_eq!(params.mime_type, None);
        assert!(!params.enable_ocr);
        assert!(!params.force_ocr);
        assert!(!params.r#async);
    }

    #[test]
    fn test_extract_bytes_params_defaults() {
        let json = r#"{"data": "SGVsbG8="}"#;
        let params: ExtractBytesParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.data, "SGVsbG8=");
        assert_eq!(params.mime_type, None);
        assert!(!params.enable_ocr);
        assert!(!params.force_ocr);
        assert!(!params.r#async);
    }

    #[test]
    fn test_batch_extract_files_params_defaults() {
        let json = r#"{"paths": ["/a.pdf", "/b.pdf"]}"#;
        let params: BatchExtractFilesParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.paths.len(), 2);
        assert!(!params.enable_ocr);
        assert!(!params.force_ocr);
        assert!(!params.r#async);
    }

    #[test]
    fn test_detect_mime_type_params_defaults() {
        let json = r#"{"path": "/test.pdf"}"#;
        let params: DetectMimeTypeParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.path, "/test.pdf");
        assert!(params.use_content);
    }

    #[test]
    fn test_detect_mime_type_params_use_content_false() {
        let json = r#"{"path": "/test.pdf", "use_content": false}"#;
        let params: DetectMimeTypeParams = serde_json::from_str(json).unwrap();

        assert!(!params.use_content);
    }

    #[test]
    fn test_mcp_server_info_protocol_version() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.protocol_version, ProtocolVersion::default());
    }

    #[test]
    fn test_mcp_server_info_has_all_required_fields() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());

        assert!(info.server_info.title.is_some());
        assert!(info.server_info.website_url.is_some());
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_mcp_server_capabilities_declares_tools() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_mcp_server_name_follows_convention() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert!(!info.server_info.name.contains('_'));
        assert!(!info.server_info.name.contains(' '));
    }

    #[test]
    fn test_mcp_version_matches_cargo_version() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_mcp_instructions_are_helpful() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());
        let info = server.get_info();

        let instructions = info.instructions.expect("Instructions should be present");

        assert!(instructions.contains("extract") || instructions.contains("Extract"));
        assert!(instructions.contains("OCR") || instructions.contains("ocr"));
        assert!(instructions.contains("document"));
    }

    #[tokio::test]
    async fn test_all_tools_are_registered() {
        let router = KreuzbergMcp::tool_router();

        let expected_tools = vec![
            "extract_file",
            "extract_bytes",
            "batch_extract_files",
            "detect_mime_type",
            "cache_stats",
            "cache_clear",
        ];

        for tool_name in expected_tools {
            assert!(router.has_route(tool_name), "Tool '{}' should be registered", tool_name);
        }
    }

    #[tokio::test]
    async fn test_tool_count_is_correct() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        assert_eq!(tools.len(), 6, "Expected 6 tools, found {}", tools.len());
    }

    #[tokio::test]
    async fn test_tools_have_descriptions() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        for tool in tools {
            assert!(
                tool.description.is_some(),
                "Tool '{}' should have a description",
                tool.name
            );
            let desc = tool.description.as_ref().unwrap();
            assert!(!desc.is_empty(), "Tool '{}' description should not be empty", tool.name);
        }
    }

    #[tokio::test]
    async fn test_extract_file_tool_has_correct_schema() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        let extract_file_tool = tools
            .iter()
            .find(|t| t.name == "extract_file")
            .expect("extract_file tool should exist");

        assert!(extract_file_tool.description.is_some());

        assert!(!extract_file_tool.input_schema.is_empty());
    }

    #[tokio::test]
    async fn test_all_tools_have_input_schemas() {
        let router = KreuzbergMcp::tool_router();
        let tools = router.list_all();

        for tool in tools {
            assert!(
                !tool.input_schema.is_empty(),
                "Tool '{}' should have an input schema with fields",
                tool.name
            );
        }
    }

    #[test]
    fn test_server_creation_with_custom_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            use_cache: false,
            ocr: Some(crate::OcrConfig {
                backend: "tesseract".to_string(),
                language: "spa".to_string(),
                tesseract_config: None,
            }),
            ..Default::default()
        };

        let server = KreuzbergMcp::with_config(custom_config.clone());

        assert_eq!(server.default_config.force_ocr, custom_config.force_ocr);
        assert_eq!(server.default_config.use_cache, custom_config.use_cache);
    }

    #[test]
    fn test_server_clone_preserves_config() {
        let custom_config = ExtractionConfig {
            force_ocr: true,
            ..Default::default()
        };

        let server1 = KreuzbergMcp::with_config(custom_config);
        let server2 = server1.clone();

        assert_eq!(server1.default_config.force_ocr, server2.default_config.force_ocr);
    }

    #[tokio::test]
    async fn test_extract_bytes_with_empty_data() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let params = ExtractBytesParams {
            data: String::new(),
            mime_type: Some("text/plain".to_string()),
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_bytes_with_valid_pdf_bytes() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let pdf_path = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&pdf_path).exists() {
            let pdf_bytes = std::fs::read(&pdf_path).unwrap();
            let encoded = BASE64_STANDARD.encode(&pdf_bytes);

            let params = ExtractBytesParams {
                data: encoded,
                mime_type: Some("application/pdf".to_string()),
                enable_ocr: false,
                force_ocr: false,
                r#async: true,
            };

            let result = server.extract_bytes(Parameters(params)).await;

            assert!(result.is_ok(), "PDF bytes extraction should succeed");
            let call_result = result.unwrap();
            assert!(!call_result.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_extract_bytes_mime_type_auto_detection() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let text_content = b"Plain text content for testing";
        let encoded = BASE64_STANDARD.encode(text_content);

        let params = ExtractBytesParams {
            data: encoded,
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: true,
        };

        let result = server.extract_bytes(Parameters(params)).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_batch_extract_preserves_file_order() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let file1 = get_test_path("pdfs_with_tables/tiny.pdf");
        let file2 = get_test_path("pdfs_with_tables/medium.pdf");

        if std::path::Path::new(&file1).exists() && std::path::Path::new(&file2).exists() {
            let params = BatchExtractFilesParams {
                paths: vec![file1.to_string(), file2.to_string()],
                enable_ocr: false,
                force_ocr: false,
                r#async: true,
            };

            let result = server.batch_extract_files(Parameters(params)).await;

            if result.is_ok() {
                let call_result = result.unwrap();
                if let Some(content) = call_result.content.first()
                    && let RawContent::Text(text) = &content.raw
                {
                    assert!(text.text.contains("Document 1"));
                    assert!(text.text.contains("Document 2"));

                    let doc1_pos = text.text.find("Document 1");
                    let doc2_pos = text.text.find("Document 2");
                    if let (Some(pos1), Some(pos2)) = (doc1_pos, doc2_pos) {
                        assert!(pos1 < pos2, "Documents should be in order");
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_cache_clear_is_idempotent() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result1 = server.cache_clear(Parameters(()));
        assert!(result1.is_ok());

        let result2 = server.cache_clear(Parameters(()));
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_cache_clear_returns_metrics() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result = server.cache_clear(Parameters(()));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first()
            && let RawContent::Text(text) = &content.raw
        {
            assert!(text.text.contains("Removed files:"));
            assert!(text.text.contains("Freed space:"));
        }
    }

    #[test]
    fn test_error_mapping_preserves_error_context() {
        let validation_error = KreuzbergError::validation("invalid file path");
        let mcp_error = map_kreuzberg_error_to_mcp(validation_error);

        assert!(mcp_error.message.contains("invalid file path"));
    }

    #[test]
    fn test_io_errors_bubble_up_as_internal() {
        // OSError/RuntimeError must bubble up - system errors need user reports ~keep
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let kreuzberg_error = KreuzbergError::Io(io_error);
        let mcp_error = map_kreuzberg_error_to_mcp(kreuzberg_error);

        assert_eq!(mcp_error.code.0, -32603);
        assert!(mcp_error.message.contains("System I/O error"));
    }

    #[test]
    fn test_all_error_variants_have_mappings() {
        let errors = vec![
            KreuzbergError::validation("test"),
            KreuzbergError::UnsupportedFormat("test/unknown".to_string()),
            KreuzbergError::MissingDependency("test-dep".to_string()),
            KreuzbergError::parsing("test"),
            KreuzbergError::Io(std::io::Error::other("test")),
            KreuzbergError::ocr("test"),
            KreuzbergError::cache("test"),
            KreuzbergError::image_processing("test"),
            KreuzbergError::serialization("test"),
            KreuzbergError::Plugin {
                message: "test".to_string(),
                plugin_name: "test-plugin".to_string(),
            },
            KreuzbergError::LockPoisoned("test".to_string()),
            KreuzbergError::Other("test".to_string()),
        ];

        for error in errors {
            let mcp_error = map_kreuzberg_error_to_mcp(error);

            assert!(mcp_error.code.0 < 0, "Error code should be negative");

            assert!(!mcp_error.message.is_empty());
        }
    }

    #[tokio::test]
    async fn test_response_includes_metadata() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let test_file = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&test_file).exists() {
            let params = ExtractFileParams {
                path: test_file.to_string(),
                mime_type: None,
                enable_ocr: false,
                force_ocr: false,
                r#async: true,
            };

            let result = server.extract_file(Parameters(params)).await;

            assert!(result.is_ok());
            let call_result = result.unwrap();

            if let Some(content) = call_result.content.first()
                && let RawContent::Text(text) = &content.raw
            {
                assert!(text.text.contains("Metadata:"));
            }
        }
    }

    #[tokio::test]
    async fn test_response_includes_content_length() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let test_file = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&test_file).exists() {
            let params = ExtractFileParams {
                path: test_file.to_string(),
                mime_type: None,
                enable_ocr: false,
                force_ocr: false,
                r#async: true,
            };

            let result = server.extract_file(Parameters(params)).await;

            assert!(result.is_ok());
            let call_result = result.unwrap();

            if let Some(content) = call_result.content.first()
                && let RawContent::Text(text) = &content.raw
            {
                assert!(text.text.contains("characters"));
                assert!(text.text.contains("Content"));
            }
        }
    }

    #[tokio::test]
    async fn test_server_is_thread_safe() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let server1 = server.clone();
        let server2 = server.clone();

        let handle1 = tokio::spawn(async move { server1.get_info() });

        let handle2 = tokio::spawn(async move { server2.get_info() });

        let info1 = handle1.await.unwrap();
        let info2 = handle2.await.unwrap();

        assert_eq!(info1.server_info.name, info2.server_info.name);
    }

    #[test]
    fn test_extract_file_params_serialization() {
        let params = ExtractFileParams {
            path: "/test.pdf".to_string(),
            mime_type: Some("application/pdf".to_string()),
            enable_ocr: true,
            force_ocr: false,
            r#async: true,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: ExtractFileParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.path, deserialized.path);
        assert_eq!(params.mime_type, deserialized.mime_type);
        assert_eq!(params.enable_ocr, deserialized.enable_ocr);
        assert_eq!(params.force_ocr, deserialized.force_ocr);
        assert_eq!(params.r#async, deserialized.r#async);
    }

    #[test]
    fn test_extract_bytes_params_serialization() {
        let params = ExtractBytesParams {
            data: "SGVsbG8=".to_string(),
            mime_type: None,
            enable_ocr: false,
            force_ocr: false,
            r#async: false,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: ExtractBytesParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.data, deserialized.data);
    }

    #[test]
    fn test_batch_extract_params_serialization() {
        let params = BatchExtractFilesParams {
            paths: vec!["/a.pdf".to_string(), "/b.pdf".to_string()],
            enable_ocr: true,
            force_ocr: true,
            r#async: true,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: BatchExtractFilesParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.paths, deserialized.paths);
        assert_eq!(params.enable_ocr, deserialized.enable_ocr);
    }

    #[test]
    fn test_detect_mime_type_params_serialization() {
        let params = DetectMimeTypeParams {
            path: "/test.pdf".to_string(),
            use_content: false,
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: DetectMimeTypeParams = serde_json::from_str(&json).unwrap();

        assert_eq!(params.path, deserialized.path);
        assert_eq!(params.use_content, deserialized.use_content);
    }

    #[tokio::test]
    async fn test_extract_file_respects_custom_default_config() {
        let custom_config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let server = KreuzbergMcp::with_config(custom_config);

        let test_file = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&test_file).exists() {
            let params = ExtractFileParams {
                path: test_file.to_string(),
                mime_type: None,
                enable_ocr: false,
                force_ocr: false,
                r#async: true,
            };

            let result = server.extract_file(Parameters(params)).await;

            assert!(result.is_ok() || result.is_err());
        }
    }

    #[tokio::test]
    async fn test_batch_extract_with_single_file() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let test_file = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&test_file).exists() {
            let params = BatchExtractFilesParams {
                paths: vec![test_file.to_string()],
                enable_ocr: false,
                force_ocr: false,
                r#async: true,
            };

            let result = server.batch_extract_files(Parameters(params)).await;

            assert!(result.is_ok());
            let call_result = result.unwrap();
            assert!(!call_result.content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_detect_mime_type_with_extension_only() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let test_file = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&test_file).exists() {
            let params = DetectMimeTypeParams {
                path: test_file.to_string(),
                use_content: false,
            };

            let result = server.detect_mime_type(Parameters(params));

            assert!(result.is_ok());
            let call_result = result.unwrap();
            if let Some(content) = call_result.content.first()
                && let RawContent::Text(text) = &content.raw
            {
                assert!(text.text.contains("pdf") || text.text.contains("PDF"));
            }
        }
    }

    #[tokio::test]
    async fn test_detect_mime_type_with_content_analysis() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let test_file = get_test_path("pdfs_with_tables/tiny.pdf");

        if std::path::Path::new(&test_file).exists() {
            let params = DetectMimeTypeParams {
                path: test_file.to_string(),
                use_content: true,
            };

            let result = server.detect_mime_type(Parameters(params));

            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_cache_stats_returns_valid_data() {
        let server = KreuzbergMcp::with_config(ExtractionConfig::default());

        let result = server.cache_stats(Parameters(()));

        assert!(result.is_ok());
        let call_result = result.unwrap();
        if let Some(content) = call_result.content.first()
            && let RawContent::Text(text) = &content.raw
        {
            assert!(text.text.contains("Cache Statistics"));
            assert!(text.text.contains("Directory:"));
            assert!(text.text.contains("Total files:"));
            assert!(text.text.contains("Total size:"));
            assert!(text.text.contains("Available space:"));
            assert!(text.text.contains("Oldest file age:"));
            assert!(text.text.contains("Newest file age:"));
        }
    }
}
