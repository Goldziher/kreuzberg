//! Model Context Protocol (MCP) server implementation.
//!
//! Provides an MCP server that exposes Kreuzberg's document extraction
//! capabilities as MCP tools for integration with AI assistants.
//!
//! # Features
//!
//! - **extract_file**: Extract content from a file by path
//! - **extract_bytes**: Extract content from base64-encoded bytes
//! - **batch_extract_files**: Extract content from multiple files in parallel
//! - **detect_mime_type**: Detect MIME type of a file
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::mcp::start_mcp_server;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     start_mcp_server().await?;
//!     Ok(())
//! }
//! ```

mod server;

pub use server::{start_mcp_server, start_mcp_server_with_config};

pub use server::{BatchExtractFilesParams, DetectMimeTypeParams, ExtractBytesParams, ExtractFileParams, KreuzbergMcp};

#[doc(hidden)]
pub use server::map_kreuzberg_error_to_mcp;
