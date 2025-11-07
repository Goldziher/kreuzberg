//! Kreuzberg CLI - Command-line interface for document intelligence.
//!
//! This binary provides a command-line interface to the Kreuzberg document intelligence
//! library, supporting document extraction, MIME type detection, caching, and batch operations.
//!
//! # Architecture
//!
//! The CLI is built using `clap` for argument parsing and provides five main commands:
//! - `extract`: Extract text/data from a single document
//! - `batch`: Process multiple documents in parallel
//! - `detect`: Identify MIME type of a file
//! - `cache`: Manage cache (clear, stats)
//! - `serve`: Start API server (requires `api` feature)
//! - `version`: Show version information
//!
//! # Configuration
//!
//! The CLI supports configuration files in TOML, YAML, or JSON formats:
//! - Explicit: `--config path/to/config.toml`
//! - Auto-discovery: Searches for `kreuzberg.{toml,yaml,json}` in current and parent directories
//! - Command-line flags override config file settings
//!
//! # Exit Codes
//!
//! - 0: Success
//! - Non-zero: Error (see stderr for details)
//!
//! # Examples
//!
//! ```bash
//! # Extract text from a PDF
//! kreuzberg extract document.pdf
//!
//! # Extract with OCR enabled
//! kreuzberg extract scanned.pdf --ocr true
//!
//! # Batch processing
//! kreuzberg batch *.pdf --format json
//!
//! # Detect MIME type
//! kreuzberg detect unknown-file.bin
//! ```

#![deny(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kreuzberg::{
    ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, OcrConfig, batch_extract_file, batch_extract_file_sync,
    detect_mime_type, extract_file, extract_file_sync,
};
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

/// Kreuzberg document intelligence CLI
#[derive(Parser)]
#[command(name = "kreuzberg")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract text from a document
    Extract {
        /// Path to the document
        path: PathBuf,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// MIME type hint (auto-detected if not provided)
        #[arg(short, long)]
        mime_type: Option<String>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,

        /// Enable OCR (overrides config file)
        #[arg(long)]
        ocr: Option<bool>,

        /// Force OCR even if text extraction succeeds (overrides config file)
        #[arg(long)]
        force_ocr: Option<bool>,

        /// Disable caching (overrides config file)
        #[arg(long)]
        no_cache: Option<bool>,

        /// Enable chunking (overrides config file)
        #[arg(long)]
        chunk: Option<bool>,

        /// Chunk size in characters (overrides config file)
        #[arg(long)]
        chunk_size: Option<usize>,

        /// Chunk overlap in characters (overrides config file)
        #[arg(long)]
        chunk_overlap: Option<usize>,

        /// Enable quality processing (overrides config file)
        #[arg(long)]
        quality: Option<bool>,

        /// Enable language detection (overrides config file)
        #[arg(long)]
        detect_language: Option<bool>,

        /// Use async extraction
        #[arg(long)]
        r#async: bool,
    },

    /// Batch extract from multiple documents
    Batch {
        /// Paths to documents
        paths: Vec<PathBuf>,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,

        /// Enable OCR (overrides config file)
        #[arg(long)]
        ocr: Option<bool>,

        /// Force OCR even if text extraction succeeds (overrides config file)
        #[arg(long)]
        force_ocr: Option<bool>,

        /// Disable caching (overrides config file)
        #[arg(long)]
        no_cache: Option<bool>,

        /// Enable quality processing (overrides config file)
        #[arg(long)]
        quality: Option<bool>,

        /// Use async extraction
        #[arg(long)]
        r#async: bool,
    },

    /// Detect MIME type of a file
    Detect {
        /// Path to the file
        path: PathBuf,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Show version information
    Version {
        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Cache management operations
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Start the API server
    #[cfg(feature = "api")]
    Serve {
        /// Host to bind to (e.g., "127.0.0.1" or "0.0.0.0")
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to
        #[arg(short, long, default_value_t = 8000)]
        port: u16,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Start the MCP (Model Context Protocol) server
    #[cfg(feature = "mcp")]
    Mcp {
        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats {
        /// Cache directory (default: .kreuzberg in current directory)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Clear the cache
    Clear {
        /// Cache directory (default: .kreuzberg in current directory)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Invalid format: {}. Use 'text' or 'json'", s)),
        }
    }
}

/// Validates that a file exists and is accessible.
///
/// Checks that the path exists in the filesystem and points to a regular file
/// (not a directory or special file). Provides user-friendly error messages if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The path does not exist in the filesystem
/// - The path exists but is not a regular file (e.g., is a directory)
fn validate_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!(
            "File not found: '{}'. Please check that the file exists and is accessible.",
            path.display()
        );
    }
    if !path.is_file() {
        anyhow::bail!(
            "Path is not a file: '{}'. Please provide a path to a regular file.",
            path.display()
        );
    }
    Ok(())
}

/// Validates chunking parameters for correctness.
///
/// Ensures that chunking configuration makes sense: size must be positive and reasonable,
/// and overlap must be smaller than chunk size. This prevents common configuration errors
/// that would lead to cryptic failures from the underlying library.
///
/// # Errors
///
/// Returns an error if:
/// - `chunk_size` is 0 (must be at least 1 character)
/// - `chunk_size` exceeds 1,000,000 characters (to prevent excessive memory usage)
/// - `chunk_overlap` is greater than or equal to `chunk_size` (overlap must be smaller)
fn validate_chunk_params(chunk_size: Option<usize>, chunk_overlap: Option<usize>) -> Result<()> {
    if let Some(size) = chunk_size {
        if size == 0 {
            anyhow::bail!("Invalid chunk size: {}. Chunk size must be greater than 0.", size);
        }
        if size > 1_000_000 {
            anyhow::bail!(
                "Invalid chunk size: {}. Chunk size must be less than 1,000,000 characters to avoid excessive memory usage.",
                size
            );
        }
    }

    if let Some(overlap) = chunk_overlap
        && let Some(size) = chunk_size
        && overlap >= size
    {
        anyhow::bail!(
            "Invalid chunk overlap: {}. Overlap ({}) must be less than chunk size ({}).",
            overlap,
            overlap,
            size
        );
    }

    Ok(())
}

/// Validates batch extraction paths for correctness.
///
/// Ensures that at least one file path is provided and that all paths point to valid,
/// accessible files. This prevents processing empty batches or failing mid-batch due
/// to invalid paths.
///
/// # Errors
///
/// Returns an error if:
/// - The paths array is empty (at least one file is required)
/// - Any path does not exist or is not a regular file
fn validate_batch_paths(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        anyhow::bail!("No files provided for batch extraction. Please provide at least one file path.");
    }

    for (i, path) in paths.iter().enumerate() {
        validate_file_exists(path).with_context(|| format!("Invalid file at position {}", i + 1))?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(std::io::stderr)
        .try_init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            path,
            config: config_path,
            mime_type,
            format,
            ocr,
            force_ocr,
            no_cache,
            chunk,
            chunk_size,
            chunk_overlap,
            quality,
            detect_language,
            r#async,
        } => {
            // Validate input parameters
            validate_file_exists(&path)?;
            validate_chunk_params(chunk_size, chunk_overlap)?;

            let mut config = load_config(config_path)?;

            if let Some(ocr_flag) = ocr {
                if ocr_flag {
                    config.ocr = Some(OcrConfig {
                        backend: "tesseract".to_string(),
                        language: "eng".to_string(),
                        tesseract_config: None,
                    });
                } else {
                    config.ocr = None;
                }
            }
            if let Some(force_ocr_flag) = force_ocr {
                config.force_ocr = force_ocr_flag;
            }
            if let Some(no_cache_flag) = no_cache {
                config.use_cache = !no_cache_flag;
            }
            if let Some(chunk_flag) = chunk {
                if chunk_flag {
                    let max_chars = chunk_size.unwrap_or(1000);
                    let max_overlap = chunk_overlap.unwrap_or(200);
                    config.chunking = Some(ChunkingConfig {
                        max_chars,
                        max_overlap,
                        embedding: None,
                        preset: None,
                    });
                } else {
                    config.chunking = None;
                }
            } else if let Some(ref mut chunking) = config.chunking {
                if let Some(max_chars) = chunk_size {
                    chunking.max_chars = max_chars;
                }
                if let Some(max_overlap) = chunk_overlap {
                    chunking.max_overlap = max_overlap;
                }
            }
            if let Some(quality_flag) = quality {
                config.enable_quality_processing = quality_flag;
            }
            if let Some(detect_language_flag) = detect_language {
                if detect_language_flag {
                    config.language_detection = Some(LanguageDetectionConfig {
                        enabled: true,
                        min_confidence: 0.8,
                        detect_multiple: false,
                    });
                } else {
                    config.language_detection = None;
                }
            }

            let path_str = path.to_string_lossy().to_string();

            let result = if r#async {
                extract_file(&path_str, mime_type.as_deref(), &config)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
                            path.display()
                        )
                    })?
            } else {
                extract_file_sync(&path_str, mime_type.as_deref(), &config).with_context(|| {
                    format!(
                        "Failed to extract file '{}'. Ensure the file is readable and the format is supported.",
                        path.display()
                    )
                })?
            };

            match format {
                OutputFormat::Text => {
                    println!("{}", result.content);
                }
                OutputFormat::Json => {
                    let output = json!({
                        "content": result.content,
                        "mime_type": result.mime_type,
                        "metadata": result.metadata,
                        "tables": result.tables.iter().map(|t| json!({
                            "cells": t.cells,
                            "markdown": t.markdown,
                            "page_number": t.page_number,
                        })).collect::<Vec<_>>(),
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output)
                            .context("Failed to serialize extraction result to JSON")?
                    );
                }
            }
        }

        Commands::Batch {
            paths,
            config: config_path,
            format,
            ocr,
            force_ocr,
            no_cache,
            quality,
            r#async,
        } => {
            // Validate input parameters
            validate_batch_paths(&paths)?;

            let mut config = load_config(config_path)?;

            if let Some(ocr_flag) = ocr {
                if ocr_flag {
                    config.ocr = Some(OcrConfig {
                        backend: "tesseract".to_string(),
                        language: "eng".to_string(),
                        tesseract_config: None,
                    });
                } else {
                    config.ocr = None;
                }
            }
            if let Some(force_ocr_flag) = force_ocr {
                config.force_ocr = force_ocr_flag;
            }
            if let Some(no_cache_flag) = no_cache {
                config.use_cache = !no_cache_flag;
            }
            if let Some(quality_flag) = quality {
                config.enable_quality_processing = quality_flag;
            }

            let path_strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

            let results = if r#async {
                batch_extract_file(path_strs, &config)
                    .await
                    .with_context(|| format!("Failed to batch extract {} documents. Check that all files are readable and formats are supported.", paths.len()))?
            } else {
                batch_extract_file_sync(path_strs, &config)
                    .with_context(|| format!("Failed to batch extract {} documents. Check that all files are readable and formats are supported.", paths.len()))?
            };

            match format {
                OutputFormat::Text => {
                    for (i, result) in results.iter().enumerate() {
                        println!("=== Document {} ===", i + 1);
                        println!("MIME Type: {}", result.mime_type);
                        println!("Content:\n{}", result.content);
                        println!();
                    }
                }
                OutputFormat::Json => {
                    let output: Vec<_> = results
                        .iter()
                        .map(|result| {
                            json!({
                                "content": result.content,
                                "mime_type": result.mime_type,
                                "metadata": result.metadata,
                                "tables": result.tables.iter().map(|t| json!({
                                    "cells": t.cells,
                                    "markdown": t.markdown,
                                    "page_number": t.page_number,
                                })).collect::<Vec<_>>(),
                            })
                        })
                        .collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output)
                            .context("Failed to serialize batch extraction results to JSON")?
                    );
                }
            }
        }

        Commands::Detect { path, format } => {
            // Validate input parameters
            validate_file_exists(&path)?;

            let path_str = path.to_string_lossy().to_string();
            let mime_type = detect_mime_type(&path_str, true).with_context(|| {
                format!(
                    "Failed to detect MIME type for file '{}'. Ensure the file is readable.",
                    path.display()
                )
            })?;

            match format {
                OutputFormat::Text => {
                    println!("{}", mime_type);
                }
                OutputFormat::Json => {
                    let output = json!({
                        "path": path_str,
                        "mime_type": mime_type,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output)
                            .context("Failed to serialize MIME type detection result to JSON")?
                    );
                }
            }
        }

        Commands::Version { format } => {
            let version = env!("CARGO_PKG_VERSION");
            let name = env!("CARGO_PKG_NAME");

            match format {
                OutputFormat::Text => {
                    println!("{} {}", name, version);
                }
                OutputFormat::Json => {
                    let output = json!({
                        "name": name,
                        "version": version,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output)
                            .context("Failed to serialize version information to JSON")?
                    );
                }
            }
        }

        #[cfg(feature = "api")]
        Commands::Serve {
            host,
            port,
            config: config_path,
        } => {
            let config = load_config(config_path)?;

            println!("Starting Kreuzberg API server on http://{}:{}...", host, port);
            kreuzberg::api::serve_with_config(&host, port, config)
                .await
                .with_context(|| format!("Failed to start API server on {}:{}. Ensure the port is not already in use and you have permission to bind to this address.", host, port))?;
        }

        #[cfg(feature = "mcp")]
        Commands::Mcp { config: config_path } => {
            let config = load_config(config_path)?;

            tracing::debug!("Starting Kreuzberg MCP server...");
            kreuzberg::mcp::start_mcp_server_with_config(config)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;
        }

        Commands::Cache { command } => {
            use kreuzberg::cache;

            // OSError/RuntimeError must bubble up - system errors need user reports ~keep
            let default_cache_dir = std::env::current_dir()
                .context("Failed to get current directory")?
                .join(".kreuzberg");

            match command {
                CacheCommands::Stats { cache_dir, format } => {
                    let cache_path = cache_dir.unwrap_or(default_cache_dir);
                    let cache_dir_str = cache_path.to_string_lossy();

                    let stats = cache::get_cache_metadata(&cache_dir_str)
                        .with_context(|| format!("Failed to get cache statistics from directory '{}'. Ensure the directory exists and is readable.", cache_dir_str))?;

                    match format {
                        OutputFormat::Text => {
                            println!("Cache Statistics");
                            println!("================");
                            println!("Directory: {}", cache_dir_str);
                            println!("Total files: {}", stats.total_files);
                            println!("Total size: {:.2} MB", stats.total_size_mb);
                            println!("Available space: {:.2} MB", stats.available_space_mb);
                            println!("Oldest file age: {:.2} days", stats.oldest_file_age_days);
                            println!("Newest file age: {:.2} days", stats.newest_file_age_days);
                        }
                        OutputFormat::Json => {
                            let output = json!({
                                "directory": cache_dir_str,
                                "total_files": stats.total_files,
                                "total_size_mb": stats.total_size_mb,
                                "available_space_mb": stats.available_space_mb,
                                "oldest_file_age_days": stats.oldest_file_age_days,
                                "newest_file_age_days": stats.newest_file_age_days,
                            });
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&output)
                                    .context("Failed to serialize cache statistics to JSON")?
                            );
                        }
                    }
                }

                CacheCommands::Clear { cache_dir, format } => {
                    let cache_path = cache_dir.unwrap_or(default_cache_dir);
                    let cache_dir_str = cache_path.to_string_lossy();

                    let (removed_files, freed_mb) =
                        cache::clear_cache_directory(&cache_dir_str).with_context(|| {
                            format!(
                                "Failed to clear cache directory '{}'. Ensure you have write permissions.",
                                cache_dir_str
                            )
                        })?;

                    match format {
                        OutputFormat::Text => {
                            println!("Cache cleared successfully");
                            println!("Directory: {}", cache_dir_str);
                            println!("Removed files: {}", removed_files);
                            println!("Freed space: {:.2} MB", freed_mb);
                        }
                        OutputFormat::Json => {
                            let output = json!({
                                "directory": cache_dir_str,
                                "removed_files": removed_files,
                                "freed_mb": freed_mb,
                            });
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&output)
                                    .context("Failed to serialize cache clear results to JSON")?
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Loads extraction configuration from a file or discovers it automatically.
///
/// This function implements the CLI's configuration hierarchy:
/// 1. Explicit config file (if `--config` flag provided)
/// 2. Auto-discovered config (searches `kreuzberg.{toml,yaml,json}` in current and parent directories)
/// 3. Default configuration (if no config file found)
///
/// # Configuration File Formats
///
/// Supports three formats, determined by file extension:
/// - `.toml`: TOML format (recommended for humans)
/// - `.yaml` or `.yml`: YAML format
/// - `.json`: JSON format
///
/// # Errors
///
/// Returns an error if:
/// - Explicit config file has unsupported extension (must be .toml, .yaml, .yml, or .json)
/// - Config file cannot be read or parsed
/// - Config file contains invalid extraction settings
fn load_config(config_path: Option<PathBuf>) -> Result<ExtractionConfig> {
    if let Some(path) = config_path {
        let path_str = path.to_string_lossy();
        // Use case-insensitive extension matching for better cross-platform compatibility
        let path_lower = path_str.to_lowercase();
        let config = if path_lower.ends_with(".toml") {
            ExtractionConfig::from_toml_file(&path)
        } else if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") {
            ExtractionConfig::from_yaml_file(&path)
        } else if path_lower.ends_with(".json") {
            ExtractionConfig::from_json_file(&path)
        } else {
            anyhow::bail!("Config file must have .toml, .yaml, .yml, or .json extension (case-insensitive)");
        };
        config.with_context(|| format!("Failed to load configuration from '{}'. Ensure the file exists, is readable, and contains valid configuration.", path.display()))
    } else {
        match ExtractionConfig::discover() {
            Ok(Some(config)) => Ok(config),
            Ok(None) => Ok(ExtractionConfig::default()),
            Err(e) => Err(e).context("Failed to auto-discover configuration file. Searched for kreuzberg.{toml,yaml,json} in current and parent directories. Use --config to specify an explicit path."),
        }
    }
}
