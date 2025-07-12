from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, ClassVar

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg import ExtractionResult
    from kreuzberg._types import ExtractionConfig


class Extractor(ABC):
    """Abstract base class for file content extraction.

    This class provides the interface for different types of content extractors.
    Subclasses are expected to implement the methods for extracting content
    either asynchronously or synchronously and determining the supported MIME types.

    Attributes:
        SUPPORTED_MIME_TYPES: The set of supported mime types - all none abstract extractors must implement this.

    Args:
        mime_type: The MIME type that this extractor handles (e.g., "application/pdf").
        config: Configuration options for the extraction process.
    """

    __slots__ = ("config", "mime_type")

    SUPPORTED_MIME_TYPES: ClassVar[set[str]]

    def __init__(self, mime_type: str, config: ExtractionConfig) -> None:
        self.mime_type = mime_type
        self.config = config

    @abstractmethod
    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        """Asynchronously extract content from a byte stream.

        Args:
            content: The byte content to extract.

        Returns:
            ExtractionResult: The extracted content along with metadata about the extraction.
        """

    @abstractmethod
    async def extract_path_async(self, path: Path) -> ExtractionResult:
        """Asynchronously extract content from a file located at the specified path.

        Args:
            path: The path to the file to process.

        Returns:
            ExtractionResult: The extracted content along with metadata about the extraction.
        """

    @abstractmethod
    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Synchronously extract content from a byte stream.

        Args:
            content: The byte content to extract.

        Returns:
            ExtractionResult: The extracted content along with metadata about the extraction.
        """

    @abstractmethod
    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Synchronously extract content from a file located at the specified path.

        Args:
            path: The path to the file to process.

        Returns:
            ExtractionResult: The extracted content along with metadata about the extraction.
        """

    @classmethod
    def supports_mimetype(cls, mime_type: str) -> bool:
        """Verify whether the extractor supports the given MIME type.

        Args:
            mime_type: The MIME type to check (e.g., "application/pdf").

        Returns:
            bool: True if the MIME type is supported, False otherwise.
        """
        return mime_type in cls.SUPPORTED_MIME_TYPES or any(
            mime_type.startswith(supported_type) for supported_type in cls.SUPPORTED_MIME_TYPES
        )

    def _apply_quality_processing(self, result: ExtractionResult) -> ExtractionResult:
        """Apply quality post-processing to extraction result if enabled.

        Args:
            result: The raw extraction result

        Returns:
            Enhanced extraction result with quality improvements (if enabled)
        """
        # Only apply quality processing if enabled in config
        if not self.config.enable_quality_processing:
            return result

        from kreuzberg._utils._quality import calculate_quality_score, clean_extracted_text

        if not result.content:
            return result

        # Clean the content
        cleaned_content = clean_extracted_text(result.content)

        # Calculate quality score
        quality_score = calculate_quality_score(cleaned_content, dict(result.metadata) if result.metadata else None)

        # Add quality metadata
        enhanced_metadata = dict(result.metadata) if result.metadata else {}
        enhanced_metadata["quality_score"] = quality_score

        # Return enhanced result
        from kreuzberg._types import ExtractionResult, normalize_metadata

        return ExtractionResult(
            content=cleaned_content,
            mime_type=result.mime_type,
            metadata=normalize_metadata(enhanced_metadata),
            chunks=result.chunks,
            detected_languages=result.detected_languages,
            tables=result.tables,
        )
