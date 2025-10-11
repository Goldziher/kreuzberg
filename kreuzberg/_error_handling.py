from __future__ import annotations

import traceback
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Callable

from kreuzberg._types import ErrorContextType, ExtractionResult, Metadata, ProcessingErrorDict
from kreuzberg.exceptions import KreuzbergError, MissingDependencyError, ValidationError


def should_exception_bubble_up(exception: Exception, context: ErrorContextType = "unknown") -> bool:
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
    try:
        return execution_func()
    except Exception as e:
        if should_exception_bubble_up(e, context):
            raise

        _add_processing_error(result, FeatureProcessingError(feature_name, e))
        return default_value


def _add_processing_error(result: ExtractionResult, error: FeatureProcessingError) -> None:
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
    for error in errors:
        _add_processing_error(result, error)

    return result


def create_error_result(
    content: str,
    mime_type: str,
    errors: list[FeatureProcessingError],
    **metadata_kwargs: Any,
) -> ExtractionResult:
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
