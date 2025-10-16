"""Extraction API - thin wrapper around Rust core with Python-specific features.

This module provides the main extraction API for kreuzberg. The core extraction
logic is implemented in Rust for performance, with Python handling additional
features like entity extraction, keyword extraction, and custom hooks.
"""

from __future__ import annotations

import traceback
from pathlib import Path
from typing import TYPE_CHECKING, Final

from kreuzberg import _internal_bindings as rust
from kreuzberg._document_classification import auto_detect_document_type
from kreuzberg._entity_extraction import extract_entities, extract_keywords
from kreuzberg._error_handling import safe_feature_execution
from kreuzberg._token_reduction import get_reduction_stats, reduce_tokens
from kreuzberg._types import (
    ChunkingConfig as PyChunkingConfig,
    EasyOCRConfig,
    ExtractionConfig,
    ExtractionResult,
    LanguageDetectionConfig as PyLanguageDetectionConfig,
    PaddleOCRConfig,
    TesseractConfig,
    TokenReductionConfig as PyTokenReductionConfig,
)
from kreuzberg.exceptions import KreuzbergError

if TYPE_CHECKING:
    from collections.abc import Sequence
    from os import PathLike


DEFAULT_CONFIG: Final[ExtractionConfig] = ExtractionConfig()


def _convert_config_to_rust(py_config: ExtractionConfig) -> rust.ExtractionConfig:
    """Convert Python ExtractionConfig to Rust ExtractionConfig.

    Python config has many more fields (validators, hooks, entities, etc.)
    Rust config only handles core extraction features.

    Args:
        py_config: Python configuration with all features

    Returns:
        Rust configuration with only Rust-supported features
    """
    # Convert OCR config
    rust_ocr = None
    if py_config.ocr is not None:
        if isinstance(py_config.ocr, TesseractConfig):
            # Rust uses simple OcrConfig with backend + language
            rust_ocr = rust.OcrConfig(backend="tesseract", language=py_config.ocr.language)
        elif isinstance(py_config.ocr, EasyOCRConfig):
            # EasyOCR will be registered as Python plugin later
            rust_ocr = rust.OcrConfig(backend="easyocr", language=py_config.ocr.language or "en")
        elif isinstance(py_config.ocr, PaddleOCRConfig):
            # PaddleOCR will be registered as Python plugin later
            rust_ocr = rust.OcrConfig(backend="paddleocr", language=py_config.ocr.language or "en")

    # Convert chunking config
    rust_chunking = None
    if py_config.chunking is not None:
        rust_chunking = rust.ChunkingConfig(
            max_chars=py_config.chunking.max_chars, max_overlap=py_config.chunking.max_overlap
        )

    # Convert language detection config
    rust_lang_detect = None
    if py_config.language_detection is not None:
        rust_lang_detect = rust.LanguageDetectionConfig(
            enabled=py_config.language_detection.enabled,
            min_confidence=py_config.language_detection.min_confidence,
            detect_multiple=py_config.language_detection.detect_multiple,
        )

    # Convert token reduction config
    rust_token_reduction = None
    if py_config.token_reduction is not None:
        # Python token reduction handled in Python layer, but pass config to Rust for metadata
        rust_token_reduction = rust.TokenReductionConfig(
            mode=py_config.token_reduction.mode,
            preserve_important_words=py_config.token_reduction.preserve_important_words,
        )

    # Convert image extraction config (if needed in future)
    rust_images = None
    if py_config.images is not None:
        rust_images = rust.ImageExtractionConfig(
            extract_images=py_config.images.extract_images,
            target_dpi=py_config.target_dpi,
            max_image_dimension=py_config.max_image_dimension,
            auto_adjust_dpi=py_config.auto_adjust_dpi,
            min_dpi=py_config.min_dpi,
            max_dpi=py_config.max_dpi,
        )

    # Convert PDF options
    rust_pdf = None
    if py_config.pdf_password:
        passwords = [py_config.pdf_password] if isinstance(py_config.pdf_password, str) else list(py_config.pdf_password)
        rust_pdf = rust.PdfConfig(extract_images=False, passwords=passwords, extract_metadata=True)

    return rust.ExtractionConfig(
        use_cache=py_config.use_cache,
        enable_quality_processing=py_config.enable_quality_processing,
        ocr=rust_ocr,
        force_ocr=py_config.force_ocr,
        chunking=rust_chunking,
        images=rust_images,
        pdf_options=rust_pdf,
        token_reduction=rust_token_reduction,
        language_detection=rust_lang_detect,
    )


def _apply_python_post_processing(
    result: ExtractionResult,
    config: ExtractionConfig,
    file_path: Path | None = None,
) -> ExtractionResult:
    """Apply Python-specific post-processing features.

    The Rust core already handles:
    - Quality processing
    - Chunking
    - Language detection
    - Post-processor plugins

    Python adds:
    - Entity extraction
    - Keyword extraction
    - Document type detection
    - Token reduction

    Args:
        result: The extraction result from Rust
        config: Extraction configuration
        file_path: Optional file path for document type detection

    Returns:
        The processed extraction result
    """
    if result.metadata is None:
        result.metadata = {}

    # Entity extraction (Python-specific, uses spacy)
    if config.entities is not None:
        result.entities = safe_feature_execution(
            feature_name="entity_extraction",
            execution_func=lambda: extract_entities(
                result.content,
                custom_patterns=config.custom_entity_patterns,
                spacy_config=config.entities,
            ),
            default_value=None,
            result=result,
        )

    # Keyword extraction (Python-specific)
    if config.keywords is not None:
        keywords_config = config.keywords
        result.keywords = safe_feature_execution(
            feature_name="keyword_extraction",
            execution_func=lambda: extract_keywords(
                result.content,
                keyword_count=keywords_config.count,
            ),
            default_value=None,
            result=result,
        )

    # Document type detection (Python-specific)
    if config.auto_detect_document_type:
        result = safe_feature_execution(
            feature_name="document_type_detection",
            execution_func=lambda: auto_detect_document_type(result, config, file_path=file_path),
            default_value=result,
            result=result,
        )

    # Token reduction (Python-specific, Rust TODO for v4.1)
    if config.token_reduction is not None and config.token_reduction.mode != "off":

        def _apply_token_reduction() -> str:
            original_content = result.content

            language_hint = None
            if result.detected_languages and len(result.detected_languages) > 0:
                language_hint = result.detected_languages[0]

            reduced_content = (
                reduce_tokens(
                    original_content,
                    config=config.token_reduction,
                    language=language_hint,
                )
                if config.token_reduction
                else original_content
            )
            reduction_stats = get_reduction_stats(original_content, reduced_content)

            if result.metadata is not None:
                result.metadata["token_reduction"] = {
                    "character_reduction_ratio": reduction_stats["character_reduction_ratio"],
                    "token_reduction_ratio": reduction_stats["token_reduction_ratio"],
                    "original_characters": reduction_stats["original_characters"],
                    "reduced_characters": reduction_stats["reduced_characters"],
                    "original_tokens": reduction_stats["original_tokens"],
                    "reduced_tokens": reduction_stats["reduced_tokens"],
                }

            return reduced_content

        result.content = safe_feature_execution(
            feature_name="token_reduction",
            execution_func=_apply_token_reduction,
            default_value=result.content,
            result=result,
        )

    return result


def _run_python_validators_sync(result: ExtractionResult, config: ExtractionConfig) -> None:
    """Run Python-specific validators (synchronous)."""
    if config.validators:
        for validator in config.validators:
            validator(result)


async def _run_python_validators_async(result: ExtractionResult, config: ExtractionConfig) -> None:
    """Run Python-specific validators (asynchronous)."""
    from kreuzberg._utils._sync import run_maybe_sync

    if config.validators:
        for validator in config.validators:
            await run_maybe_sync(validator, result)


def _run_python_hooks_sync(result: ExtractionResult, config: ExtractionConfig) -> ExtractionResult:
    """Run Python-specific post-processing hooks (synchronous)."""
    if not config.post_processing_hooks:
        return result

    for i, post_processor in enumerate(config.post_processing_hooks):
        try:
            result = post_processor(result)
        except (KreuzbergError, ValueError, RuntimeError, TypeError) as e:
            if result.metadata is None:
                result.metadata = {}
            error_list = result.metadata.setdefault("processing_errors", [])
            if isinstance(error_list, list):
                error_list.append(
                    {
                        "feature": f"post_processing_hook_{i}",
                        "error_type": type(e).__name__,
                        "error_message": str(e),
                        "traceback": traceback.format_exc(),
                    }
                )

    return result


async def _run_python_hooks_async(result: ExtractionResult, config: ExtractionConfig) -> ExtractionResult:
    """Run Python-specific post-processing hooks (asynchronous)."""
    from kreuzberg._utils._sync import run_maybe_sync

    if not config.post_processing_hooks:
        return result

    for i, post_processor in enumerate(config.post_processing_hooks):
        try:
            result = await run_maybe_sync(post_processor, result)
        except (KreuzbergError, ValueError, RuntimeError, TypeError) as e:
            if result.metadata is None:
                result.metadata = {}
            error_list = result.metadata.setdefault("processing_errors", [])
            if isinstance(error_list, list):
                error_list.append(
                    {
                        "feature": f"post_processing_hook_{i}",
                        "error_type": type(e).__name__,
                        "error_message": str(e),
                        "traceback": traceback.format_exc(),
                    }
                )

    return result


# ============================================================================
# Synchronous API
# ============================================================================


def extract_bytes_sync(content: bytes, mime_type: str, config: ExtractionConfig = DEFAULT_CONFIG) -> ExtractionResult:
    """Extract the textual content from a given byte string representing a file's contents.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing.

    Args:
        content: The content to extract.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.
    """
    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core extraction (handles quality, chunking, language detection)
    result = rust.extract_bytes_sync(content, mime_type, rust_config)

    # Apply Python-specific post-processing
    result = _apply_python_post_processing(result, config)

    # Run Python validators
    _run_python_validators_sync(result, config)

    # Run Python post-processing hooks
    result = _run_python_hooks_sync(result, config)

    return result


def extract_file_sync(
    file_path: PathLike[str] | str, mime_type: str | None = None, config: ExtractionConfig = DEFAULT_CONFIG
) -> ExtractionResult:
    """Synchronous version of extract_file.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing.

    Args:
        file_path: The path to the file.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.

    Raises:
        ValidationError: If the file path or configuration is invalid.
    """
    path = Path(file_path)

    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core extraction (handles quality, chunking, language detection)
    result = rust.extract_file_sync(str(path), mime_type, rust_config)

    # Apply Python-specific post-processing
    result = _apply_python_post_processing(result, config, file_path=path)

    # Run Python validators
    _run_python_validators_sync(result, config)

    # Run Python post-processing hooks
    result = _run_python_hooks_sync(result, config)

    return result


def batch_extract_file_sync(
    file_paths: Sequence[PathLike[str] | str], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Synchronous version of batch_extract_file with parallel processing.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing applied to each result.

    Args:
        file_paths: A sequence of paths to files to extract text from.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input paths.
    """
    # Convert paths to strings
    paths_str = [str(Path(p)) for p in file_paths]

    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core batch extraction
    results = rust.batch_extract_files_sync(paths_str, rust_config)

    # Apply Python-specific post-processing to each result
    processed_results = []
    for result, file_path in zip(results, file_paths, strict=True):
        result = _apply_python_post_processing(result, config, file_path=Path(file_path))
        _run_python_validators_sync(result, config)
        result = _run_python_hooks_sync(result, config)
        processed_results.append(result)

    return processed_results


def batch_extract_bytes_sync(
    contents: Sequence[tuple[bytes, str]], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Synchronous version of batch_extract_bytes with parallel processing.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing applied to each result.

    Args:
        contents: A sequence of tuples containing (content, mime_type) pairs.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input contents.
    """
    # Separate contents and mime_types for Rust API
    data_list = [content for content, _ in contents]
    mime_types = [mime_type for _, mime_type in contents]

    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core batch extraction
    results = rust.batch_extract_bytes_sync(data_list, mime_types, rust_config)

    # Apply Python-specific post-processing to each result
    processed_results = []
    for result in results:
        result = _apply_python_post_processing(result, config)
        _run_python_validators_sync(result, config)
        result = _run_python_hooks_sync(result, config)
        processed_results.append(result)

    return processed_results


# ============================================================================
# Asynchronous API
# ============================================================================


async def extract_bytes(content: bytes, mime_type: str, config: ExtractionConfig = DEFAULT_CONFIG) -> ExtractionResult:
    """Extract the textual content from a given byte string representing a file's contents.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing.

    Args:
        content: The content to extract.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.
    """
    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core extraction (handles quality, chunking, language detection)
    result = await rust.extract_bytes(content, mime_type, rust_config)

    # Apply Python-specific post-processing
    result = _apply_python_post_processing(result, config)

    # Run Python validators
    await _run_python_validators_async(result, config)

    # Run Python post-processing hooks
    result = await _run_python_hooks_async(result, config)

    return result


async def extract_file(
    file_path: PathLike[str] | str, mime_type: str | None = None, config: ExtractionConfig = DEFAULT_CONFIG
) -> ExtractionResult:
    """Extract the textual content from a given file.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing.

    Args:
        file_path: The path to the file.
        mime_type: The mime type of the content.
        config: Extraction options object, defaults to the default object.

    Returns:
        The extracted content and the mime type of the content.

    Raises:
        ValidationError: If the file path or configuration is invalid.
    """
    path = Path(file_path)

    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core extraction (handles quality, chunking, language detection)
    result = await rust.extract_file(str(path), mime_type, rust_config)

    # Apply Python-specific post-processing
    result = _apply_python_post_processing(result, config, file_path=path)

    # Run Python validators
    await _run_python_validators_async(result, config)

    # Run Python post-processing hooks
    result = await _run_python_hooks_async(result, config)

    return result


async def batch_extract_file(
    file_paths: Sequence[PathLike[str] | str], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Extract text from multiple files concurrently with optimizations.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing applied to each result.

    Args:
        file_paths: A sequence of paths to files to extract text from.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input paths.
    """
    # Convert paths to strings
    paths_str = [str(Path(p)) for p in file_paths]

    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core batch extraction
    results = await rust.batch_extract_files(paths_str, rust_config)

    # Apply Python-specific post-processing to each result
    processed_results = []
    for result, file_path in zip(results, file_paths, strict=True):
        result = _apply_python_post_processing(result, config, file_path=Path(file_path))
        await _run_python_validators_async(result, config)
        result = await _run_python_hooks_async(result, config)
        processed_results.append(result)

    return processed_results


async def batch_extract_bytes(
    contents: Sequence[tuple[bytes, str]], config: ExtractionConfig = DEFAULT_CONFIG
) -> list[ExtractionResult]:
    """Extract text from multiple byte contents concurrently with optimizations.

    This is a thin wrapper around the Rust core extraction with Python-specific
    post-processing applied to each result.

    Args:
        contents: A sequence of tuples containing (content, mime_type) pairs.
        config: Extraction options object, defaults to the default object.

    Returns:
        A list of extraction results in the same order as the input contents.
    """
    # Separate contents and mime_types for Rust API
    data_list = [content for content, _ in contents]
    mime_types = [mime_type for _, mime_type in contents]

    # Convert Python config to Rust config
    rust_config = _convert_config_to_rust(config)

    # Call Rust core batch extraction
    results = await rust.batch_extract_bytes(data_list, mime_types, rust_config)

    # Apply Python-specific post-processing to each result
    processed_results = []
    for result in results:
        result = _apply_python_post_processing(result, config)
        await _run_python_validators_async(result, config)
        result = await _run_python_hooks_async(result, config)
        processed_results.append(result)

    return processed_results
