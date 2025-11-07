from __future__ import annotations

import hashlib
import io
from pathlib import Path
from typing import TYPE_CHECKING, Any

import anyio

from kreuzberg._utils._cache import get_ocr_cache

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage

    from kreuzberg._types import ExtractionResult


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

    cached_result = await ocr_cache.aget(**cache_kwargs)
    if cached_result is not None:
        return cached_result

    if ocr_cache.is_processing(**cache_kwargs):
        event = ocr_cache.mark_processing(**cache_kwargs)
        await anyio.to_thread.run_sync(event.wait)

        cached_result = await ocr_cache.aget(**cache_kwargs)
        if cached_result is not None:
            return cached_result

    ocr_cache.mark_processing(**cache_kwargs)
    return None


def handle_cache_lookup_sync(cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
    ocr_cache = get_ocr_cache()

    cached_result = ocr_cache.get(**cache_kwargs)
    if cached_result is not None:
        return cached_result

    if ocr_cache.is_processing(**cache_kwargs):
        event = ocr_cache.mark_processing(**cache_kwargs)
        event.wait()

        cached_result = ocr_cache.get(**cache_kwargs)
        if cached_result is not None:
            return cached_result

    ocr_cache.mark_processing(**cache_kwargs)
    return None


async def cache_and_complete_async(
    result: ExtractionResult,
    cache_kwargs: dict[str, Any],
    use_cache: bool,
) -> None:
    ocr_cache = get_ocr_cache()

    if use_cache:
        await ocr_cache.aset(result, **cache_kwargs)

    ocr_cache.mark_complete(**cache_kwargs)


def cache_and_complete_sync(
    result: ExtractionResult,
    cache_kwargs: dict[str, Any],
    use_cache: bool,
) -> None:
    ocr_cache = get_ocr_cache()

    if use_cache:
        ocr_cache.set(result, **cache_kwargs)

    ocr_cache.mark_complete(**cache_kwargs)


def mark_processing_complete(cache_kwargs: dict[str, Any]) -> None:
    ocr_cache = get_ocr_cache()
    ocr_cache.mark_complete(**cache_kwargs)
