# Kreuzberg

[![Rust](https://img.shields.io/crates/v/kreuzberg?label=Rust)](https://crates.io/crates/kreuzberg)
[![Python](https://img.shields.io/pypi/v/kreuzberg?label=Python)](https://pypi.org/project/kreuzberg/)
[![TypeScript](https://img.shields.io/npm/v/@kreuzberg/node?label=TypeScript)](https://www.npmjs.com/package/@kreuzberg/node)
[![WASM](https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM)](https://www.npmjs.com/package/@kreuzberg/wasm)
[![Ruby](https://img.shields.io/gem/v/kreuzberg?label=Ruby)](https://rubygems.org/gems/kreuzberg)
[![Java](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![Go](https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go)](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg)
[![C#](https://img.shields.io/nuget/v/Goldziher.Kreuzberg?label=C%23)](https://www.nuget.org/packages/Goldziher.Kreuzberg/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)

**A polyglot document intelligence framework with a Rust core.** Extract text, metadata, and structured information from PDFs, Office documents, images, and 56 formats. Available for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, and C#—or use via CLI, REST API, or MCP server.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/kreuzberg-dev/kreuzberg/issues) you encounter. Help us make the stable release better!

## Why use Kreuzberg

- **Truly polyglot** – Native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, C#
- **Production-ready** – Battle-tested with comprehensive error handling and validation
- **56 formats** – PDF, Office documents, images, HTML, XML, emails, archives, and more
- **OCR built-in** – Multiple backends (Tesseract, EasyOCR, PaddleOCR) with table extraction support
- **Flexible deployment** – Use as library, CLI tool, REST API server, or MCP server
- **Memory efficient** – Streaming parsers with constant memory usage for multi-GB files

📖 **[Complete Documentation](https://kreuzberg.dev/)** • 🚀 **[Installation Guides](#installation)**

## Kreuzberg Cloud (Coming Soon)

Don't want to manage Rust infrastructure? **Kreuzberg Cloud** is a managed document extraction API launching at the beginning of 2026.

- Hosted REST API with async jobs and webhooks
- Built-in chunking and embeddings for RAG pipelines
- Premium OCR backends for 95%+ accuracy
- No infrastructure to maintain

## Installation

Each language binding provides comprehensive documentation with examples and best practices. Choose your platform to get started:

- **[Python](packages/python/README.md)** – Installation, basic usage, async/sync APIs
- **[Ruby](packages/ruby/README.md)** – Installation, basic usage, configuration
- **[TypeScript/Node.js](crates/kreuzberg-node/README.md)** – Installation, types, promises
- **[Go](packages/go/README.md)** – Installation, native library setup, sync/async extraction + batch APIs
  _Note: Windows builds use MinGW and don't support embeddings (ONNX Runtime requires MSVC)_
- **[Java](packages/java/README.md)** – Installation, FFM API usage, Maven/Gradle setup
- **[C#](packages/csharp/README.md)** – Installation, P/Invoke usage, NuGet package
- **[Rust](crates/kreuzberg/README.md)** – Crate usage, features, async/sync APIs
- **[CLI](https://kreuzberg.dev/cli/usage/)** – Command-line usage, batch processing, options

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

**[Complete Format Documentation](https://kreuzberg.dev/reference/formats/)**

## Key Features

### OCR with Table Extraction

Multiple OCR backends (Tesseract, EasyOCR, PaddleOCR) with intelligent table detection and reconstruction. Extract structured data from scanned documents and images with configurable accuracy thresholds.

**[OCR Backend Documentation →](https://kreuzberg.dev/guides/ocr/)**

### Batch Processing

Process multiple documents concurrently with configurable parallelism. Optimize throughput for large-scale document processing workloads with automatic resource management.

**[Batch Processing Guide →](https://kreuzberg.dev/features/#batch-processing)**

### Password-Protected PDFs

Handle encrypted PDFs with single or multiple password attempts. Supports both RC4 and AES encryption with automatic fallback strategies.

**[PDF Configuration →](https://kreuzberg.dev/migration/v3-to-v4/#password-protected-pdfs)**

### Language Detection

Automatic language detection in extracted text using fast-langdetect. Configure confidence thresholds and access per-language statistics.

**[Language Detection Guide →](https://kreuzberg.dev/features/#language-detection)**

### Metadata Extraction

Extract comprehensive metadata from all supported formats: authors, titles, creation dates, page counts, EXIF data, and format-specific properties.

**[Metadata Guide →](https://kreuzberg.dev/snippets/go/pdf_metadata_extractor/)**

## Deployment Options

### REST API Server

Production-ready API server with OpenAPI documentation, health checks, and telemetry support. Deploy standalone or in containers with automatic format detection and streaming support.

**[API Server Documentation →](https://kreuzberg.dev/guides/api-server/)**

### MCP Server (AI Integration)

Model Context Protocol server for Claude and other AI assistants. Enables AI agents to extract and process documents directly with full configuration support.

**[MCP Server Documentation →](https://kreuzberg.dev/guides/api-server/#mcp-server_1)**

### Docker

Official Docker images available in multiple variants:

- **Core** (~1.0-1.3GB): Tesseract OCR, modern Office formats
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
- **[User Guide](https://kreuzberg.dev/guides/extraction/)** – Comprehensive usage guide
- **[API Reference](https://kreuzberg.dev/reference/api-python/)** – Complete API documentation
- **[Format Support](https://kreuzberg.dev/reference/formats/)** – Supported file formats
- **[OCR Backends](https://kreuzberg.dev/guides/ocr/)** – OCR engine setup
- **[CLI Guide](https://kreuzberg.dev/cli/usage/)** – Command-line usage
- **[Migration Guide](https://kreuzberg.dev/migration/v3-to-v4/)** – Upgrading from v3

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details. Kreuzberg’s open-source is released under the MIT license, one of the most permissive licenses available ❤️
This means you can use it freely in both commercial and closed-source products with no obligations, no viral effects, and no licensing restrictions.

Unlike AGPL-licensed PDF engines like MuPDF (which require you to open-source your entire codebase unless you buy a commercial license), MIT is safe for enterprise adoption and creates zero legal friction.
