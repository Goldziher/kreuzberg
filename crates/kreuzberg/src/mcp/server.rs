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
    ExtractionConfig, ExtractionResult as KreuzbergResult, batch_extract_file, batch_extract_file_sync,
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
#[derive(Clone)]
pub struct KreuzbergMcp {
    tool_router: ToolRouter<KreuzbergMcp>,
}

#[tool_router]
impl KreuzbergMcp {
    /// Create a new Kreuzberg MCP server instance.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
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
        let config = build_config(params.enable_ocr, params.force_ocr);

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
        // Decode base64
        let bytes = BASE64_STANDARD
            .decode(&params.data)
            .map_err(|e| McpError::invalid_params(format!("Invalid base64: {}", e), None))?;

        let config = build_config(params.enable_ocr, params.force_ocr);

        // extract_bytes requires a MIME type, use empty string as default for auto-detection
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
        let config = build_config(params.enable_ocr, params.force_ocr);

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
}

#[tool_handler]
impl ServerHandler for KreuzbergMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "kreuzberg-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: Some("Kreuzberg Document Intelligence".to_string()),
                website_url: Some("https://goldziher.github.io/kreuzberg/".to_string()),
            },
            instructions: Some(
                "Kreuzberg document intelligence MCP server. \
                 Tools: extract_file, extract_bytes, batch_extract_files, detect_mime_type. \
                 Extracts text, metadata, and tables from PDFs, Office docs, images (OCR), emails, and more."
                    .to_string(),
            ),
        }
    }
}

impl Default for KreuzbergMcp {
    fn default() -> Self {
        Self::new()
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
    let service = KreuzbergMcp::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build extraction config from MCP parameters.
fn build_config(enable_ocr: bool, force_ocr: bool) -> ExtractionConfig {
    ExtractionConfig {
        ocr: if enable_ocr {
            Some(crate::OcrConfig {
                backend: "tesseract".to_string(),
                language: "eng".to_string(),
            })
        } else {
            None
        },
        force_ocr,
        ..Default::default()
    }
}

/// Format extraction result as human-readable text.
fn format_extraction_result(result: &KreuzbergResult) -> String {
    let mut response = String::new();

    // Content
    response.push_str(&format!("Content ({} characters):\n", result.content.len()));
    response.push_str(&result.content);
    response.push_str("\n\n");

    // Metadata
    if !result.metadata.is_empty() {
        response.push_str("Metadata:\n");
        response.push_str(&serde_json::to_string_pretty(&result.metadata).unwrap_or_default());
        response.push_str("\n\n");
    }

    // Tables
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

        let tools = router.list_all();
        assert_eq!(tools.len(), 4);
    }

    #[test]
    fn test_server_info() {
        let server = KreuzbergMcp::new();
        let info = server.get_info();

        assert_eq!(info.server_info.name, "kreuzberg-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_build_config() {
        let config = build_config(false, false);
        assert!(config.ocr.is_none());
        assert!(!config.force_ocr);

        let config = build_config(true, false);
        assert!(config.ocr.is_some());
        assert!(!config.force_ocr);

        let config = build_config(true, true);
        assert!(config.ocr.is_some());
        assert!(config.force_ocr);
    }
}
