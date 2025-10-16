"""Kreuzberg - Multi-language document intelligence framework.

This is a thin Python wrapper around a high-performance Rust core.
All extraction logic, chunking, quality processing, and language detection
are implemented in Rust for maximum performance.

Python-specific features:
- OCR backends: EasyOCR, PaddleOCR
- API server: Litestar REST API
- CLI: Command-line interface (proxies to Rust binary)
- MCP: Model Context Protocol server

Architecture:
- Rust handles: Extraction, parsing, chunking, quality, language detection
- Python adds: OCR backends, API, CLI, optional NLP features
"""

from importlib.metadata import version

# Direct re-exports from Rust bindings
from kreuzberg._internal_bindings import (
    # Configuration types
    ChunkingConfig,
    ExtractionConfig,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    TokenReductionConfig,
    # Result types
    ExtractionResult,
    ExtractedTable,
    # Core extraction functions (sync)
    extract_file_sync,
    extract_bytes_sync,
    batch_extract_files_sync,
    batch_extract_bytes_sync,
    # Core extraction functions (async)
    extract_file,
    extract_bytes,
    batch_extract_files,
    batch_extract_bytes,
    # MIME utilities
    detect_mime_type,
    validate_mime_type,
    # Plugin functions
    register_ocr_backend,
    list_ocr_backends,
    unregister_ocr_backend,
)

__version__ = version("kreuzberg")

__all__ = [
    # Version
    "__version__",
    # Configuration
    "ChunkingConfig",
    "ExtractionConfig",
    "ImageExtractionConfig",
    "LanguageDetectionConfig",
    "OcrConfig",
    "PdfConfig",
    "TokenReductionConfig",
    # Results
    "ExtractionResult",
    "ExtractedTable",
    # Sync functions
    "extract_file_sync",
    "extract_bytes_sync",
    "batch_extract_files_sync",
    "batch_extract_bytes_sync",
    # Async functions
    "extract_file",
    "extract_bytes",
    "batch_extract_files",
    "batch_extract_bytes",
    # MIME utilities
    "detect_mime_type",
    "validate_mime_type",
    # Plugin functions
    "register_ocr_backend",
    "list_ocr_backends",
    "unregister_ocr_backend",
]
