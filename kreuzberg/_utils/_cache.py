from __future__ import annotations

import io
import os
import threading
import time
from contextlib import suppress
from io import StringIO
from pathlib import Path
from typing import Any, Generic, TypeVar, cast

import polars as pl
from anyio import Path as AsyncPath

from kreuzberg._internal_bindings import (
    clear_cache_directory as _rust_clear_directory,
)
from kreuzberg._internal_bindings import (
    generate_cache_key as _rust_generate_key,
)
from kreuzberg._internal_bindings import (
    get_cache_metadata as _rust_get_metadata,
)
from kreuzberg._internal_bindings import (
    is_cache_valid as _rust_is_valid,
)
from kreuzberg._internal_bindings import (
    smart_cleanup_cache as _rust_smart_cleanup,
)
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._ref import Ref
from kreuzberg._utils._serialization import deserialize, serialize
from kreuzberg._utils._sync import run_sync

T = TypeVar("T")

CACHE_CLEANUP_FREQUENCY = 100
MIN_FREE_SPACE_MB = 1000.0  # Minimum 1GB free space


class KreuzbergCache(Generic[T]):
    def __init__(
        self,
        cache_type: str,
        cache_dir: Path | str | None = None,
        max_cache_size_mb: float = 500.0,
        max_age_days: int = 30,
    ) -> None:
        if cache_dir is None:
            cache_dir = Path.cwd() / ".kreuzberg" / cache_type

        self.cache_dir = Path(cache_dir)
        self.cache_type = cache_type
        self.max_cache_size_mb = max_cache_size_mb
        self.max_age_days = max_age_days

        self.cache_dir.mkdir(parents=True, exist_ok=True)

        # In-memory tracking of processing state (session-scoped)  # ~keep
        self._processing: dict[str, threading.Event] = {}
        self._lock = threading.Lock()

    def _get_cache_key(self, **kwargs: Any) -> str:
        return _rust_generate_key(**kwargs)

    def _get_cache_path(self, cache_key: str) -> Path:
        return self.cache_dir / f"{cache_key}.msgpack"

    def _is_cache_valid(self, cache_path: Path) -> bool:
        return _rust_is_valid(str(cache_path), float(self.max_age_days))

    def _serialize_result(self, result: T) -> dict[str, Any]:
        if isinstance(result, list) and result and isinstance(result[0], dict) and "df" in result[0]:
            serialized_data = []
            for item in result:
                if isinstance(item, dict) and "df" in item:
                    serialized_item = {k: v for k, v in item.items() if k != "df"}
                    if item["df"] is not None:
                        buffer = io.BytesIO()
                        if hasattr(item["df"], "write_parquet"):
                            item["df"].write_parquet(buffer)
                            serialized_item["df_parquet"] = buffer.getvalue()
                        elif hasattr(item["df"], "write_csv"):
                            item["df"].write_csv(buffer)
                            serialized_item["df_parquet"] = buffer.getvalue()
                        else:
                            serialized_item["df_parquet"] = None
                    else:
                        serialized_item["df_parquet"] = None
                    serialized_data.append(serialized_item)
                else:
                    serialized_data.append(item)
            return {"type": "TableDataList", "data": serialized_data, "cached_at": time.time()}

        return {"type": type(result).__name__, "data": result, "cached_at": time.time()}

    def _deserialize_result(self, cached_data: dict[str, Any]) -> T:
        data = cached_data["data"]

        if cached_data.get("type") == "TableDataList" and isinstance(data, list):
            deserialized_data = []
            for item in data:
                if isinstance(item, dict) and ("df_parquet" in item or "df_csv" in item):
                    deserialized_item = {k: v for k, v in item.items() if k not in ("df_parquet", "df_csv")}

                    if "df_parquet" in item:
                        if item["df_parquet"] is None:
                            deserialized_item["df"] = pl.DataFrame()
                        else:
                            buffer = io.BytesIO(item["df_parquet"])
                            try:
                                deserialized_item["df"] = pl.read_parquet(buffer)
                            except Exception:  # noqa: BLE001
                                deserialized_item["df"] = pl.DataFrame()
                    elif "df_csv" in item:
                        if item["df_csv"] is None or item["df_csv"] == "" or item["df_csv"] == "\n":
                            deserialized_item["df"] = pl.DataFrame()
                        else:
                            deserialized_item["df"] = pl.read_csv(StringIO(item["df_csv"]))
                    deserialized_data.append(deserialized_item)
                else:
                    deserialized_data.append(item)
            return cast("T", deserialized_data)

        if cached_data.get("type") == "ExtractionResult" and isinstance(data, dict):
            return cast("T", ExtractionResult(**data))

        return cast("T", data)

    def _cleanup_cache(self) -> None:
        with suppress(OSError, ValueError, TypeError):
            _rust_smart_cleanup(
                str(self.cache_dir),
                float(self.max_age_days),
                float(self.max_cache_size_mb),
                MIN_FREE_SPACE_MB,
            )

    def get(self, **kwargs: Any) -> T | None:
        cache_key = self._get_cache_key(**kwargs)
        cache_path = self._get_cache_path(cache_key)

        if not self._is_cache_valid(cache_path):
            return None

        try:
            content = cache_path.read_bytes()
            cached_data = deserialize(content, dict)
            return self._deserialize_result(cached_data)
        except (OSError, ValueError, KeyError):
            with suppress(OSError):
                cache_path.unlink(missing_ok=True)
            return None

    def set(self, result: T, **kwargs: Any) -> None:
        cache_key = self._get_cache_key(**kwargs)
        cache_path = self._get_cache_path(cache_key)

        try:
            serialized = self._serialize_result(result)
            content = serialize(serialized)
            cache_path.write_bytes(content)

            if hash(cache_key) % CACHE_CLEANUP_FREQUENCY == 0:
                self._cleanup_cache()
        except (OSError, TypeError, ValueError):
            pass

    async def aget(self, **kwargs: Any) -> T | None:
        cache_key = self._get_cache_key(**kwargs)
        cache_path = AsyncPath(self._get_cache_path(cache_key))

        if not await run_sync(self._is_cache_valid, Path(cache_path)):
            return None

        try:
            content = await cache_path.read_bytes()
            cached_data = deserialize(content, dict)
            return self._deserialize_result(cached_data)
        except (OSError, ValueError, KeyError):
            with suppress(Exception):
                await cache_path.unlink(missing_ok=True)
            return None

    async def aset(self, result: T, **kwargs: Any) -> None:
        cache_key = self._get_cache_key(**kwargs)
        cache_path = AsyncPath(self._get_cache_path(cache_key))

        try:
            serialized = self._serialize_result(result)
            content = serialize(serialized)
            await cache_path.write_bytes(content)

            if hash(cache_key) % 100 == 0:
                await run_sync(self._cleanup_cache)
        except (OSError, TypeError, ValueError):
            pass

    def is_processing(self, **kwargs: Any) -> bool:
        cache_key = self._get_cache_key(**kwargs)
        with self._lock:
            return cache_key in self._processing

    def mark_processing(self, **kwargs: Any) -> threading.Event:
        cache_key = self._get_cache_key(**kwargs)

        with self._lock:
            if cache_key not in self._processing:
                self._processing[cache_key] = threading.Event()
            return self._processing[cache_key]

    def mark_complete(self, **kwargs: Any) -> None:
        cache_key = self._get_cache_key(**kwargs)

        with self._lock:
            if cache_key in self._processing:
                event = self._processing.pop(cache_key)
                event.set()

    def clear(self) -> None:
        with suppress(OSError):
            _rust_clear_directory(str(self.cache_dir))

        with self._lock:
            pass

    def get_stats(self) -> dict[str, Any]:
        try:
            stats = _rust_get_metadata(str(self.cache_dir))
            avg_size_kb = (stats.total_size_mb * 1024 / stats.total_files) if stats.total_files else 0

            return {
                "cache_type": self.cache_type,
                "cached_results": stats.total_files,
                "processing_results": len(self._processing),
                "total_cache_size_mb": stats.total_size_mb,
                "avg_result_size_kb": avg_size_kb,
                "cache_dir": str(self.cache_dir),
                "max_cache_size_mb": self.max_cache_size_mb,
                "max_age_days": self.max_age_days,
                "available_space_mb": stats.available_space_mb,
                "oldest_file_age_days": stats.oldest_file_age_days,
                "newest_file_age_days": stats.newest_file_age_days,
            }
        except OSError:
            return {
                "cache_type": self.cache_type,
                "cached_results": 0,
                "processing_results": len(self._processing),
                "total_cache_size_mb": 0.0,
                "avg_result_size_kb": 0.0,
                "cache_dir": str(self.cache_dir),
                "max_cache_size_mb": self.max_cache_size_mb,
                "max_age_days": self.max_age_days,
            }


def _create_ocr_cache() -> KreuzbergCache[ExtractionResult]:
    cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
    cache_dir: Path | None = None
    if cache_dir_str:
        cache_dir = Path(cache_dir_str) / "ocr"

    return KreuzbergCache[ExtractionResult](
        cache_type="ocr",
        cache_dir=cache_dir,
        max_cache_size_mb=float(os.environ.get("KREUZBERG_OCR_CACHE_SIZE_MB", "500")),
        max_age_days=int(os.environ.get("KREUZBERG_OCR_CACHE_AGE_DAYS", "30")),
    )


_ocr_cache_ref = Ref("ocr_cache", _create_ocr_cache)


def get_ocr_cache() -> KreuzbergCache[ExtractionResult]:
    return _ocr_cache_ref.get()


def _create_document_cache() -> KreuzbergCache[ExtractionResult]:
    cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
    cache_dir: Path | None = None
    if cache_dir_str:
        cache_dir = Path(cache_dir_str) / "documents"

    return KreuzbergCache[ExtractionResult](
        cache_type="documents",
        cache_dir=cache_dir,
        max_cache_size_mb=float(os.environ.get("KREUZBERG_DOCUMENT_CACHE_SIZE_MB", "1000")),
        max_age_days=int(os.environ.get("KREUZBERG_DOCUMENT_CACHE_AGE_DAYS", "7")),
    )


_document_cache_ref = Ref("document_cache", _create_document_cache)


def get_document_cache() -> KreuzbergCache[ExtractionResult]:
    return _document_cache_ref.get()


def _create_table_cache() -> KreuzbergCache[Any]:
    cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
    cache_dir: Path | None = None
    if cache_dir_str:
        cache_dir = Path(cache_dir_str) / "tables"

    return KreuzbergCache[Any](
        cache_type="tables",
        cache_dir=cache_dir,
        max_cache_size_mb=float(os.environ.get("KREUZBERG_TABLE_CACHE_SIZE_MB", "200")),
        max_age_days=int(os.environ.get("KREUZBERG_TABLE_CACHE_AGE_DAYS", "30")),
    )


_table_cache_ref = Ref("table_cache", _create_table_cache)


def get_table_cache() -> KreuzbergCache[Any]:
    return _table_cache_ref.get()


def _create_mime_cache() -> KreuzbergCache[str]:
    cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
    cache_dir: Path | None = None
    if cache_dir_str:
        cache_dir = Path(cache_dir_str) / "mime"

    return KreuzbergCache[str](
        cache_type="mime",
        cache_dir=cache_dir,
        max_cache_size_mb=float(os.environ.get("KREUZBERG_MIME_CACHE_SIZE_MB", "50")),
        max_age_days=int(os.environ.get("KREUZBERG_MIME_CACHE_AGE_DAYS", "60")),
    )


_mime_cache_ref = Ref("mime_cache", _create_mime_cache)


def get_mime_cache() -> KreuzbergCache[str]:
    return _mime_cache_ref.get()


def clear_all_caches() -> None:
    if _ocr_cache_ref.is_initialized():
        get_ocr_cache().clear()
    if _document_cache_ref.is_initialized():
        get_document_cache().clear()
    if _table_cache_ref.is_initialized():
        get_table_cache().clear()
    if _mime_cache_ref.is_initialized():
        get_mime_cache().clear()

    _ocr_cache_ref.clear()
    _document_cache_ref.clear()
    _table_cache_ref.clear()
    _mime_cache_ref.clear()
