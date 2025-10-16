from __future__ import annotations

import io
import logging
import time
import zlib
from abc import ABC, abstractmethod
from multiprocessing import cpu_count
from typing import TYPE_CHECKING, Any, ClassVar

from PIL import Image

from kreuzberg._internal_bindings import calculate_quality_score, clean_extracted_text
from kreuzberg._ocr import get_ocr_backend
from kreuzberg._types import (
    ExtractedImage,
    ExtractionResult,
    ImageOCRResult,
    normalize_metadata,
)
from kreuzberg._utils._serialization import to_dict
from kreuzberg._utils._sync import run_taskgroup_batched

if TYPE_CHECKING:
    from pathlib import Path

    from kreuzberg._types import ExtractionConfig

MAX_TOTAL_IMAGE_SIZE_MB = 100
MAX_SINGLE_IMAGE_SIZE_MB = 50
MAX_TOTAL_IMAGE_SIZE = MAX_TOTAL_IMAGE_SIZE_MB * 1024 * 1024
MAX_SINGLE_IMAGE_SIZE = MAX_SINGLE_IMAGE_SIZE_MB * 1024 * 1024

logger = logging.getLogger(__name__)


class Extractor(ABC):
    __slots__ = ("config", "mime_type")

    SUPPORTED_MIME_TYPES: ClassVar[set[str]]

    def __init__(self, mime_type: str, config: ExtractionConfig) -> None:
        self.mime_type = mime_type
        self.config = config

    @abstractmethod
    async def extract_bytes_async(self, content: bytes) -> ExtractionResult: ...

    @abstractmethod
    async def extract_path_async(self, path: Path) -> ExtractionResult: ...

    @abstractmethod
    def extract_bytes_sync(self, content: bytes) -> ExtractionResult: ...

    @abstractmethod
    def extract_path_sync(self, path: Path) -> ExtractionResult: ...

    @classmethod
    def supports_mimetype(cls, mime_type: str) -> bool:
        return mime_type in cls.SUPPORTED_MIME_TYPES or any(
            mime_type.startswith(supported_type) for supported_type in cls.SUPPORTED_MIME_TYPES
        )

    def _apply_quality_processing(self, result: ExtractionResult) -> ExtractionResult:
        if not self.config.enable_quality_processing:
            return result

        if not result.content:
            return result

        cleaned_content = clean_extracted_text(result.content)

        metadata_dict = dict(result.metadata) if result.metadata else None
        quality_score = calculate_quality_score(cleaned_content, metadata_dict)

        enhanced_metadata = (dict(result.metadata) if result.metadata else {}) | {"quality_score": quality_score}

        deduplicated_images = self._deduplicate_images(result.images) if result.images else []

        return ExtractionResult(
            content=cleaned_content,
            mime_type=result.mime_type,
            metadata=normalize_metadata(enhanced_metadata),
            tables=result.tables,
            chunks=result.chunks,
            images=deduplicated_images,
            image_ocr_results=result.image_ocr_results,
            entities=result.entities,
            keywords=result.keywords,
            detected_languages=result.detected_languages,
            document_type=result.document_type,
            document_type_confidence=result.document_type_confidence,
            layout=result.layout,
        )

    def _check_image_memory_limits(self, images: list[ExtractedImage]) -> list[ExtractedImage]:
        if not images:
            return []

        images_with_sizes = [(img, len(img.data)) for img in images]

        valid_images = []
        for img, size in images_with_sizes:
            if size <= MAX_SINGLE_IMAGE_SIZE:
                valid_images.append((img, size))
            else:
                logger.warning(
                    "Skipping image %s: size %d MB exceeds limit of %d MB",
                    img.filename or "unknown",
                    size // (1024 * 1024),
                    MAX_SINGLE_IMAGE_SIZE_MB,
                )

        total_size = sum(size for _, size in valid_images)

        if total_size <= MAX_TOTAL_IMAGE_SIZE:
            return [img for img, _ in valid_images]

        logger.warning(
            "Total image size %d MB exceeds limit of %d MB, selecting subset",
            total_size // (1024 * 1024),
            MAX_TOTAL_IMAGE_SIZE_MB,
        )

        sorted_images = sorted(valid_images, key=lambda x: x[1])
        selected = []
        current_size = 0

        for img, img_size in sorted_images:
            if current_size + img_size <= MAX_TOTAL_IMAGE_SIZE:
                selected.append(img)
                current_size += img_size
            else:
                logger.debug("Skipping image %s: would exceed total memory limit", img.filename or "unknown")

        return selected

    _SMALL_IMAGE_THRESHOLD = 1024
    _HASH_SAMPLE_SIZE = 512

    def _compute_image_hash(self, img: ExtractedImage) -> int:
        data_len = len(img.data)

        if data_len < self._SMALL_IMAGE_THRESHOLD:
            return zlib.crc32(img.data) & 0xFFFFFFFF

        hash_components = [
            str(data_len).encode(),
            img.data[: self._HASH_SAMPLE_SIZE],
            img.data[-self._HASH_SAMPLE_SIZE :],
            img.format.encode() if img.format else b"",
        ]

        combined = b"".join(hash_components)
        return zlib.crc32(combined) & 0xFFFFFFFF

    def _deduplicate_images(self, images: list[ExtractedImage]) -> list[ExtractedImage]:
        if not images:
            return images

        if self.config.images is None or not self.config.images.deduplicate:
            return images

        seen_hashes = set()
        unique_images = []

        for img in images:
            img_hash = self._compute_image_hash(img)
            if img_hash not in seen_hashes:
                seen_hashes.add(img_hash)
                unique_images.append(img)
            else:
                logger.debug("Filtered duplicate image: %s", img.filename)

        if len(unique_images) < len(images):
            logger.info("Deduplicated %d images to %d unique", len(images), len(unique_images))

        return unique_images

    def _prepare_ocr_config(self) -> dict[str, Any]:
        cache_enabled = self.config.use_cache

        if self.config.ocr is None:
            return {"use_cache": cache_enabled}

        cfg = to_dict(self.config.ocr)
        # Remove backend field - it's used for tagged union discrimination, not constructor arg ~keep
        cfg.pop("backend", None)
        cfg.setdefault("use_cache", cache_enabled)
        return cfg

    def _build_backend_kwargs(self, backend: Any, cfg: dict[str, Any]) -> dict[str, Any]:
        kwargs = dict(cfg)
        use_cache_flag = kwargs.pop("use_cache", True)

        backend_module = backend.__class__.__module__.lower()
        backend_name = backend.__class__.__name__.lower()

        if "tesseract" in backend_module or "tesseract" in backend_name:
            return kwargs

        kwargs.setdefault("use_cache", use_cache_flag)
        return kwargs

    def _validate_image_for_ocr(self, img: ExtractedImage) -> str | None:
        if self.config.images is None:
            return None

        fmt = img.format.lower()
        if fmt not in self.config.images.ocr_allowed_formats:
            return f"Unsupported format: {img.format}"

        if img.dimensions is not None:
            w, h = img.dimensions

            if self.config.images.ocr_min_dimensions is not None:
                min_w, min_h = self.config.images.ocr_min_dimensions
                if w < min_w or h < min_h:
                    return f"Too small: {w}x{h}"

            max_w, max_h = self.config.images.ocr_max_dimensions
            if w > max_w or h > max_h:
                return f"Too large: {w}x{h}"

        return None

    async def _ocr_single_image(self, target: ExtractedImage, backend: Any, cfg: dict[str, Any]) -> ImageOCRResult:
        try:
            start = time.time()
            pil_img = Image.open(io.BytesIO(target.data))
            backend_kwargs = self._build_backend_kwargs(backend, cfg)
            ocr_res = await backend.process_image(pil_img, **backend_kwargs)
            duration = time.time() - start
            return ImageOCRResult(
                image=target,
                ocr_result=ocr_res,
                confidence_score=None,
                processing_time=duration,
            )
        except ValueError as e:  # pragma: no cover
            return ImageOCRResult(
                image=target,
                ocr_result=ExtractionResult(content="", mime_type="text/plain", metadata={}),
                skipped_reason=f"OCR failed: {type(e).__name__}: {e}",
            )
        except TypeError as e:  # pragma: no cover
            return ImageOCRResult(
                image=target,
                ocr_result=ExtractionResult(content="", mime_type="text/plain", metadata={}),
                skipped_reason=f"Backend error: {type(e).__name__}: {e}",
            )

    async def _process_images_with_ocr(
        self, images: tuple[ExtractedImage, ...] | list[ExtractedImage]
    ) -> list[ImageOCRResult]:
        if not images or self.config.images is None or self.config.images.ocr_min_dimensions is None:
            return []

        images_list = list(self._deduplicate_images(list(images)))
        images_list = self._check_image_memory_limits(images_list)

        if self.config.ocr is None:
            return []

        cfg = self._prepare_ocr_config()
        backend_name = self.config.ocr.backend if hasattr(self.config.ocr, "backend") else "tesseract"
        backend = get_ocr_backend(backend_name)

        results: list[ImageOCRResult] = []
        tasks = []

        for img in images_list:
            skip_reason = self._validate_image_for_ocr(img)
            if skip_reason:
                results.append(
                    ImageOCRResult(
                        image=img,
                        ocr_result=ExtractionResult(content="", mime_type="text/plain", metadata={}),
                        skipped_reason=skip_reason,
                    )
                )
            else:
                tasks.append(self._ocr_single_image(img, backend, cfg))

        if tasks:
            batch_size = max(1, min(len(tasks), cpu_count()))
            results.extend(await run_taskgroup_batched(*tasks, batch_size=batch_size))

        return results
