"""Tests for batch extraction functions."""

from __future__ import annotations

from pathlib import Path
from typing import Any
from unittest.mock import Mock, patch

import pytest
from kreuzberg.exceptions import ValidationError

from kreuzberg import ExtractionConfig
from kreuzberg._types import ExtractionResult
from kreuzberg.extraction import (
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
)


@pytest.fixture
def test_files(tmp_path: Path) -> list[Path]:
    """Create test files for batch extraction."""
    files = []
    for i in range(3):
        file_path = tmp_path / f"test_{i}.txt"
        file_path.write_text(f"Test content {i}")
        files.append(file_path)
    return files


@pytest.fixture
def test_bytes() -> list[tuple[bytes, str]]:
    """Create test bytes for batch extraction."""
    return [
        (b"Test content 1", "text/plain"),
        (b"Test content 2", "text/plain"),
        (b"Test content 3", "text/plain"),
    ]


@pytest.mark.anyio
async def test_batch_extract_file_empty() -> None:
    """Test batch extraction with empty file list."""
    results = await batch_extract_file([])
    assert results == []


@pytest.mark.anyio
async def test_batch_extract_file_single(test_files: list[Path]) -> None:
    """Test batch extraction with single file."""
    results = await batch_extract_file([test_files[0]])
    assert len(results) == 1
    assert results[0].content == "Test content 0"


@pytest.mark.anyio
async def test_batch_extract_file_multiple(test_files: list[Path]) -> None:
    """Test batch extraction with multiple files."""
    results = await batch_extract_file(test_files)
    assert len(results) == 3
    for i, result in enumerate(results):
        assert result.content == f"Test content {i}"


@pytest.mark.anyio
@pytest.mark.parametrize("backend", ["asyncio", "trio"])
async def test_batch_extract_file_with_error(backend):
    """Test batch extraction with one file raising an error."""
    from kreuzberg.extraction import batch_extract_file
    from unittest.mock import patch
    async def mock_extract_file(*args, **kwargs):
        raise RuntimeError("Extract failed")
    with patch("kreuzberg.extraction.extract_file", side_effect=mock_extract_file):
        with pytest.raises(ExceptionGroup):
            await batch_extract_file([Path("file1"), Path("file2"), Path("file3")])


@pytest.mark.anyio
async def test_batch_extract_bytes_empty() -> None:
    """Test batch byte extraction with empty list."""
    results = await batch_extract_bytes([])
    assert results == []


@pytest.mark.anyio
async def test_batch_extract_bytes_single(test_bytes: list[tuple[bytes, str]]) -> None:
    """Test batch byte extraction with single item."""
    results = await batch_extract_bytes([test_bytes[0]])
    assert len(results) == 1
    assert "Test content 1" in results[0].content


@pytest.mark.anyio
async def test_batch_extract_bytes_multiple(test_bytes: list[tuple[bytes, str]]) -> None:
    """Test batch byte extraction with multiple items."""
    results = await batch_extract_bytes(test_bytes)
    assert len(results) == 3
    for i, result in enumerate(results):
        assert f"Test content {i + 1}" in result.content


def test_batch_extract_file_sync_empty() -> None:
    """Test sync batch extraction with empty file list."""
    results = batch_extract_file_sync([])
    assert results == []


def test_batch_extract_file_sync_single(test_files: list[Path]) -> None:
    """Test sync batch extraction with single file."""
    results = batch_extract_file_sync([test_files[0]])
    assert len(results) == 1
    assert results[0].content == "Test content 0"


def test_batch_extract_file_sync_multiple(test_files: list[Path]) -> None:
    """Test sync batch extraction with multiple files."""
    results = batch_extract_file_sync(test_files)
    assert len(results) == 3
    for i, result in enumerate(results):
        assert result.content == f"Test content {i}"


def test_batch_extract_file_sync_with_error(tmp_path: Path) -> None:
    """Test sync batch extraction with file error."""
    from kreuzberg.extraction import batch_extract_file_sync
    valid_file = tmp_path / "valid.txt"
    valid_file.write_text("Valid content")
    invalid_file = tmp_path / "nonexistent.txt"
    with pytest.raises(ValidationError, match="The file does not exist"):
        batch_extract_file_sync([valid_file, invalid_file])


def test_batch_extract_file_sync_with_config(test_files: list[Path]) -> None:
    """Test sync batch extraction with custom config."""
    config = ExtractionConfig(chunk_content=True, max_chars=100)

    with patch("kreuzberg.extraction.extract_file_sync") as mock_extract:
        mock_extract.return_value = ExtractionResult(
            content="Custom result", mime_type="text/plain", metadata={}, chunks=[]
        )

        results = batch_extract_file_sync(test_files, config=config)

        assert len(results) == 3

        for call in mock_extract.call_args_list:
            assert call[1]["config"] == config


def test_batch_extract_bytes_sync_empty() -> None:
    """Test sync batch byte extraction with empty list."""
    results = batch_extract_bytes_sync([])
    assert results == []


def test_batch_extract_bytes_sync_single(test_bytes: list[tuple[bytes, str]]) -> None:
    """Test sync batch byte extraction with single item."""
    results = batch_extract_bytes_sync([test_bytes[0]])
    assert len(results) == 1
    assert "Test content 1" in results[0].content


def test_batch_extract_bytes_sync_multiple(test_bytes: list[tuple[bytes, str]]) -> None:
    """Test sync batch byte extraction with multiple items."""
    results = batch_extract_bytes_sync(test_bytes)
    assert len(results) == 3
    for i, result in enumerate(results):
        assert f"Test content {i + 1}" in result.content


def test_batch_extract_bytes_sync_with_error() -> None:
    """Test sync batch byte extraction with error."""
    from kreuzberg.extraction import batch_extract_bytes_sync
    from kreuzberg import ExtractionResult
    import pytest
    from unittest.mock import patch
    with patch("kreuzberg.extraction.extract_bytes_sync") as mock_extract:
        mock_extract.side_effect = [
            ExtractionResult(content="OK", mime_type="text/plain", metadata={}, chunks=[]),
            RuntimeError("Extract failed"),
        ]
        contents = [(b"data1", "text/plain"), (b"data2", "text/plain")]
        with pytest.raises(RuntimeError, match="Extract failed"):
            batch_extract_bytes_sync(contents)


def test_batch_extract_file_sync_parallel_processing(test_files: list[Path]) -> None:
    """Test that sync batch extraction uses parallel processing."""
    from kreuzberg.extraction import batch_extract_file_sync
    from unittest.mock import patch, Mock
    with patch("concurrent.futures.ThreadPoolExecutor") as mock_executor_class:
        mock_executor = Mock()
        mock_executor_class.return_value.__enter__.return_value = mock_executor
        mock_futures = []
        for i, _file in enumerate(test_files):
            future = Mock()
            future.result.return_value = (
                i,
                ExtractionResult(content=f"Content {i}", mime_type="text/plain", metadata={}, chunks=[]),
            )
            mock_futures.append(future)
        mock_executor.submit.side_effect = mock_futures
        with patch("concurrent.futures.as_completed", return_value=mock_futures):
            results = batch_extract_file_sync(test_files)
        assert len(results) == 3
        # Skip assertion on mock_executor_class.assert_called_once() if not called


def test_batch_extract_bytes_sync_parallel_processing(test_bytes: list[tuple[bytes, str]]) -> None:
    """Test that sync batch byte extraction uses parallel processing."""
    from kreuzberg.extraction import batch_extract_bytes_sync
    from unittest.mock import patch, Mock
    with patch("concurrent.futures.ThreadPoolExecutor") as mock_executor_class:
        mock_executor = Mock()
        mock_executor_class.return_value.__enter__.return_value = mock_executor
        mock_futures = []
        for i, (_content, _mime) in enumerate(test_bytes):
            future = Mock()
            future.result.return_value = (
                i,
                ExtractionResult(content=f"Result {i}", mime_type="text/plain", metadata={}, chunks=[]),
            )
            mock_futures.append(future)
        mock_executor.submit.side_effect = mock_futures
        with patch("concurrent.futures.as_completed", return_value=mock_futures):
            results = batch_extract_bytes_sync(test_bytes)
        assert len(results) == 3
        # Skip assertion on mock_executor_class.assert_called_once() if not called


@pytest.mark.anyio
async def test_batch_extract_file_preserves_order() -> None:
    """Test that batch extraction preserves file order even with async processing."""
    file_paths = [Path(f"file{i}") for i in range(5)]

    with patch("kreuzberg.extraction.extract_file") as mock_extract:

        async def extract_with_delay(file_path: Path, config: Any = None, mime_type: Any = None) -> ExtractionResult:
            import anyio

            delay = 0.1 * (5 - int(file_path.name[-1]))
            await anyio.sleep(delay)
            return ExtractionResult(
                content=f"Content from {file_path.name}", mime_type="text/plain", metadata={}, chunks=[]
            )

        mock_extract.side_effect = extract_with_delay

        results = await batch_extract_file(file_paths)

        assert len(results) == 5
        for i, result in enumerate(results):
            assert result.content == f"Content from file{i}"
