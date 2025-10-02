# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.0.0] - Unreleased

### Breaking Changes

⚠️ **Version 4.0 has no backward compatibility with v3.x configurations. See the [Migration Guide](https://kreuzberg.dev/getting-started/migration-guide/) for detailed migration instructions.**

#### Configuration System Redesign

Complete redesign of the configuration system with breaking changes to all configuration patterns:

**OCR Configuration - Tagged Union Design:**

- **v3.x**: `ocr_backend="tesseract", ocr_config=TesseractConfig()`
- **v4.0**: `ocr=TesseractConfig()` (backend determined by config type)
- No backward compatibility - all v3 config patterns will raise `ValidationError`

**Feature Flags → Config Objects:**

- **v3.x**: `extract_tables=True`, `extract_keywords=True`, `chunk_content=True`
- **v4.0**: `tables=TableExtractionConfig()`, `keywords=KeywordExtractionConfig()`, `chunking=ChunkingConfig()`

**Type System Changes:**

- **msgspec.Struct**: All configs now use `msgspec.Struct` (frozen, immutable) instead of dataclasses
- **Tuples Required**: All sequence parameters must use tuples instead of lists
- **Method Removal**: `config.to_dict()` removed - use `msgspec.structs.asdict(config)` instead

**Class Renames:**

- `GMFTConfig` → `TableExtractionConfig`
- `VisionTablesConfig` → `TableExtractionConfig`
- `SpacyEntityExtractionConfig` → `EntityExtractionConfig`
- `ImageOCRConfig` → `ImageExtractionConfig`

**Parameter Renames:**

- `keyword_count` → `top_k` (in KeywordExtractionConfig)
- `min_dimensions` → `ocr_min_dimensions` (in ImageExtractionConfig)
- `max_dimensions` → `ocr_max_dimensions` (in ImageExtractionConfig)
- `allowed_formats` → `ocr_allowed_formats` (in ImageExtractionConfig)

**API Server Changes:**

- Query parameter configuration removed
- Must use `config` field in multipart form data
- Config must use v4 nested structure

#### Hybrid Rust-Python Architecture

Version 4.0 introduces a hybrid architecture where performance-critical operations are implemented in Rust while maintaining the Python API:

- **Build System**: Migrated from Hatchling to Maturin for Rust-Python integration
- **Dependencies**: Removed `python-pptx`, `python-calamine`, and `chardetng-py` (replaced by native Rust implementations)
- **Python 3.10+ Required**: Now using modern union syntax (`|` instead of `Union`)

#### Table Extraction Configuration Redesign

Complete redesign of table extraction configuration with simplified options:

**Old Configuration (v3.x):**

```python
from kreuzberg import ExtractionConfig, GMFTConfig

config = ExtractionConfig(
    extract_tables=True,
    vision_tables_config=GMFTConfig(
        detection_threshold=0.7,
    ),
)
```

**New Configuration (v4.0):**

```python
from kreuzberg import ExtractionConfig, TableExtractionConfig

config = ExtractionConfig(
    tables=TableExtractionConfig(
        detection_threshold=0.7,
    ),
)
```

**Removed Options:** Complex internal tuning parameters removed for simplicity (e.g., `formatter_base_threshold`, `cell_required_confidence`, `remove_null_rows`, `semantic_spanning_cells`, `large_table_*` parameters)

#### Dependencies Changes

- **Removed Python Dependencies**: `python-pptx`, `python-calamine`, `chardetng-py`
- **Build Requirements**: Now requires `maturin>=1.9.0` instead of `hatchling`

#### Removed Deprecated Parameters

All deprecated configuration parameters have been removed:

- **EasyOCRConfig**: `use_gpu` parameter removed (use `device` instead)
- **PaddleOCRConfig**: `use_gpu`, `gpu_mem`, `gpu_memory_limit`, `use_angle_cls`, and legacy detection thresholds removed
- **TableExtractionConfig** (formerly GMFTConfig): Complex internal tuning parameters removed
- **ExtractionConfig**: All flat feature flags and parameters moved to nested config objects

**📖 See the [Migration Guide](https://kreuzberg.dev/getting-started/migration-guide/) for detailed migration instructions and code examples.**

### Added

#### Rust-Powered Performance Improvements

- **XML Extraction**: Streaming Rust parser using quick-xml for memory-efficient processing of multi-GB XML files
- **Excel Extraction**: Native Rust implementation using Calamine for ~3x speed improvement
- **PPTX Extraction**: Complete Rust rewrite with streaming support and enhanced metadata extraction
- **Email Parsing**: Full MSG format support with Rust implementation for improved reliability
- **Image Preprocessing**: 2.6x speedup for OCR image preparation with memory optimization
- **Token Reduction**: 5-10x faster text optimization with Rust implementation
- **Cache Management**: High-performance caching system with automatic cleanup and statistics
- **Table Processing**: Arrow IPC bridge for efficient data exchange between Rust and Python

#### Enhanced Vision-Based Table Extraction

- **TATR v1.1 Support**: Latest Table Transformer models with improved accuracy
- **Model Variants**: Support for specialized models (all/pub/fin variants)
- **Device Selection**: Separate GPU/CPU configuration for detection and structure models
- **Model Caching**: Configurable HuggingFace model cache directory
- **Batch Processing**: Configurable batch sizes for GPU processing
- **Mixed Precision**: Optional FP16 inference for improved GPU performance

#### New Utilities

- **PyTorch Management**: Centralized PyTorch dependency handling and device detection
- **Cross-Platform Wheels**: Cibuildwheel integration for universal wheel distribution
- **Enhanced Testing**: Comprehensive Rust test coverage with performance benchmarks

#### New Format Support

- **Plain Text & Markdown**: Native support for `text/plain` and `text/markdown` with comprehensive metadata extraction
    - Extract line count, word count, character count
    - For markdown: extract headers, links, code blocks with language detection
    - Fully integrated with chunking, compression, and entity extraction features
- **XML Documents**: Native support for XML files (`application/xml`, `text/xml`, `image/svg+xml`) with streaming parser
- **Legacy Word Documents**: Support for `.doc` files (`application/msword`) via LibreOffice conversion
- **Legacy PowerPoint**: Support for `.ppt` files (`application/vnd.ms-powerpoint`) via LibreOffice conversion
- **LibreOffice Integration**: Automatic conversion utilities with timeout protection, validation, and comprehensive error handling

### Changed

- **Performance**: Significant speed improvements across all major operations
- **Memory Usage**: Optimized memory consumption for large document processing
- **Error Handling**: Enhanced error messages with better context and debugging information
- **Configuration**: Simplified configuration interfaces with sensible defaults
- **Type Safety**: All configuration objects are now hashable and frozen dataclasses

### Removed

- **Python Implementations**: Replaced Excel, PPTX, and email extractors with Rust versions
- **Legacy Dependencies**: Removed Python-based office document parsing libraries
- **Complex Vision-Based Table Extraction Options**: Simplified configuration by removing internal tuning parameters

### Fixed

- **Windows Compatibility**: Resolved platform-specific issues and casting warnings
- **PyO3 Integration**: Fixed deprecation warnings and improved type signatures
- **CI/CD Pipeline**: Unified testing workflows with comprehensive Rust support
- **PPTX Extraction**: Enhanced image extraction and metadata handling
- **Excel Float Formatting**: Improved decimal place display consistency
- **Async File Operations**: Fixed blocking file operations in pandoc extractor to use AsyncPath properly

## [3.19.0] - 2025-09-28

### Added

- **Systematic Error Handling**: Implemented context-aware exception handling with proper error tracking and metadata preservation
- **Type-safe Error Utilities**: Added `_error_handling.py` module with type-safe error handling utilities
- **Error Context Types**: Added `ErrorContextType` literal type for classifying error contexts (batch_processing, optional_feature, single_extraction)
- **Processing Error Tracking**: Added structured `ProcessingErrorDict` type for comprehensive error information with tracebacks

### Changed

- **Critical System Errors Policy**: OSError and RuntimeError now always bubble up to users for proper bug reporting
- **Batch Processing**: Now returns partial results with error information instead of failing completely
- **Optional Features**: Preserve successful extraction results when optional features fail
- **Entity/Keyword Extraction**: Removed silent suppression of RuntimeError/OSError - these now bubble up as per policy

### Fixed

- **Test Fixture Paths**: Corrected all test fixture paths to match actual file locations
- **CI Configuration**: Fixed missing `needs` declaration in python-tests job
- **Coverage Job**: Enabled coverage job for all branches to work with DeepSource
- **spaCy Model Installation**: Properly handle SystemExit from spaCy CLI when pip is unavailable

### Breaking Changes

- **Error Handling**: RuntimeError and OSError in keyword extraction and OCR processing will now bubble up instead of being silently handled. This ensures critical system issues are reported to developers.

## [3.18.0] - 2025-09-27

### Added

- **Configurable API Server**: Environment variable configuration for upload limits and server settings ([#150](https://github.com/Goldziher/kreuzberg/pull/150))
- **HOCR Document Processing**: Comprehensive HTML-based OCR document processing support with automatic detection and clean conversion ([#152](https://github.com/Goldziher/kreuzberg/pull/152))

### Fixed

- **spaCy Model Auto-download**: Improved compatibility with uv package manager by adding fallback for missing spaCy models ([#151](https://github.com/Goldziher/kreuzberg/pull/151), fixes [#145](https://github.com/Goldziher/kreuzberg/issues/145))
- **Empty HTML Error**: Resolved `EmptyHtmlError` when processing image-based PDFs that produce empty HOCR output ([fixes #149](https://github.com/Goldziher/kreuzberg/issues/149))

## [3.17.3] - 2025-09-23

### Added

- **Performance Improvement**: Migrated from pre-commit to prek for better performance ([#142](https://github.com/Goldziher/kreuzberg/pull/142))

### Fixed

- **Entity Extraction**: Auto-download missing spaCy models for entity extraction ([#144](https://github.com/Goldziher/kreuzberg/pull/144), fixes [#143](https://github.com/Goldziher/kreuzberg/issues/143))

## [3.17.2] - 2025-09-22

### Changed

- **Dependencies**: Updated html-to-markdown to latest version

## [3.17.1] - 2025-09-21

### Fixed

- **Language Detection**: Removed problematic multilingual import and added model parameter support (fixes [#137](https://github.com/Goldziher/kreuzberg/issues/137))
- **Dependencies**: Updated core dependencies

## [3.17.0] - 2025-09-17

### Added

- **Token Reduction**: Advanced text optimization with streaming support for better performance
- **Workflow Optimization**: Added concurrency settings to cancel in-progress CI runs

### Fixed

- **OCR Markdown Escaping**: Resolved excessive escaping in OCR output (fixes [#133](https://github.com/Goldziher/kreuzberg/issues/133))
- **Test Coverage**: Comprehensive improvements to CI test coverage

## [3.16.0] - 2025-09-16

### Added

- **Enhanced JSON Extraction**: Schema analysis and custom field detection
- **Comprehensive Test Coverage**: Significant improvements across all modules
- **Internal Streaming**: Optimization for html-to-markdown conversions

### Fixed

- **CI Environment**: Added xfail markers for environment-specific test issues
- **Type Annotations**: Resolved mypy errors and missing imports

## [3.15.0] - 2025-09-14

### Added

- **Comprehensive Image Extraction**: Full support for image-based document processing
- **Image OCR Configuration**: New ImageOCRConfig for fine-tuned image processing
- **Performance Benchmarks**: Added comprehensive benchmarking tools

### Fixed

- **Test Coverage**: Improved coverage across core modules
- **CI Formatting**: Resolved pre-commit and ruff violations

## [3.14.1] - 2025-09-13

### Fixed

- **API Serialization**: Added polars DataFrame and PIL Image serialization support
- **Configuration Merging**: Resolved TypeError in API config handling

## [3.14.0] - 2025-09-13

### Added

- **1GB Upload Limit**: Enhanced API with comprehensive OpenAPI documentation
- **DPI Configuration System**: Comprehensive DPI control for OCR processing
- **Polars Migration**: Complete migration from pandas to polars for better performance

### Fixed

- **Table Detection**: Improved error handling for empty DataFrames in vision-based table extraction (fixes [#128](https://github.com/Goldziher/kreuzberg/issues/128))
- **CI Coverage**: Enhanced robustness of lcov coverage combining

## [3.13.3] - 2025-09-10

### Fixed

- **Regression Issues**: Resolved CI test failures and PDF extraction regressions (fixes [#126](https://github.com/Goldziher/kreuzberg/issues/126))
- **XLS File Handling**: Improved compatibility with Excel files
- **Test Optimization**: Optimized OCR tests for better CI performance

## [3.13.2] - 2025-09-04

### Fixed

- **CLI Extract Command**: Resolved command-line interface issues
- **Docker Builds**: Added numpy as core dependency to fix build failures

## [3.13.1] - 2025-09-04

### Fixed

- **Docker Compatibility**: Resolved build failures with numpy dependency

## [3.13.0] - 2025-09-04

### Added

- **OCR Caching**: Implemented caching for EasyOCR and PaddleOCR backends (closes [#121](https://github.com/Goldziher/kreuzberg/issues/121))
- **Runtime Configuration API**: Query parameters and headers for dynamic configuration

### Changed

- **Performance**: Significant improvements in OCR processing speed

## [3.12.0] - 2025-08-30

### Added

- **Tesseract TSV Output**: New table extraction format with polars integration
- **Benchmarks CLI**: Unified command-line interface for performance testing
- **HTML Conversion Optimization**: Improved PDF processing and HTML conversion performance

### Removed

- **TesseractTableExtractor**: Removed in favor of new TSV approach
- **Scipy Dependency**: No longer required

## [3.11.6] - 2025-08-25

### Fixed

- **Docker Workflow**: Improved disk space management during builds

## [3.11.5] - 2025-08-25

### Fixed

- **Docker Testing**: Improved test reliability with better image selection

## [3.11.4] - 2025-08-24

### Fixed

- **Docker Permissions**: Resolved permission issues in containerized environments
- **Documentation**: Clarified Docker image contents and usage

## [3.11.3] - 2025-08-24

### Added

- **Docker E2E Testing**: Comprehensive end-to-end testing infrastructure for Docker images

### Fixed

- **Test Exit Codes**: Improved reliability of Docker-based tests

## [3.11.2] - 2025-08-15

### Fixed

- **Vision-Based Table Extraction**: Handle empty DataFrames to prevent pandas.errors.EmptyDataError

## [3.11.1] - 2025-08-13

### Fixed

- **EasyOCR Device Parameters**: Removed problematic device-related parameters from readtext() calls

## [3.11.0] - 2025-08-01

### Added

- **Python 3.10+ Syntax**: Modern syntax optimizations and type annotations

### Changed

- **Coverage Requirements**: Reduced from 95% to 85% for more practical CI/CD

## [3.10.1] - 2025-07-31

### Added

- **Comprehensive Testing**: Added extensive tests for Tesseract OCR, API module, and core components
- **Test Coverage**: Significant improvements across extraction and configuration modules

## [3.10.0] - 2025-07-29

### Added

- **Enhanced Test Suite**: Comprehensive tests for entity extraction, vision-based table extraction edge cases, and CLI modules
- **Performance Optimizations**: Improved test reliability with retry mechanisms

## [3.9.1] - 2025-07-29

### Fixed

- **Test Reliability**: Resolved mypy unused-ignore errors and improved CI stability

## [3.9.0] - 2025-07-17

### Added

- **Spreadsheet Metadata**: Enhanced extraction of metadata from spreadsheet files
- **Timezone Handling**: Improved handling of timezone information in extracted data

## [3.8.2] - 2025-07-13

### Fixed

- **Package Management**: Resolved compatibility issues with modern Python packaging

## [3.8.1] - 2025-07-13

### Fixed

- **Critical Bug Fixes**: Various stability and performance improvements

## [3.8.0] - 2025-07-12

### Added

- **Enhanced Metadata Extraction**: Improved metadata handling across all file formats

## [3.7.0] - 2025-07-11

### Added

- **Advanced OCR Configuration**: Enhanced control over OCR processing parameters

## [3.6.2] - 2025-07-11

### Fixed

- **Performance Optimizations**: Various speed and memory usage improvements

## [3.6.1] - 2025-07-04

### Fixed

- **Bug Fixes**: Minor stability improvements

## [3.6.0] - 2025-07-04

### Added

- **Enhanced Configuration**: Improved configuration system with better validation

## [3.5.0] - 2025-07-04

### Added

- **Language Detection**: Automatic language detection using fast-langdetect library
- **Configuration Enhancements**: Extended ExtractionConfig with language detection options

## [3.4.2] - 2025-07-03

### Fixed

- **Stability Improvements**: Various bug fixes and performance optimizations

## [3.4.1] - 2025-07-03

### Fixed

- **Minor Bug Fixes**: Resolved edge cases in document processing

## [3.4.0] - 2025-07-03

### Added

- **Enhanced Document Support**: Improved support for various document formats

## [3.3.0] - 2025-07-02

### Added

- **Performance Improvements**: Significant speed optimizations for large documents

## [3.2.0] - 2025-06-23

### Added

- **Advanced Extraction Features**: Enhanced extraction capabilities for complex documents

## [3.1.7] - 2025-06-09

### Fixed

- **Critical Bug Fixes**: Resolved issues with specific document types

## [3.1.6] - 2025-05-26

### Fixed

- **Stability Improvements**: Enhanced reliability for edge cases

## [3.1.5] - 2025-05-13

### Fixed

- **Performance Optimizations**: Improved processing speed for large files

## [3.1.4] - 2025-04-26

### Fixed

- **Bug Fixes**: Minor improvements to extraction accuracy

## [3.1.3] - 2025-04-10

### Fixed

- **Compatibility Issues**: Resolved platform-specific problems

## [3.1.2] - 2025-04-08

### Fixed

- **Edge Case Handling**: Improved robustness for unusual document formats

## [3.1.1] - 2025-04-02

### Fixed

- **Minor Bug Fixes**: Small improvements to stability

## [3.1.0] - 2025-03-28

### Added

- **Feature Enhancements**: New capabilities for document processing

## [3.0.1] - 2025-03-26

### Fixed

- **Post-Release Fixes**: Resolved issues discovered after 3.0.0 release

## [3.0.0] - 2025-03-23

### Breaking Changes

- **Complete Architecture Redesign**: Major refactor of internal structure
- **Playa PDF Integration**: Replaced pypdfium2 with playa for better PDF handling
- **New Extractor Registry**: Centralized extractor management system
- **OCR Backend Integration**: Unified interface for EasyOCR and PaddleOCR

### Added

- **Chunking Support**: Text chunking capabilities for large documents
- **Comprehensive Documentation**: Complete API documentation and guides
- **Hook System**: Pre and post-processing hooks for custom workflows
- **Multiple OCR Backends**: Support for Tesseract, EasyOCR, and PaddleOCR
- **Enhanced Configuration**: Improved configuration system with validation

### Changed

- **Modern Python**: Updated to support Python 3.8+ with modern syntax
- **Async/Sync**: Full async support with sync wrappers
- **Error Handling**: Comprehensive exception hierarchy

### Removed

- **Legacy APIs**: Removed deprecated v2.x interfaces

## [2.1.2] - 2025-03-01

### Fixed

- **Stability Improvements**: Enhanced reliability and error handling

## [2.1.1] - 2025-03-01

### Fixed

- **PDF Validation**: Resolved false positives in PDF validation function

## [2.1.0] - 2025-02-15

### Added

- **OCR Pre-processing**: Enhanced image preprocessing for better OCR accuracy
- **Multi-language Support**: Improved Tesseract multilingual string handling

### Fixed

- **Linux Performance**: Resolved test slowness issues on Linux systems

## [2.0.1] - 2025-02-16

### Fixed

- **Critical Fixes**: Resolved issues from 2.0.0 release

## [2.0.0] - 2025-02-15

### Breaking Changes

- **API Redesign**: Major changes to public interfaces
- **Python 3.8+ Required**: Dropped support for older Python versions

### Added

- **Async Support**: Full asynchronous processing capabilities
- **Enhanced Error Handling**: Comprehensive exception system
- **Batch Processing**: Support for processing multiple documents
- **Multi-sheet Excel**: Extract data from multiple Excel worksheets
- **PDF Text Detection**: Automatic detection of corrupted searchable text with OCR fallback
- **Worker Process Management**: Improved resource management for Pandoc and Tesseract

### Changed

- **Performance**: Significant speed improvements through async processing
- **Memory Management**: Better resource utilization and cleanup

### Fixed

- **Windows Compatibility**: Resolved Windows-specific CI and runtime issues
- **Memory Leaks**: Fixed semaphore-related memory issues

## [1.7.0] - 2025-02-08

### Added

- **Enhanced OCR**: Pass PSM and language parameters from extraction configuration
- **Resource Management**: Pandoc now runs with capacity limiter for better resource control

## [1.6.0] - 2025-02-06

### Added

- **Excel File Support**: Complete support for .xlsx and .xls files using calamine
- **Multi-sheet Processing**: Extract data from multiple Excel worksheets

## [1.5.0] - 2025-02-04

### Changed

- **Pandoc Integration**: Replaced pypandoc with direct pandoc subprocess calls for better control

## [1.4.0] - 2025-02-02

### Changed

- **Tesseract Integration**: Replaced pytesseract with direct tesseract subprocess calls for improved performance

## [1.3.0] - 2025-01-31

### Added

- **Enhanced Features**: Expanded document processing capabilities

## [1.2.0] - 2025-01-29

### Added

- **New Functionality**: Additional extraction features

## [1.1.0] - 2025-01-27

### Added

- **Feature Additions**: Enhanced document support

## [1.0.0] - 2025-02-01

### Added

- **Initial Release**: Core document extraction functionality
- **Basic PDF Support**: Text extraction from PDF documents
- **OCR Integration**: Tesseract OCR for image-based documents
- **Multi-format Support**: Basic support for various document formats
- **Async Architecture**: Built from ground up with async/await support
- **MIT License**: Open source under MIT license

### Features

- Text extraction from PDFs, images, and basic document formats
- Tesseract OCR integration for non-searchable documents
- Async/sync dual interface
- Extensible architecture for future enhancements
- Python 3.8+ compatibility
