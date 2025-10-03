from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any

import msgspec

from kreuzberg._internal_bindings import generate_cache_key
from kreuzberg._types import ExtractionConfig, ExtractionResult
from kreuzberg._utils._cache import KreuzbergCache
from kreuzberg._utils._serialization import deserialize, serialize, to_dict

if TYPE_CHECKING:
    import threading

    from kreuzberg._internal_bindings import DocumentCacheStatsDict


class DocumentCache:
    """Document cache wrapper using Rust GenericCache backend."""

    def __init__(self) -> None:
        self._cache: KreuzbergCache[Any] = KreuzbergCache(cache_type="documents")

    def _get_cache_key(self, file_path: Path | str, config: ExtractionConfig | None = None) -> str:
        path = Path(file_path).resolve()

        try:
            stat = path.stat()
            file_info = {
                "path": str(path),
                "size": stat.st_size,
                "mtime": stat.st_mtime,
            }
        except OSError:
            file_info = {"path": str(path), "size": 0, "mtime": 0}

        # V4: Convert entire config to dict for cache key
        config_info = {}
        if config:
            config_info = to_dict(config, include_none=False)

        cache_data = {**file_info, **config_info}
        return generate_cache_key(**cache_data)

    def get(self, file_path: Path | str, config: ExtractionConfig | None = None) -> ExtractionResult | None:
        cache_key = self._get_cache_key(file_path, config)
        raw_bytes = self._cache.get(cache_key, source_file=str(Path(file_path).resolve()))

        if raw_bytes is None:
            return None

        # Deserialize bytes to ExtractionResult using our serialization utils
        try:
            return deserialize(raw_bytes, target_type=ExtractionResult)
        except (TypeError, ValueError, msgspec.MsgspecError):
            # Corrupted cache, clean up
            cache_path = self._cache.cache_dir / f"{cache_key}.msgpack"
            cache_path.unlink(missing_ok=True)
            return None

    def set(self, file_path: Path | str, config: ExtractionConfig | None, result: ExtractionResult) -> None:
        cache_key = self._get_cache_key(file_path, config)
        # Serialize using our utils
        serialized = serialize(result)
        # Store as bytes
        self._cache.set(cache_key, serialized, source_file=str(Path(file_path).resolve()))

    def is_processing(self, file_path: Path | str, config: ExtractionConfig | None = None) -> bool:
        cache_key = self._get_cache_key(file_path, config)
        return self._cache.is_processing(cache_key)

    def mark_processing(self, file_path: Path | str, config: ExtractionConfig | None = None) -> threading.Event:
        cache_key = self._get_cache_key(file_path, config)
        return self._cache.mark_processing(cache_key)

    def mark_complete(self, file_path: Path | str, config: ExtractionConfig | None = None) -> None:
        cache_key = self._get_cache_key(file_path, config)
        self._cache.mark_complete(cache_key)

    def clear(self) -> None:
        self._cache.clear()

    def get_stats(self) -> DocumentCacheStatsDict:
        rust_stats = self._cache.get_stats()
        return {
            "cached_documents": rust_stats["total_files"],
            "processing_documents": 0,  # Processing state is tracked in Rust
            "total_cache_size_mb": rust_stats["total_size_mb"],
        }


_document_cache = DocumentCache()


def get_document_cache() -> DocumentCache:
    return _document_cache


def clear_document_cache() -> None:
    _document_cache.clear()
