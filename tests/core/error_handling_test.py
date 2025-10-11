from __future__ import annotations

import pytest

from kreuzberg._error_handling import (
    FeatureProcessingError,
    create_error_result,
    preserve_result_with_errors,
    safe_feature_execution,
    should_exception_bubble_up,
)
from kreuzberg._types import ExtractionResult
from kreuzberg.exceptions import MissingDependencyError, OCRError, ParsingError, ValidationError


def test_should_exception_bubble_up_system_exit() -> None:
    assert should_exception_bubble_up(SystemExit(0)) is True  # type: ignore[arg-type]


def test_should_exception_bubble_up_keyboard_interrupt() -> None:
    assert should_exception_bubble_up(KeyboardInterrupt()) is True  # type: ignore[arg-type]


def test_should_exception_bubble_up_memory_error() -> None:
    assert should_exception_bubble_up(MemoryError()) is True


def test_should_exception_bubble_up_os_error() -> None:
    assert should_exception_bubble_up(OSError("test")) is True


def test_should_exception_bubble_up_runtime_error() -> None:
    assert should_exception_bubble_up(RuntimeError("test")) is True


def test_should_exception_bubble_up_missing_dependency() -> None:
    error = MissingDependencyError.create_for_package(
        dependency_group="test", functionality="test", package_name="test_package"
    )
    assert should_exception_bubble_up(error) is True


def test_should_exception_bubble_up_validation_error_batch_processing() -> None:
    error = ValidationError("test validation", context={})
    assert should_exception_bubble_up(error, context="batch_processing") is False


def test_should_exception_bubble_up_validation_error_optional_feature() -> None:
    error = ValidationError("test validation", context={})
    assert should_exception_bubble_up(error, context="optional_feature") is False


def test_should_exception_bubble_up_validation_error_default() -> None:
    error = ValidationError("test validation", context={})
    assert should_exception_bubble_up(error, context="unknown") is True


def test_should_exception_bubble_up_kreuzberg_error_optional_feature() -> None:
    error = OCRError("test OCR error", context={})
    assert should_exception_bubble_up(error, context="optional_feature") is False


def test_should_exception_bubble_up_kreuzberg_error_batch_processing() -> None:
    error = ParsingError("test parsing error", context={})
    assert should_exception_bubble_up(error, context="batch_processing") is False


def test_should_exception_bubble_up_io_error_optional_feature() -> None:
    error = OSError("test IO error")
    assert should_exception_bubble_up(error, context="optional_feature") is True


def test_should_exception_bubble_up_import_error_optional_feature() -> None:
    error = ImportError("test import error")
    assert should_exception_bubble_up(error, context="optional_feature") is False


def test_should_exception_bubble_up_import_error_default() -> None:
    error = ImportError("test import error")
    assert should_exception_bubble_up(error, context="unknown") is True


def test_feature_processing_error_properties() -> None:
    try:
        raise ValueError("test error message")
    except ValueError as e:
        error = FeatureProcessingError("test_feature", e)
        assert error.feature == "test_feature"
        assert error.error_type == "ValueError"
        assert error.error_message == "test error message"
        assert isinstance(error.traceback, str)
        assert "ValueError" in error.traceback


def test_feature_processing_error_to_dict() -> None:
    try:
        raise RuntimeError("runtime error")
    except RuntimeError as e:
        error = FeatureProcessingError("runtime_feature", e)
        error_dict = error.to_dict()
        assert error_dict["feature"] == "runtime_feature"
        assert error_dict["error_type"] == "RuntimeError"
        assert error_dict["error_message"] == "runtime error"
        assert "traceback" in error_dict


def test_safe_feature_execution_success() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    def successful_func() -> str:
        return "success_value"

    value = safe_feature_execution("test_feature", successful_func, "default", result)
    assert value == "success_value"
    assert "processing_errors" not in result.metadata


def test_safe_feature_execution_failure_with_default() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    def failing_func() -> None:
        raise ImportError("test failure")

    value = safe_feature_execution("test_feature", failing_func, "default_value", result, context="optional_feature")
    assert value == "default_value"
    assert "processing_errors" in result.metadata
    assert len(result.metadata["processing_errors"]) == 1
    assert result.metadata["processing_errors"][0]["feature"] == "test_feature"


def test_safe_feature_execution_bubble_up() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    def failing_func() -> None:
        raise OSError("system error")

    with pytest.raises(OSError, match="system error"):
        safe_feature_execution("test_feature", failing_func, "default", result)


def test_safe_feature_execution_no_metadata() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    def failing_func() -> None:
        raise ImportError("test failure")

    value = safe_feature_execution("test_feature", failing_func, "default_value", result, context="optional_feature")
    assert value == "default_value"
    assert result.metadata is not None
    assert "processing_errors" in result.metadata


def test_safe_feature_execution_existing_processing_errors() -> None:
    result = ExtractionResult(
        content="test",
        mime_type="text/plain",
        metadata={
            "processing_errors": [
                {"feature": "previous", "error_type": "Error", "error_message": "prev", "traceback": "prev traceback"}
            ]
        },
        chunks=[],
    )

    def failing_func() -> None:
        raise ImportError("new failure")

    safe_feature_execution("test_feature", failing_func, "default", result, context="optional_feature")
    assert len(result.metadata["processing_errors"]) == 2


def test_safe_feature_execution_malformed_processing_errors() -> None:
    result = ExtractionResult(
        content="test",
        mime_type="text/plain",
        metadata={"processing_errors": "not_a_list"},  # type: ignore[typeddict-item]
        chunks=[],
    )

    def failing_func() -> None:
        raise ImportError("test failure")

    safe_feature_execution("test_feature", failing_func, "default", result, context="optional_feature")
    assert isinstance(result.metadata["processing_errors"], list)
    assert len(result.metadata["processing_errors"]) == 1


def test_preserve_result_with_errors_single_error() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    try:
        raise ValueError("error1")
    except ValueError as e:
        error = FeatureProcessingError("feature1", e)

    preserved = preserve_result_with_errors(result, [error])
    assert "processing_errors" in preserved.metadata
    assert len(preserved.metadata["processing_errors"]) == 1


def test_preserve_result_with_errors_multiple_errors() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    try:
        raise ValueError("error0")
    except ValueError as e0:
        error0 = FeatureProcessingError("feature0", e0)
    try:
        raise ValueError("error1")
    except ValueError as e1:
        error1 = FeatureProcessingError("feature1", e1)
    try:
        raise ValueError("error2")
    except ValueError as e2:
        error2 = FeatureProcessingError("feature2", e2)

    errors = [error0, error1, error2]

    preserved = preserve_result_with_errors(result, errors)
    assert len(preserved.metadata["processing_errors"]) == 3


def test_preserve_result_with_errors_no_metadata() -> None:
    result = ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[])

    try:
        raise ValueError("error")
    except ValueError as e:
        error = FeatureProcessingError("feature", e)

    preserved = preserve_result_with_errors(result, [error])
    assert preserved.metadata is not None
    assert "processing_errors" in preserved.metadata


def test_create_error_result() -> None:
    try:
        raise ValueError("error1")
    except ValueError as e1:
        error1 = FeatureProcessingError("feature1", e1)

    try:
        raise RuntimeError("error2")
    except RuntimeError as e2:
        error2 = FeatureProcessingError("feature2", e2)

    result = create_error_result("error content", "text/plain", [error1, error2], extra_info="test")

    assert result.content == "error content"
    assert result.mime_type == "text/plain"
    assert "error" in result.metadata
    assert "Multiple processing errors occurred: 2 errors" in result.metadata["error"]
    assert result.metadata["error_context"]["error_count"] == 2
    assert result.metadata["error_context"]["extra_info"] == "test"
    assert len(result.metadata["processing_errors"]) == 2
    assert result.chunks == []
    assert result.entities == []


def test_create_error_result_single_error() -> None:
    try:
        raise ValueError("single error")
    except ValueError as e:
        error = FeatureProcessingError("feature", e)

    result = create_error_result("error", "text/plain", [error])
    assert "1 errors" in result.metadata["error"]
    assert result.metadata["error_context"]["error_count"] == 1


def test_create_error_result_with_metadata_kwargs() -> None:
    try:
        raise ValueError("error")
    except ValueError as e:
        error = FeatureProcessingError("feature", e)

    result = create_error_result("error", "text/plain", [error], file_path="/test/path", operation="test_op")

    assert result.metadata["error_context"]["file_path"] == "/test/path"
    assert result.metadata["error_context"]["operation"] == "test_op"
