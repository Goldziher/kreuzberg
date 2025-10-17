"""Tests for the extraction API.

These tests verify the simplified extraction API that delegates to the Rust core.
All postprocessing is now handled by Rust, not Python.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg import (
    ChunkingConfig,
    ExtractionConfig,
    ExtractionResult,
    LanguageDetectionConfig,
    PostProcessorConfig,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.anyio
async def test_extract_bytes_with_valid_mime_type() -> None:
    """Test async extraction from bytes with explicit MIME type."""
    content = b"Hello, World!"
    mime_type = "text/plain"

    result = await extract_bytes(content, mime_type)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Hello, World!"
    assert result.mime_type == "text/plain"
    assert "line_count" in result.metadata
    assert "word_count" in result.metadata
    assert "character_count" in result.metadata


def test_extract_bytes_sync_with_valid_mime_type() -> None:
    """Test sync extraction from bytes with explicit MIME type."""
    content = b"Hello, World!"
    mime_type = "text/plain"

    result = extract_bytes_sync(content, mime_type)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Hello, World!"
    assert result.mime_type == "text/plain"
    assert "line_count" in result.metadata
    assert "word_count" in result.metadata


@pytest.mark.anyio
async def test_extract_file_nonexistent_file() -> None:
    """Test that extracting a nonexistent file raises an error."""
    nonexistent_path = "/nonexistent/file.txt"

    # Rust core raises ValueError for validation errors
    with pytest.raises((ValueError, ValidationError, OSError)):
        await extract_file(nonexistent_path)


def test_extract_file_sync_nonexistent_file() -> None:
    """Test that extracting a nonexistent file raises an error (sync)."""
    nonexistent_path = "/nonexistent/file.txt"

    # Rust core raises ValueError for validation errors
    with pytest.raises((ValueError, ValidationError, OSError)):
        extract_file_sync(nonexistent_path)


@pytest.mark.anyio
async def test_extract_file_with_cache_disabled(tmp_path: Path) -> None:
    """Test extraction with caching disabled."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    config = ExtractionConfig(use_cache=False)
    result = await extract_file(str(test_file), config=config)

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


def test_extract_file_sync_with_cache_disabled(tmp_path: Path) -> None:
    """Test extraction with caching disabled (sync)."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    config = ExtractionConfig(use_cache=False)
    result = extract_file_sync(str(test_file), config=config)

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_with_cache_hit(tmp_path: Path) -> None:
    """Test that caching works correctly (async)."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content for cache")

    config = ExtractionConfig(use_cache=True)

    result1 = await extract_file(str(test_file), config=config)
    result2 = await extract_file(str(test_file), config=config)

    assert result1.content == result2.content
    assert result1.content == "Test content for cache"
    assert result1.mime_type == result2.mime_type == "text/plain"


def test_extract_file_sync_with_cache_hit(tmp_path: Path) -> None:
    """Test that caching works correctly (sync)."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content for cache sync")

    config = ExtractionConfig(use_cache=True)

    result1 = extract_file_sync(str(test_file), config=config)
    result2 = extract_file_sync(str(test_file), config=config)

    assert result1.content == result2.content
    assert result1.content == "Test content for cache sync"
    assert result1.mime_type == result2.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_bytes_with_chunking() -> None:
    """Test extraction with chunking enabled."""
    content = b"This is a long text that should be chunked into smaller pieces for processing."
    mime_type = "text/plain"
    config = ExtractionConfig(chunking=ChunkingConfig(max_chars=20, max_overlap=5))

    result = await extract_bytes(content, mime_type, config)

    # Chunking is handled by Rust now
    assert "chunk_count" in result.metadata


def test_extract_bytes_sync_with_chunking() -> None:
    """Test extraction with chunking enabled (sync)."""
    content = b"This is a long text that should be chunked into smaller pieces for processing."
    mime_type = "text/plain"
    config = ExtractionConfig(chunking=ChunkingConfig(max_chars=20, max_overlap=5))

    result = extract_bytes_sync(content, mime_type, config)

    # Chunking is handled by Rust now
    assert "chunk_count" in result.metadata


@pytest.mark.anyio
async def test_extract_bytes_with_language_detection() -> None:
    """Test extraction with language detection enabled."""
    content = b"This is some English text for language detection."
    mime_type = "text/plain"
    config = ExtractionConfig(language_detection=LanguageDetectionConfig())

    result = await extract_bytes(content, mime_type, config)

    # Language detection is handled by Rust now
    assert result.detected_languages is not None


def test_extract_bytes_sync_with_language_detection() -> None:
    """Test extraction with language detection enabled (sync)."""
    content = b"This is some English text for language detection."
    mime_type = "text/plain"
    config = ExtractionConfig(language_detection=LanguageDetectionConfig())

    result = extract_bytes_sync(content, mime_type, config)

    # Language detection is handled by Rust now
    assert result.detected_languages is not None


@pytest.mark.anyio
async def test_extract_bytes_with_postprocessor_config() -> None:
    """Test extraction with postprocessor config."""
    content = b"Test content"
    mime_type = "text/plain"

    # Postprocessing is now configured via ExtractionConfig.postprocessor
    config = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            enabled_processors=["entity_extraction", "keyword_extraction"],
        )
    )

    result = await extract_bytes(content, mime_type, config)

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


def test_extract_bytes_sync_with_postprocessor_config() -> None:
    """Test extraction with postprocessor config (sync)."""
    content = b"Test content"
    mime_type = "text/plain"

    # Postprocessing is now configured via ExtractionConfig.postprocessor
    config = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=False,  # Disable all postprocessing
        )
    )

    result = extract_bytes_sync(content, mime_type, config)

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_with_html_extractor(tmp_path: Path) -> None:
    """Test HTML extraction."""
    test_file = tmp_path / "test.html"
    test_file.write_text("<html><body><h1>Test HTML File</h1></body></html>")

    result = await extract_file(str(test_file), mime_type="text/html")

    assert "Test HTML File" in result.content
    # HTML extractor may return text/html or text/markdown depending on conversion
    assert result.mime_type in ("text/html", "text/markdown")


def test_extract_file_sync_with_html_extractor(tmp_path: Path) -> None:
    """Test HTML extraction (sync)."""
    test_file = tmp_path / "test.html"
    test_file.write_text("<html><body><h1>Test HTML File</h1></body></html>")

    result = extract_file_sync(str(test_file), mime_type="text/html")

    assert "Test HTML File" in result.content
    # HTML extractor may return text/html or text/markdown depending on conversion
    assert result.mime_type in ("text/html", "text/markdown")
