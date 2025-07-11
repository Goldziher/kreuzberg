"""Extractous backend implementation."""

from __future__ import annotations

import time
from typing import TYPE_CHECKING, Any

from kreuzberg._backends.base import ExtractionBackend
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._types import ExtractionResult

try:
    from extractous import Extractor

    HAS_EXTRACTOUS = True
except ImportError:
    Extractor = None
    HAS_EXTRACTOUS = False


class ExtractousBackend(ExtractionBackend):
    """Extractous backend for text extraction."""

    def __init__(self, config: dict[str, Any] | None = None) -> None:
        """Initialize the Extractous backend.

        Args:
            config: Optional configuration dictionary

        Raises:
            ImportError: If Extractous is not installed
        """
        if not HAS_EXTRACTOUS:
            raise ImportError("Extractous is not installed. Install with: pip install extractous")

        self.config = config or {}
        self.extractor = Extractor()

        # Configure extractor with reasonable defaults
        max_text_length = self.config.get("max_text_length", 10_000_000)  # 10MB default
        self.extractor.set_extract_string_max_length(max_text_length)

        # Configure OCR if available
        self._configure_ocr()

    def _configure_ocr(self) -> None:
        """Configure OCR settings for Extractous."""
        try:
            from extractous import TesseractOcrConfig

            # Get language configuration
            language = self.config.get("ocr_language", "eng")

            # Configure OCR for image-based documents
            ocr_config = TesseractOcrConfig().set_language(language)
            self.extractor.set_ocr_config(ocr_config)

        except ImportError:
            # OCR config not available, continue without OCR
            pass

    def extract(self, file_path: str | Path, **_kwargs: Any) -> ExtractionResult:
        """Extract text from a file using Extractous.

        Args:
            file_path: Path to the file to extract from
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """
        from kreuzberg._types import ExtractionResult

        start_time = time.perf_counter()

        try:
            # Extract text and metadata from file
            result = self.extractor.extract_file_to_string(str(file_path))

            if isinstance(result, tuple):
                text, metadata = result
            else:
                text = result
                metadata = {}

            end_time = time.perf_counter()
            extraction_time = end_time - start_time

            # Add backend information to metadata
            if metadata is None:
                metadata = {}

            # Normalize and enhance metadata
            from kreuzberg._types import normalize_metadata

            enhanced_metadata = {
                **metadata,
                "backend": "extractous",
                "extraction_time": extraction_time,
                "word_count": len(text.split()) if text else 0,
                "char_count": len(text) if text else 0,
            }

            return ExtractionResult(
                content=text,
                metadata=normalize_metadata(enhanced_metadata),
                mime_type="text/plain",  # Default to plain text
            )

        except Exception as e:
            end_time = time.perf_counter()
            extraction_time = end_time - start_time

            raise ParsingError(
                f"Extractous extraction failed: {e}",
                context={
                    "file_path": str(file_path),
                    "backend": "extractous",
                    "extraction_time": extraction_time,
                },
            ) from e

    def extract_bytes(self, data: bytes, mime_type: str, **_kwargs: Any) -> ExtractionResult:
        """Extract text from raw bytes using Extractous.

        Args:
            data: Raw bytes of the document
            mime_type: MIME type of the document
            **kwargs: Additional configuration options

        Returns:
            ExtractionResult object containing extracted text and metadata
        """
        from kreuzberg._types import ExtractionResult

        start_time = time.perf_counter()

        try:
            # Extract text and metadata from bytes
            result = self.extractor.extract_bytes_to_string(data)

            if isinstance(result, tuple):
                text, metadata = result
            else:
                text = result
                metadata = {}

            end_time = time.perf_counter()
            extraction_time = end_time - start_time

            # Add backend information to metadata
            if metadata is None:
                metadata = {}

            # Normalize and enhance metadata
            from kreuzberg._types import normalize_metadata

            enhanced_metadata = {
                **metadata,
                "backend": "extractous",
                "extraction_time": extraction_time,
                "provided_mime_type": mime_type,
                "word_count": len(text.split()) if text else 0,
                "char_count": len(text) if text else 0,
            }

            return ExtractionResult(
                content=text,
                metadata=normalize_metadata(enhanced_metadata),
                mime_type=mime_type,
            )

        except Exception as e:
            end_time = time.perf_counter()
            extraction_time = end_time - start_time

            raise ParsingError(
                f"Extractous extraction failed: {e}",
                context={
                    "mime_type": mime_type,
                    "backend": "extractous",
                    "extraction_time": extraction_time,
                },
            ) from e

    def supports_format(self, file_type: str) -> bool:
        """Check if Extractous supports a given file format.

        Args:
            file_type: File extension or MIME type to check

        Returns:
            True if the format is supported, False otherwise
        """
        # Extractous supported formats based on the integration document
        supported_extensions = {
            ".json",
            ".yaml",
            ".yml",
            ".eml",
            ".msg",  # Exclusive formats
            ".docx",
            ".odt",
            ".org",
            ".rst",  # Superior reliability
            ".pdf",
            ".html",
            ".epub",
            ".csv",  # Competitive formats
        }

        supported_mime_types = {
            "application/json",
            "text/json",
            "application/x-yaml",
            "application/yaml",
            "text/yaml",
            "text/x-yaml",
            "message/rfc822",  # EML
            "application/vnd.ms-outlook",  # MSG
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",  # DOCX
            "application/vnd.oasis.opendocument.text",  # ODT
            "text/x-org",  # ORG
            "text/x-rst",  # RST
            "application/pdf",
            "text/html",
            "application/epub+zip",
            "text/csv",
        }

        if file_type.startswith("."):
            return file_type.lower() in supported_extensions
        return file_type.lower() in supported_mime_types

    def get_capabilities(self) -> dict[str, Any]:
        """Get information about Extractous's capabilities.

        Returns:
            Dictionary containing capability information
        """
        return {
            "name": "extractous",
            "version": "0.1.0+",
            "language": "rust",
            "supports_ocr": True,
            "supports_async": False,  # Extractous is sync-only
            "supports_batch": False,  # No built-in batch processing
            "memory_efficient": True,  # Rust memory management
            "speed_optimized": True,  # Rust performance
            "exclusive_formats": [
                "application/json",
                "application/x-yaml",
                "message/rfc822",  # EML
                "application/vnd.ms-outlook",  # MSG
            ],
            "preferred_formats": [
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",  # DOCX
                "application/vnd.oasis.opendocument.text",  # ODT
                "text/x-org",  # ORG
                "text/x-rst",  # RST
            ],
            "supported_formats": [
                "application/json",
                "text/json",
                "application/x-yaml",
                "application/yaml",
                "text/yaml",
                "message/rfc822",
                "application/vnd.ms-outlook",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "application/vnd.oasis.opendocument.text",
                "text/x-org",
                "text/x-rst",
                "application/pdf",
                "text/html",
                "application/epub+zip",
                "text/csv",
            ],
        }

    @property
    def name(self) -> str:
        """Get the name of this backend."""
        return "extractous"
