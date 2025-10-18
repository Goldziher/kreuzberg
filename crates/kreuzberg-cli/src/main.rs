#![deny(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kreuzberg::{
    ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, OcrConfig, batch_extract_file, batch_extract_file_sync,
    detect_mime_type, extract_file, extract_file_sync,
};
use serde_json::json;
use std::path::PathBuf;

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

#[tokio::main]
async fn main() -> Result<()> {
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
            // Load config from file or discover
            let mut config = load_config(config_path)?;

            // Apply CLI overrides
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
                    config.chunking = Some(ChunkingConfig { max_chars, max_overlap });
                } else {
                    config.chunking = None;
                }
            } else {
                // Override chunk size/overlap if chunking is already enabled
                if config.chunking.is_some() {
                    if let Some(max_chars) = chunk_size {
                        config.chunking.as_mut().unwrap().max_chars = max_chars;
                    }
                    if let Some(max_overlap) = chunk_overlap {
                        config.chunking.as_mut().unwrap().max_overlap = max_overlap;
                    }
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
                    .context("Failed to extract document")?
            } else {
                extract_file_sync(&path_str, mime_type.as_deref(), &config).context("Failed to extract document")?
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
                    println!("{}", serde_json::to_string_pretty(&output)?);
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
            // Load config from file or discover
            let mut config = load_config(config_path)?;

            // Apply CLI overrides
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
                    .context("Failed to batch extract documents")?
            } else {
                batch_extract_file_sync(path_strs, &config).context("Failed to batch extract documents")?
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
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
            }
        }

        Commands::Detect { path, format } => {
            let path_str = path.to_string_lossy().to_string();
            let mime_type = detect_mime_type(&path_str, true).context("Failed to detect MIME type")?;

            match format {
                OutputFormat::Text => {
                    println!("{}", mime_type);
                }
                OutputFormat::Json => {
                    let output = json!({
                        "path": path_str,
                        "mime_type": mime_type,
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
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
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
            }
        }

        Commands::Cache { command } => {
            use kreuzberg::cache;

            let default_cache_dir = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".kreuzberg");

            match command {
                CacheCommands::Stats { cache_dir, format } => {
                    let cache_path = cache_dir.unwrap_or(default_cache_dir);
                    let cache_dir_str = cache_path.to_string_lossy();

                    let stats = cache::get_cache_metadata(&cache_dir_str).context("Failed to get cache stats")?;

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
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        }
                    }
                }

                CacheCommands::Clear { cache_dir, format } => {
                    let cache_path = cache_dir.unwrap_or(default_cache_dir);
                    let cache_dir_str = cache_path.to_string_lossy();

                    let (removed_files, freed_mb) =
                        cache::clear_cache_directory(&cache_dir_str).context("Failed to clear cache")?;

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
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Load configuration from file or discover in current/parent directories.
///
/// If `config_path` is provided, loads from that file (supports TOML, YAML, JSON).
/// Otherwise, uses `ExtractionConfig::discover()` to find config in current/parent directories.
/// If no config is found, returns default configuration.
fn load_config(config_path: Option<PathBuf>) -> Result<ExtractionConfig> {
    if let Some(path) = config_path {
        // Load from specified path
        let path_str = path.to_string_lossy();
        let config = if path_str.ends_with(".toml") {
            ExtractionConfig::from_toml_file(&path)
        } else if path_str.ends_with(".yaml") || path_str.ends_with(".yml") {
            ExtractionConfig::from_yaml_file(&path)
        } else if path_str.ends_with(".json") {
            ExtractionConfig::from_json_file(&path)
        } else {
            anyhow::bail!("Config file must have .toml, .yaml, .yml, or .json extension");
        };
        config.context("Failed to load config file")
    } else {
        // Discover config in current/parent directories
        match ExtractionConfig::discover() {
            Ok(Some(config)) => Ok(config),
            Ok(None) => {
                // No config found, use default
                Ok(ExtractionConfig::default())
            }
            Err(e) => Err(e).context("Failed to discover config file"),
        }
    }
}
