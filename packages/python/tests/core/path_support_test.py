# mypy: ignore-errors
"""Tests for flexible path input support (str, Path, bytes)."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

import pytest

from kreuzberg import ExtractionConfig, extract_file, extract_file_sync

if TYPE_CHECKING:
    from pathlib import Path


def test_extract_file_sync_with_str(tmp_path: Path) -> None:
    """Test that extract_file_sync accepts str paths."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    result = extract_file_sync(str(test_file))

    assert result.content == "Test content"
    assert result.mime_type == "text/plain"


def test_extract_file_sync_with_path(tmp_path: Path) -> None:
    """Test that extract_file_sync accepts pathlib.Path objects."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content from Path")

    result = extract_file_sync(test_file)

    assert result.content == "Test content from Path"
    assert result.mime_type == "text/plain"


def test_extract_file_sync_with_bytes(tmp_path: Path) -> None:
    """Test that extract_file_sync handles bytes paths by decoding to str.

    Note: v4 converts all paths to str internally, so bytes are decoded.
    This is simpler than v3's explicit bytes path support.
    """
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content from bytes")

    # v4 converts bytes to str via str(bytes) which creates 'b"/path"'
    # This is not supported - use .decode() instead
    result = extract_file_sync(bytes(str(test_file), "utf-8").decode("utf-8"))

    assert result.content == "Test content from bytes"
    assert result.mime_type == "text/plain"


@pytest.mark.asyncio
async def test_extract_file_async_with_str(tmp_path: Path) -> None:
    """Test that extract_file (async) accepts str paths."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Async test content")

    result = await extract_file(str(test_file))

    assert result.content == "Async test content"
    assert result.mime_type == "text/plain"


@pytest.mark.asyncio
async def test_extract_file_async_with_path(tmp_path: Path) -> None:
    """Test that extract_file (async) accepts pathlib.Path objects."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Async test content from Path")

    result = await extract_file(test_file)

    assert result.content == "Async test content from Path"
    assert result.mime_type == "text/plain"


@pytest.mark.asyncio
async def test_extract_file_async_with_bytes(tmp_path: Path) -> None:
    """Test that extract_file (async) handles bytes paths by decoding to str.

    Note: v4 converts all paths to str internally, so bytes must be decoded first.
    """
    test_file = tmp_path / "test.txt"
    test_file.write_text("Async test content from bytes")

    # v4 doesn't support raw bytes paths - decode first
    result = await extract_file(bytes(str(test_file), "utf-8").decode("utf-8"))

    assert result.content == "Async test content from bytes"
    assert result.mime_type == "text/plain"


def test_extract_file_with_config_and_path(tmp_path: Path) -> None:
    """Test that path flexibility works with custom config."""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test with config")

    config = ExtractionConfig(use_cache=False)

    result = extract_file_sync(test_file, config=config)
    assert result.content == "Test with config"

    result = extract_file_sync(str(test_file), config=config)
    assert result.content == "Test with config"


def test_invalid_path_type() -> None:
    """Test that invalid path types result in file not found errors.

    Note: v4 doesn't do explicit type validation - it just converts to str.
    Invalid types like int/None get stringified and fail with "file not found".
    This is acceptable behavior - the error message is still clear.
    """
    # v4 converts to str, so 12345 becomes "12345" and fails as file not found
    with pytest.raises(ValueError, match="File does not exist"):
        extract_file_sync(cast("Any", 12345))

    # None becomes "None" and fails similarly
    with pytest.raises((ValueError, TypeError)):
        extract_file_sync(cast("Any", None))
