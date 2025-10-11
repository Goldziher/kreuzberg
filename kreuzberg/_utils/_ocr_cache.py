from __future__ import annotations

import hashlib
import inspect
import io
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import anyio

from kreuzberg._internal_bindings import generate_cache_key
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._cache import get_ocr_cache
from kreuzberg._utils._serialization import deserialize, serialize

if TYPE_CHECKING:
    from collections.abc import Callable

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


async def _call_cache_method_async(
    cache: Any,
    method_name: str,
    cache_kwargs: dict[str, Any],
    cache_key: str,
) -> tuple[bool, Any]:
    method = getattr(cache, method_name, None)
    if method is None:
        return False, None

    result, handled = await _attempt_async_candidate(lambda: method(**cache_kwargs))
    if handled:
        return True, result

    result, handled = await _attempt_async_candidate(lambda: method(cache_key))
    if handled:
        return True, result

    result, handled = await _attempt_async_candidate(lambda: method(cache_key=cache_key))
    if handled:
        return True, result

    return True, None


def _call_cache_method(
    cache: Any,
    method_name: str,
    cache_kwargs: dict[str, Any],
    cache_key: str,
) -> Any:
    method = getattr(cache, method_name, None)
    if method is None:
        return None

    result, handled = _attempt_sync_candidate(lambda: method(**cache_kwargs))
    if handled:
        return result

    result, handled = _attempt_sync_candidate(lambda: method(cache_key))
    if handled:
        return result

    result, handled = _attempt_sync_candidate(lambda: method(cache_key=cache_key))
    if handled:
        return result

    return None


async def _attempt_async_candidate(call: Callable[[], Any]) -> tuple[Any, bool]:
    try:
        result = call()
    except TypeError:
        return None, False

    if inspect.isawaitable(result):
        result = await result
    return result, True


def _attempt_sync_candidate(call: Callable[[], Any]) -> tuple[Any, bool]:
    try:
        return call(), True
    except TypeError:
        return None, False


async def _store_async_result(
    cache: Any,
    result: ExtractionResult,
    serialized: bytes,
    cache_kwargs: dict[str, Any],
    cache_key: str,
) -> bool:
    async_store = getattr(cache, "aset", None)
    if async_store is None:
        return False

    store_method = cast("Any", async_store)

    try:
        await store_method(result, **cache_kwargs)
        return True
    except TypeError:
        pass

    try:
        await store_method(cache_key, serialized)
        return True
    except TypeError:
        pass

    try:
        await store_method(cache_key=cache_key, value=serialized)
        return True
    except TypeError:
        return False


def _store_sync_result(
    cache: Any,
    result: ExtractionResult,
    serialized: bytes,
    cache_kwargs: dict[str, Any],
    cache_key: str,
) -> bool:
    sync_store = getattr(cache, "set", None)
    if sync_store is None:
        return False

    store_method = cast("Any", sync_store)

    try:
        store_method(result, **cache_kwargs)
        return True
    except TypeError:
        pass

    try:
        store_method(cache_key, serialized)
        return True
    except TypeError:
        pass

    try:
        store_method(cache_key=cache_key, value=serialized)
        return True
    except TypeError:
        return False


async def handle_cache_lookup_async(cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)

    used_async_interface, cached = await _call_cache_method_async(
        ocr_cache,
        "aget",
        cache_kwargs,
        cache_key,
    )
    if cached is None and not used_async_interface:
        cached = _call_cache_method(ocr_cache, "get", cache_kwargs, cache_key)

    if cached is not None:
        return _deserialize_result(cached)

    in_progress = _call_cache_method(ocr_cache, "is_processing", cache_kwargs, cache_key)

    if in_progress:
        event = _call_cache_method(ocr_cache, "mark_processing", cache_kwargs, cache_key)
        if event is not None:
            await anyio.to_thread.run_sync(event.wait)

        used_async_interface, cached = await _call_cache_method_async(
            ocr_cache,
            "aget",
            cache_kwargs,
            cache_key,
        )
        if cached is None and not used_async_interface:
            cached = _call_cache_method(ocr_cache, "get", cache_kwargs, cache_key)

        if cached is not None:
            return _deserialize_result(cached)

    _call_cache_method(ocr_cache, "mark_processing", cache_kwargs, cache_key)
    return None


def handle_cache_lookup_sync(cache_kwargs: dict[str, Any]) -> ExtractionResult | None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)

    cached = _call_cache_method(ocr_cache, "get", cache_kwargs, cache_key)
    if cached is not None:
        return _deserialize_result(cached)

    in_progress = _call_cache_method(ocr_cache, "is_processing", cache_kwargs, cache_key)

    if in_progress:
        event = _call_cache_method(ocr_cache, "mark_processing", cache_kwargs, cache_key)
        if event is not None:
            event.wait()

        cached = _call_cache_method(ocr_cache, "get", cache_kwargs, cache_key)
        if cached is not None:
            return _deserialize_result(cached)

    _call_cache_method(ocr_cache, "mark_processing", cache_kwargs, cache_key)
    return None


async def cache_and_complete_async(
    result: ExtractionResult,
    cache_kwargs: dict[str, Any],
    use_cache: bool,
) -> None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)
    serialized = serialize(result)

    if use_cache:
        stored = await _store_async_result(ocr_cache, result, serialized, cache_kwargs, cache_key)
        if not stored:
            _store_sync_result(ocr_cache, result, serialized, cache_kwargs, cache_key)

    try:
        ocr_cache.mark_complete(**cache_kwargs)
    except TypeError:
        try:
            ocr_cache.mark_complete(cache_key)
        except TypeError:
            ocr_cache.mark_complete(cache_key=cache_key)


def cache_and_complete_sync(
    result: ExtractionResult,
    cache_kwargs: dict[str, Any],
    use_cache: bool,
) -> None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)
    serialized = serialize(result)

    if use_cache:
        _store_sync_result(ocr_cache, result, serialized, cache_kwargs, cache_key)

    try:
        ocr_cache.mark_complete(**cache_kwargs)
    except TypeError:
        try:
            ocr_cache.mark_complete(cache_key)
        except TypeError:
            ocr_cache.mark_complete(cache_key=cache_key)


def mark_processing_complete(cache_kwargs: dict[str, Any]) -> None:
    ocr_cache = get_ocr_cache()
    cache_key = generate_cache_key(**cache_kwargs)
    try:
        ocr_cache.mark_complete(**cache_kwargs)
    except TypeError:
        try:
            ocr_cache.mark_complete(cache_key)
        except TypeError:
            ocr_cache.mark_complete(cache_key=cache_key)


def _deserialize_result(value: Any) -> ExtractionResult | None:
    if value is None:
        return None
    if isinstance(value, ExtractionResult):
        return value
    if isinstance(value, bytearray):
        return deserialize(bytes(value), target_type=ExtractionResult)
    if isinstance(value, (bytes, str)):
        return deserialize(value, target_type=ExtractionResult)
    return deserialize(serialize(value), target_type=ExtractionResult)
