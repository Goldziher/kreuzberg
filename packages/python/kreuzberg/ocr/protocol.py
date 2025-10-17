"""Protocol for Python OCR backends compatible with Rust FFI bridge.

This module defines the interface that Python OCR backends must implement
to be registered with the Rust extraction core via the FFI bridge.
"""

from __future__ import annotations

from typing import Any, Protocol


class OcrBackendProtocol(Protocol):
    """Protocol for OCR backends compatible with Rust FFI bridge.

    Python OCR backends implementing this protocol can be registered with the
    Rust extraction core using `register_ocr_backend()`. Once registered, they
    can be used by all extraction APIs (Python, Rust CLI, API server, MCP).

    Required Methods:
        name: Return backend name (e.g., 'easyocr', 'paddleocr')
        supported_languages: Return list of supported language codes
        process_image: Process image bytes and return extraction result

    Optional Methods:
        process_file: Process file from path (defaults to reading and calling process_image)
        initialize: Called when backend is registered (e.g., load models)
        shutdown: Called when backend is unregistered (e.g., cleanup resources)
        version: Return backend version string (defaults to '1.0.0')

    Example:
        >>> class MyOcrBackend:
        ...     def name(self) -> str:
        ...         return "my-ocr"
        ...
        ...     def supported_languages(self) -> list[str]:
        ...         return ["eng", "deu", "fra"]
        ...
        ...     def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        ...         # Process image and extract text
        ...         return {
        ...             "content": "extracted text",
        ...             "metadata": {"confidence": 0.95, "width": 800, "height": 600},
        ...             "tables": [],
        ...         }
        >>> from kreuzberg import register_ocr_backend
        >>> backend = MyOcrBackend()
        >>> register_ocr_backend(backend)

    """

    def name(self) -> str:
        """Return backend name (e.g., 'easyocr', 'paddleocr').

        The name must be unique across all registered backends and is used
        to identify and select the backend for OCR operations.

        Returns:
            Backend name as lowercase string with no spaces

        """
        ...

    def supported_languages(self) -> list[str]:
        """Return list of supported language codes.

        Language codes should use standard ISO 639 codes (e.g., 'eng', 'deu')
        or library-specific codes (e.g., 'ch_sim', 'ch_tra' for Chinese).

        Returns:
            List of supported language code strings

        """
        ...

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        """Process image bytes and return extraction result.

        This is the core OCR method that takes raw image data and returns
        extracted text with metadata. The method should handle all image
        formats supported by the backend.

        Args:
            image_bytes: Raw image data (PNG, JPEG, TIFF, etc.)
            language: Language code for OCR (must be in supported_languages())

        Returns:
            Dictionary with extraction result:
            {
                "content": "extracted text content",
                "metadata": {
                    "width": 800,           # Optional: image width
                    "height": 600,          # Optional: image height
                    "confidence": 0.95,     # Optional: OCR confidence score
                    ...                     # Any other metadata
                },
                "tables": []                # Optional: extracted tables
            }

        """
        ...

    # Optional methods below

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        """Process file from path (optional, defaults to reading and calling process_image).

        Backends can override this method if they have optimized file processing
        that avoids loading the entire file into memory.

        Args:
            path: Path to image file
            language: Language code for OCR

        Returns:
            Same format as process_image()

        """
        ...

    def initialize(self) -> None:
        """Initialize backend (optional, called when backend is registered).

        Use this method to:
        - Load ML models
        - Initialize GPU/CUDA
        - Download required data files
        - Set up caching

        This method is called exactly once when the backend is registered
        via `register_ocr_backend()`.
        """
        ...

    def shutdown(self) -> None:
        """Shutdown backend (optional, called when backend is unregistered).

        Use this method to:
        - Cleanup temporary files
        - Release GPU memory
        - Close file handles
        - Save cached data

        This method is called when the backend is unregistered via
        `unregister_ocr_backend()` or when the program exits.
        """
        ...

    def version(self) -> str:
        """Return backend version (optional, defaults to '1.0.0').

        Returns:
            Version string (e.g., '2.1.0')

        """
        ...
