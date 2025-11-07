from __future__ import annotations

import tempfile
import threading
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._ocr_cache import (
    build_cache_kwargs,
    cache_and_complete_async,
    cache_and_complete_sync,
    generate_image_hash,
    get_file_info,
    handle_cache_lookup_async,
    handle_cache_lookup_sync,
    mark_processing_complete,
)
from PIL import Image

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture
def sample_image() -> Image.Image:
    return Image.new("RGB", (100, 100), color="red")


@pytest.fixture
def sample_extraction_result() -> ExtractionResult:
    return ExtractionResult(
        content="Sample OCR text",
        mime_type="text/plain",
        metadata={},
        chunks=[],
    )


@pytest.fixture
def temp_file() -> Generator[Path, None, None]:
    with tempfile.NamedTemporaryFile(delete=False, suffix=".txt") as tmp:
        tmp.write(b"test content")
        tmp_path = Path(tmp.name)

    yield tmp_path

    tmp_path.unlink(missing_ok=True)


class TestGetFileInfo:
    def test_get_file_info_existing_file(self, temp_file: Path) -> None:
        file_info = get_file_info(temp_file)

        assert file_info["path"] == str(temp_file.resolve())
        assert file_info["size"] > 0
        assert file_info["mtime"] > 0

    def test_get_file_info_nonexistent_file(self) -> None:
        nonexistent = Path("/nonexistent/file.txt")
        file_info = get_file_info(nonexistent)

        assert file_info["path"] == str(nonexistent)
        assert file_info["size"] == 0
        assert file_info["mtime"] == 0


class TestGenerateImageHash:
    def test_generate_image_hash_consistent(self, sample_image: Image.Image) -> None:
        hash1 = generate_image_hash(sample_image)
        hash2 = generate_image_hash(sample_image)

        assert hash1 == hash2
        assert len(hash1) == 16
        assert isinstance(hash1, str)

    def test_generate_image_hash_different_images(self) -> None:
        image1 = Image.new("RGB", (100, 100), color="red")
        image2 = Image.new("RGB", (100, 100), color="blue")

        hash1 = generate_image_hash(image1)
        hash2 = generate_image_hash(image2)

        assert hash1 != hash2

    def test_generate_image_hash_mode_conversion(self) -> None:
        image_cmyk = Image.new("CMYK", (100, 100), color=(255, 0, 0, 0))

        hash_cmyk = generate_image_hash(image_cmyk)

        image_rgb = image_cmyk.convert("RGB")
        hash_rgb = generate_image_hash(image_rgb)

        assert hash_cmyk == hash_rgb


class TestBuildCacheKwargs:
    def test_build_cache_kwargs_basic(self) -> None:
        config = {"language": "eng", "confidence": 0.8}
        kwargs = build_cache_kwargs("tesseract", config)

        expected = {
            "ocr_backend": "tesseract",
            "ocr_config": str(sorted(config.items())),
        }

        assert kwargs == expected

    def test_build_cache_kwargs_with_image_hash(self) -> None:
        config = {"language": "eng"}
        image_hash = "abc123def456"

        kwargs = build_cache_kwargs("easyocr", config, image_hash=image_hash)

        assert kwargs["ocr_backend"] == "easyocr"
        assert kwargs["image_hash"] == image_hash
        assert "ocr_config" in kwargs

    def test_build_cache_kwargs_with_file_info(self, temp_file: Path) -> None:
        config = {"language": "eng"}
        file_info = get_file_info(temp_file)

        kwargs = build_cache_kwargs("paddleocr", config, file_info=file_info)

        assert kwargs["ocr_backend"] == "paddleocr"
        assert kwargs["file_info"] == str(sorted(file_info.items()))
        assert "ocr_config" in kwargs


class TestHandleCacheLookupAsync:
    @pytest.mark.anyio
    async def test_cache_hit(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.aget = AsyncMock(return_value=sample_extraction_result)
            mock_get_cache.return_value = mock_cache

            result = await handle_cache_lookup_async(cache_kwargs)

            assert result == sample_extraction_result
            mock_cache.aget.assert_called_once_with(**cache_kwargs)

    @pytest.mark.anyio
    async def test_cache_miss_no_processing(self) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.aget = AsyncMock(return_value=None)
            mock_cache.is_processing.return_value = False
            mock_cache.mark_processing.return_value = MagicMock()
            mock_get_cache.return_value = mock_cache

            result = await handle_cache_lookup_async(cache_kwargs)

            assert result is None
            mock_cache.mark_processing.assert_called_once_with(**cache_kwargs)

    @pytest.mark.anyio
    async def test_cache_miss_with_processing(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.aget = AsyncMock()
            mock_cache.aget.side_effect = [None, sample_extraction_result]
            mock_cache.is_processing.return_value = True

            # anyio.to_thread.run_sync is used legitimately to wait for cache processing completion ~keep
            real_event = threading.Event()
            real_event.set()
            mock_cache.mark_processing.return_value = real_event
            mock_get_cache.return_value = mock_cache

            result = await handle_cache_lookup_async(cache_kwargs)

            assert result == sample_extraction_result


class TestHandleCacheLookupSync:
    def test_cache_hit(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.get.return_value = sample_extraction_result
            mock_get_cache.return_value = mock_cache

            result = handle_cache_lookup_sync(cache_kwargs)

            assert result == sample_extraction_result
            mock_cache.get.assert_called_once_with(**cache_kwargs)

    def test_cache_miss_no_processing(self) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.get.return_value = None
            mock_cache.is_processing.return_value = False
            mock_get_cache.return_value = mock_cache

            result = handle_cache_lookup_sync(cache_kwargs)

            assert result is None
            mock_cache.mark_processing.assert_called_once_with(**cache_kwargs)


class TestCacheAndCompleteAsync:
    @pytest.mark.anyio
    async def test_cache_enabled(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.aset = AsyncMock()
            mock_cache.mark_complete = MagicMock()
            mock_get_cache.return_value = mock_cache

            await cache_and_complete_async(sample_extraction_result, cache_kwargs, use_cache=True)

            mock_cache.aset.assert_called_once_with(sample_extraction_result, **cache_kwargs)
            mock_cache.mark_complete.assert_called_once_with(**cache_kwargs)

    @pytest.mark.anyio
    async def test_cache_disabled(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.aset = AsyncMock()
            mock_cache.mark_complete = MagicMock()
            mock_get_cache.return_value = mock_cache

            await cache_and_complete_async(sample_extraction_result, cache_kwargs, use_cache=False)

            mock_cache.aset.assert_not_called()
            mock_cache.mark_complete.assert_called_once_with(**cache_kwargs)


class TestCacheAndCompleteSync:
    def test_cache_enabled(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.set = MagicMock()
            mock_cache.mark_complete = MagicMock()
            mock_get_cache.return_value = mock_cache

            cache_and_complete_sync(sample_extraction_result, cache_kwargs, use_cache=True)

            mock_cache.set.assert_called_once_with(sample_extraction_result, **cache_kwargs)
            mock_cache.mark_complete.assert_called_once_with(**cache_kwargs)

    def test_cache_disabled(self, sample_extraction_result: ExtractionResult) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_cache.set = MagicMock()
            mock_cache.mark_complete = MagicMock()
            mock_get_cache.return_value = mock_cache

            cache_and_complete_sync(sample_extraction_result, cache_kwargs, use_cache=False)

            mock_cache.set.assert_not_called()
            mock_cache.mark_complete.assert_called_once_with(**cache_kwargs)


class TestMarkProcessingComplete:
    def test_mark_processing_complete(self) -> None:
        cache_kwargs = {"test": "value"}

        with patch("kreuzberg._utils._ocr_cache.get_ocr_cache") as mock_get_cache:
            mock_cache = MagicMock()
            mock_get_cache.return_value = mock_cache

            mark_processing_complete(cache_kwargs)

            mock_cache.mark_complete.assert_called_once_with(**cache_kwargs)
