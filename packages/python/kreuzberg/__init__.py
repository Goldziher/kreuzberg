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

# CRITICAL: This must be imported FIRST before any Rust bindings
# It sets up dynamic library paths for bundled native libraries (pdfium, etc.)
import logging
from importlib.metadata import version

from kreuzberg import _setup_lib_path  # noqa: F401

logger = logging.getLogger(__name__)

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
    # MIME utilities
    detect_mime_type,
    # OCR backend plugin functions
    list_ocr_backends,
    # PostProcessor plugin functions
    list_post_processors,
    register_ocr_backend,
    register_post_processor,
    # Servers (MCP and API)
    start_api_server,
    start_mcp_server,
    unregister_ocr_backend,
    unregister_post_processor,
    # MIME validation
    validate_mime_type,
)

# Exception classes
from kreuzberg.exceptions import (
    KreuzbergError,
    MissingDependencyError,
    OCRError,
    ParsingError,
    ValidationError,
)

# Extraction functions from Python wrapper (with postprocessor support)
from kreuzberg.extraction import (
    PostProcessorConfig,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_files,
    batch_extract_files_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
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
    "PostProcessorConfig",
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
    # Servers (MCP and API)
    "start_api_server",
    "start_mcp_server",
    # Exceptions
    "KreuzbergError",
    "ValidationError",
    "ParsingError",
    "OCRError",
    "MissingDependencyError",
]

# Auto-register Python postprocessors (if dependencies are installed)
# This triggers registration of entity extraction, keyword extraction, etc.
try:
    from . import postprocessors  # noqa: F401
except ImportError:
    # Optional postprocessor dependencies not installed
    pass
except Exception:
    # Unexpected error during postprocessor registration
    logger.warning("Failed to register postprocessors", exc_info=True)

# Auto-register Python OCR backends (if dependencies are installed)
# Each backend is tried independently since they have separate dependency groups

# Try to auto-register EasyOCR (optional dependency group: easyocr)
try:
    from kreuzberg.ocr.easyocr import EasyOCRBackend  # noqa: F401, PLC0415

    _easyocr_backend = EasyOCRBackend()
    register_ocr_backend(_easyocr_backend)
except ImportError:
    # EasyOCR dependencies not installed
    pass
except Exception:
    # Unexpected error during EasyOCR registration
    logger.warning("Failed to register EasyOCR backend", exc_info=True)

# Try to auto-register PaddleOCR (optional dependency group: paddleocr)
try:
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend  # noqa: F401, PLC0415

    _paddleocr_backend = PaddleOCRBackend()
    register_ocr_backend(_paddleocr_backend)
except ImportError:
    # PaddleOCR dependencies not installed
    pass
except Exception:
    # Unexpected error during PaddleOCR registration
    logger.warning("Failed to register PaddleOCR backend", exc_info=True)
