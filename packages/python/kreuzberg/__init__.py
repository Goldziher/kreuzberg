"""Kreuzberg - Multi-language document intelligence framework.

This is a thin Python wrapper around a high-performance Rust core.
All extraction logic, chunking, quality processing, and language detection
are implemented in Rust for maximum performance.

Python-specific features:
- OCR backends: EasyOCR, PaddleOCR (Python-based OCR engines)
- Custom PostProcessors: Register your own Python processing logic

Architecture:
- Rust handles: Extraction, parsing, chunking, quality, language detection, NLP (keyword extraction), API server, MCP server, CLI
- Python adds: OCR backends (EasyOCR, PaddleOCR), custom postprocessors

Creating Custom PostProcessors:
    >>> from kreuzberg import PostProcessorProtocol, register_post_processor, ExtractionResult
    >>>
    >>> class MyProcessor:
    ...     def name(self) -> str:
    ...         return "my_processor"
    ...
    ...     def process(self, result: ExtractionResult) -> ExtractionResult:
    ...         result.metadata["custom_field"] = "custom_value"
    ...         return result
    ...
    ...     def processing_stage(self) -> str:
    ...         return "middle"
    >>>
    >>> register_post_processor(MyProcessor())
"""

from __future__ import annotations

import hashlib
import json
import threading

# CRITICAL: This must be imported FIRST before any Rust bindings
# It sets up dynamic library paths for bundled native libraries (pdfium, etc.)
from importlib.metadata import version
from typing import TYPE_CHECKING, Any

from kreuzberg import _setup_lib_path  # noqa: F401
from kreuzberg._internal_bindings import (
    ChunkingConfig,
    ExtractionConfig,
    ExtractionResult,
    ImageExtractionConfig,
    ImagePreprocessingConfig,
    LanguageDetectionConfig,
    OcrConfig,
    PdfConfig,
    PostProcessorConfig,
    TesseractConfig,
    TokenReductionConfig,
    clear_post_processors,
    register_ocr_backend,
    register_post_processor,
    unregister_post_processor,
)
from kreuzberg._internal_bindings import (
    batch_extract_bytes as batch_extract_bytes_impl,
)
from kreuzberg._internal_bindings import (
    batch_extract_bytes_sync as batch_extract_bytes_sync_impl,
)
from kreuzberg._internal_bindings import (
    batch_extract_files as batch_extract_files_impl,
)
from kreuzberg._internal_bindings import (
    batch_extract_files_sync as batch_extract_files_sync_impl,
)
from kreuzberg._internal_bindings import (
    extract_bytes as extract_bytes_impl,
)
from kreuzberg._internal_bindings import (
    extract_bytes_sync as extract_bytes_sync_impl,
)
from kreuzberg._internal_bindings import (
    extract_file as extract_file_impl,
)
from kreuzberg._internal_bindings import (
    extract_file_sync as extract_file_sync_impl,
)
from kreuzberg.exceptions import MissingDependencyError
from kreuzberg.postprocessors.protocol import PostProcessorProtocol

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg.ocr.easyocr import EasyOCRBackend  # noqa: F401
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend  # noqa: F401

__version__ = version("kreuzberg")

# ============================================================================
# Public API
# ============================================================================

__all__ = [
    # Configuration classes
    "ChunkingConfig",
    "ExtractionConfig",
    # Result types
    "ExtractionResult",
    "ImageExtractionConfig",
    "ImagePreprocessingConfig",
    "LanguageDetectionConfig",
    "OcrConfig",
    "PdfConfig",
    "PostProcessorConfig",
    # Plugin system
    "PostProcessorProtocol",
    "TesseractConfig",
    "TokenReductionConfig",
    # Version
    "__version__",
    # Extraction functions
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_files",
    "batch_extract_files_sync",
    "clear_post_processors",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
    "register_post_processor",
    "unregister_post_processor",
]

# ============================================================================
# Module-level Plugin Caches
# ============================================================================

# Cache registered plugins to avoid re-instantiation with same kwargs
# Key: (plugin_name, kwargs_hash) -> plugin instance
_REGISTERED_OCR_BACKENDS: dict[tuple[str, str], Any] = {}

# Thread safety lock for cache operations
_OCR_CACHE_LOCK = threading.Lock()

# Maximum cache size to prevent memory leaks in long-running processes
_MAX_CACHE_SIZE = 10


def _hash_kwargs(kwargs: dict[str, Any]) -> str:
    try:
        serialized = json.dumps(kwargs, sort_keys=True, default=str)
        return hashlib.md5(serialized.encode()).hexdigest()  # noqa: S324
    except (TypeError, ValueError):
        # Fallback for truly non-serializable objects
        # Use repr as best effort, but this may not be stable
        return hashlib.md5(repr(kwargs).encode()).hexdigest()  # noqa: S324


def _ensure_ocr_backend_registered(
    config: ExtractionConfig,
    easyocr_kwargs: dict[str, Any] | None,
    paddleocr_kwargs: dict[str, Any] | None,
) -> None:
    if config.ocr is None:
        return

    backend_name = config.ocr.backend

    # Skip tesseract (native Rust backend, no Python registration needed)
    if backend_name == "tesseract":
        return

    # Get kwargs for this backend
    kwargs_map = {
        "easyocr": easyocr_kwargs or {},
        "paddleocr": paddleocr_kwargs or {},
    }
    kwargs = kwargs_map.get(backend_name, {})

    # Thread-safe cache check and registration
    with _OCR_CACHE_LOCK:
        cache_key = (backend_name, _hash_kwargs(kwargs))

        # Check cache again inside lock (double-checked locking pattern)
        if cache_key in _REGISTERED_OCR_BACKENDS:
            return  # Already registered with these kwargs

        # Evict oldest entry if cache is full (simple FIFO eviction)
        if len(_REGISTERED_OCR_BACKENDS) >= _MAX_CACHE_SIZE:
            oldest_key = next(iter(_REGISTERED_OCR_BACKENDS))
            del _REGISTERED_OCR_BACKENDS[oldest_key]

        # Lazy import to avoid ImportError if optional dependencies not installed
        backend: Any
        if backend_name == "easyocr":
            try:
                from kreuzberg.ocr.easyocr import EasyOCRBackend  # noqa: PLC0415

                # Set language from config if not in kwargs
                if "languages" not in kwargs:
                    kwargs["languages"] = [config.ocr.language]

                backend = EasyOCRBackend(**kwargs)
            except ImportError as e:
                raise MissingDependencyError.create_for_package(
                    dependency_group="easyocr",
                    functionality="EasyOCR backend",
                    package_name="easyocr",
                ) from e
        elif backend_name == "paddleocr":
            try:
                from kreuzberg.ocr.paddleocr import PaddleOCRBackend  # noqa: PLC0415

                # Set language from config if not in kwargs
                if "lang" not in kwargs:
                    kwargs["lang"] = config.ocr.language

                backend = PaddleOCRBackend(**kwargs)
            except ImportError as e:
                raise MissingDependencyError.create_for_package(
                    dependency_group="paddleocr",
                    functionality="PaddleOCR backend",
                    package_name="paddleocr",
                ) from e
        else:
            # Unknown backend - silently skip (might be registered separately)
            return

        # Register with Rust core
        register_ocr_backend(backend)
        _REGISTERED_OCR_BACKENDS[cache_key] = backend


# ============================================================================
# Synchronous Extraction Functions
# ============================================================================


def extract_file_sync(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from a file (synchronous).

    Args:
        file_path: Path to the file (str or pathlib.Path)
        mime_type: Optional MIME type hint (auto-detected if None)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options (languages, use_gpu, beam_width, etc.)
        paddleocr_kwargs: PaddleOCR initialization options (lang, use_angle_cls, show_log, etc.)

    Returns:
        ExtractionResult with content, metadata, and tables

    Example:
        >>> from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig
        >>> # Basic usage
        >>> result = extract_file_sync("document.pdf")
        >>>
        >>> # With Tesseract configuration
        >>> config = ExtractionConfig(
        ...     ocr=OcrConfig(
        ...         backend="tesseract",
        ...         language="eng",
        ...         tesseract_config=TesseractConfig(
        ...             psm=6,
        ...             enable_table_detection=True,
        ...             tessedit_char_whitelist="0123456789",
        ...         ),
        ...     )
        ... )
        >>> result = extract_file_sync("invoice.pdf", config=config)
        >>>
        >>> # With EasyOCR custom options
        >>> config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="eng"))
        >>> result = extract_file_sync("scanned.pdf", config=config, easyocr_kwargs={"use_gpu": True, "beam_width": 10})
    """
    if config is None:
        config = ExtractionConfig()

    # Lazy register OCR backend with caching
    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return extract_file_sync_impl(str(file_path), mime_type, config)


def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from bytes (synchronous).

    Args:
        data: File content as bytes or bytearray
        mime_type: MIME type of the data (required for format detection)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        ExtractionResult with content, metadata, and tables
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return extract_bytes_sync_impl(bytes(data), mime_type, config)


def batch_extract_files_sync(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple files in parallel (synchronous).

    Args:
        paths: List of file paths
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per file)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return batch_extract_files_sync_impl([str(p) for p in paths], config)


def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple byte arrays in parallel (synchronous).

    Args:
        data_list: List of file contents as bytes/bytearray
        mime_types: List of MIME types (one per data item)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per data item)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return batch_extract_bytes_sync_impl([bytes(d) for d in data_list], mime_types, config)


# ============================================================================
# Asynchronous Extraction Functions
# ============================================================================


async def extract_file(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from a file (asynchronous).

    Args:
        file_path: Path to the file (str or pathlib.Path)
        mime_type: Optional MIME type hint (auto-detected if None)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        ExtractionResult with content, metadata, and tables
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await extract_file_impl(str(file_path), mime_type, config)


async def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult:
    """Extract content from bytes (asynchronous).

    Args:
        data: File content as bytes or bytearray
        mime_type: MIME type of the data (required for format detection)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        ExtractionResult with content, metadata, and tables
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await extract_bytes_impl(bytes(data), mime_type, config)


async def batch_extract_files(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple files in parallel (asynchronous).

    Args:
        paths: List of file paths
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per file)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await batch_extract_files_impl([str(p) for p in paths], config)


async def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]:
    """Extract content from multiple byte arrays in parallel (asynchronous).

    Args:
        data_list: List of file contents as bytes/bytearray
        mime_types: List of MIME types (one per data item)
        config: Extraction configuration (uses defaults if None)
        easyocr_kwargs: EasyOCR initialization options
        paddleocr_kwargs: PaddleOCR initialization options

    Returns:
        List of ExtractionResults (one per data item)
    """
    if config is None:
        config = ExtractionConfig()

    _ensure_ocr_backend_registered(config, easyocr_kwargs, paddleocr_kwargs)

    return await batch_extract_bytes_impl([bytes(d) for d in data_list], mime_types, config)
