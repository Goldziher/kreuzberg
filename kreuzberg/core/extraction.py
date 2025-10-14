"""Core extraction functions - thin wrappers delegating to main extraction module.

This module provides the main entry points for document extraction.
For now, it delegates to the existing extraction module to maintain compatibility.
In Phase 2+, we will gradually migrate extraction logic here.
"""

from os import PathLike
from pathlib import Path
from typing import TYPE_CHECKING

from kreuzberg._types import ExtractionConfig, ExtractionResult
from kreuzberg.extraction import (
    DEFAULT_CONFIG,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)

if TYPE_CHECKING:
    from collections.abc import Sequence

__all__ = [
    "DEFAULT_CONFIG",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_file",
    "batch_extract_file_sync",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
]


async def extract_file_async(
    file_path: PathLike[str] | str,
    mime_type: str | None = None,
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> ExtractionResult:
    """Extract content from a file (async).

    This is the main async entry point for document extraction. It:
    1. Validates mime type
    2. Gets appropriate extractor from registry
    3. Calls extractor (which calls Rust bindings)
    4. Applies post-processing features (Python)
    5. Returns result

    Args:
        file_path: Path to the file to extract
        mime_type: Optional MIME type. If None, will be auto-detected.
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        ExtractionResult containing extracted content and metadata
    """
    return await extract_file(file_path=file_path, mime_type=mime_type, config=config)


def extract_file_sync_wrapper(
    file_path: Path | str,
    mime_type: str | None = None,
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> ExtractionResult:
    """Extract content from a file (sync).

    Synchronous version of extract_file.

    Args:
        file_path: Path to the file to extract
        mime_type: Optional MIME type. If None, will be auto-detected.
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        ExtractionResult containing extracted content and metadata
    """
    return extract_file_sync(file_path=file_path, mime_type=mime_type, config=config)


async def extract_bytes_async(
    content: bytes,
    mime_type: str,
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> ExtractionResult:
    """Extract content from raw bytes (async).

    Args:
        content: Raw bytes of the document
        mime_type: MIME type of the content
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        ExtractionResult containing extracted content and metadata
    """
    return await extract_bytes(content=content, mime_type=mime_type, config=config)


def extract_bytes_sync_wrapper(
    content: bytes,
    mime_type: str,
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> ExtractionResult:
    """Extract content from raw bytes (sync).

    Args:
        content: Raw bytes of the document
        mime_type: MIME type of the content
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        ExtractionResult containing extracted content and metadata
    """
    return extract_bytes_sync(content=content, mime_type=mime_type, config=config)


async def batch_extract_file_async(
    file_paths: "Sequence[PathLike[str] | str]",
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> list[ExtractionResult]:
    """Extract content from multiple files (async).

    Args:
        file_paths: Sequence of file paths to extract
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        List of ExtractionResults in the same order as input paths
    """
    return await batch_extract_file(file_paths=file_paths, config=config)


def batch_extract_file_sync_wrapper(
    file_paths: "Sequence[PathLike[str] | str]",
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> list[ExtractionResult]:
    """Extract content from multiple files (sync).

    Args:
        file_paths: Sequence of file paths to extract
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        List of ExtractionResults in the same order as input paths
    """
    return batch_extract_file_sync(file_paths=file_paths, config=config)


async def batch_extract_bytes_async(
    contents: "Sequence[tuple[bytes, str]]",
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> list[ExtractionResult]:
    """Extract content from multiple byte contents (async).

    Args:
        contents: Sequence of (content, mime_type) tuples
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        List of ExtractionResults in the same order as input contents
    """
    return await batch_extract_bytes(contents=contents, config=config)


def batch_extract_bytes_sync_wrapper(
    contents: "Sequence[tuple[bytes, str]]",
    config: ExtractionConfig = DEFAULT_CONFIG,
) -> list[ExtractionResult]:
    """Extract content from multiple byte contents (sync).

    Args:
        contents: Sequence of (content, mime_type) tuples
        config: Extraction configuration. Defaults to DEFAULT_CONFIG.

    Returns:
        List of ExtractionResults in the same order as input contents
    """
    return batch_extract_bytes_sync(contents=contents, config=config)
