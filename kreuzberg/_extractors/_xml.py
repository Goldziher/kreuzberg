"""XML extraction using streaming Rust parser.

This module provides XML text extraction with streaming support for large files.
Uses quick-xml Rust library for high-performance event-based parsing.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import XmlExtractionResult, parse_xml
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from pathlib import Path


class XMLExtractor(Extractor):
    """Extract text from XML files using streaming parser.

    Uses Rust quick-xml library for high-performance streaming extraction.
    Suitable for multi-GB XML files as it doesn't load entire document into memory.

    Supported MIME types:
    - application/xml
    - text/xml
    - image/svg+xml
    """

    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {
        "application/xml",
        "text/xml",
        "image/svg+xml",
    }

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        """Extract text from XML bytes asynchronously.

        Args:
            content: XML file content as bytes

        Returns:
            ExtractionResult with extracted text and metadata
        """
        return await run_sync(self.extract_bytes_sync, content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Extract text from XML bytes synchronously.

        Uses streaming Rust parser for memory-efficient extraction.

        Args:
            content: XML file content as bytes

        Returns:
            ExtractionResult with extracted text and metadata
        """
        try:
            xml_result: XmlExtractionResult = parse_xml(
                content,
                preserve_whitespace=False,
            )

            metadata: dict[str, Any] = {
                "element_count": xml_result.element_count,
                "unique_elements": len(xml_result.unique_elements),
            }

            result = ExtractionResult(
                content=xml_result.content,
                mime_type=self.mime_type,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except (OSError, RuntimeError, SystemExit, KeyboardInterrupt, MemoryError):
            raise  # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        except Exception as e:
            raise ParsingError(
                "Failed to parse XML content",
                context={"mime_type": self.mime_type, "error": str(e)},
            ) from e

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        """Extract text from XML file path asynchronously.

        Args:
            path: Path to XML file

        Returns:
            ExtractionResult with extracted text and metadata
        """
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Extract text from XML file path synchronously.

        Args:
            path: Path to XML file

        Returns:
            ExtractionResult with extracted text and metadata
        """
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
