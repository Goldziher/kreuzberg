"""Exception classes for Kreuzberg.

All Kreuzberg exceptions inherit from KreuzbergError and support optional context
for debugging information.
"""

import json
from dataclasses import dataclass
from enum import IntEnum
from typing import Any


class ErrorCode(IntEnum):
    """Error codes from the Kreuzberg FFI layer.

    Attributes:
        SUCCESS: No error occurred
        GENERIC_ERROR: Generic/unknown error
        PANIC: A panic occurred in the library
        INVALID_ARGUMENT: Invalid argument provided
        IO_ERROR: I/O operation failed
        PARSING_ERROR: Document parsing failed
        OCR_ERROR: OCR processing failed
        MISSING_DEPENDENCY: Required dependency not found

    Example:
        >>> from kreuzberg import get_last_error_code, ErrorCode
        >>> code = get_last_error_code()
        >>> if code == ErrorCode.PANIC:
        ...     print("A panic occurred")
    """

    SUCCESS = 0
    GENERIC_ERROR = 1
    PANIC = 2
    INVALID_ARGUMENT = 3
    IO_ERROR = 4
    PARSING_ERROR = 5
    OCR_ERROR = 6
    MISSING_DEPENDENCY = 7


@dataclass(frozen=True, slots=True)
class PanicContext:
    """Structured panic context information from FFI layer.

    Attributes:
        file: Source file where panic occurred
        line: Line number in source file
        function: Function name where panic occurred
        message: Panic message
        timestamp_secs: Unix timestamp (seconds since epoch) when panic occurred

    Example:
        >>> import json
        >>> from kreuzberg import get_last_panic_context, PanicContext
        >>> context_json = get_last_panic_context()
        >>> if context_json:
        ...     data = json.loads(context_json)
        ...     context = PanicContext(**data)
        ...     print(f"Panic at {context.file}:{context.line}")
    """

    file: str
    line: int
    function: str
    message: str
    timestamp_secs: int

    @classmethod
    def from_json(cls, json_str: str) -> "PanicContext":
        """Parse panic context from JSON string.

        Args:
            json_str: JSON string with panic context (as returned by get_last_panic_context)

        Returns:
            PanicContext dataclass instance

        Raises:
            ValueError: If JSON is invalid or missing required fields
        """
        data = json.loads(json_str)
        return cls(**data)


class KreuzbergError(Exception):
    """Base exception class for all Kreuzberg errors.

    All Kreuzberg exceptions support an optional context dictionary for debugging
    information. The context is serialized to JSON when the exception is converted
    to a string.

    Args:
        message: Human-readable error message
        context: Optional dictionary with debugging context (file paths, config, etc.)

    Example:
        >>> raise KreuzbergError("Failed to parse document", context={"file": "document.pdf", "page": 5})

    """

    def __init__(self, message: str, *, context: dict[str, Any] | None = None) -> None:
        super().__init__(message)
        self.message = message
        self.context = context

    def __str__(self) -> str:
        """Format error with context as JSON."""
        error_name = self.__class__.__name__
        if self.context:
            serialized_context = self._serialize_context(self.context)
            context_json = json.dumps(serialized_context, sort_keys=True)
            return f"{error_name}: {self.message}\nContext: {context_json}"
        return f"{error_name}: {self.message}"

    @staticmethod
    def _serialize_context(context: dict[str, Any]) -> dict[str, Any]:
        def serialize_value(value: Any) -> Any:
            if isinstance(value, bytes):
                return value.decode("utf-8", errors="replace")
            if isinstance(value, Exception):
                return {"type": type(value).__name__, "message": str(value)}
            if isinstance(value, tuple):
                return [serialize_value(item) for item in value]
            if isinstance(value, list):
                return [serialize_value(item) for item in value]
            if isinstance(value, dict):
                return {k: serialize_value(v) for k, v in value.items()}
            return value

        serialized: dict[str, Any] = serialize_value(context)
        return serialized


class ValidationError(KreuzbergError):
    """Raised when input validation fails.

    This includes configuration validation, parameter validation, and input
    data validation errors.

    Example:
        >>> raise ValidationError("Invalid language code", context={"language": "xyz", "supported": ["en", "de"]})

    """


class ParsingError(KreuzbergError):
    """Raised when document parsing fails.

    This includes errors from extractors when they cannot parse a document
    (corrupt files, unsupported features, etc.).

    Example:
        >>> raise ParsingError("Failed to parse PDF", context={"file": "document.pdf", "extractor": "pdf"})

    """


class OCRError(KreuzbergError):
    """Raised when OCR processing fails.

    This includes errors from OCR backends during text extraction from images.

    Example:
        >>> raise OCRError("OCR processing failed", context={"backend": "tesseract", "language": "en"})

    """


class MissingDependencyError(KreuzbergError):
    """Raised when a required dependency is not installed.

    This includes missing Python packages and missing system dependencies
    (tesseract, pandoc, etc.).

    Example:
        >>> raise MissingDependencyError(
        ...     "EasyOCR not installed", context={"package": "easyocr", "install_command": "pip install kreuzberg[easyocr]"}
        ... )

    """

    @classmethod
    def create_for_package(
        cls,
        *,
        dependency_group: str,
        functionality: str,
        package_name: str,
    ) -> "MissingDependencyError":
        """Create a MissingDependencyError for a missing package.

        This is a convenience method for creating standardized error messages
        for missing optional dependencies.

        Args:
            dependency_group: The optional dependency group (e.g., "ocr", "api", "cli")
            functionality: Description of what functionality requires this package
            package_name: Name of the missing package

        Returns:
            MissingDependencyError with formatted message and context

        Example:
            >>> error = MissingDependencyError.create_for_package(
            ...     dependency_group="easyocr", functionality="EasyOCR backend", package_name="easyocr"
            ... )
            >>> raise error

        """
        install_cmd = f"pip install kreuzberg[{dependency_group}]"
        message = f"Missing required dependency '{package_name}' for {functionality}. Install with: {install_cmd}"
        context = {
            "package": package_name,
            "dependency_group": dependency_group,
            "functionality": functionality,
            "install_command": install_cmd,
        }
        return cls(message, context=context)


class CacheError(KreuzbergError):
    """Raised when cache operations fail.

    This includes errors during cache reads, writes, or invalidations.

    Example:
        >>> raise CacheError("Failed to write cache", context={"path": "/tmp/cache", "operation": "write"})

    """


class ImageProcessingError(KreuzbergError):
    """Raised when image manipulation fails.

    This includes errors during image resizing, format conversion, or other
    image processing operations.

    Example:
        >>> raise ImageProcessingError("Failed to resize image", context={"width": 1920, "height": 1080})

    """


class PluginError(KreuzbergError):
    """Raised when plugin operations fail.

    This includes errors during plugin initialization, registration, or execution.

    Example:
        >>> raise PluginError("Plugin initialization failed", context={"plugin_name": "pdf-extractor"})

    """
