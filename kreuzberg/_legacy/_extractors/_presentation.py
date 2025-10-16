from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, ClassVar

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import extract_pptx_from_bytes_msgpack, extract_pptx_from_path_msgpack
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, POWER_POINT_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, Metadata
from kreuzberg._utils._serialization import deserialize
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from pathlib import Path

logger = logging.getLogger(__name__)


class PresentationExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {POWER_POINT_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return self._extract_from_bytes(content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return self._extract_from_path(str(path))

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return self._extract_from_bytes(content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return self._extract_from_path(str(path))

    def _extract_from_bytes(self, content: bytes) -> ExtractionResult:
        try:
            extract_images = self.config.images is not None
            msgpack_result = extract_pptx_from_bytes_msgpack(content, extract_images)
            extraction_result = deserialize(msgpack_result, dict)

            images = []
            if extract_images:
                images = [
                    ExtractedImage(data=bytes(img["data"]), format=img["format"], page_number=img.get("slide_number"))
                    for img in extraction_result.get("images", [])
                ]

            metadata = self._convert_metadata_dict(extraction_result["metadata"])

            result = ExtractionResult(
                content=extraction_result["content"],
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=metadata,
                chunks=[],
                images=images,
            )

            return self._apply_quality_processing(result)

        except (OSError, RuntimeError, SystemExit, KeyboardInterrupt, MemoryError):
            raise  # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        except Exception as e:
            logger.error("Failed to extract PPTX: %s", e)
            raise ParsingError(f"PPTX extraction failed: {e}") from e

    def _extract_from_path(self, path: str) -> ExtractionResult:
        try:
            extract_images = self.config.images is not None
            msgpack_result = extract_pptx_from_path_msgpack(path, extract_images)
            extraction_result = deserialize(msgpack_result, dict)

            images = []
            if extract_images:
                images = [
                    ExtractedImage(data=bytes(img["data"]), format=img["format"], page_number=img.get("slide_number"))
                    for img in extraction_result.get("images", [])
                ]

            metadata = self._convert_metadata_dict(extraction_result["metadata"])

            result = ExtractionResult(
                content=extraction_result["content"],
                mime_type=MARKDOWN_MIME_TYPE,
                metadata=metadata,
                chunks=[],
                images=images,
            )

            return self._apply_quality_processing(result)

        except (OSError, RuntimeError, SystemExit, KeyboardInterrupt, MemoryError):
            raise  # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        except Exception as e:
            logger.error("Failed to extract PPTX: %s", e)
            raise ParsingError(f"PPTX extraction failed: {e}") from e

    def _convert_metadata_dict(self, metadata_dict: dict[str, Any]) -> Metadata:
        metadata: Metadata = {}

        if title := metadata_dict.get("title"):
            metadata["title"] = title
        if author := metadata_dict.get("author"):
            metadata["authors"] = author
        if description := metadata_dict.get("description"):
            metadata["description"] = description
        if summary := metadata_dict.get("summary"):
            metadata["summary"] = summary
        if fonts := metadata_dict.get("fonts"):
            metadata["fonts"] = fonts

        return metadata
