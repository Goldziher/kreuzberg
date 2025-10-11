from __future__ import annotations

import contextlib
import os
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, ClassVar

from anyio import Path as AsyncPath
from PIL import Image

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import IMAGE_MIME_TO_EXT, IMAGE_MIME_TYPES
from kreuzberg._ocr import get_ocr_backend
from kreuzberg._types import ExtractedImage
from kreuzberg._utils._image_preprocessing import normalize_image_dpi
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping

    from kreuzberg._types import ExtractionResult


class ImageExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = IMAGE_MIME_TYPES

    IMAGE_MIME_TYPE_EXT_MAP: ClassVar[Mapping[str, str]] = IMAGE_MIME_TO_EXT

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        extension = self._get_extension_from_mime_type(self.mime_type)
        file_path, unlink = await create_temp_file(f".{extension}")
        await AsyncPath(file_path).write_bytes(content)
        try:
            return await self.extract_path_async(file_path)
        finally:
            await unlink()

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        if self.config.ocr is None:
            raise ValidationError("OCR is not configured, cannot perform OCR on images")

        image = await run_sync(Image.open, str(path))
        try:
            normalized_image, preprocessing_metadata = normalize_image_dpi(image, self.config)

            backend_name = self.config.ocr.backend if hasattr(self.config.ocr, "backend") else "tesseract"
            backend = get_ocr_backend(backend_name)

            ocr_config = self._prepare_ocr_config()
            backend_kwargs = self._build_backend_kwargs(backend, ocr_config)
            result = await backend.process_image(normalized_image, **backend_kwargs)

            if preprocessing_metadata:
                result.metadata["image_preprocessing"] = preprocessing_metadata

            if self.config.images is not None:
                content = await AsyncPath(path).read_bytes()
                result.images = [self._create_self_reference_image(content, self.mime_type)]

            return self._apply_quality_processing(result)
        finally:
            image.close()
            if normalized_image is not image:
                normalized_image.close()

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        extension = self._get_extension_from_mime_type(self.mime_type)
        fd, temp_path = tempfile.mkstemp(suffix=f".{extension}")

        try:
            with os.fdopen(fd, "wb") as f:
                f.write(content)

            return self.extract_path_sync(Path(temp_path))
        finally:
            with contextlib.suppress(OSError):
                Path(temp_path).unlink()

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        if self.config.ocr is None:
            raise ValidationError("OCR is not configured, cannot perform OCR on images")

        image = Image.open(str(path))
        try:
            normalized_image, preprocessing_metadata = normalize_image_dpi(image, self.config)

            backend_name = self.config.ocr.backend if hasattr(self.config.ocr, "backend") else "tesseract"
            backend = get_ocr_backend(backend_name)

            ocr_config = self._prepare_ocr_config()
            backend_kwargs = self._build_backend_kwargs(backend, ocr_config)
            result = backend.process_image_sync(normalized_image, **backend_kwargs)

            if preprocessing_metadata:
                result.metadata["image_preprocessing"] = preprocessing_metadata

            if self.config.images is not None:
                content = path.read_bytes()
                result.images = [self._create_self_reference_image(content, self.mime_type)]

            return self._apply_quality_processing(result)
        finally:
            image.close()
            if normalized_image is not image:
                normalized_image.close()

    def _get_extension_from_mime_type(self, mime_type: str) -> str:
        if mime_type in self.IMAGE_MIME_TYPE_EXT_MAP:
            return self.IMAGE_MIME_TYPE_EXT_MAP[mime_type]

        for k, v in self.IMAGE_MIME_TYPE_EXT_MAP.items():
            if k.startswith(mime_type):
                return v

        raise ValidationError("unsupported mimetype", context={"mime_type": mime_type})

    def _create_self_reference_image(self, image_data: bytes, mime_type: str) -> ExtractedImage:
        return ExtractedImage(
            data=image_data,
            format=IMAGE_MIME_TO_EXT.get(mime_type, "unknown"),
            filename="source_image",
            page_number=1,
        )
