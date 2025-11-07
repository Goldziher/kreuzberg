from __future__ import annotations

import hashlib
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

from kreuzberg._types import ExtractionResult
from kreuzberg._utils._ref import Ref
from kreuzberg._utils._serialization import deserialize, serialize
from kreuzberg._utils._sync import run_sync

T = TypeVar("T")

CACHE_CLEANUP_FREQUENCY = 100


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
        if not kwargs:
            return "empty"

        parts = []
        for key in sorted(kwargs):
            value = kwargs[key]
            if isinstance(value, (str, int, float, bool)):
                parts.append(f"{key}={value}")
            elif isinstance(value, bytes):
                parts.append(f"{key}=bytes:{len(value)}")
            else:
                parts.append(f"{key}={type(value).__name__}:{value!s}")

        cache_str = "&".join(parts)
        return hashlib.sha256(cache_str.encode()).hexdigest()[:16]

    def _get_cache_path(self, cache_key: str) -> Path:
        return self.cache_dir / f"{cache_key}.msgpack"

    def _is_cache_valid(self, cache_path: Path) -> bool:
        try:
            if not cache_path.exists():
                return False

            mtime = cache_path.stat().st_mtime
            age_days = (time.time() - mtime) / (24 * 3600)

            return age_days <= self.max_age_days
        except OSError:
            return False

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
        try:
            cache_files = list(self.cache_dir.glob("*.msgpack"))
            cutoff_time = time.time() - (self.max_age_days * 24 * 3600)

            remaining_files = []
            for cache_file in cache_files:
                try:
                    if cache_file.stat().st_mtime < cutoff_time:
                        cache_file.unlink(missing_ok=True)
                    else:
                        remaining_files.append(cache_file)
                except OSError:  # noqa: PERF203
                    continue

            cache_files = remaining_files

            total_size = sum(cache_file.stat().st_size for cache_file in cache_files if cache_file.exists()) / (
                1024 * 1024
            )

            if total_size > self.max_cache_size_mb:
                cache_files.sort(key=lambda f: f.stat().st_mtime if f.exists() else 0)

                for cache_file in cache_files:
                    try:
                        size_mb = cache_file.stat().st_size / (1024 * 1024)
                        cache_file.unlink(missing_ok=True)
                        total_size -= size_mb

                        if total_size <= self.max_cache_size_mb * 0.8:
                            break
                    except OSError:
                        continue
        except (OSError, ValueError, TypeError):
            pass

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
        try:
            for cache_file in self.cache_dir.glob("*.msgpack"):
                cache_file.unlink(missing_ok=True)
        except OSError:
            pass

        with self._lock:
            pass

    def get_stats(self) -> dict[str, Any]:
        try:
            cache_files = list(self.cache_dir.glob("*.msgpack"))
            total_size = sum(cache_file.stat().st_size for cache_file in cache_files if cache_file.exists())

            return {
                "cache_type": self.cache_type,
                "cached_results": len(cache_files),
                "processing_results": len(self._processing),
                "total_cache_size_mb": total_size / 1024 / 1024,
                "avg_result_size_kb": (total_size / len(cache_files) / 1024) if cache_files else 0,
                "cache_dir": str(self.cache_dir),
                "max_cache_size_mb": self.max_cache_size_mb,
                "max_age_days": self.max_age_days,
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
