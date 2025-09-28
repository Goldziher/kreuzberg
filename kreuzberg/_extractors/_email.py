from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import build_email_text_output, extract_email_content
from kreuzberg._mime_types import EML_MIME_TYPE, MSG_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, ImageOCRResult, normalize_metadata
from kreuzberg._utils._sync import run_maybe_async, run_sync

if TYPE_CHECKING:
    from pathlib import Path


class EmailExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {EML_MIME_TYPE, MSG_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def _extract_images_from_rust_attachments(self, rust_result: Any) -> list[ExtractedImage]:
        """Extract images from Rust EmailExtractionResultDTO attachments."""
        images: list[ExtractedImage] = []

        for idx, attachment in enumerate(rust_result.attachments, start=1):
            if not attachment.is_image or not attachment.data:
                continue

            name = attachment.filename or attachment.name
            mime_type = attachment.mime_type or "application/octet-stream"

            if not mime_type.startswith("image/"):
                continue

            fmt = mime_type.split("/", 1)[1].lower() if "/" in mime_type else "unknown"

            if name and "." in name:
                ext = name.rsplit(".", 1)[-1].lower()
                if ext:
                    fmt = ext

            filename = name or f"attachment_image_{idx}.{fmt}"
            images.append(
                ExtractedImage(
                    data=bytes(attachment.data),
                    format=fmt,
                    filename=filename,
                    page_number=None,
                )
            )

        return images

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        try:
            # Use Rust implementation for email parsing
            rust_result = extract_email_content(content, self.mime_type)

            # Build combined text using Rust function
            combined_text = build_email_text_output(rust_result)

            # Convert Rust metadata to Python dict
            metadata = dict(rust_result.metadata)

            result = ExtractionResult(
                content=combined_text,
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

            if self.config.extract_images:
                images = self._extract_images_from_rust_attachments(rust_result)
                result.images = images
                if self.config.ocr_extracted_images and result.images:
                    image_ocr_results: list[ImageOCRResult] = run_maybe_async(
                        self._process_images_with_ocr, result.images
                    )
                    result.image_ocr_results = image_ocr_results

            return result

        except Exception as e:
            msg = f"Failed to parse email content: {e}"
            raise RuntimeError(msg) from e

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
