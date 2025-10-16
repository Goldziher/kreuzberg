from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._internal_bindings import build_email_text_output, extract_email_content
from kreuzberg._mime_types import EML_MIME_TYPE, MSG_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, ImageOCRResult, normalize_metadata
from kreuzberg._utils._sync import run_maybe_async, run_sync
from kreuzberg.exceptions import ParsingError

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
            rust_result = extract_email_content(content, self.mime_type)

            combined_text = build_email_text_output(rust_result)

            metadata = dict(rust_result.metadata)

            result = ExtractionResult(
                content=combined_text,
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

            if self.config.images is not None:
                images = self._extract_images_from_rust_attachments(rust_result)
                result.images = images
                if self.config.images.ocr_min_dimensions is not None and result.images:
                    image_ocr_results: list[ImageOCRResult] = run_maybe_async(
                        self._process_images_with_ocr, result.images
                    )
                    result.image_ocr_results = image_ocr_results

            return result

        except (OSError, RuntimeError, SystemExit, KeyboardInterrupt, MemoryError):
            raise  # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        except Exception as e:
            raise ParsingError(
                "Failed to parse email content", context={"mime_type": self.mime_type, "error": str(e)}
            ) from e

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
