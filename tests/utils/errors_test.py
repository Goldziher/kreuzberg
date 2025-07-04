"""Tests for error handling utilities."""

from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING
from unittest.mock import patch

import pytest

from kreuzberg._utils._errors import (
    BatchExtractionResult,
    create_error_context,
    is_resource_error,
    is_transient_error,
    should_retry,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def test_file(tmp_path: Path) -> Path:
    """Create a test file."""
    file_path = tmp_path / "test.txt"
    file_path.write_text("test content")
    return file_path


def test_create_error_context_basic() -> None:
    """Test basic error context creation."""
    context = create_error_context(operation="test_op")

    assert context["operation"] == "test_op"
    assert "timestamp" in context
    assert datetime.fromisoformat(context["timestamp"])


def test_create_error_context_with_file(test_file: Path) -> None:
    """Test error context with file information."""
    context = create_error_context(
        operation="file_read",
        file_path=test_file,
    )

    assert "file" in context
    assert context["file"]["path"] == str(test_file)
    assert context["file"]["name"] == "test.txt"
    assert context["file"]["exists"] is True
    assert context["file"]["size"] == 12


def test_create_error_context_with_nonexistent_file() -> None:
    """Test error context with non-existent file."""
    context = create_error_context(
        operation="file_read",
        file_path="/nonexistent/file.txt",
    )

    assert "file" in context
    assert context["file"]["exists"] is False
    assert context["file"]["size"] is None


def test_create_error_context_with_error() -> None:
    """Test error context with exception."""
    try:
        raise ValueError("Test error message")
    except ValueError as e:
        context = create_error_context(
            operation="test_op",
            error=e,
        )

    assert "error" in context
    assert context["error"]["type"] == "ValueError"
    assert context["error"]["message"] == "Test error message"
    assert "traceback" in context["error"]
    assert any("ValueError" in line for line in context["error"]["traceback"])


def test_create_error_context_with_extra() -> None:
    """Test error context with extra fields."""
    context = create_error_context(
        operation="test_op",
        custom_field="custom_value",
        count=42,
    )

    assert context["custom_field"] == "custom_value"
    assert context["count"] == 42


def test_create_error_context_with_resource_error() -> None:
    """Test error context with resource-related error includes system info."""
    try:
        raise MemoryError("Out of memory")
    except MemoryError as e:
        context = create_error_context(
            operation="process",
            error=e,
        )

    assert "system" in context
    assert "memory_available_mb" in context["system"]
    assert "memory_percent" in context["system"]
    assert "cpu_percent" in context["system"]
    assert "process_count" in context["system"]
    assert "platform" in context["system"]


def test_create_error_context_system_info_error() -> None:
    """Test error context when system info collection fails."""
    with patch("psutil.virtual_memory", side_effect=Exception("psutil error")):
        try:
            raise RuntimeError("Resource exhausted")
        except RuntimeError as e:
            context = create_error_context(
                operation="test",
                error=e,
            )

        assert "system" not in context
        assert "error" in context


def test_create_error_context_with_path_string() -> None:
    """Test error context with string path."""
    import os

    test_path = "/some/path/file.txt"
    context = create_error_context(
        operation="test",
        file_path=test_path,
    )

    expected_path = os.path.normpath(test_path)
    assert context["file"]["path"] == expected_path
    assert context["file"]["name"] == "file.txt"


def test_is_transient_error() -> None:
    """Test transient error detection."""

    assert is_transient_error(OSError("Resource temporarily unavailable"))
    assert is_transient_error(PermissionError("Access denied"))
    assert is_transient_error(TimeoutError("Operation timed out"))
    assert is_transient_error(ConnectionError("Connection refused"))
    assert is_transient_error(OSError("Too many open files"))
    assert is_transient_error(RuntimeError("Cannot allocate memory"))
    assert is_transient_error(BrokenPipeError("Broken pipe"))
    assert is_transient_error(FileExistsError("File is locked"))
    assert is_transient_error(OSError("File in use"))

    assert not is_transient_error(ValueError("Invalid value"))
    assert not is_transient_error(TypeError("Wrong type"))
    assert not is_transient_error(KeyError("Missing key"))
    assert not is_transient_error(AttributeError("No attribute"))


def test_is_resource_error() -> None:
    """Test resource error detection."""

    assert is_resource_error(MemoryError("Out of memory"))
    assert is_resource_error(OSError("Cannot allocate memory"))
    assert is_resource_error(OSError("Too many open files"))
    assert is_resource_error(RuntimeError("Resource exhausted"))
    assert is_resource_error(RuntimeError("Thread limit reached"))
    assert is_resource_error(RuntimeError("Process limit exceeded"))
    assert is_resource_error(OSError("File descriptor limit"))
    assert is_resource_error(RuntimeError("CPU usage too high"))

    assert not is_resource_error(ValueError("Invalid value"))
    assert not is_resource_error(TypeError("Wrong type"))
    assert not is_resource_error(FileNotFoundError("File not found"))
    assert not is_resource_error(KeyError("Missing key"))


def test_should_retry() -> None:
    """Test retry logic."""

    error = TimeoutError("Request timed out")
    assert should_retry(error, attempt=1, max_attempts=3)
    assert should_retry(error, attempt=2, max_attempts=3)
    assert not should_retry(error, attempt=3, max_attempts=3)

    val_error = ValidationError("Invalid input")
    assert not should_retry(val_error, attempt=1)

    val_error2 = ValueError("Bad value")
    assert not should_retry(val_error2, attempt=1)


def test_should_retry_max_attempts() -> None:
    """Test retry logic respects max attempts."""
    error = TimeoutError("Timeout")

    assert should_retry(error, attempt=1)
    assert should_retry(error, attempt=2)
    assert not should_retry(error, attempt=3)

    assert should_retry(error, attempt=4, max_attempts=5)
    assert not should_retry(error, attempt=5, max_attempts=5)


class TestBatchExtractionResult:
    """Tests for BatchExtractionResult class."""

    def test_init(self) -> None:
        """Test initialization."""
        result = BatchExtractionResult()

        assert result.successful == []
        assert result.failed == []
        assert result.total_count == 0
        assert result.success_count == 0
        assert result.failure_count == 0
        assert result.success_rate == 0.0

    def test_add_success(self) -> None:
        """Test adding successful results."""
        result = BatchExtractionResult()
        result.total_count = 3

        result.add_success(0, "Result 1")
        result.add_success(2, "Result 3")

        assert result.success_count == 2
        assert result.successful == [(0, "Result 1"), (2, "Result 3")]

    def test_add_failure(self) -> None:
        """Test adding failed results."""
        result = BatchExtractionResult()
        result.total_count = 3

        error = ValueError("Test error")
        context = {"file": "test.txt"}

        result.add_failure(1, error, context)

        assert result.failure_count == 1
        assert len(result.failed) == 1

        index, error_info = result.failed[0]
        assert index == 1
        assert error_info["error"]["type"] == "ValueError"
        assert error_info["error"]["message"] == "Test error"
        assert error_info["context"] == context

    def test_success_rate(self) -> None:
        """Test success rate calculation."""
        result = BatchExtractionResult()
        result.total_count = 4

        result.add_success(0, "R1")
        result.add_success(1, "R2")
        result.add_success(3, "R4")
        result.add_failure(2, ValueError("Error"), {})

        assert result.success_rate == 75.0

    def test_get_ordered_results(self) -> None:
        """Test getting results in original order."""
        result = BatchExtractionResult()
        result.total_count = 5

        result.add_success(0, "First")
        result.add_success(2, "Third")
        result.add_success(4, "Fifth")
        result.add_failure(1, ValueError("E1"), {})
        result.add_failure(3, RuntimeError("E2"), {})

        ordered = result.get_ordered_results()

        assert ordered == ["First", None, "Third", None, "Fifth"]

    def test_get_summary(self) -> None:
        """Test getting operation summary."""
        result = BatchExtractionResult()
        result.total_count = 3

        result.add_success(0, "OK1")
        result.add_success(2, "OK2")
        result.add_failure(1, ValueError("Bad value"), {"file": "test.txt"})

        summary = result.get_summary()

        assert summary["total"] == 3
        assert summary["successful"] == 2
        assert summary["failed"] == 1
        assert summary["success_rate"] == "66.7%"

        assert len(summary["failures"]) == 1
        failure = summary["failures"][0]
        assert failure["index"] == 1
        assert failure["error"] == "ValueError"
        assert failure["message"] == "Bad value"

    def test_empty_batch_success_rate(self) -> None:
        """Test success rate for empty batch."""
        result = BatchExtractionResult()

        assert result.success_rate == 0.0

        summary = result.get_summary()
        assert summary["success_rate"] == "0.0%"
