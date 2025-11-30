# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![npm](https://img.shields.io/npm/v/@kreuzberg/node)](https://www.npmjs.com/package/@kreuzberg/node)
[![RubyGems](https://badge.fury.io/rb/kreuzberg.svg)](https://rubygems.org/gems/kreuzberg)
[![Go Reference](https://pkg.go.dev/badge/github.com/Goldziher/kreuzberg/packages/go/kreuzberg.svg)](https://pkg.go.dev/github.com/Goldziher/kreuzberg/packages/go/kreuzberg)
[![Maven Central](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![NuGet](https://img.shields.io/nuget/v/Goldziher.Kreuzberg)](https://www.nuget.org/packages/Goldziher.Kreuzberg/)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**A polyglot document intelligence framework with a Rust core.** Extract text, metadata, and structured information from PDFs, Office documents, images, and 56 formats. Available for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, and C#—or use via CLI, REST API, or MCP server.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/Goldziher/kreuzberg/issues) you encounter. Help us make the stable release better!

## Why Kreuzberg

- **Truly polyglot** – Native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, C#
- **Production-ready** – Battle-tested with comprehensive error handling and validation
- **56 formats** – PDF, Office documents, images, HTML, XML, emails, archives, and more
- **OCR built-in** – Multiple backends (Tesseract, EasyOCR, PaddleOCR) with table extraction support
- **Flexible deployment** – Use as library, CLI tool, REST API server, or MCP server
- **Memory efficient** – Streaming parsers with constant memory usage for multi-GB files

📖 **[Complete Documentation](https://kreuzberg.dev/)** • 🚀 **[Installation Guides](#installation)**

## Kreuzberg Cloud (Coming Soon)

Don't want to manage Rust infrastructure? **Kreuzberg Cloud** is a managed document extraction API launching soon.

- REST API with async jobs and webhooks
- Built-in chunking and embeddings for RAG pipelines
- Premium OCR backends for 95%+ accuracy
- No infrastructure to maintain

**[Join the waitlist →](https://kreuzberg.cloud)**

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
npm install @kreuzberg/node
```

**[TypeScript/Node.js Documentation →](crates/kreuzberg-node/README.md)**

### Go

```bash
go get github.com/Goldziher/kreuzberg/packages/go/kreuzberg@latest
```

Build the FFI crate (`cargo build -p kreuzberg-ffi --release`) and set `LD_LIBRARY_PATH`/`DYLD_FALLBACK_LIBRARY_PATH` to `target/release` so cgo can locate `libkreuzberg_ffi`.

**[Go Documentation →](packages/go/README.md)**

### Java

```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.0.0-rc.1</version>
</dependency>
```

Or with Gradle:

```gradle
implementation 'dev.kreuzberg:kreuzberg:4.0.0-rc.1'
```

Requires Java 25+ with Foreign Function & Memory API (Panama). Build the FFI crate (`cargo build -p kreuzberg-ffi --release`) for native library access.

**[Java Documentation →](packages/java/README.md)**

### C#

```bash
dotnet add package Goldziher.Kreuzberg --version 4.0.0-rc.1
```

Requires .NET 10.0+. Build the FFI crate (`cargo build -p kreuzberg-ffi --release`) and ensure the native library is accessible.

**[C# Documentation →](packages/csharp/README.md)**

### Rust

```toml
[dependencies]
# Use git dependency for full feature support (including embeddings)
kreuzberg = { git = "https://github.com/Goldziher/kreuzberg", tag = "v4.0.0" }

# Or use a specific branch
# kreuzberg = { git = "https://github.com/Goldziher/kreuzberg", branch = "main" }
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
- **[Go Quick Start →](packages/go/README.md)** – Installation, native library setup, sync/async extraction + batch APIs
- **[Java Quick Start →](packages/java/README.md)** – Installation, FFM API usage, Maven/Gradle setup
- **[C# Quick Start →](packages/csharp/README.md)** – Installation, P/Invoke usage, NuGet package
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

## Comparison with Alternatives

| Feature | Kreuzberg | docling | unstructured | LlamaParse |
|---------|-----------|---------|--------------|------------|
| **Formats** | 56 | PDF, DOCX | 30+ | PDF only |
| **Self-hosted** | ✅ Yes (MIT) | ✅ Yes | ✅ Yes | ❌ API only |
| **Programming Languages** | Rust, Python, Ruby, TS, Java, Go, C# | Python | Python | API (any) |
| **Table Extraction** | ✅ Good | ✅ Good | ✅ Basic | ✅ Excellent |
| **OCR** | ✅ Multiple backends | ✅ Yes | ✅ Yes | ✅ Yes |
| **Embeddings** | ✅ Built-in | ❌ No | ❌ No | ❌ No |
| **Chunking** | ✅ Built-in | ❌ No | ✅ Yes | ❌ No |
| **Cost** | Free (MIT) | Free (MIT) | Free (Apache 2.0) | $0.003/page |
| **Air-gap deployments** | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No |

**When to use Kreuzberg:**
- ✅ Need high throughput (thousands of documents)
- ✅ Memory-constrained environments
- ✅ Non-Python ecosystems (Ruby, TypeScript, Java, Go)
- ✅ RAG pipelines (built-in chunking + embeddings)
- ✅ Self-hosted or air-gapped deployments
- ✅ Multi-GB files requiring streaming

**When to consider alternatives:**
- **LlamaParse**: If you need best-in-class table extraction and only process PDFs (requires internet, paid)
- **docling**: If you're Python-only and don't need extreme performance
- **unstructured**: If you need extensive pre-built integrations with vector databases

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
