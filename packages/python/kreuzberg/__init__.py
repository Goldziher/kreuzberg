"""Kreuzberg - Multi-language document intelligence framework.

This is a thin Python wrapper around a high-performance Rust core.
All extraction logic, chunking, quality processing, and language detection
are implemented in Rust for maximum performance.

Python-specific features:
- OCR backends: EasyOCR, PaddleOCR
- PostProcessors: Entity extraction, keyword extraction, category detection
- API server: Litestar REST API
- CLI: Command-line interface (proxies to Rust binary)
- MCP: Model Context Protocol server

Architecture:
- Rust handles: Extraction, parsing, chunking, quality, language detection
- Python adds: OCR backends, postprocessors, API, CLI, optional NLP features
"""

from importlib.metadata import version

# Direct re-exports from Rust bindings
from kreuzberg._internal_bindings import (
    # Configuration types
    ChunkingConfig,
    ExtractedTable,
    ExtractionConfig,
    # Result types
    ExtractionResult,
    ImageExtractionConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    TokenReductionConfig,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_files,
    batch_extract_files_sync,
    # MIME utilities
    detect_mime_type,
    extract_bytes,
    extract_bytes_sync,
    # Core extraction functions (async)
    extract_file,
    # Core extraction functions (sync)
    extract_file_sync,
    # OCR backend plugin functions
    list_ocr_backends,
    # PostProcessor plugin functions
    list_post_processors,
    register_ocr_backend,
    register_post_processor,
    unregister_ocr_backend,
    unregister_post_processor,
    # MIME validation
    validate_mime_type,
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
    # OCR backend plugin functions
    "register_ocr_backend",
    "list_ocr_backends",
    "unregister_ocr_backend",
    # PostProcessor plugin functions
    "register_post_processor",
    "list_post_processors",
    "unregister_post_processor",
]

# Auto-register Python postprocessors (if dependencies are installed)
# This triggers registration of entity extraction, keyword extraction, etc.
try:
    from . import postprocessors  # noqa: F401
except ImportError:
    # Optional postprocessor dependencies not installed
    pass
