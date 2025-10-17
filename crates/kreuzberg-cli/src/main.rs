#![deny(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kreuzberg::{
    ChunkingConfig, ExtractionConfig, ImageExtractionConfig, LanguageDetectionConfig, OcrConfig, PdfConfig,
    TokenReductionConfig, batch_extract_file, batch_extract_file_sync, detect_mime_type, extract_file,
    extract_file_sync,
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

        /// MIME type hint (auto-detected if not provided)
        #[arg(short, long)]
        mime_type: Option<String>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,

        /// Enable OCR
        #[arg(long)]
        ocr: bool,

        /// Force OCR even if text extraction succeeds
        #[arg(long)]
        force_ocr: bool,

        /// Disable caching
        #[arg(long)]
        no_cache: bool,

        /// Enable chunking
        #[arg(long)]
        chunk: bool,

        /// Chunk size in characters
        #[arg(long, default_value = "1000")]
        chunk_size: usize,

        /// Chunk overlap in characters
        #[arg(long, default_value = "200")]
        chunk_overlap: usize,

        /// Enable quality processing
        #[arg(long)]
        quality: bool,

        /// Enable language detection
        #[arg(long)]
        detect_language: bool,

        /// Use async extraction
        #[arg(long)]
        r#async: bool,
    },

    /// Batch extract from multiple documents
    Batch {
        /// Paths to documents
        paths: Vec<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,

        /// Enable OCR
        #[arg(long)]
        ocr: bool,

        /// Force OCR even if text extraction succeeds
        #[arg(long)]
        force_ocr: bool,

        /// Disable caching
        #[arg(long)]
        no_cache: bool,

        /// Enable quality processing
        #[arg(long)]
        quality: bool,

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
            let config = ConfigBuilder {
                ocr,
                force_ocr,
                use_cache: !no_cache,
                chunk,
                chunk_size,
                chunk_overlap,
                quality,
                detect_language,
            }
            .build();

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
            format,
            ocr,
            force_ocr,
            no_cache,
            quality,
            r#async,
        } => {
            let config = ConfigBuilder {
                ocr,
                force_ocr,
                use_cache: !no_cache,
                chunk: false,
                chunk_size: 1000,
                chunk_overlap: 200,
                quality,
                detect_language: false,
            }
            .build();

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
    }

    Ok(())
}

struct ConfigBuilder {
    ocr: bool,
    force_ocr: bool,
    use_cache: bool,
    chunk: bool,
    chunk_size: usize,
    chunk_overlap: usize,
    quality: bool,
    detect_language: bool,
}

impl ConfigBuilder {
    fn build(self) -> ExtractionConfig {
        ExtractionConfig {
            use_cache: self.use_cache,
            enable_quality_processing: self.quality,
            ocr: if self.ocr {
                Some(OcrConfig {
                    backend: "tesseract".to_string(),
                    language: "eng".to_string(),
                })
            } else {
                None
            },
            force_ocr: self.force_ocr,
            chunking: if self.chunk {
                Some(ChunkingConfig {
                    max_chars: self.chunk_size,
                    max_overlap: self.chunk_overlap,
                })
            } else {
                None
            },
            images: Some(ImageExtractionConfig {
                extract_images: true,
                target_dpi: 300,
                max_image_dimension: 4096,
                auto_adjust_dpi: true,
                min_dpi: 72,
                max_dpi: 600,
            }),
            pdf_options: Some(PdfConfig {
                extract_images: false,
                passwords: None,
                extract_metadata: true,
            }),
            token_reduction: Some(TokenReductionConfig {
                mode: "off".to_string(),
                preserve_important_words: true,
            }),
            language_detection: if self.detect_language {
                Some(LanguageDetectionConfig {
                    enabled: true,
                    min_confidence: 0.8,
                    detect_multiple: false,
                })
            } else {
                None
            },
        }
    }
}
