"""General-purpose file-based caching layer for Kreuzberg."""

from __future__ import annotations

import hashlib
import os
import threading
import time
from contextlib import suppress
from pathlib import Path
from typing import Any, Generic, TypeVar

from anyio import Path as AsyncPath

from kreuzberg._types import ExtractionResult
from kreuzberg._utils._serialization import deserialize, serialize
from kreuzberg._utils._sync import run_sync

T = TypeVar("T")


class KreuzbergCache(Generic[T]):
    """File-based cache for Kreuzberg operations.

    Provides both sync and async interfaces for caching extraction results,
    OCR results, table data, and other expensive operations to disk.
    """

    def __init__(
        self,
        cache_type: str,
        cache_dir: Path | str | None = None,
        max_cache_size_mb: float = 500.0,
        max_age_days: int = 30,
    ) -> None:
        """Initialize cache.

        Args:
            cache_type: Type of cache (e.g., 'ocr', 'tables', 'documents', 'mime')
            cache_dir: Cache directory (defaults to .kreuzberg/{cache_type} in cwd)
            max_cache_size_mb: Maximum cache size in MB (default: 500MB)
            max_age_days: Maximum age of cached results in days (default: 30 days)
        """
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
        """Generate cache key from kwargs.

        Args:
            **kwargs: Key-value pairs to generate cache key from

        Returns:
            Unique cache key string
        """
        # Use more efficient string building for cache key
        if not kwargs:
            return "empty"

        # Build key string efficiently
        parts = []
        for key in sorted(kwargs):
            value = kwargs[key]
            # Convert common types efficiently
            if isinstance(value, (str, int, float, bool)):
                parts.append(f"{key}={value}")
            elif isinstance(value, bytes):
                parts.append(f"{key}=bytes:{len(value)}")
            else:
                parts.append(f"{key}={type(value).__name__}:{value!s}")

        cache_str = "&".join(parts)
        return hashlib.sha256(cache_str.encode()).hexdigest()[:16]

    def _get_cache_path(self, cache_key: str) -> Path:
        """Get cache file path for key."""
        return self.cache_dir / f"{cache_key}.msgpack"

    def _is_cache_valid(self, cache_path: Path) -> bool:
        """Check if cached result is still valid."""
        try:
            if not cache_path.exists():
                return False

            mtime = cache_path.stat().st_mtime
            age_days = (time.time() - mtime) / (24 * 3600)

            return age_days <= self.max_age_days
        except OSError:
            return False

    def _serialize_result(self, result: T) -> dict[str, Any]:
        """Serialize result for caching with metadata."""
        return {"type": type(result).__name__, "data": result, "cached_at": time.time()}

    def _deserialize_result(self, cached_data: dict[str, Any]) -> T:
        """Deserialize cached result."""
        data = cached_data["data"]

        if cached_data.get("type") == "ExtractionResult" and isinstance(data, dict):
            from kreuzberg._types import ExtractionResult

            return ExtractionResult(**data)  # type: ignore[return-value]

        return data  # type: ignore[no-any-return]

    def _cleanup_cache(self) -> None:
        """Clean up old and oversized cache entries."""
        try:
            cache_files = list(self.cache_dir.glob("*.msgpack"))

            cutoff_time = time.time() - (self.max_age_days * 24 * 3600)
            for cache_file in cache_files[:]:
                try:
                    if cache_file.stat().st_mtime < cutoff_time:
                        cache_file.unlink(missing_ok=True)
                        cache_files.remove(cache_file)
                except OSError:  # noqa: PERF203
                    continue

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
        """Get cached result (sync).

        Args:
            **kwargs: Key-value pairs to generate cache key from

        Returns:
            Cached result if available, None otherwise
        """
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
        """Cache result (sync).

        Args:
            result: Result to cache
            **kwargs: Key-value pairs to generate cache key from
        """
        cache_key = self._get_cache_key(**kwargs)
        cache_path = self._get_cache_path(cache_key)

        try:
            serialized = self._serialize_result(result)
            content = serialize(serialized)
            cache_path.write_bytes(content)

            if hash(cache_key) % 100 == 0:
                self._cleanup_cache()
        except (OSError, TypeError, ValueError):
            pass

    async def aget(self, **kwargs: Any) -> T | None:
        """Get cached result (async).

        Args:
            **kwargs: Key-value pairs to generate cache key from

        Returns:
            Cached result if available, None otherwise
        """
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
        """Cache result (async).

        Args:
            result: Result to cache
            **kwargs: Key-value pairs to generate cache key from
        """
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
        """Check if operation is currently being processed."""
        cache_key = self._get_cache_key(**kwargs)
        with self._lock:
            return cache_key in self._processing

    def mark_processing(self, **kwargs: Any) -> threading.Event:
        """Mark operation as being processed and return event to wait on."""
        cache_key = self._get_cache_key(**kwargs)

        with self._lock:
            if cache_key not in self._processing:
                self._processing[cache_key] = threading.Event()
            return self._processing[cache_key]

    def mark_complete(self, **kwargs: Any) -> None:
        """Mark operation processing as complete."""
        cache_key = self._get_cache_key(**kwargs)

        with self._lock:
            if cache_key in self._processing:
                event = self._processing.pop(cache_key)
                event.set()

    def clear(self) -> None:
        """Clear all cached results."""
        try:
            for cache_file in self.cache_dir.glob("*.msgpack"):
                cache_file.unlink(missing_ok=True)
        except OSError:
            pass

        with self._lock:
            pass

    def get_stats(self) -> dict[str, Any]:
        """Get cache statistics."""
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


_ocr_cache: KreuzbergCache[ExtractionResult] | None = None
_document_cache: KreuzbergCache[ExtractionResult] | None = None
_table_cache: KreuzbergCache[Any] | None = None
_mime_cache: KreuzbergCache[str] | None = None


def get_ocr_cache() -> KreuzbergCache[ExtractionResult]:
    """Get the global OCR cache instance."""
    global _ocr_cache
    if _ocr_cache is None:
        cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
        cache_dir: Path | None = None
        if cache_dir_str:
            cache_dir = Path(cache_dir_str) / "ocr"

        _ocr_cache = KreuzbergCache[ExtractionResult](
            cache_type="ocr",
            cache_dir=cache_dir,
            max_cache_size_mb=float(os.environ.get("KREUZBERG_OCR_CACHE_SIZE_MB", "500")),
            max_age_days=int(os.environ.get("KREUZBERG_OCR_CACHE_AGE_DAYS", "30")),
        )
    return _ocr_cache


def get_document_cache() -> KreuzbergCache[ExtractionResult]:
    """Get the global document cache instance."""
    global _document_cache
    if _document_cache is None:
        cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
        cache_dir: Path | None = None
        if cache_dir_str:
            cache_dir = Path(cache_dir_str) / "documents"

        _document_cache = KreuzbergCache[ExtractionResult](
            cache_type="documents",
            cache_dir=cache_dir,
            max_cache_size_mb=float(os.environ.get("KREUZBERG_DOCUMENT_CACHE_SIZE_MB", "1000")),
            max_age_days=int(os.environ.get("KREUZBERG_DOCUMENT_CACHE_AGE_DAYS", "7")),
        )
    return _document_cache


def get_table_cache() -> KreuzbergCache[Any]:
    """Get the global table cache instance."""
    global _table_cache
    if _table_cache is None:
        cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
        cache_dir: Path | None = None
        if cache_dir_str:
            cache_dir = Path(cache_dir_str) / "tables"

        _table_cache = KreuzbergCache[Any](
            cache_type="tables",
            cache_dir=cache_dir,
            max_cache_size_mb=float(os.environ.get("KREUZBERG_TABLE_CACHE_SIZE_MB", "200")),
            max_age_days=int(os.environ.get("KREUZBERG_TABLE_CACHE_AGE_DAYS", "30")),
        )
    return _table_cache


def get_mime_cache() -> KreuzbergCache[str]:
    """Get the global MIME type cache instance."""
    global _mime_cache
    if _mime_cache is None:
        cache_dir_str = os.environ.get("KREUZBERG_CACHE_DIR")
        cache_dir: Path | None = None
        if cache_dir_str:
            cache_dir = Path(cache_dir_str) / "mime"

        _mime_cache = KreuzbergCache[str](
            cache_type="mime",
            cache_dir=cache_dir,
            max_cache_size_mb=float(os.environ.get("KREUZBERG_MIME_CACHE_SIZE_MB", "50")),
            max_age_days=int(os.environ.get("KREUZBERG_MIME_CACHE_AGE_DAYS", "60")),
        )
    return _mime_cache


def clear_all_caches() -> None:
    """Clear all caches."""
    get_ocr_cache().clear()
    get_document_cache().clear()
    get_table_cache().clear()
    get_mime_cache().clear()
