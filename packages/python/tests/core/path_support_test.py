"""Tests for flexible path input support (str, Path, bytes)."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionConfig, extract_file, extract_file_sync

if TYPE_CHECKING:
    from pathlib import Path


def test_extract_file_sync_with_str(tmp_path: Path) -> None:
    """Test that extract_file_sync accepts str paths."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    # Pass as string
    result = extract_file_sync(str(test_file))

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


def test_extract_file_sync_with_path(tmp_path: Path) -> None:
    """Test that extract_file_sync accepts pathlib.Path objects."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content from Path")

    # Pass as Path object
    result = extract_file_sync(test_file)

    assert result.content == "Test content from Path"
    assert result.mime_type == "text/plain"


def test_extract_file_sync_with_bytes(tmp_path: Path) -> None:
    """Test that extract_file_sync accepts bytes paths (Unix)."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content from bytes")

    # Pass as bytes
    result = extract_file_sync(bytes(str(test_file), "utf-8"))

    assert result.content == "Test content from bytes"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_async_with_str(tmp_path: Path) -> None:
    """Test that extract_file (async) accepts str paths."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Async test content")

    # Pass as string
    result = await extract_file(str(test_file))

    assert result.content == "Async test content"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_async_with_path(tmp_path: Path) -> None:
    """Test that extract_file (async) accepts pathlib.Path objects."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Async test content from Path")

    # Pass as Path object
    result = await extract_file(test_file)

    assert result.content == "Async test content from Path"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_file_async_with_bytes(tmp_path: Path) -> None:
    """Test that extract_file (async) accepts bytes paths (Unix)."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Async test content from bytes")

    # Pass as bytes
    result = await extract_file(bytes(str(test_file), "utf-8"))

    assert result.content == "Async test content from bytes"
    assert result.mime_type == "text/plain"


def test_extract_file_with_config_and_path(tmp_path: Path) -> None:
    """Test that path flexibility works with custom config."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test with config")

    config = ExtractionConfig(use_cache=False)

    # Test with Path object
    result = extract_file_sync(test_file, config=config)
    assert result.content == "Test with config"

    # Test with string
    result = extract_file_sync(str(test_file), config=config)
    assert result.content == "Test with config"


def test_invalid_path_type() -> None:
    """Test that invalid path types raise TypeError."""
    with pytest.raises(TypeError, match="Path must be a string, pathlib.Path, or bytes"):
        extract_file_sync(12345)  # type: ignore[arg-type]

    with pytest.raises(TypeError, match="Path must be a string, pathlib.Path, or bytes"):
        extract_file_sync(None)  # type: ignore[arg-type]
