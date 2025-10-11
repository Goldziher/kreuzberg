from __future__ import annotations

import tempfile
import threading
from pathlib import Path
from typing import TYPE_CHECKING

import msgspec
import pytest

from kreuzberg._types import ExtractionConfig, ExtractionResult
from kreuzberg._utils._cache import KreuzbergCache, clear_all_caches, get_document_cache, get_mime_cache
from kreuzberg._utils._document_cache import DocumentCache

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture
def temp_cache_dir() -> Generator[Path, None, None]:
    with tempfile.TemporaryDirectory() as temp_dir:
        yield Path(temp_dir)


@pytest.fixture
def cache(temp_cache_dir: Path) -> KreuzbergCache[bytes]:
    return KreuzbergCache[bytes](cache_type="test", cache_dir=temp_cache_dir, max_cache_size_mb=10.0, max_age_days=1)


def test_kreuzberg_cache_init(temp_cache_dir: Path) -> None:
    cache = KreuzbergCache[bytes](cache_type="test", cache_dir=temp_cache_dir)
    assert cache.cache_type == "test"
    assert cache.cache_dir == temp_cache_dir / "test"
    assert cache.max_cache_size_mb == 500.0
    assert cache.max_age_days == 30


def test_kreuzberg_cache_get_set(cache: KreuzbergCache[bytes]) -> None:
    cache_key = "test_key_123"
    data = msgspec.msgpack.encode("test value")
    cache.set(cache_key, data)

    result = cache.get(cache_key)
    assert result == data
    assert msgspec.msgpack.decode(result) == "test value"


def test_kreuzberg_cache_get_miss(cache: KreuzbergCache[bytes]) -> None:
    result = cache.get("nonexistent_key")
    assert result is None


def test_kreuzberg_cache_processing_coordination(cache: KreuzbergCache[bytes]) -> None:
    cache_key = "processing_test"

    assert not cache.is_processing(cache_key)

    event = cache.mark_processing(cache_key)
    assert isinstance(event, threading.Event)
    assert cache.is_processing(cache_key)
    assert not event.is_set()

    cache.mark_complete(cache_key)
    assert not cache.is_processing(cache_key)
    assert event.is_set()


def test_kreuzberg_cache_clear(cache: KreuzbergCache[bytes]) -> None:
    cache.set("key1", msgspec.msgpack.encode("value1"))
    cache.set("key2", msgspec.msgpack.encode("value2"))

    assert cache.get("key1") is not None
    assert cache.get("key2") is not None

    removed, freed = cache.clear()
    assert removed == 2
    assert freed > 0

    assert cache.get("key1") is None
    assert cache.get("key2") is None


def test_kreuzberg_cache_stats(cache: KreuzbergCache[bytes]) -> None:
    cache.set("key1", msgspec.msgpack.encode("value1"))
    cache.set("key2", msgspec.msgpack.encode("value2"))

    stats = cache.get_stats()
    assert stats["total_files"] == 2
    assert stats["total_size_mb"] > 0
    assert stats["available_space_mb"] > 0


def test_kreuzberg_cache_source_file_tracking(cache: KreuzbergCache[bytes], temp_cache_dir: Path) -> None:
    source_file = temp_cache_dir / "source.txt"
    source_file.write_text("original content")

    cache_key = "test_key"
    data = msgspec.msgpack.encode("cached data")
    cache.set(cache_key, data, source_file=str(source_file))

    result = cache.get(cache_key, source_file=str(source_file))
    assert result == data

    source_file.write_text("modified content with different size to invalidate")

    result = cache.get(cache_key, source_file=str(source_file))
    assert result is None


@pytest.fixture
def doc_cache(temp_cache_dir: Path) -> DocumentCache:
    from typing import Any

    doc_cache = DocumentCache()
    doc_cache._cache = KreuzbergCache[Any](cache_type="documents", cache_dir=temp_cache_dir)
    return doc_cache


def test_document_cache_get_set(doc_cache: DocumentCache, temp_cache_dir: Path) -> None:
    test_file = temp_cache_dir / "test.pdf"
    test_file.write_text("test content")

    config = ExtractionConfig()
    result = ExtractionResult(content="extracted text", mime_type="application/pdf", metadata={}, chunks=[], tables=[])

    doc_cache.set(test_file, config, result)

    cached = doc_cache.get(test_file, config)
    assert cached is not None
    assert cached.content == "extracted text"


def test_document_cache_processing_workflow(doc_cache: DocumentCache, temp_cache_dir: Path) -> None:
    test_file = temp_cache_dir / "test.pdf"
    test_file.write_text("test content")

    config = ExtractionConfig()

    assert not doc_cache.is_processing(test_file, config)

    event = doc_cache.mark_processing(test_file, config)
    assert isinstance(event, threading.Event)
    assert doc_cache.is_processing(test_file, config)

    doc_cache.mark_complete(test_file, config)
    assert not doc_cache.is_processing(test_file, config)
    assert event.is_set()


def test_document_cache_file_change_invalidation(doc_cache: DocumentCache, temp_cache_dir: Path) -> None:
    test_file = temp_cache_dir / "test.pdf"
    test_file.write_text("original content")

    config = ExtractionConfig()
    result = ExtractionResult(content="extracted text", mime_type="application/pdf", metadata={}, chunks=[], tables=[])

    doc_cache.set(test_file, config, result)
    assert doc_cache.get(test_file, config) is not None

    test_file.write_text("modified content with different size for invalidation test")

    assert doc_cache.get(test_file, config) is None


def test_document_cache_stats(doc_cache: DocumentCache, temp_cache_dir: Path) -> None:
    test_file = temp_cache_dir / "test.pdf"
    test_file.write_text("test content")

    config = ExtractionConfig()
    result = ExtractionResult(content="extracted text", mime_type="application/pdf", metadata={}, chunks=[], tables=[])

    doc_cache.set(test_file, config, result)

    stats = doc_cache.get_stats()
    assert stats["cached_documents"] == 1
    assert stats["total_cache_size_mb"] > 0


def test_get_mime_cache() -> None:
    cache = get_mime_cache()
    assert isinstance(cache, KreuzbergCache)
    assert cache.cache_type == "mime"


def test_get_document_cache() -> None:
    cache = get_document_cache()
    assert isinstance(cache, DocumentCache)


def test_clear_all_caches() -> None:
    mime_cache = get_mime_cache()
    mime_cache.set("test_key", msgspec.msgpack.encode("test/html"))

    get_document_cache()

    clear_all_caches()

    assert mime_cache.get("test_key") is None
