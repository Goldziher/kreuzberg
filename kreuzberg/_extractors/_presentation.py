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
        self._extractor = PptxExtractor(self.config.extract_images)

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return self._extract_from_bytes(content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return self._extract_from_path(str(path))

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return self._extract_from_bytes(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return self._extract_from_path(str(path))

    def _extract_from_bytes(self, content: bytes) -> ExtractionResult:
        """Extract PPTX from bytes."""
        try:
            extraction_result = self._extractor.extract_from_bytes(content)

            result = ExtractionResult(
                content=extraction_result.content,
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._convert_metadata(extraction_result.metadata),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except Exception as e:
            logger.error("Failed to extract PPTX: %s", e)
            raise

    def _extract_from_path(self, path: str) -> ExtractionResult:
        """Extract PPTX from file path."""
        try:
            extraction_result = self._extractor.extract_from_path(path)

            result = ExtractionResult(
                content=extraction_result.content,
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=self._convert_metadata(extraction_result.metadata),
                chunks=[],
            )

            return self._apply_quality_processing(result)

        except Exception as e:
            logger.error("Failed to extract PPTX: %s", e)
            raise

    def _convert_metadata(self, metadata_obj: Any) -> Metadata:
        """Convert metadata to Python format."""
        metadata: Metadata = {}

        if metadata_obj.title:
            metadata["title"] = metadata_obj.title
        if metadata_obj.author:
            metadata["authors"] = metadata_obj.author
        if metadata_obj.description:
            metadata["description"] = metadata_obj.description
        if metadata_obj.summary:
            metadata["summary"] = metadata_obj.summary
        if metadata_obj.fonts:
            metadata["fonts"] = metadata_obj.fonts

        return metadata
