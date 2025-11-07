from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath
from html_to_markdown import HtmlToMarkdownError
from html_to_markdown._html_to_markdown import (
    InlineImageConfig,
    convert_with_inline_images,
)
from html_to_markdown._html_to_markdown import (
    convert as rust_convert,
)

from kreuzberg._extractors._base import MAX_SINGLE_IMAGE_SIZE, Extractor
from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, HTMLToMarkdownConfig
from kreuzberg._utils._string import safe_decode
from kreuzberg._utils._sync import run_maybe_async, run_sync

if TYPE_CHECKING:
    from pathlib import Path

logger = logging.getLogger(__name__)


class HTMLExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {HTML_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        result = await run_sync(self.extract_bytes_sync, content)
        if self.config.extract_images and self.config.ocr_extracted_images and result.images:
            result.image_ocr_results = await self._process_images_with_ocr(result.images)
        return result

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        result = await run_sync(self.extract_bytes_sync, content)
        if self.config.extract_images and self.config.ocr_extracted_images and result.images:
            result.image_ocr_results = await self._process_images_with_ocr(result.images)
        return result

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        extraction_config = self.config
        html_content = safe_decode(content)
        if extraction_config and extraction_config.html_to_markdown_config is not None:
            html_config = extraction_config.html_to_markdown_config
        else:
            html_config = HTMLToMarkdownConfig()
        conversion_options, _ = html_config.to_options()

        extract_inline_images = bool(extraction_config and extraction_config.extract_images)
        run_ocr_on_images = bool(
            extraction_config and extraction_config.extract_images and extraction_config.ocr_extracted_images
        )
        inline_image_config = None
        if extract_inline_images:
            inline_image_config = InlineImageConfig(
                max_decoded_size_bytes=MAX_SINGLE_IMAGE_SIZE,
                filename_prefix=None,
                capture_svg=True,
                infer_dimensions=True,
            )

        try:
            if extract_inline_images:
                markdown, images_payload, warnings = convert_with_inline_images(
                    html_content,
                    options=conversion_options,
                    image_config=inline_image_config,
                )
            else:
                markdown = rust_convert(
                    html_content,
                    conversion_options,
                )
                images_payload = []
                warnings = []
        except (HtmlToMarkdownError, ValueError) as exc:
            logger.exception("Failed to convert HTML to Markdown: %s", exc)
            markdown = ""
            images_payload = []
            warnings = []

        for warning in warnings:
            self._log_inline_warning(warning)

        extraction_result = ExtractionResult(content=markdown, mime_type=MARKDOWN_MIME_TYPE, metadata={})

        inline_images = [self._build_extracted_image(image) for image in images_payload]
        if inline_images:
            extraction_result.images = inline_images
            if run_ocr_on_images:
                extraction_result.image_ocr_results = run_maybe_async(
                    self._process_images_with_ocr,
                    inline_images,
                )

        return self._apply_quality_processing(extraction_result)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)

    @staticmethod
    def _build_extracted_image(image: dict[str, Any]) -> ExtractedImage:
        dimensions_value = image.get("dimensions")
        dimensions = tuple(dimensions_value) if dimensions_value else None
        return ExtractedImage(
            data=image["data"],
            format=image["format"],
            filename=image.get("filename"),
            description=image.get("description"),
            dimensions=dimensions,
        )

    @staticmethod
    def _log_inline_warning(warning: Any) -> None:
        if isinstance(warning, dict):
            index = warning.get("index")
            message = warning.get("message")
            if index is not None and message:
                logger.warning("Inline image %s: %s", index, message)
            elif message:
                logger.warning("Inline image warning: %s", message)
            else:
                logger.warning("Inline image warning received with no message")
            return

        message = getattr(warning, "message", None)
        index = getattr(warning, "index", None)
        if message and index is not None:
            logger.warning("Inline image %s: %s", index, message)
        elif message:
            logger.warning("Inline image warning: %s", message)
        else:
            logger.warning("Inline image warning received with no message")
