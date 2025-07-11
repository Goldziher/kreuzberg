from __future__ import annotations

import contextlib
from pathlib import Path
from typing import TYPE_CHECKING, Any, ClassVar, cast

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import IMAGE_MIME_TYPES
from kreuzberg._ocr import get_ocr_backend
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping

    from kreuzberg._types import ExtractionResult, Metadata


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

        from kreuzberg._types import ExtractionResult

        # Extract OCR result and image metadata separately
        ocr_result = await get_ocr_backend(self.config.ocr_backend).process_file(path, **self.config.get_config_dict())
        image_metadata = await self._extract_image_metadata_async(path)

        # Merge OCR metadata with image metadata
        combined_metadata = cast("Metadata", {**image_metadata, **ocr_result.metadata})

        return ExtractionResult(
            content=ocr_result.content,
            mime_type=ocr_result.mime_type,
            metadata=combined_metadata,
            chunks=ocr_result.chunks,
        )

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Pure sync implementation of extract_bytes."""
        import os
        import tempfile

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
        """Pure sync implementation of extract_path."""
        if self.config.ocr_backend is None:
            raise ValidationError("ocr_backend is None, cannot perform OCR")

        from kreuzberg._types import ExtractionResult

        # Extract image metadata first
        image_metadata = self._extract_image_metadata_sync(path)

        if self.config.ocr_backend == "tesseract":
            from kreuzberg._multiprocessing.sync_tesseract import process_batch_images_sync_pure
            from kreuzberg._ocr._tesseract import TesseractConfig

            if isinstance(self.config.ocr_config, TesseractConfig):
                config = self.config.ocr_config
            else:
                config = TesseractConfig()

            results = process_batch_images_sync_pure([str(path)], config)
            if results:
                ocr_result = results[0]
                # Merge OCR metadata with image metadata
                tesseract_metadata = cast("Metadata", {**image_metadata, **ocr_result.metadata})
                return ExtractionResult(
                    content=ocr_result.content,
                    mime_type=ocr_result.mime_type,
                    metadata=tesseract_metadata,
                    chunks=ocr_result.chunks,
                )
            return ExtractionResult(content="", mime_type="text/plain", metadata=image_metadata, chunks=[])

        if self.config.ocr_backend == "paddleocr":
            from kreuzberg._multiprocessing.sync_paddleocr import process_image_sync_pure as paddle_process
            from kreuzberg._ocr._paddleocr import PaddleOCRConfig

            paddle_config = (
                self.config.ocr_config if isinstance(self.config.ocr_config, PaddleOCRConfig) else PaddleOCRConfig()
            )

            ocr_result = paddle_process(path, paddle_config)
            # Merge OCR metadata with image metadata
            paddle_metadata = cast("Metadata", {**image_metadata, **ocr_result.metadata})
            return ExtractionResult(
                content=ocr_result.content,
                mime_type=ocr_result.mime_type,
                metadata=paddle_metadata,
                chunks=ocr_result.chunks,
            )

        if self.config.ocr_backend == "easyocr":
            from kreuzberg._multiprocessing.sync_easyocr import process_image_sync_pure as easy_process
            from kreuzberg._ocr._easyocr import EasyOCRConfig

            easy_config = (
                self.config.ocr_config if isinstance(self.config.ocr_config, EasyOCRConfig) else EasyOCRConfig()
            )

            ocr_result = easy_process(path, easy_config)
            # Merge OCR metadata with image metadata
            easy_metadata = cast("Metadata", {**image_metadata, **ocr_result.metadata})
            return ExtractionResult(
                content=ocr_result.content,
                mime_type=ocr_result.mime_type,
                metadata=easy_metadata,
                chunks=ocr_result.chunks,
            )

        raise NotImplementedError(f"Sync OCR not implemented for {self.config.ocr_backend}")

    def _get_extension_from_mime_type(self, mime_type: str) -> str:
        if mime_type in self.IMAGE_MIME_TYPE_EXT_MAP:
            return self.IMAGE_MIME_TYPE_EXT_MAP[mime_type]

        for k, v in self.IMAGE_MIME_TYPE_EXT_MAP.items():
            if k.startswith(mime_type):
                return v

        raise ValidationError("unsupported mimetype", context={"mime_type": mime_type})

    async def _extract_image_metadata_async(self, path: Path) -> Metadata:
        """Extract metadata from image file asynchronously."""
        from kreuzberg._utils._sync import run_sync

        return await run_sync(self._extract_image_metadata_sync, path)

    def _extract_image_metadata_sync(self, path: Path) -> Metadata:
        """Extract comprehensive metadata from image file."""
        from PIL import Image

        metadata: Metadata = {}

        try:
            with Image.open(path) as img:
                # Basic image information
                if img.width and img.height:
                    metadata["width"] = img.width
                    metadata["height"] = img.height

                # Image format and properties
                if img.format:
                    if "comments" not in metadata:
                        metadata["comments"] = f"Format: {img.format}"
                    else:
                        metadata["comments"] += f"; Format: {img.format}"

                if img.mode:
                    if "comments" not in metadata:
                        metadata["comments"] = f"Color mode: {img.mode}"
                    else:
                        metadata["comments"] += f"; Color mode: {img.mode}"

                # Extract EXIF data if available
                if hasattr(img, "_getexif") and img._getexif():
                    exif_data = img._getexif()
                    self._process_exif_data(exif_data, metadata)

                # Extract other image info (JFIF, etc.)
                if hasattr(img, "info") and img.info:
                    self._process_image_info(img.info, metadata)

        except Exception:
            # If image processing fails, at least return basic file info
            pass

        return metadata

    def _process_exif_data(self, exif_data: dict[int, Any], metadata: Metadata) -> None:
        """Process EXIF data and map to standardized metadata fields."""
        from PIL.ExifTags import TAGS

        for tag_id, value in exif_data.items():
            tag = TAGS.get(tag_id, tag_id)

            # Map common EXIF tags to metadata fields
            if tag == "Artist" and isinstance(value, str):
                if "authors" not in metadata:
                    metadata["authors"] = [value]
                elif value not in metadata["authors"]:
                    metadata["authors"].append(value)
            elif tag == "Copyright" and isinstance(value, str):
                metadata["copyright"] = value
            elif tag == "ImageDescription" and isinstance(value, str):
                metadata["description"] = value
            elif tag == "Software" and isinstance(value, str):
                metadata["created_by"] = value
            elif tag == "DateTime" and isinstance(value, str):
                # Convert EXIF datetime format to ISO
                try:
                    from datetime import datetime

                    dt = datetime.strptime(value, "%Y:%m:%d %H:%M:%S")
                    metadata["created_at"] = dt.isoformat()
                except ValueError:
                    # Store as-is if parsing fails
                    metadata["created_at"] = value
            elif tag == "Make" and isinstance(value, str):
                # Camera/device manufacturer
                if "comments" not in metadata:
                    metadata["comments"] = f"Camera: {value}"
                else:
                    metadata["comments"] += f"; Camera: {value}"
            elif tag == "Model" and isinstance(value, str):
                # Camera/device model
                if "comments" not in metadata:
                    metadata["comments"] = f"Model: {value}"
                else:
                    metadata["comments"] += f"; Model: {value}"

    def _process_image_info(self, info: dict[str, Any], metadata: Metadata) -> None:
        """Process PIL image info (JFIF, etc.) and add to metadata."""
        # Add DPI information if available
        if "dpi" in info:
            dpi = info["dpi"]
            min_dpi_tuple_length = 2
            if isinstance(dpi, (tuple, list)) and len(dpi) >= min_dpi_tuple_length:
                if "comments" not in metadata:
                    metadata["comments"] = f"DPI: {dpi[0]}x{dpi[1]}"
                else:
                    metadata["comments"] += f"; DPI: {dpi[0]}x{dpi[1]}"

        # Add JFIF version if available
        if "jfif_version" in info:
            version = info["jfif_version"]
            min_version_tuple_length = 2
            if isinstance(version, (tuple, list)) and len(version) >= min_version_tuple_length:
                version_str = f"{version[0]}.{version[1]}"
                if "version" not in metadata:
                    metadata["version"] = f"JFIF {version_str}"
