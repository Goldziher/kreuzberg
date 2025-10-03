"""OCR cache helper functions (compatibility layer)."""

from __future__ import annotations

import hashlib
import io
from pathlib import Path
from typing import TYPE_CHECKING, Any

import anyio

from kreuzberg._internal_bindings import generate_cache_key
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._cache import get_ocr_cache
from kreuzberg._utils._serialization import deserialize, serialize

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage


def get_file_info(path: Path) -> dict[str, Any]:
    path_obj = path if isinstance(path, Path) else Path(path)

    try:
        stat = path_obj.stat()
        return {
            "path": str(path_obj.resolve()),
            "size": stat.st_size,
            "mtime": stat.st_mtime,
        }
    except OSError:
        return {
            "path": str(path_obj),
            "size": 0,
            "mtime": 0,
        }


def generate_image_hash(image: PILImage) -> str:
    save_image = image
    if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
        save_image = image.convert("RGB")

    image_buffer = io.BytesIO()
    save_image.save(image_buffer, format="PNG")
    image_content = image_buffer.getvalue()

    return hashlib.sha256(image_content).hexdigest()[:16]


def build_cache_kwargs(
    backend_name: str,
    config_dict: dict[str, Any],
    image_hash: str | None = None,
    file_info: dict[str, Any] | None = None,
) -> dict[str, Any]:
    cache_kwargs = {
        "ocr_backend": backend_name,
        "ocr_config": str(sorted(config_dict.items())),
    }

    if image_hash:
        cache_kwargs["image_hash"] = image_hash
    if file_info:
        cache_kwargs["file_info"] = str(sorted(file_info.items()))

    return cache_kwargs


async def handle_cache_lookup_async(cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)

    if (cached_bytes := ocr_cache.get(cache_key)) is not None:
        return deserialize(cached_bytes, target_type=ExtractionResult)

    if ocr_cache.is_processing(cache_key):
        event = ocr_cache.mark_processing(cache_key)
        await anyio.to_thread.run_sync(event.wait)

        if (cached_bytes := ocr_cache.get(cache_key)) is not None:
            return deserialize(cached_bytes, target_type=ExtractionResult)

    ocr_cache.mark_processing(cache_key)
    return None


def handle_cache_lookup_sync(cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)

    if (cached_bytes := ocr_cache.get(cache_key)) is not None:
        return deserialize(cached_bytes, target_type=ExtractionResult)

    if ocr_cache.is_processing(cache_key):
        event = ocr_cache.mark_processing(cache_key)
        event.wait()

        if (cached_bytes := ocr_cache.get(cache_key)) is not None:
            return deserialize(cached_bytes, target_type=ExtractionResult)

    ocr_cache.mark_processing(cache_key)
    return None


async def cache_and_complete_async(
    result: ExtractionResult,
    cache_kwargs: dict[str, Any],
    use_cache: bool,
) -> None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)

    if use_cache:
        ocr_cache.set(cache_key, serialize(result))

    ocr_cache.mark_complete(cache_key)


def cache_and_complete_sync(
    result: ExtractionResult,
    cache_kwargs: dict[str, Any],
    use_cache: bool,
) -> None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)

    if use_cache:
        ocr_cache.set(cache_key, serialize(result))

    ocr_cache.mark_complete(cache_key)


def mark_processing_complete(cache_kwargs: dict[str, Any]) -> None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)
    ocr_cache.mark_complete(cache_key)
