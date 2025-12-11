# kreuzberg-cli

Command-line interface for the Kreuzberg document intelligence library.

## Overview

This crate provides a production-ready CLI tool for document extraction, MIME type detection, batch processing, and cache management. It exposes the core extraction capabilities of the Kreuzberg Rust library through an easy-to-use command-line interface.

The CLI supports 56 file formats including PDF, DOCX, PPTX, XLSX, images, HTML, and more, with optional OCR support for scanned documents.

## Architecture

### Binary Structure

```
Kreuzberg Core Library (crates/kreuzberg)
    ↓
Kreuzberg CLI (crates/kreuzberg-cli) ← This crate
    ↓
Command-line interface with configuration and caching
```

### Features

- **Extract Command**: Extract text, tables, and metadata from single documents
- **Batch Command**: Process multiple documents in parallel with optimized concurrency
- **Detect Command**: Identify MIME type of any file with magic byte analysis
- **Cache Commands**: Manage extraction result cache (stats, clear)
- **Serve Command** (requires `api` feature): Start REST API server for remote document processing
- **MCP Command** (requires `mcp` feature): Start Model Context Protocol server for AI integration
- **Version Command**: Display version information in text or JSON format
- **Configuration**: TOML, YAML, or JSON config files with auto-discovery

## Installation

### From Source

```bash
cargo install --path crates/kreuzberg-cli
```

Or via the workspace:

```bash
cargo build --release -p kreuzberg-cli
```

### Platform-Specific Requirements

#### OCR Support (Optional)

To enable optical character recognition for scanned documents:

- **macOS**: `brew install tesseract`
- **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr`
- **Windows**: Download from [tesseract-ocr/tesseract](https://github.com/tesseract-ocr/tesseract)

#### Legacy Office Format Support (Optional)

For `.doc` and `.ppt` file extraction:

- **macOS**: `brew install libreoffice`
- **Ubuntu/Debian**: `sudo apt-get install libreoffice`

## Quick Start

### Basic Text Extraction

```bash
# Extract text from a PDF
kreuzberg extract document.pdf

# Extract with JSON output
kreuzberg extract document.pdf --format json
```

### Extract with OCR

```bash
# Enable OCR for scanned documents
kreuzberg extract scanned.pdf --ocr true

# Force OCR even if text extraction succeeds
kreuzberg extract mixed.pdf --force-ocr true
```

### Batch Processing

```bash
# Process multiple documents in parallel
kreuzberg batch *.pdf --format json

# Process with custom configuration
kreuzberg batch documents/*.docx --config config.toml --format json
```

### MIME Type Detection

```bash
# Detect file type
kreuzberg detect unknown-file

# JSON output
kreuzberg detect unknown-file --format json
```

### Cache Management

```bash
# View cache statistics
kreuzberg cache stats

# Clear the cache
kreuzberg cache clear --cache-dir /path/to/cache

# Custom cache directory
kreuzberg cache stats --cache-dir ~/.kreuzberg-cache
```

### API Server (with `api` feature)

```bash
# Start API server on localhost:8000
kreuzberg serve

# Custom host and port
kreuzberg serve --host 0.0.0.0 --port 3000

# With configuration file
kreuzberg serve --config kreuzberg.toml --host 127.0.0.1 --port 8080
```

### MCP Server (with `mcp` feature)

```bash
# Start Model Context Protocol server
kreuzberg mcp

# With configuration file
kreuzberg mcp --config kreuzberg.toml
```

## Configuration

The CLI supports configuration files in TOML, YAML, or JSON formats. Configuration can be:

1. **Explicit**: Passed via `--config /path/to/config.{toml,yaml,json}`
2. **Auto-discovered**: Searches for `kreuzberg.{toml,yaml,json}` in current and parent directories
3. **Default**: Uses built-in defaults if no config found

### Example Configuration (TOML)

```toml
# Basic extraction settings
use_cache = true
enable_quality_processing = true
force_ocr = false

# OCR configuration
[ocr]
backend = "tesseract"
language = "eng"

[ocr.tesseract_config]
enable_table_detection = true
psm = 6
min_confidence = 50.0

# Text chunking (useful for LLM processing)
[chunking]
max_chars = 1000
max_overlap = 200

# PDF-specific options
[pdf_options]
extract_images = true
extract_metadata = true
passwords = []

# Language detection
[language_detection]
enabled = true
min_confidence = 0.8
detect_multiple = false

# Image extraction
[images]
extract_images = true
target_dpi = 300
max_image_dimension = 4096
auto_adjust_dpi = true
```

### Configuration Overrides

Command-line flags override configuration file settings:

```bash
# Override OCR setting from config
kreuzberg extract document.pdf --config config.toml --ocr false

# Override chunking settings
kreuzberg extract long.pdf --chunk true --chunk-size 2000 --chunk-overlap 400

# Disable cache despite config file
kreuzberg extract document.pdf --no-cache true

# Enable language detection
kreuzberg extract multilingual.pdf --detect-language true
```

## Command Reference

### extract

Extract text, tables, and metadata from a document.

```bash
kreuzberg extract <PATH> [OPTIONS]
```

**Options:**
- `--config <PATH>`: Configuration file (TOML, YAML, or JSON)
- `--mime-type <TYPE>`: MIME type hint (auto-detected if not provided)
- `--format <FORMAT>`: Output format (`text` or `json`), default: `text`
- `--ocr <true|false>`: Enable/disable OCR
- `--force-ocr <true|false>`: Force OCR even if text extraction succeeds
- `--no-cache <true|false>`: Disable result caching
- `--chunk <true|false>`: Enable text chunking
- `--chunk-size <SIZE>`: Chunk size in characters (default: 1000)
- `--chunk-overlap <SIZE>`: Overlap between chunks (default: 200)
- `--quality <true|false>`: Enable quality processing
- `--detect-language <true|false>`: Enable language detection

**Examples:**

```bash
# Simple extraction
kreuzberg extract invoice.pdf

# With configuration and JSON output
kreuzberg extract document.pdf --config config.toml --format json

# With chunking for LLM processing
kreuzberg extract report.pdf --chunk true --chunk-size 2000

# With OCR for scanned document
kreuzberg extract scanned.pdf --ocr true --format json
```

### batch

Process multiple documents in parallel.

```bash
kreuzberg batch <PATHS>... [OPTIONS]
```

**Options:**
- `--config <PATH>`: Configuration file (TOML, YAML, or JSON)
- `--format <FORMAT>`: Output format (`text` or `json`), default: `json`
- `--ocr <true|false>`: Enable/disable OCR
- `--force-ocr <true|false>`: Force OCR even if text extraction succeeds
- `--no-cache <true|false>`: Disable result caching
- `--quality <true|false>`: Enable quality processing

**Examples:**

```bash
# Batch process multiple files
kreuzberg batch doc1.pdf doc2.docx doc3.xlsx

# With glob patterns
kreuzberg batch *.pdf *.docx

# With custom configuration
kreuzberg batch documents/* --config batch-config.toml --format json

# With OCR
kreuzberg batch scanned/*.pdf --ocr true --format json
```

### detect

Identify the MIME type of a file.

```bash
kreuzberg detect <PATH> [OPTIONS]
```

**Options:**
- `--format <FORMAT>`: Output format (`text` or `json`), default: `text`

**Examples:**

```bash
# Simple detection
kreuzberg detect unknown-file

# JSON output
kreuzberg detect mystery.bin --format json
```

### cache

Manage extraction result cache.

```bash
kreuzberg cache <COMMAND> [OPTIONS]
```

**Subcommands:**

#### stats

Show cache statistics.

```bash
kreuzberg cache stats [--cache-dir <DIR>] [--format <FORMAT>]
```

**Options:**
- `--cache-dir <DIR>`: Cache directory (default: `.kreuzberg` in current directory)
- `--format <FORMAT>`: Output format (`text` or `json`), default: `text`

#### clear

Clear the cache.

```bash
kreuzberg cache clear [--cache-dir <DIR>] [--format <FORMAT>]
```

**Options:**
- `--cache-dir <DIR>`: Cache directory (default: `.kreuzberg` in current directory)
- `--format <FORMAT>`: Output format (`text` or `json`), default: `text`

**Examples:**

```bash
# View cache statistics
kreuzberg cache stats

# Clear cache with custom directory
kreuzberg cache clear --cache-dir ~/.kreuzberg-cache

# JSON output
kreuzberg cache stats --format json
```

### serve (requires `api` feature)

Start the REST API server.

```bash
kreuzberg serve [OPTIONS]
```

**Options:**
- `--host <HOST>`: Host to bind to (default: `127.0.0.1`)
- `--port <PORT>`: Port to bind to (default: `8000`)
- `--config <PATH>`: Configuration file (TOML, YAML, or JSON)

**Examples:**

```bash
# Default: localhost:8000
kreuzberg serve

# Public access on port 3000
kreuzberg serve --host 0.0.0.0 --port 3000

# With custom configuration
kreuzberg serve --config server-config.toml --port 8080
```

### mcp (requires `mcp` feature)

Start the Model Context Protocol server.

```bash
kreuzberg mcp [OPTIONS]
```

**Options:**
- `--config <PATH>`: Configuration file (TOML, YAML, or JSON)

**Examples:**

```bash
# Start MCP server
kreuzberg mcp

# With custom configuration
kreuzberg mcp --config mcp-config.toml
```

### version

Show version information.

```bash
kreuzberg version [--format <FORMAT>]
```

**Options:**
- `--format <FORMAT>`: Output format (`text` or `json`), default: `text`

**Examples:**

```bash
# Display version
kreuzberg version

# JSON output
kreuzberg version --format json
```

## Output Formats

### Text Format

The default human-readable format:

```bash
kreuzberg extract document.pdf
# Output:
# Document content here...
```

### JSON Format

For programmatic integration:

```bash
kreuzberg extract document.pdf --format json
# Output:
# {
#   "content": "Document content...",
#   "mime_type": "application/pdf",
#   "metadata": { "title": "...", "author": "..." },
#   "tables": [{ "markdown": "...", "cells": [...], "page_number": 0 }]
# }
```

## Supported File Formats

| Category | Formats |
|----------|---------|
| **Documents** | PDF, DOCX, DOC, PPTX, PPT, XLSX, XLS, ODT, ODP, ODS, RTF |
| **Images** | PNG, JPEG, JPG, WEBP, BMP, TIFF, GIF |
| **Web** | HTML, XHTML, XML |
| **Text** | TXT, MD, CSV, TSV, JSON, YAML, TOML |
| **Email** | EML, MSG |
| **Archives** | ZIP, TAR, 7Z |
| **Other** | 30+ additional formats |

## Exit Codes

- `0`: Successful execution
- `Non-zero`: Error occurred (check stderr for details)

## Logging

Control logging verbosity with the `RUST_LOG` environment variable:

```bash
# Show info-level logs (default)
RUST_LOG=info kreuzberg extract document.pdf

# Show detailed debug logs
RUST_LOG=debug kreuzberg extract document.pdf

# Show only warnings and errors
RUST_LOG=warn kreuzberg extract document.pdf

# Suppress all logs
RUST_LOG=error kreuzberg extract document.pdf

# Show logs from specific modules
RUST_LOG=kreuzberg=debug kreuzberg extract document.pdf
```

## Performance Tips

1. **Use batch processing** for multiple files instead of sequential extraction:
   ```bash
   kreuzberg batch *.pdf  # Parallel processing
   ```

2. **Enable caching** to avoid reprocessing the same documents:
   ```bash
   # Cache is enabled by default
   kreuzberg extract document.pdf
   ```

3. **Use appropriate chunk sizes** for LLM processing:
   ```bash
   kreuzberg extract long.pdf --chunk true --chunk-size 2000
   ```

4. **Tune OCR settings** for better performance:
   ```bash
   kreuzberg extract scanned.pdf --ocr true
   # Adjust tesseract_config in configuration file for optimization
   ```

5. **Monitor cache size** and clear when needed:
   ```bash
   kreuzberg cache stats
   kreuzberg cache clear
   ```

## Features

### Default Features
None by default. The binary includes core extraction.

### Optional Features

- **`api`**: Enable the REST API server (`kreuzberg serve` command)
- **`mcp`**: Enable Model Context Protocol server (`kreuzberg mcp` command)
- **`all`**: Enable all features (`api` + `mcp`)

### Building with Features

```bash
# Build with all features
cargo build --release -p kreuzberg-cli --features all

# Build with specific features
cargo build --release -p kreuzberg-cli --features api,mcp
```

## Troubleshooting

### File Not Found Error

Ensure the file path is correct and the file is readable:

```bash
# Check if file exists
ls -l /path/to/document.pdf

# Try with absolute path
kreuzberg extract /absolute/path/to/document.pdf
```

### OCR Not Working

Verify Tesseract is installed:

```bash
tesseract --version

# If not found:
# macOS: brew install tesseract
# Ubuntu: sudo apt-get install tesseract-ocr
# Windows: Download from https://github.com/tesseract-ocr/tesseract
```

### Configuration File Not Found

Check that the configuration file has the correct format and location:

```bash
# Use explicit path
kreuzberg extract document.pdf --config /absolute/path/to/config.toml

# Or place kreuzberg.toml in current directory
ls -l kreuzberg.toml
```

### Out of Memory with Large Files

Use chunking to reduce memory usage:

```bash
kreuzberg extract large-document.pdf --chunk true --chunk-size 1000
```

### Cache Directory Permissions

Ensure write access to the cache directory:

```bash
# Check permissions
ls -ld .kreuzberg

# Or use a custom directory with appropriate permissions
kreuzberg extract document.pdf --config config.toml
# In config.toml: cache_dir = "/tmp/kreuzberg-cache"
```

## Key Files

- `src/main.rs`: CLI implementation with command definitions and argument parsing
- `Cargo.toml`: Package metadata and dependencies

## Building

### Development Build

```bash
cargo build -p kreuzberg-cli
```

### Release Build

```bash
cargo build --release -p kreuzberg-cli
```

### With All Features

```bash
cargo build --release -p kreuzberg-cli --features all
```

## Testing

```bash
# Run CLI tests
cargo test -p kreuzberg-cli

# With logging
RUST_LOG=debug cargo test -p kreuzberg-cli -- --nocapture
```

## Performance Characteristics

- **Single file extraction**: Typically 10-100ms depending on file size and format
- **Batch processing**: Near-linear scaling with 8 concurrent extractions by default
- **OCR processing**: 100-500ms per page depending on image quality and language
- **Caching**: Sub-millisecond retrieval for cached results

## References

- **Kreuzberg Core**: `../kreuzberg/`
- **Main Documentation**: https://kreuzberg.dev
- **GitHub Repository**: https://github.com/kreuzberg-dev/kreuzberg
- **Configuration Guide**: See example configuration sections above

## Contributing

We welcome contributions! Please see the main Kreuzberg repository for contribution guidelines.

## License

MIT
