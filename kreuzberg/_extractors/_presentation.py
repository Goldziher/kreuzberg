from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, ClassVar

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import PptxExtractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, POWER_POINT_MIME_TYPE
from kreuzberg._types import ExtractionResult, Metadata

if TYPE_CHECKING:
    from pathlib import Path

logger = logging.getLogger(__name__)


class PresentationExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {POWER_POINT_MIME_TYPE}

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        # Initialize Rust PPTX extractor with image extraction setting
        self._rust_extractor = PptxExtractor(self.config.extract_images)

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return self._extract_with_rust(content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return self._extract_with_rust_path(str(path))

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return self._extract_with_rust(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return self._extract_with_rust_path(str(path))

    def _extract_with_rust(self, content: bytes) -> ExtractionResult:
        """Extract PPTX using Rust implementation from bytes."""
        try:
            rust_result = self._rust_extractor.extract_from_bytes(content)

            # Convert Rust result to Python ExtractionResult
            result = ExtractionResult(
                content=rust_result.content,
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._convert_rust_metadata(rust_result.metadata),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except Exception as e:
            logger.error("Failed to extract PPTX with Rust implementation: %s", e)
            raise

    def _extract_with_rust_path(self, path: str) -> ExtractionResult:
        """Extract PPTX using Rust implementation from file path."""
        try:
            rust_result = self._rust_extractor.extract_from_path(path)

            # Convert Rust result to Python ExtractionResult
            result = ExtractionResult(
                content=rust_result.content,
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._convert_rust_metadata(rust_result.metadata),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except Exception as e:
            logger.error("Failed to extract PPTX with Rust implementation: %s", e)
            raise

    def _convert_rust_metadata(self, rust_metadata: Any) -> Metadata:
        """Convert Rust metadata to Python metadata format."""
        metadata: Metadata = {}

        if rust_metadata.title:
            metadata["title"] = rust_metadata.title
        if rust_metadata.author:
            metadata["authors"] = rust_metadata.author
        if rust_metadata.description:
            metadata["description"] = rust_metadata.description
        if rust_metadata.summary:
            metadata["summary"] = rust_metadata.summary
        if rust_metadata.fonts:
            metadata["fonts"] = rust_metadata.fonts

        return metadata
