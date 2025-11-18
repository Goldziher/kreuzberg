# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![npm](https://img.shields.io/npm/v/@goldziher/kreuzberg)](https://www.npmjs.com/package/@goldziher/kreuzberg)
[![RubyGems](https://badge.fury.io/rb/kreuzberg.svg)](https://rubygems.org/gems/kreuzberg)
[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**A polyglot document intelligence framework with a Rust core.** Extract text, metadata, and structured information from PDFs, Office documents, images, and 50+ formats. Available for Rust, Python, Ruby, TypeScript/Node.js—or use via CLI, REST API, or MCP server.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/Goldziher/kreuzberg/issues) you encounter. Help us make the stable release better!

## Why Kreuzberg

- **Rust-powered core** – High-performance native code for text extraction
- **Truly polyglot** – Native bindings for Rust, Python, Ruby, and TypeScript/Node.js
- **Production-ready** – Battle-tested with comprehensive error handling and validation
- **50+ file format families** – PDF, Office documents, images, HTML, XML, emails, archives, and more
- **OCR built-in** – Multiple backends (Tesseract, EasyOCR, PaddleOCR) with table extraction support
- **Flexible deployment** – Use as library, CLI tool, REST API server, or MCP server
- **Memory efficient** – Streaming parsers handle multi-GB files with constant memory usage

📖 **[Complete Documentation](https://kreuzberg.dev/)** • 🚀 **[Installation Guides](#installation)**

## Installation

### Python

```bash
pip install kreuzberg
```

**[Python Documentation →](packages/python/README.md)**

### Ruby

```bash
gem install kreuzberg
```

**[Ruby Documentation →](packages/ruby/README.md)**

### TypeScript/Node.js

```bash
npm install @goldziher/kreuzberg
```

**[TypeScript/Node.js Documentation →](packages/typescript/README.md)**

### Rust

```toml
[dependencies]
kreuzberg = "4.0"
```

**[Rust Documentation →](crates/kreuzberg/README.md)**

### CLI

```bash
brew install goldziher/tap/kreuzberg
```

```bash
cargo install kreuzberg-cli
```

**[CLI Documentation →](https://kreuzberg.dev/cli/)**

## Quick Start

Each language binding provides comprehensive documentation with examples and best practices. Choose your platform to get started:

- **[Python Quick Start →](packages/python/README.md)** – Installation, basic usage, async/sync APIs
- **[Ruby Quick Start →](packages/ruby/README.md)** – Installation, basic usage, configuration
- **[TypeScript/Node.js Quick Start →](packages/typescript/README.md)** – Installation, types, promises
- **[Rust Quick Start →](crates/kreuzberg/README.md)** – Crate usage, features, async/sync APIs
- **[CLI Quick Start →](https://kreuzberg.dev/cli/)** – Command-line usage, batch processing, options

## Supported Formats

### Documents & Productivity

| Format | Extensions | Metadata | Tables | Images |
|--------|-----------|----------|--------|--------|
| PDF | `.pdf` | ✅ | ✅ | ✅ |
| Word | `.docx`, `.doc` | ✅ | ✅ | ✅ |
| Excel | `.xlsx`, `.xls`, `.ods` | ✅ | ✅ | ❌ |
| PowerPoint | `.pptx`, `.ppt` | ✅ | ✅ | ✅ |
| Rich Text | `.rtf` | ✅ | ❌ | ❌ |
| EPUB | `.epub` | ✅ | ❌ | ❌ |

### Images

All image formats support OCR: `.jpg`, `.jpeg`, `.png`, `.tiff`, `.tif`, `.bmp`, `.gif`, `.webp`, `.jp2`

### Web & Structured Data

| Format | Extensions | Features |
|--------|-----------|----------|
| HTML | `.html`, `.htm` | Metadata extraction, link preservation |
| XML | `.xml` | Streaming parser for multi-GB files |
| JSON | `.json` | Intelligent field detection |
| YAML | `.yaml` | Structure preservation |
| TOML | `.toml` | Configuration parsing |

### Email & Archives

| Format | Extensions | Features |
|--------|-----------|----------|
| Email | `.eml`, `.msg` | Full metadata, attachment extraction |
| Archives | `.zip`, `.tar`, `.gz`, `.7z` | File listing, metadata |

### Academic & Technical

LaTeX (`.tex`), BibTeX (`.bib`), Jupyter (`.ipynb`), reStructuredText (`.rst`), Org Mode (`.org`), Markdown (`.md`)

**[Complete Format Documentation](https://kreuzberg.dev/formats/)**

## Key Features

### OCR with Table Extraction

Multiple OCR backends (Tesseract, EasyOCR, PaddleOCR) with intelligent table detection and reconstruction. Extract structured data from scanned documents and images with configurable accuracy thresholds.

**[OCR Backend Documentation →](https://kreuzberg.dev/user-guide/ocr-backends/)**

### Batch Processing

Process multiple documents concurrently with configurable parallelism. Optimize throughput for large-scale document processing workloads with automatic resource management.

**[Batch Processing Guide →](https://kreuzberg.dev/user-guide/batch-processing/)**

### Password-Protected PDFs

Handle encrypted PDFs with single or multiple password attempts. Supports both RC4 and AES encryption with automatic fallback strategies.

**[PDF Configuration →](https://kreuzberg.dev/user-guide/pdf-extraction/)**

### Language Detection

Automatic language detection in extracted text using fast-langdetect. Configure confidence thresholds and access per-language statistics.

**[Language Detection Guide →](https://kreuzberg.dev/user-guide/language-detection/)**

### Metadata Extraction

Extract comprehensive metadata from all supported formats: authors, titles, creation dates, page counts, EXIF data, and format-specific properties.

**[Metadata Guide →](https://kreuzberg.dev/user-guide/metadata/)**

## Deployment Options

### REST API Server

Production-ready API server with OpenAPI documentation, health checks, and telemetry support. Deploy standalone or in containers with automatic format detection and streaming support.

**[API Server Documentation →](https://kreuzberg.dev/user-guide/api-server/)**

### MCP Server (AI Integration)

Model Context Protocol server for Claude and other AI assistants. Enables AI agents to extract and process documents directly with full configuration support.

**[MCP Server Documentation →](https://kreuzberg.dev/user-guide/mcp-server/)**

### Docker

Official Docker images available in multiple variants:

- **Core** (~1.0-1.3GB): Tesseract OCR, Pandoc, modern Office formats
- **Full** (~1.5-2.1GB): Adds LibreOffice for legacy Office formats (.doc, .ppt)

All images support API server, CLI, and MCP server modes with automatic platform detection for linux/amd64 and linux/arm64.

**[Docker Deployment Guide →](https://kreuzberg.dev/guides/docker/)**

## Architecture

Kreuzberg is built with a Rust core for efficient document extraction and processing.

### Design Principles

- **Rust core** – Native code for text extraction and processing
- **Async throughout** – Asynchronous processing with Tokio runtime
- **Memory efficient** – Streaming parsers for large files
- **Parallel batch processing** – Configurable concurrency for multiple documents
- **Zero-copy operations** – Efficient data handling where possible

## Documentation

- **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** – Setup and dependencies
- **[User Guide](https://kreuzberg.dev/user-guide/)** – Comprehensive usage guide
- **[API Reference](https://kreuzberg.dev/api-reference/)** – Complete API documentation
- **[Format Support](https://kreuzberg.dev/formats/)** – Supported file formats
- **[OCR Backends](https://kreuzberg.dev/user-guide/ocr-backends/)** – OCR engine setup
- **[CLI Guide](https://kreuzberg.dev/cli/)** – Command-line usage
- **[Migration Guide](https://kreuzberg.dev/migration/v3-to-v4/)** – Upgrading from v3

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.
