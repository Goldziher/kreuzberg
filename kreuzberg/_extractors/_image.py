from __future__ import annotations

from typing import TYPE_CHECKING, ClassVar

import anyio
from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import IMAGE_MIME_TYPES
from kreuzberg._ocr import get_ocr_backend
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping
    from pathlib import Path

    from kreuzberg._types import ExtractionResult


class ImageExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = IMAGE_MIME_TYPES

    IMAGE_MIME_TYPE_EXT_MAP: ClassVar[Mapping[str, str]] = {
        "image/bmp": "bmp",
        "image/x-bmp": "bmp",
        "image/x-ms-bmp": "bmp",
        "image/gif": "gif",
        "image/jpeg": "jpg",
        "image/pjpeg": "jpg",
        "image/png": "png",
        "image/tiff": "tiff",
        "image/x-tiff": "tiff",
        "image/jp2": "jp2",
        "image/jpx": "jpx",
        "image/jpm": "jpm",
        "image/mj2": "mj2",
        "image/webp": "webp",
        "image/x-portable-anymap": "pnm",
        "image/x-portable-bitmap": "pbm",
        "image/x-portable-graymap": "pgm",
        "image/x-portable-pixmap": "ppm",
    }

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        extension = self._get_extension_from_mime_type(self.mime_type)
        file_path, unlink = await create_temp_file(f".{extension}")
        await AsyncPath(file_path).write_bytes(content)
        try:
            return await self.extract_path_async(file_path)
        finally:
            await unlink()

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        if self.config.ocr_backend is None:
            raise ValidationError("ocr_backend is None, cannot perform OCR")

        # Use detected_languages if available
        ocr_config = self.config.get_config_dict().copy()
        detected_languages = getattr(self.config, 'detected_languages', None)
        if not detected_languages and hasattr(self.config, 'auto_detect_language') and self.config.auto_detect_language:
            # Try to get from ExtractionResult if present (for future extensibility)
            pass  # ExtractionResult not available here, so only config is checked
        if detected_languages:
            # For Tesseract and PaddleOCR, use 'language'; for EasyOCR, use 'language_list'
            if 'language' in ocr_config:
                ocr_config['language'] = '+'.join(detected_languages)
            if 'language_list' in ocr_config:
                ocr_config['language_list'] = detected_languages
        return await get_ocr_backend(self.config.ocr_backend).process_file(path, **ocr_config)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return anyio.run(self.extract_bytes_async, content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return anyio.run(self.extract_path_async, path)

    def _get_extension_from_mime_type(self, mime_type: str) -> str:
        if mime_type in self.IMAGE_MIME_TYPE_EXT_MAP:
            return self.IMAGE_MIME_TYPE_EXT_MAP[mime_type]

        for k, v in self.IMAGE_MIME_TYPE_EXT_MAP.items():
            if k.startswith(mime_type):
                return v

        raise ValidationError("unsupported mimetype", context={"mime_type": mime_type})
