from __future__ import annotations

import logging
from typing import TYPE_CHECKING, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import MAX_SINGLE_IMAGE_SIZE, Extractor
from kreuzberg._internal_bindings import process_html, safe_decode  # type: ignore[attr-defined]
from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE
from kreuzberg._types import (
    ExtractedImage,
    ExtractionResult,
    HTMLToMarkdownConfig,
    html_to_markdown_config_to_options,
)
from kreuzberg._utils._sync import run_maybe_async, run_sync

if TYPE_CHECKING:
    from pathlib import Path

logger = logging.getLogger(__name__)


class HTMLExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {HTML_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        result = await run_sync(self.extract_bytes_sync, content)
        if self.config.images is not None and self.config.images.ocr_min_dimensions is not None and result.images:
            result.image_ocr_results = await self._process_images_with_ocr(result.images)
        return result

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        result = await run_sync(self.extract_bytes_sync, content)
        if self.config.images is not None and self.config.images.ocr_min_dimensions is not None and result.images:
            result.image_ocr_results = await self._process_images_with_ocr(result.images)
        return result

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        config = self.config.html_to_markdown if self.config else None
        if config is None:
            config = HTMLToMarkdownConfig()
        options = html_to_markdown_config_to_options(config)
        html_content = safe_decode(content)
        extract_images = self.config.images is not None
        markdown_result, images_payload, warnings = process_html(
            html_content, options, extract_images, MAX_SINGLE_IMAGE_SIZE
        )

        for warning in warnings:
            logger.warning(warning)

        extraction_result = ExtractionResult(content=markdown_result, mime_type=MARKDOWN_MIME_TYPE, metadata={})

        if extract_images and images_payload:
            extraction_result.images = [
                ExtractedImage(
                    data=image_info["data"],
                    format=image_info["format"],
                    filename=image_info.get("filename"),
                    description=image_info.get("description"),
                    dimensions=image_info.get("dimensions"),
                )
                for image_info in images_payload
            ]
            if self.config.images.ocr_min_dimensions is not None:  # type: ignore[union-attr]
                extraction_result.image_ocr_results = run_maybe_async(
                    self._process_images_with_ocr, extraction_result.images
                )

        return self._apply_quality_processing(extraction_result)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
