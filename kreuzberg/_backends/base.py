"""Base interface for text extraction backends."""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._types import ExtractionResult


class ExtractionBackend(ABC):
    """Abstract base class for text extraction backends."""

    @abstractmethod
    def extract(self, file_path: str | Path, **kwargs: Any) -> ExtractionResult:
        """Extract text from a file.

        Args:
            file_path: Path to the file to extract from
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """

    @abstractmethod
    def extract_bytes(self, data: bytes, mime_type: str, **kwargs: Any) -> ExtractionResult:
        """Extract text from raw bytes.

        Args:
            data: Raw bytes of the document
            mime_type: MIME type of the document
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """

    @abstractmethod
    def supports_format(self, file_type: str) -> bool:
        """Check if this backend supports a given file format.

        Args:
            file_type: File extension or MIME type to check

        Returns:
            True if the format is supported, False otherwise
        """

    @abstractmethod
    def get_capabilities(self) -> dict[str, Any]:
        """Get information about this backend's capabilities.

        Returns:
            Dictionary containing capability information
        """

    @property
    @abstractmethod
    def name(self) -> str:
        """Get the name of this backend."""
