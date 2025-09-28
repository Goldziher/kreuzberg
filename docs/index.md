# Kreuzberg

[![Benchmarks](https://img.shields.io/badge/benchmarks-fastest%20CPU-orange)](https://benchmarks.kreuzberg.dev/)
[![PyPI version](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Kreuzberg is a document intelligence framework that transforms unstructured documents into structured, machine-readable data. Built on a foundation of established open source technologies—PDFium for PDF processing, Tesseract for optical character recognition, and Pandoc for universal document conversion—Kreuzberg provides a unified interface for extracting text, metadata, and structural information from diverse document formats.

The framework emphasizes extensibility, allowing developers to integrate custom extractors and processing hooks while maintaining consistent APIs and error handling across all document types.

## Core Capabilities

Kreuzberg addresses the complete document intelligence pipeline through a modular, extensible architecture designed for production environments.

### Performance Characteristics

- **Performance**: Fastest text extraction framework in its category
- **Resource Efficiency**: Minimal installation footprint and memory usage
- **Reliability**: 100% extraction success rate across 20+ tested file formats
- **Architecture**: Hybrid Rust-Python implementation with native extensions (PDFium, Tesseract) and Python async/await support
- **Benchmarks**: [Comprehensive performance analysis](https://benchmarks.kreuzberg.dev/)

### Engineering Principles

- **Test Coverage**: Comprehensive test suites ensuring code reliability
- **API Design**: True async/await implementation alongside synchronous APIs
- **Error Handling**: Consistent exception hierarchy with detailed context
- **Type Safety**: Full type annotations for enhanced developer experience
- **Profiling**: Continuous performance monitoring and optimization

### Developer Integration

- **Zero Configuration**: Functional defaults with progressive configuration options
- **AI Tool Integration**: Native Model Context Protocol (MCP) server implementation
- **IDE Support**: Complete type annotations and docstrings for intelligent code completion
- **Documentation**: Comprehensive API reference with practical examples

### Deployment Architecture

- **Containerization**: Multi-architecture Docker images (linux/amd64, linux/arm64)
- **Serverless**: Optimized for AWS Lambda, Google Cloud Functions, Azure Functions
- **Processing Modes**: CPU-based with optional GPU acceleration (EasyOCR, PaddleOCR)
- **Data Sovereignty**: Local processing without external API dependencies
- **Interface Options**: Command-line interface, REST API with runtime configuration, MCP server

### Document Intelligence Features

- **Format Support**: 20+ document types including PDF, DOCX, PPTX, email (EML/MSG), images, HTML, and structured data formats
- **OCR Engines**: Tesseract (default), EasyOCR, PaddleOCR with automatic fallback strategies
- **Data Extraction**: Text content, document metadata, table structures, embedded images, and media resources
- **Image Processing**: Extract embedded images from documents with optional OCR text recognition
- **Processing Capabilities**: Content chunking for RAG pipelines, language detection, format preservation
- **Document Classification**: Automatic document type detection (contracts, forms, invoices, receipts, reports)
- **Extensibility**: Plugin architecture for custom extractors and hooks

## Architecture Philosophy

Kreuzberg combines the best of both worlds: performance-critical operations implemented in Rust for maximum speed, built on established open source foundations including Pandoc's universal document conversion, PDFium's robust PDF handling, and Tesseract's proven OCR technology. This hybrid approach ensures both reliability and performance.

The framework is designed for modern document processing workflows including Retrieval Augmented Generation (RAG) pipelines, batch document analysis, and real-time content extraction in cloud-native environments. Version 4.0 introduces significant performance improvements through native Rust implementations of Excel, PowerPoint, email parsing, and text processing operations.

## Quick Navigation

### Essential Resources

- **[Getting Started](getting-started/index.md)** - Installation and first steps
- **[User Guide](user-guide/index.md)** - Comprehensive usage documentation
- **[Table Extraction](user-guide/table-extraction.md)** - Multiple approaches for extracting tables
- **[API Reference](api-reference/index.md)** - Complete API documentation
- **[CLI Guide](cli.md)** - Command-line interface reference
- **[Changelog](CHANGELOG.md)** - Version history and release notes

### Integration Guides

- **[Docker Deployment](user-guide/docker.md)** - Containerized deployment
- **[REST API Server](user-guide/api-server.md)** - HTTP API setup
- **[MCP Server](user-guide/mcp-server.md)** - AI tool integration
- **[Performance Analysis](advanced/performance.md)** - Optimization guide
