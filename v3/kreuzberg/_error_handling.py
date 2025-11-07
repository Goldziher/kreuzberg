"""Type-safe error handling utilities for extraction pipeline."""

from __future__ import annotations

import traceback
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Callable

from kreuzberg._types import ErrorContextType, ExtractionResult, Metadata, ProcessingErrorDict
from kreuzberg.exceptions import KreuzbergError, MissingDependencyError, ValidationError


def should_exception_bubble_up(exception: Exception, context: ErrorContextType = "unknown") -> bool:
    """Determine if an exception should bubble up or be handled gracefully.

    Args:
        exception: The exception to classify
        context: The context where the exception occurred (e.g., "batch_processing", "single_extraction", "optional_feature")

    Returns:
        True if the exception should bubble up, False if it should be handled gracefully

    """
    if isinstance(exception, (SystemExit, KeyboardInterrupt, MemoryError, OSError, RuntimeError)):
        return True

    if isinstance(exception, MissingDependencyError):
        return True

    if isinstance(exception, ValidationError):
        if context == "batch_processing":
            return False

        return context != "optional_feature"

    if isinstance(exception, KreuzbergError) and context == "optional_feature":
        return False

    if context == "batch_processing":
        return isinstance(exception, (SystemExit, KeyboardInterrupt, MemoryError, OSError, RuntimeError))

    return not (context == "optional_feature" and isinstance(exception, (IOError, ImportError)))


class FeatureProcessingError:
    """Type-safe processing error for extraction features."""

    def __init__(self, feature: str, error: Exception) -> None:
        self._feature = feature
        self._error = error
        self._traceback = traceback.format_exc()

    @property
    def feature(self) -> str:
        return self._feature

    @property
    def error_type(self) -> str:
        return type(self._error).__name__

    @property
    def error_message(self) -> str:
        return str(self._error)

    @property
    def traceback(self) -> str:
        return self._traceback

    def to_dict(self) -> ProcessingErrorDict:
        return {
            "feature": self.feature,
            "error_type": self.error_type,
            "error_message": self.error_message,
            "traceback": self.traceback,
        }


def safe_feature_execution(
    feature_name: str,
    execution_func: Callable[[], Any],
    default_value: Any,
    result: ExtractionResult,
    context: ErrorContextType = "optional_feature",
) -> Any:
    """Safely execute a feature extraction function with proper error handling.

    Args:
        feature_name: Name of the feature being executed
        execution_func: Function to execute that may raise exceptions
        default_value: Default value to return if execution fails
        result: ExtractionResult to update with error information
        context: The context for exception handling decisions

    Returns:
        Either the successful result or the default value

    """
    try:
        return execution_func()
    except Exception as e:
        if should_exception_bubble_up(e, context):
            raise

        _add_processing_error(result, FeatureProcessingError(feature_name, e))
        return default_value


def _add_processing_error(result: ExtractionResult, error: FeatureProcessingError) -> None:
    """Add a processing error to the result metadata in a type-safe way."""
    if result.metadata is None:
        result.metadata = {}

    if "processing_errors" not in result.metadata:
        result.metadata["processing_errors"] = []

    errors_list = result.metadata["processing_errors"]
    if isinstance(errors_list, list):
        errors_list.append(error.to_dict())
    else:
        result.metadata["processing_errors"] = [error.to_dict()]


def preserve_result_with_errors(
    result: ExtractionResult,
    errors: list[FeatureProcessingError],
) -> ExtractionResult:
    """Preserve a successful extraction result while adding error information.

    This is used when core extraction succeeds but optional features fail.

    Args:
        result: The successful extraction result
        errors: List of errors that occurred during optional processing

    Returns:
        The result with error information added to metadata

    """
    for error in errors:
        _add_processing_error(result, error)

    return result


def create_error_result(
    content: str,
    mime_type: str,
    errors: list[FeatureProcessingError],
    **metadata_kwargs: Any,
) -> ExtractionResult:
    """Create an error result with proper type safety.

    Args:
        content: Error content to include
        mime_type: MIME type of the result
        errors: List of errors that occurred
        **metadata_kwargs: Additional metadata to include

    Returns:
        An ExtractionResult with error information

    """
    metadata: Metadata = {
        "error": f"Multiple processing errors occurred: {len(errors)} errors",
        "error_context": {
            "error_count": len(errors),
            "errors": [error.to_dict() for error in errors],
            **metadata_kwargs,
        },
        "processing_errors": [error.to_dict() for error in errors],
    }

    return ExtractionResult(
        content=content,
        chunks=[],
        mime_type=mime_type,
        metadata=metadata,
        entities=[],
        keywords=[],
        detected_languages=[],
        tables=[],
        images=[],
        image_ocr_results=[],
    )
