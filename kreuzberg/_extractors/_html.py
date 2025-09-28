from __future__ import annotations

import base64
import binascii
import io
import logging
from typing import TYPE_CHECKING, ClassVar

import html_to_markdown
from anyio import Path as AsyncPath
from bs4 import BeautifulSoup
from PIL import Image

from kreuzberg._extractors._base import MAX_SINGLE_IMAGE_SIZE, Extractor
from kreuzberg._internal_bindings import safe_decode
from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, HTMLToMarkdownConfig
from kreuzberg._utils._html_streaming import should_use_streaming
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
        config = self.config.html_to_markdown_config if self.config else None
        if config is None:
            config = HTMLToMarkdownConfig()

        config_dict = config.to_dict()

        html_content = safe_decode(content)

        use_streaming, chunk_size = should_use_streaming(len(content))
        config_dict["stream_processing"] = use_streaming
        config_dict["chunk_size"] = chunk_size

        result = html_to_markdown.convert_to_markdown(html_content, **config_dict)

        extraction_result = ExtractionResult(content=result, mime_type=MARKDOWN_MIME_TYPE, metadata={})

        if self.config.extract_images:
            extraction_result.images = self._extract_images_from_html(html_content)
            if self.config.ocr_extracted_images and extraction_result.images:
                extraction_result.image_ocr_results = run_maybe_async(
                    self._process_images_with_ocr, extraction_result.images
                )

        return self._apply_quality_processing(extraction_result)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)

    def _extract_images_from_html(self, html_content: str) -> list[ExtractedImage]:
        images: list[ExtractedImage] = []
        soup = BeautifulSoup(html_content, "xml")

        for img in soup.find_all("img"):
            src_val = img.get("src")
            if isinstance(src_val, str) and src_val.startswith("data:image/"):
                try:
                    header, data = src_val.split(",", 1)
                    mime_type = header.split(";")[0].split(":")[1]
                    format_name = mime_type.split("/")[1]

                    if not data or len(data) < 4:
                        logger.debug("Skipping empty or too small base64 data")
                        continue

                    if len(data) > 67 * 1024 * 1024:
                        logger.warning("Skipping base64 image larger than 67MB")
                        continue

                    image_data = base64.b64decode(data)

                    if len(image_data) > MAX_SINGLE_IMAGE_SIZE:
                        logger.warning(
                            "Skipping decoded image larger than %dMB", MAX_SINGLE_IMAGE_SIZE // (1024 * 1024)
                        )
                        continue

                    dimensions = None
                    try:
                        with Image.open(io.BytesIO(image_data)) as pil_img:
                            dimensions = pil_img.size
                    except (OSError, ValueError) as e:  # pragma: no cover
                        logger.debug("Could not determine image dimensions for %s: %s", format_name, e)

                    alt_val = img.get("alt")
                    desc = alt_val if isinstance(alt_val, str) else None
                    images.append(
                        ExtractedImage(
                            data=image_data,
                            format=format_name,
                            filename=f"embedded_image_{len(images) + 1}.{format_name}",
                            description=desc,
                            dimensions=dimensions,
                        )
                    )
                except (ValueError, binascii.Error) as e:
                    logger.warning("Failed to extract base64 image: %s", e)

        def extract_svg_safe(svg_element: object) -> ExtractedImage | None:
            try:
                svg_content = str(svg_element).encode("utf-8")

                def _get_attr_safe(obj: object, attr: str) -> str | None:
                    get_method = getattr(obj, "get", None)
                    if callable(get_method):
                        result = get_method(attr)
                        return result if isinstance(result, str) else None
                    return None

                title_or_aria = _get_attr_safe(svg_element, "title") or _get_attr_safe(svg_element, "aria-label")
                desc_svg = title_or_aria if isinstance(title_or_aria, str) else None
                return ExtractedImage(
                    data=svg_content,
                    format="svg",
                    filename=f"inline_svg_{len(images) + 1}.svg",
                    description=desc_svg,
                )
            except (UnicodeEncodeError, AttributeError) as e:
                logger.warning("Failed to extract SVG: %s", e)
                return None

        svg_images = [extract_svg_safe(svg) for svg in soup.find_all("svg")]
        images.extend(img for img in svg_images if img is not None)

        return images
