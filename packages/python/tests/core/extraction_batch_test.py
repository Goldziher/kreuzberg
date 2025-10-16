from __future__ import annotations

import multiprocessing as mp
from pathlib import Path
from typing import Any
from unittest.mock import Mock, patch

import pytest

from kreuzberg import (
    ExtractionConfig,
    ExtractionResult,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
)


@pytest.mark.anyio
async def test_batch_extract_file_empty_list() -> None:
    result = await batch_extract_file([])
    assert result == []


@pytest.mark.anyio
async def test_batch_extract_file_single_file(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    result = await batch_extract_file([test_file])

    assert len(result) == 1
    assert result[0].content == "Test content"
    assert result[0].mime_type == "text/plain"


@pytest.mark.anyio
async def test_batch_extract_file_multiple_files(tmp_path: Path) -> None:
    file1 = tmp_path / "test1.txt"
    file2 = tmp_path / "test2.txt"
    file1.write_text("Content 1")
    file2.write_text("Content 2")

    config = ExtractionConfig(use_cache=False)
    result = await batch_extract_file([file1, file2], config)

    assert len(result) == 2
    assert result[0].content == "Content 1"
    assert result[1].content == "Content 2"


@pytest.mark.anyio
async def test_batch_extract_file_with_error() -> None:
    nonexistent_file = Path("/nonexistent/file.txt")

    result = await batch_extract_file([nonexistent_file])

    assert len(result) == 1
    assert "Error:" in result[0].content
    assert "ValidationError" in result[0].content
    assert result[0].mime_type == "text/plain"
    assert "error" in result[0].metadata
    assert "error_context" in result[0].metadata


@pytest.mark.anyio
async def test_batch_extract_file_mixed_success_and_error(tmp_path: Path) -> None:
    valid_file = tmp_path / "valid.txt"
    valid_file.write_text("Valid content")
    invalid_file = Path("/nonexistent/invalid.txt")

    result = await batch_extract_file([valid_file, invalid_file])

    assert len(result) == 2
    assert result[0].content == "Valid content"
    assert "Error:" in result[1].content
    assert "error" in result[1].metadata


@pytest.mark.anyio
async def test_batch_extract_file_concurrency_limits() -> None:
    files = [f"/tmp/file_{i}.txt" for i in range(50)]

    with patch("kreuzberg.extraction.extract_file") as mock_extract:
        call_count = 0
        max_concurrent = 0
        current_concurrent = 0

        async def mock_extract_func(*args: Any, **kwargs: Any) -> ExtractionResult:
            nonlocal call_count, max_concurrent, current_concurrent
            current_concurrent += 1
            max_concurrent = max(max_concurrent, current_concurrent)
            call_count += 1

            import anyio

            await anyio.sleep(0.01)

            current_concurrent -= 1
            return ExtractionResult(content=f"Content {call_count}", mime_type="text/plain", metadata={}, chunks=[])

        mock_extract.side_effect = mock_extract_func

        result = await batch_extract_file(files)

    assert len(result) == 50
    assert call_count == 50
    assert max_concurrent > 0
    assert max_concurrent <= 50


@pytest.mark.anyio
async def test_batch_extract_bytes_empty_list() -> None:
    result = await batch_extract_bytes([])
    assert result == []


@pytest.mark.anyio
async def test_batch_extract_bytes_single_content() -> None:
    content = (b"Test content", "text/plain")

    result = await batch_extract_bytes([content])

    assert len(result) == 1
    assert result[0].content == "Test content"
    assert result[0].mime_type == "text/plain"


@pytest.mark.anyio
async def test_batch_extract_bytes_multiple_contents() -> None:
    contents = [(b"Content 1", "text/plain"), (b"Content 2", "text/plain"), (b"Content 3", "text/plain")]

    result = await batch_extract_bytes(contents)

    assert len(result) == 3
    assert result[0].content == "Content 1"
    assert result[1].content == "Content 2"
    assert result[2].content == "Content 3"


@pytest.mark.anyio
async def test_batch_extract_bytes_with_error() -> None:
    with patch("kreuzberg.extraction.extract_bytes") as mock_extract:
        mock_extract.side_effect = ValueError("Test error")

        contents = [(b"Test content", "text/plain")]
        result = await batch_extract_bytes(contents)

    assert len(result) == 1
    assert "Error:" in result[0].content
    assert "ValueError" in result[0].content
    assert result[0].mime_type == "text/plain"
    assert "error" in result[0].metadata
    assert "error_context" in result[0].metadata


def test_batch_extract_file_sync_empty_list() -> None:
    result = batch_extract_file_sync([])
    assert result == []


def test_batch_extract_file_sync_single_file(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    result = batch_extract_file_sync([test_file])

    assert len(result) == 1
    assert result[0].content == "Test content"
    assert result[0].mime_type == "text/plain"


def test_batch_extract_file_sync_single_file_no_parallelism(tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    with patch("kreuzberg.extraction.ThreadPoolExecutor") as mock_executor:
        result = batch_extract_file_sync([test_file])

    mock_executor.assert_not_called()
    assert len(result) == 1
    assert result[0].content == "Test content"


def test_batch_extract_file_sync_multiple_files(tmp_path: Path) -> None:
    file1 = tmp_path / "test1.txt"
    file2 = tmp_path / "test2.txt"
    file1.write_text("Content 1")
    file2.write_text("Content 2")

    config = ExtractionConfig(use_cache=False)
    result = batch_extract_file_sync([file1, file2], config)

    assert len(result) == 2
    assert result[0].content == "Content 1"
    assert result[1].content == "Content 2"


def test_batch_extract_file_sync_with_error() -> None:
    nonexistent_files = [Path("/nonexistent/file1.txt"), Path("/nonexistent/file2.txt")]

    result = batch_extract_file_sync(nonexistent_files)

    assert len(result) == 2
    for res in result:
        assert "Error:" in res.content
        assert "ValidationError" in res.content
        assert res.mime_type == "text/plain"
        assert "error" in res.metadata


def test_batch_extract_file_sync_worker_count() -> None:
    files = [f"/tmp/file_{i}.txt" for i in range(20)]

    with (
        patch("kreuzberg.extraction.ThreadPoolExecutor") as mock_executor,
        patch("kreuzberg.extraction.extract_file_sync"),
    ):
        mock_executor_instance = Mock()
        mock_executor.return_value.__enter__ = Mock(return_value=mock_executor_instance)
        mock_executor.return_value.__exit__ = Mock(return_value=None)

        mock_future = Mock()
        mock_future.result.return_value = (
            0,
            ExtractionResult(content="Test", mime_type="text/plain", metadata={}, chunks=[]),
        )
        mock_executor_instance.submit.return_value = mock_future

        with patch("kreuzberg.extraction.as_completed", return_value=[mock_future]):
            batch_extract_file_sync(files)

    expected_workers = min(len(files), mp.cpu_count())
    mock_executor.assert_called_once_with(max_workers=expected_workers)


def test_batch_extract_bytes_sync_empty_list() -> None:
    result = batch_extract_bytes_sync([])
    assert result == []


def test_batch_extract_bytes_sync_single_content() -> None:
    content = (b"Test content", "text/plain")

    result = batch_extract_bytes_sync([content])

    assert len(result) == 1
    assert result[0].content == "Test content"
    assert result[0].mime_type == "text/plain"


def test_batch_extract_bytes_sync_single_content_no_parallelism() -> None:
    content = (b"Test content", "text/plain")

    with patch("kreuzberg.extraction.ThreadPoolExecutor") as mock_executor:
        result = batch_extract_bytes_sync([content])

    mock_executor.assert_not_called()
    assert len(result) == 1
    assert result[0].content == "Test content"


def test_batch_extract_bytes_sync_multiple_contents() -> None:
    contents = [(b"Content 1", "text/plain"), (b"Content 2", "text/plain"), (b"Content 3", "text/plain")]

    result = batch_extract_bytes_sync(contents)

    assert len(result) == 3
    assert result[0].content == "Content 1"
    assert result[1].content == "Content 2"
    assert result[2].content == "Content 3"


def test_batch_extract_bytes_sync_with_error() -> None:
    with patch("kreuzberg.extraction.extract_bytes_sync") as mock_extract:
        mock_extract.side_effect = ValueError("Test error")

        contents = [(b"Test content", "text/plain"), (b"Another content", "text/plain")]
        result = batch_extract_bytes_sync(contents)

    assert len(result) == 2
    for res in result:
        assert "Error:" in res.content
        assert "ValueError" in res.content
        assert res.mime_type == "text/plain"
        assert "error" in res.metadata


def test_batch_extract_bytes_sync_worker_count() -> None:
    contents = [(b"Content", "text/plain") for _ in range(15)]

    with patch("kreuzberg.extraction.ThreadPoolExecutor") as mock_executor:
        mock_executor_instance = Mock()
        mock_executor.return_value.__enter__ = Mock(return_value=mock_executor_instance)
        mock_executor.return_value.__exit__ = Mock(return_value=None)

        mock_future = Mock()
        mock_future.result.return_value = (
            0,
            ExtractionResult(content="Test", mime_type="text/plain", metadata={}, chunks=[]),
        )
        mock_executor_instance.submit.return_value = mock_future

        with patch("kreuzberg.extraction.as_completed", return_value=[mock_future]):
            batch_extract_bytes_sync(contents)

    expected_workers = min(len(contents), mp.cpu_count())
    mock_executor.assert_called_once_with(max_workers=expected_workers)


@pytest.mark.anyio
async def test_batch_extract_bytes_error_context_includes_index() -> None:
    with patch("kreuzberg.extraction.extract_bytes") as mock_extract:
        mock_extract.side_effect = ValueError("Test extraction error")

        contents = [(b"Content 1", "text/plain"), (b"Content 2", "text/plain")]
        result = await batch_extract_bytes(contents)

    assert len(result) == 2

    assert result[0].metadata["error_context"]["index"] == 0
    assert result[0].metadata["error_context"]["operation"] == "batch_extract_bytes"
    assert result[0].metadata["error_context"]["mime_type"] == "text/plain"
    assert result[0].metadata["error_context"]["content_size"] == 9
    assert result[1].metadata["error_context"]["index"] == 1
    assert result[1].metadata["error_context"]["operation"] == "batch_extract_bytes"


@pytest.mark.anyio
async def test_batch_extract_file_error_context_includes_index() -> None:
    nonexistent_files = [Path("/nonexistent/file1.txt"), Path("/nonexistent/file2.txt")]

    result = await batch_extract_file(nonexistent_files)

    assert len(result) == 2

    assert result[0].metadata["error_context"]["index"] == 0
    assert result[0].metadata["error_context"]["operation"] == "batch_extract_file"
    assert "file_path" in result[0].metadata["error_context"] or "file1.txt" in str(result[0].metadata["error_context"])
    assert result[1].metadata["error_context"]["index"] == 1
    assert result[1].metadata["error_context"]["operation"] == "batch_extract_file"
    assert "file_path" in result[1].metadata["error_context"] or "file2.txt" in str(result[1].metadata["error_context"])


def test_batch_extract_bytes_sync_error_context_preserves_ordering() -> None:
    contents = [(b"Content 1", "text/plain"), (b"Content 2", "text/plain"), (b"Content 3", "text/plain")]

    with patch("kreuzberg.extraction.extract_bytes_sync") as mock_extract:

        def side_effect(content: Any, mime_type: Any, config: Any = None) -> ExtractionResult:
            content_str = content.decode("utf-8")
            if "Content 2" in content_str:
                raise ValueError("Error on content 2")
            return ExtractionResult(content=content_str, mime_type=mime_type, metadata={}, chunks=[])

        mock_extract.side_effect = side_effect
        result = batch_extract_bytes_sync(contents)

    assert len(result) == 3
    assert result[0].content == "Content 1"
    assert "Error:" in result[1].content
    assert "ValueError" in result[1].content
    assert result[2].content == "Content 3"


def test_batch_extract_file_sync_error_context_preserves_ordering() -> None:
    files = [
        Path("/file1.txt"),
        Path("/nonexistent.txt"),
        Path("/file3.txt"),
    ]

    with patch("kreuzberg.extraction.extract_file_sync") as mock_extract:

        def side_effect(file_path: Any, mime_type: Any = None, config: Any = None) -> ExtractionResult:
            if "nonexistent" in str(file_path):
                raise ValueError("File not found")
            return ExtractionResult(
                content=f"Content from {file_path.name}", mime_type="text/plain", metadata={}, chunks=[]
            )

        mock_extract.side_effect = side_effect
        result = batch_extract_file_sync(files)

    assert len(result) == 3
    assert "file1.txt" in result[0].content
    assert "Error:" in result[1].content
    assert "ValueError" in result[1].content
    assert "file3.txt" in result[2].content
