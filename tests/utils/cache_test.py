from __future__ import annotations

import os
import tempfile
import time
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import patch

import pytest

from kreuzberg._types import ExtractionResult
from kreuzberg._utils._cache import (
    KreuzbergCache,
    _document_cache_ref,
    _mime_cache_ref,
    _ocr_cache_ref,
    _table_cache_ref,
    clear_all_caches,
    get_document_cache,
    get_mime_cache,
    get_ocr_cache,
    get_table_cache,
)

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture
def temp_cache_dir() -> Generator[Path, None, None]:
    with tempfile.TemporaryDirectory() as temp_dir:
        yield Path(temp_dir)


@pytest.fixture
def cache(temp_cache_dir: Path) -> KreuzbergCache[ExtractionResult]:
    return KreuzbergCache[ExtractionResult](
        cache_type="test", cache_dir=temp_cache_dir, max_cache_size_mb=10.0, max_age_days=1
    )


def test_cache_init_default_dir() -> None:
    cache = KreuzbergCache[str](cache_type="test")

    expected_dir = Path.cwd() / ".kreuzberg" / "test"
    assert cache.cache_dir == expected_dir
    assert cache.cache_type == "test"
    assert cache.max_cache_size_mb == 500.0
    assert cache.max_age_days == 30


def test_cache_init_custom_dir(temp_cache_dir: Path) -> None:
    cache = KreuzbergCache[str](cache_type="custom", cache_dir=temp_cache_dir, max_cache_size_mb=100.0, max_age_days=7)

    assert cache.cache_dir == temp_cache_dir
    assert cache.cache_type == "custom"
    assert cache.max_cache_size_mb == 100.0
    assert cache.max_age_days == 7


def test_get_cache_key(cache: KreuzbergCache[ExtractionResult]) -> None:
    key1 = cache._get_cache_key(file_path="/test/file.pdf", config="default")
    key2 = cache._get_cache_key(config="default", file_path="/test/file.pdf")
    key3 = cache._get_cache_key(file_path="/test/other.pdf", config="default")

    # Same params in different order should produce same key  # ~keep
    assert key1 == key2

    # Different params should produce different key  # ~keep
    assert key1 != key3

    # Keys should be 32 characters (128-bit hash for security)  # ~keep
    assert len(key1) == 32


def test_get_cache_path(cache: KreuzbergCache[ExtractionResult]) -> None:
    cache_key = "test1234567890ab"
    cache_path = cache._get_cache_path(cache_key)

    expected_path = cache.cache_dir / "test1234567890ab.msgpack"
    assert cache_path == expected_path


def test_is_cache_valid_nonexistent(cache: KreuzbergCache[ExtractionResult]) -> None:
    cache_path = cache.cache_dir / "nonexistent.msgpack"
    assert not cache._is_cache_valid(cache_path)


def test_is_cache_valid_fresh_file(cache: KreuzbergCache[ExtractionResult]) -> None:
    cache_path = cache.cache_dir / "fresh.msgpack"
    cache_path.write_text("test content")

    assert cache._is_cache_valid(cache_path)


def test_is_cache_valid_old_file(cache: KreuzbergCache[ExtractionResult]) -> None:
    cache_path = cache.cache_dir / "old.msgpack"
    cache_path.write_text("test content")

    old_time = time.time() - (cache.max_age_days + 1) * 24 * 3600
    os.utime(cache_path, (old_time, old_time))

    assert not cache._is_cache_valid(cache_path)


def test_is_cache_valid_os_error(cache: KreuzbergCache[ExtractionResult]) -> None:
    cache_path = cache.cache_dir / "error.msgpack"

    with patch("pathlib.Path.stat", side_effect=OSError("Permission denied")):
        assert not cache._is_cache_valid(cache_path)


def test_serialize_result(cache: KreuzbergCache[ExtractionResult]) -> None:
    result = ExtractionResult(content="Test content", mime_type="text/plain", metadata={}, chunks=[], tables=[])

    serialized = cache._serialize_result(result)

    assert serialized["type"] == "ExtractionResult"
    assert serialized["data"] == result
    assert "cached_at" in serialized
    assert isinstance(serialized["cached_at"], float)


def test_deserialize_result_extraction_result(cache: KreuzbergCache[ExtractionResult]) -> None:
    result_data = {
        "content": "Test content",
        "mime_type": "text/plain",
        "metadata": {"title": "Test"},
        "chunks": ["chunk1"],
        "tables": [],
    }

    cached_data = {"type": "ExtractionResult", "data": result_data, "cached_at": time.time()}

    deserialized = cache._deserialize_result(cached_data)

    assert isinstance(deserialized, ExtractionResult)
    assert deserialized.content == "Test content"
    assert deserialized.mime_type == "text/plain"
    assert deserialized.metadata == {"title": "Test"}


def test_deserialize_result_regular_object(cache: KreuzbergCache[str]) -> None:
    cached_data = {"type": "str", "data": "test string", "cached_at": time.time()}

    deserialized = cache._deserialize_result(cached_data)
    assert deserialized == "test string"


def test_get_hit(cache: KreuzbergCache[str]) -> None:
    cache.set("test_value", key1="value1", key2="value2")

    result = cache.get(key1="value1", key2="value2")
    assert result == "test_value"


def test_get_miss(cache: KreuzbergCache[str]) -> None:
    result = cache.get(key1="nonexistent")
    assert result is None


def test_set(cache: KreuzbergCache[str]) -> None:
    cache.set("test_value", key1="value1", key2="value2")

    cache_key = cache._get_cache_key(key1="value1", key2="value2")
    cache_path = cache._get_cache_path(cache_key)
    assert cache_path.exists()

    result = cache.get(key1="value1", key2="value2")
    assert result == "test_value"


@pytest.mark.anyio
async def test_aget_hit(cache: KreuzbergCache[str]) -> None:
    await cache.aset("test_value", key1="value1", key2="value2")

    result = await cache.aget(key1="value1", key2="value2")
    assert result == "test_value"


@pytest.mark.anyio
async def test_aget_miss(cache: KreuzbergCache[str]) -> None:
    result = await cache.aget(key1="nonexistent")
    assert result is None


@pytest.mark.anyio
async def test_aset(cache: KreuzbergCache[str]) -> None:
    await cache.aset("test_value", key1="value1", key2="value2")

    cache_key = cache._get_cache_key(key1="value1", key2="value2")
    cache_path = cache._get_cache_path(cache_key)
    assert cache_path.exists()

    result = await cache.aget(key1="value1", key2="value2")
    assert result == "test_value"


def test_clear(cache: KreuzbergCache[str]) -> None:
    cache.set("value1", key="test1")
    cache.set("value2", key="test2")

    assert cache.get(key="test1") == "value1"
    assert cache.get(key="test2") == "value2"

    cache.clear()

    assert cache.get(key="test1") is None
    assert cache.get(key="test2") is None


def test_cleanup_cache(cache: KreuzbergCache[str]) -> None:
    cache_path = cache.cache_dir / "expired.msgpack"
    cache_path.write_text("expired content")

    old_time = time.time() - (cache.max_age_days + 1) * 24 * 3600
    os.utime(cache_path, (old_time, old_time))

    cache.set("fresh_value", key="fresh")

    cache._cleanup_cache()

    assert not cache_path.exists()
    assert cache.get(key="fresh") == "fresh_value"


def test_cleanup_cache_size_limit(cache: KreuzbergCache[str]) -> None:
    for i in range(20):
        cache.set(f"value_{i}" * 1000, key=f"test_{i}")

    initial_files = list(cache.cache_dir.glob("*.msgpack"))
    initial_count = len(initial_files)

    cache._cleanup_cache()

    remaining_files = list(cache.cache_dir.glob("*.msgpack"))
    remaining_count = len(remaining_files)

    assert remaining_count <= initial_count
    assert remaining_count <= 20


def test_cleanup_cache_exception_handling(cache: KreuzbergCache[str]) -> None:
    with patch("pathlib.Path.glob", side_effect=OSError("Permission denied")):
        cache._cleanup_cache()


def test_get_serialization_error(cache: KreuzbergCache[str]) -> None:
    cache_key = cache._get_cache_key(key="test")
    cache_path = cache._get_cache_path(cache_key)
    cache_path.write_bytes(b"corrupted msgpack data")

    result = cache.get(key="test")
    assert result is None

    assert not cache_path.exists()


def test_set_serialization_error(cache: KreuzbergCache[str]) -> None:
    unserializable = lambda x: x  # noqa: E731

    with patch("kreuzberg._utils._cache.serialize", side_effect=TypeError("Serialize error")):
        cache.set(unserializable, key="test")  # type: ignore


def test_is_processing(cache: KreuzbergCache[str]) -> None:
    assert not cache.is_processing(key="test")

    event = cache.mark_processing(key="test")
    assert cache.is_processing(key="test")
    assert not event.is_set()

    cache.mark_complete(key="test")
    assert not cache.is_processing(key="test")
    assert event.is_set()


def test_mark_processing_duplicate(cache: KreuzbergCache[str]) -> None:
    event1 = cache.mark_processing(key="test")
    event2 = cache.mark_processing(key="test")

    assert event1 is event2


def test_mark_complete_nonexistent(cache: KreuzbergCache[str]) -> None:
    cache.mark_complete(key="nonexistent")


def test_get_stats(cache: KreuzbergCache[str]) -> None:
    cache.set("value1", key="test1")
    cache.set("value2", key="test2")

    stats = cache.get_stats()

    assert stats["cache_type"] == "test"
    assert stats["cached_results"] == 2
    assert stats["processing_results"] == 0
    assert stats["total_cache_size_mb"] > 0
    assert stats["avg_result_size_kb"] > 0
    assert str(cache.cache_dir) in stats["cache_dir"]
    assert stats["max_cache_size_mb"] == 10.0
    assert stats["max_age_days"] == 1


def test_get_stats_os_error(cache: KreuzbergCache[str]) -> None:
    with patch("pathlib.Path.glob", side_effect=OSError("Permission denied")):
        stats = cache.get_stats()

        assert stats["cache_type"] == "test"
        assert stats["cached_results"] == 0
        assert stats["total_cache_size_mb"] == 0.0
        assert stats["avg_result_size_kb"] == 0.0


def test_get_ocr_cache() -> None:
    _ocr_cache_ref.clear()

    cache = get_ocr_cache()
    assert isinstance(cache, KreuzbergCache)
    assert cache.cache_type == "ocr"

    cache2 = get_ocr_cache()
    assert cache is cache2


def test_get_ocr_cache_with_env_vars() -> None:
    _ocr_cache_ref.clear()

    with patch.dict(
        os.environ,
        {
            "KREUZBERG_CACHE_DIR": "/tmp/test_cache",
            "KREUZBERG_OCR_CACHE_SIZE_MB": "100",
            "KREUZBERG_OCR_CACHE_AGE_DAYS": "7",
        },
    ):
        cache = get_ocr_cache()
        assert cache.max_cache_size_mb == 100.0
        assert cache.max_age_days == 7


def test_get_document_cache() -> None:
    _document_cache_ref.clear()
    cache = get_document_cache()
    assert isinstance(cache, KreuzbergCache)
    assert cache.cache_type == "documents"


def test_get_table_cache() -> None:
    _table_cache_ref.clear()
    cache = get_table_cache()
    assert isinstance(cache, KreuzbergCache)
    assert cache.cache_type == "tables"


def test_get_mime_cache() -> None:
    _mime_cache_ref.clear()
    cache = get_mime_cache()
    assert isinstance(cache, KreuzbergCache)
    assert cache.cache_type == "mime"


def test_clear_all_caches() -> None:
    _ocr_cache_ref.clear()
    _document_cache_ref.clear()
    _table_cache_ref.clear()
    _mime_cache_ref.clear()

    get_ocr_cache().set(
        ExtractionResult(content="test", mime_type="text/plain", metadata={}, chunks=[], tables=[]), key="test"
    )
    get_mime_cache().set("application/pdf", key="test")

    clear_all_caches()

    assert not _ocr_cache_ref.is_initialized()
    assert not _document_cache_ref.is_initialized()
    assert not _table_cache_ref.is_initialized()
    assert not _mime_cache_ref.is_initialized()


def test_cleanup_cache_periodic_trigger(cache: KreuzbergCache[str]) -> None:
    with patch.object(cache, "_cleanup_cache") as mock_cleanup:
        for i in range(200):
            cache_key = cache._get_cache_key(test_key=f"test_{i}")
            if hash(cache_key) % 100 == 0:
                cache.set(f"value_{i}", test_key=f"test_{i}")
                mock_cleanup.assert_called()
                break
        else:
            with patch("builtins.hash", return_value=0):
                cache.set("value", test_key="trigger")
                mock_cleanup.assert_called()


@pytest.mark.anyio
async def test_async_cleanup_cache_periodic_trigger(cache: KreuzbergCache[str]) -> None:
    with patch.object(cache, "_cleanup_cache") as mock_cleanup:
        for i in range(200):
            cache_key = cache._get_cache_key(test_key=f"test_{i}")
            if hash(cache_key) % 100 == 0:
                await cache.aset(f"value_{i}", test_key=f"test_{i}")
                mock_cleanup.assert_called()
                break
        else:
            with patch("builtins.hash", return_value=0):
                await cache.aset("value", test_key="trigger")
                mock_cleanup.assert_called()


@pytest.mark.anyio
async def test_aget_serialization_error(cache: KreuzbergCache[str]) -> None:
    cache_key = cache._get_cache_key(key="test")
    cache_path = cache._get_cache_path(cache_key)
    cache_path.write_bytes(b"corrupted msgpack data")

    result = await cache.aget(key="test")
    assert result is None


@pytest.mark.anyio
async def test_aset_serialization_error(cache: KreuzbergCache[str]) -> None:
    unserializable = lambda x: x  # noqa: E731

    with patch("kreuzberg._utils._cache.serialize", side_effect=TypeError("Serialize error")):
        await cache.aset(unserializable, key="test")  # type: ignore
