"""Kreuzberg native backend implementation."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from kreuzberg._backends.base import ExtractionBackend
from kreuzberg._mime_types import SUPPORTED_MIME_TYPES, validate_mime_type
from kreuzberg.extraction import extract_bytes_sync, extract_file_sync

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._types import ExtractionConfig, ExtractionResult


class KreuzbergBackend(ExtractionBackend):
    """Native Kreuzberg backend for text extraction."""

    def __init__(self, config: ExtractionConfig | None = None) -> None:
        """Initialize the Kreuzberg backend.

        Args:
            config: Optional extraction configuration
        """
        self.config = config

    def extract(self, file_path: str | Path, **kwargs: Any) -> ExtractionResult:
        """Extract text from a file using Kreuzberg.

        Args:
            file_path: Path to the file to extract from
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """
        config = kwargs.get("config", self.config)
        if config is None:
            from kreuzberg._types import ExtractionConfig

            config = ExtractionConfig()
        result = extract_file_sync(file_path, config=config)

        # Add backend information to metadata
        if result.metadata is None:
            result.metadata = {}
        result.metadata["backend"] = "kreuzberg"

        return result

    def extract_bytes(self, data: bytes, mime_type: str, **kwargs: Any) -> ExtractionResult:
        """Extract text from raw bytes using Kreuzberg.

        Args:
            data: Raw bytes of the document
            mime_type: MIME type of the document
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """
        config = kwargs.get("config", self.config)
        if config is None:
            from kreuzberg._types import ExtractionConfig

            config = ExtractionConfig()
        result = extract_bytes_sync(data, mime_type=mime_type, config=config)

        # Add backend information to metadata
        if result.metadata is None:
            result.metadata = {}
        result.metadata["backend"] = "kreuzberg"

        return result

    def supports_format(self, file_type: str) -> bool:
        """Check if Kreuzberg supports a given file format.

        Args:
            file_type: File extension or MIME type to check

        Returns:
            True if the format is supported, False otherwise
        """
        try:
            # Try to validate the MIME type
            if file_type.startswith("."):
                # Convert extension to MIME type
                from kreuzberg._mime_types import EXT_TO_MIME_TYPE

                mime_type = EXT_TO_MIME_TYPE.get(file_type.lower())
                if mime_type:
                    validate_mime_type(mime_type=mime_type, check_file_exists=False)
                    return True
            else:
                # Assume it's already a MIME type
                validate_mime_type(mime_type=file_type, check_file_exists=False)
                return True
        except Exception:
            return False

        return False

    def get_capabilities(self) -> dict[str, Any]:
        """Get information about Kreuzberg's capabilities.

        Returns:
            Dictionary containing capability information
        """
        return {
            "name": "kreuzberg",
            "version": "3.7.0",
            "supported_formats": list(SUPPORTED_MIME_TYPES),
            "supports_ocr": True,
            "supports_async": True,
            "supports_batch": True,
            "memory_efficient": True,
            "speed_optimized": True,
            "exclusive_formats": [],  # No exclusive formats for Kreuzberg
            "preferred_formats": [
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",  # XLSX
                "application/vnd.ms-excel",  # XLS
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",  # PPTX
                "text/markdown",  # MD
                "image/*",  # Images (OCR)
            ],
        }

    @property
    def name(self) -> str:
        """Get the name of this backend."""
        return "kreuzberg"
