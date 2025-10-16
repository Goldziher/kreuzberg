from __future__ import annotations

import threading
from pathlib import Path
from typing import TYPE_CHECKING, Any, Generic, TypeVar

import msgspec

from kreuzberg._internal_bindings import GenericCache as RustGenericCache
from kreuzberg._utils._ref import Ref

if TYPE_CHECKING:
    from kreuzberg._internal_bindings import CacheStatsDict

T = TypeVar("T")


class KreuzbergCache(Generic[T]):
    def __init__(
        self,
        cache_type: str,
        cache_dir: Path | str | None = None,
        max_cache_size_mb: float = 500.0,
        max_age_days: int = 30,
    ) -> None:
        cache_dir_str = str(cache_dir) if cache_dir else None
        self._cache = RustGenericCache(
            cache_type=cache_type,
            cache_dir=cache_dir_str,
            max_age_days=float(max_age_days),
            max_cache_size_mb=max_cache_size_mb,
            min_free_space_mb=1000.0,
        )
        self.cache_type = cache_type
        self.cache_dir = Path(self._cache.cache_dir)
        self.max_cache_size_mb = max_cache_size_mb
        self.max_age_days = max_age_days

        self._processing_events: dict[str, threading.Event] = {}
        self._lock = threading.Lock()

    def get(self, cache_key: str, source_file: str | None = None) -> T | None:
        data = self._cache.get(cache_key, source_file)
        if data is None:
            return None
        return data  # type: ignore[return-value]

    def set(self, cache_key: str, value: T, source_file: str | None = None) -> None:
        try:
            if isinstance(value, bytes):
                self._cache.set(cache_key, value, source_file)
            else:
                data = msgspec.msgpack.encode(value)
                self._cache.set(cache_key, data, source_file)
        except (TypeError, ValueError, msgspec.MsgspecError):
            pass

    def is_processing(self, cache_key: str) -> bool:
        with self._lock:
            return cache_key in self._processing_events or self._cache.is_processing(cache_key)

    def mark_processing(self, cache_key: str) -> threading.Event:
        with self._lock:
            if cache_key not in self._processing_events:
                self._processing_events[cache_key] = threading.Event()
            event = self._processing_events[cache_key]

        self._cache.mark_processing(cache_key)
        return event

    def mark_complete(self, cache_key: str) -> None:
        with self._lock:
            if cache_key in self._processing_events:
                event = self._processing_events.pop(cache_key)
                event.set()

        self._cache.mark_complete(cache_key)

    def clear(self) -> tuple[int, float]:
        result = self._cache.clear()
        with self._lock:
            self._processing_events.clear()
        return result

    def get_stats(self) -> CacheStatsDict:
        return {
            "total_files": self._cache.get_stats().total_files,
            "total_size_mb": self._cache.get_stats().total_size_mb,
            "available_space_mb": self._cache.get_stats().available_space_mb,
            "oldest_file_age_days": self._cache.get_stats().oldest_file_age_days,
            "newest_file_age_days": self._cache.get_stats().newest_file_age_days,
        }


_mime_cache_ref: Ref[KreuzbergCache[bytes]] = Ref("mime_cache", lambda: KreuzbergCache(cache_type="mime"))
_ocr_cache_ref: Ref[KreuzbergCache[bytes]] = Ref("ocr_cache", lambda: KreuzbergCache(cache_type="ocr"))
_table_cache_ref: Ref[KreuzbergCache[bytes]] = Ref("table_cache", lambda: KreuzbergCache(cache_type="tables"))
_document_cache_ref: Ref[Any] = Ref(
    "document_cache",
    lambda: __import__("kreuzberg._utils._document_cache", fromlist=["get_document_cache"]).get_document_cache(),
)


def get_mime_cache() -> KreuzbergCache[bytes]:
    return _mime_cache_ref.get()


def get_ocr_cache() -> KreuzbergCache[bytes]:
    return _ocr_cache_ref.get()


def get_table_cache() -> KreuzbergCache[bytes]:
    return _table_cache_ref.get()


def get_document_cache() -> Any:
    return _document_cache_ref.get()


def clear_all_caches() -> None:
    if _mime_cache_ref.is_initialized():
        _mime_cache_ref.get().clear()
    if _ocr_cache_ref.is_initialized():
        _ocr_cache_ref.get().clear()
    if _table_cache_ref.is_initialized():
        _table_cache_ref.get().clear()
    if _document_cache_ref.is_initialized():
        _document_cache_ref.get().clear()

    _mime_cache_ref.clear()
    _ocr_cache_ref.clear()
    _table_cache_ref.clear()
    _document_cache_ref.clear()
