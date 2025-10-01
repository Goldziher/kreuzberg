from __future__ import annotations

from typing import TYPE_CHECKING, Any

import pytest

from kreuzberg._extractors._base import Extractor
from kreuzberg._types import (
    ExtractedImage,
    ExtractionConfig,
    ExtractionResult,
    ImageExtractionConfig,
    ImageOCRResult,
    PSMMode,
    TesseractConfig,
)

if TYPE_CHECKING:
    from pathlib import Path


class MockExtractor(Extractor):
    SUPPORTED_MIME_TYPES = {"test/mock"}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="test content", mime_type="text/plain", metadata={})

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="test content", mime_type="text/plain", metadata={})

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="test content", mime_type="text/plain", metadata={})

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="test content", mime_type="text/plain", metadata={})


def test_supports_mimetype_prefix_matching() -> None:
    class TestExtractor(Extractor):
        SUPPORTED_MIME_TYPES = {"application/vnd."}

        async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={})

        async def extract_path_async(self, path: Path) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={})

        def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={})

        def extract_path_sync(self, path: Path) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={})

    assert TestExtractor.supports_mimetype("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    assert TestExtractor.supports_mimetype("application/vnd.ms-excel")
    assert not TestExtractor.supports_mimetype("text/plain")


def test_apply_quality_processing_enabled_with_content() -> None:
    config = ExtractionConfig(enable_quality_processing=True)
    extractor = MockExtractor(mime_type="test/mock", config=config)

    test_image = ExtractedImage(data=b"test_image_data", format="png", filename="test.png")

    original_result = ExtractionResult(
        content="Test with   excessive    whitespace and a   b   c scattered chars",
        mime_type="text/plain",
        metadata={},
        images=[test_image],
    )

    processed_result = extractor._apply_quality_processing(original_result)

    assert processed_result.content != original_result.content
    assert "quality_score" in processed_result.metadata
    assert processed_result.mime_type == original_result.mime_type
    assert len(processed_result.images) <= len(original_result.images)


def test_apply_quality_processing_empty_content() -> None:
    config = ExtractionConfig(enable_quality_processing=True)
    extractor = MockExtractor(mime_type="test/mock", config=config)

    original_result = ExtractionResult(content="", mime_type="text/plain", metadata={})

    processed_result = extractor._apply_quality_processing(original_result)
    assert processed_result is original_result


def test_apply_quality_processing_disabled() -> None:
    config = ExtractionConfig(enable_quality_processing=False)
    extractor = MockExtractor(mime_type="test/mock", config=config)

    original_result = ExtractionResult(content="Some test content", mime_type="text/plain", metadata={})

    processed_result = extractor._apply_quality_processing(original_result)
    assert processed_result is original_result


def test_prepare_ocr_config_with_none() -> None:
    config = ExtractionConfig(ocr=None)
    extractor = MockExtractor(mime_type="test/mock", config=config)

    result_config = extractor._prepare_ocr_config()

    assert result_config == {"use_cache": True}
    assert "use_cache" in result_config


def test_prepare_ocr_config_with_user_config() -> None:
    tesseract_config = TesseractConfig(language="spa", psm=PSMMode.SINGLE_BLOCK)
    config = ExtractionConfig(ocr=tesseract_config)
    extractor = MockExtractor(mime_type="test/mock", config=config)

    result_config = extractor._prepare_ocr_config()

    assert result_config["language"] == "spa"
    assert result_config["psm"] == PSMMode.SINGLE_BLOCK.value
    assert "use_cache" in result_config


def test_prepare_ocr_config_easyocr() -> None:
    from kreuzberg._types import EasyOCRConfig

    config = ExtractionConfig(ocr=EasyOCRConfig())
    extractor = MockExtractor(mime_type="test/mock", config=config)

    result_config = extractor._prepare_ocr_config()

    assert "use_cache" in result_config
    assert isinstance(result_config, dict)
    assert "language" in result_config


def test_prepare_ocr_config_paddleocr() -> None:
    from kreuzberg._types import PaddleOCRConfig

    config = ExtractionConfig(ocr=PaddleOCRConfig())
    extractor = MockExtractor(mime_type="test/mock", config=config)

    result_config = extractor._prepare_ocr_config()

    assert "use_cache" in result_config
    assert isinstance(result_config, dict)
    assert "language" in result_config


def test_deduplicate_images_disabled() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig(deduplicate=False))
    extractor = MockExtractor(mime_type="test/mock", config=config)

    test_images = [
        ExtractedImage(data=b"image1", format="png", filename="test1.png"),
        ExtractedImage(data=b"image2", format="png", filename="test2.png"),
    ]

    result = extractor._deduplicate_images(test_images)
    assert result is test_images


def test_deduplicate_images_with_duplicates(caplog: Any) -> None:
    config = ExtractionConfig(images=ImageExtractionConfig(deduplicate=True))
    extractor = MockExtractor(mime_type="test/mock", config=config)

    test_images = [
        ExtractedImage(data=b"same_data", format="png", filename="test1.png"),
        ExtractedImage(data=b"same_data", format="png", filename="test2.png"),
        ExtractedImage(data=b"different_data", format="png", filename="test3.png"),
    ]

    import logging

    caplog.set_level(logging.DEBUG)
    result = extractor._deduplicate_images(test_images)

    assert len(result) == 2
    assert any("Deduplicated" in record.message for record in caplog.records)


@pytest.mark.anyio
async def test_process_images_with_ocr_no_backend() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)), ocr=None)
    extractor = MockExtractor(mime_type="test/mock", config=config)

    test_images = [ExtractedImage(data=b"image_data", format="png", filename="test.png")]

    results = await extractor._process_images_with_ocr(test_images)
    assert results == []


@pytest.mark.anyio
async def test_process_images_with_ocr_with_tasks() -> None:
    from unittest.mock import AsyncMock, patch

    config = ExtractionConfig(images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)), ocr=TesseractConfig())
    extractor = MockExtractor(mime_type="test/mock", config=config)

    test_image = ExtractedImage(data=b"fake_image_data", format="png", filename="test.png", dimensions=(100, 100))

    mock_backend = AsyncMock()
    mock_result = ExtractionResult(content="OCR text", mime_type="text/plain", metadata={})

    with (
        patch("kreuzberg._extractors._base.get_ocr_backend", return_value=mock_backend),
        patch.object(extractor, "_ocr_single_image", return_value=AsyncMock(return_value=mock_result)) as mock_ocr,
    ):
        results = await extractor._process_images_with_ocr([test_image])

        assert len(results) == 1
        mock_ocr.assert_called_once()


def test_check_image_memory_limits_empty() -> None:
    config = ExtractionConfig()
    extractor = MockExtractor(mime_type="test/mock", config=config)

    result = extractor._check_image_memory_limits([])
    assert result == []


def test_check_image_memory_limits_single_large_image() -> None:
    config = ExtractionConfig()
    extractor = MockExtractor(mime_type="test/mock", config=config)

    large_data = b"x" * (55 * 1024 * 1024)
    large_image = ExtractedImage(data=large_data, format="png", filename="large_image.png")

    result = extractor._check_image_memory_limits([large_image])
    assert result == []


def test_check_image_memory_limits_total_size_exceeded(caplog: Any) -> None:
    import logging

    config = ExtractionConfig()
    extractor = MockExtractor(mime_type="test/mock", config=config)

    image_data = b"x" * (40 * 1024 * 1024)

    images = [
        ExtractedImage(data=image_data, format="png", filename="img1.png"),
        ExtractedImage(data=image_data, format="png", filename="img2.png"),
        ExtractedImage(data=image_data, format="png", filename="img3.png"),
    ]

    caplog.set_level(logging.WARNING)
    result = extractor._check_image_memory_limits(images)

    assert len(result) < len(images)
    assert any("Total image size" in record.message for record in caplog.records)


def test_compute_image_hash_small_image() -> None:
    config = ExtractionConfig()
    extractor = MockExtractor(mime_type="test/mock", config=config)

    small_image = ExtractedImage(data=b"small_image_data", format="png", filename="small.png")

    hash1 = extractor._compute_image_hash(small_image)
    hash2 = extractor._compute_image_hash(small_image)

    assert hash1 == hash2
    assert isinstance(hash1, int)


def test_compute_image_hash_large_image() -> None:
    config = ExtractionConfig()
    extractor = MockExtractor(mime_type="test/mock", config=config)

    large_data = b"x" * 2000
    large_image = ExtractedImage(data=large_data, format="png", filename="large.png")

    hash1 = extractor._compute_image_hash(large_image)
    hash2 = extractor._compute_image_hash(large_image)

    assert hash1 == hash2
    assert isinstance(hash1, int)

    different_image = ExtractedImage(data=b"y" * 2000, format="png", filename="different.png")
    hash3 = extractor._compute_image_hash(different_image)
    assert hash1 != hash3


def test_validate_image_for_ocr_unsupported_format() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig(ocr_allowed_formats=frozenset({"png", "jpg"})))
    extractor = MockExtractor(mime_type="test/mock", config=config)

    unsupported_image = ExtractedImage(data=b"image_data", format="bmp", filename="test.bmp")

    reason = extractor._validate_image_for_ocr(unsupported_image)
    assert reason == "Unsupported format: bmp"


def test_validate_image_for_ocr_too_small() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_allowed_formats=frozenset({"png"}), ocr_min_dimensions=(100, 100))
    )
    extractor = MockExtractor(mime_type="test/mock", config=config)

    small_image = ExtractedImage(data=b"image_data", format="png", filename="small.png", dimensions=(50, 50))

    reason = extractor._validate_image_for_ocr(small_image)
    assert reason == "Too small: 50x50"


def test_validate_image_for_ocr_too_large() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_allowed_formats=frozenset({"png"}), ocr_max_dimensions=(1000, 1000))
    )
    extractor = MockExtractor(mime_type="test/mock", config=config)

    large_image = ExtractedImage(data=b"image_data", format="png", filename="large.png", dimensions=(2000, 2000))

    reason = extractor._validate_image_for_ocr(large_image)
    assert reason == "Too large: 2000x2000"


def test_validate_image_for_ocr_valid() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(
            ocr_allowed_formats=frozenset({"png"}), ocr_min_dimensions=(50, 50), ocr_max_dimensions=(1000, 1000)
        )
    )
    extractor = MockExtractor(mime_type="test/mock", config=config)

    valid_image = ExtractedImage(data=b"image_data", format="png", filename="valid.png", dimensions=(100, 100))

    reason = extractor._validate_image_for_ocr(valid_image)
    assert reason is None


def test_validate_image_for_ocr_no_dimensions() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(
            ocr_allowed_formats=frozenset({"svg", "png"}),
            ocr_min_dimensions=(100, 100),
            ocr_max_dimensions=(1000, 1000),
        )
    )
    extractor = MockExtractor(mime_type="test/mock", config=config)

    no_dimensions_image = ExtractedImage(data=b"<svg>...</svg>", format="svg", filename="vector.svg", dimensions=None)

    reason = extractor._validate_image_for_ocr(no_dimensions_image)
    assert reason is None


@pytest.mark.anyio
async def test_ocr_single_image_success() -> None:
    from unittest.mock import AsyncMock

    config = ExtractionConfig()
    extractor = MockExtractor(mime_type="test/mock", config=config)

    png_data = (
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01"
        b"\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\x00\x00"
        b"\x00\x01\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00IEND\xaeB`\x82"
    )

    test_image = ExtractedImage(data=png_data, format="png", filename="test.png", dimensions=(100, 100))

    mock_backend = AsyncMock()
    mock_ocr_result = ExtractionResult(content="OCR text", mime_type="text/plain", metadata={})
    mock_backend.process_image.return_value = mock_ocr_result

    cfg = {"language": "eng"}

    result = await extractor._ocr_single_image(test_image, mock_backend, cfg)

    assert isinstance(result, ImageOCRResult)
    assert result.image == test_image
    assert result.ocr_result == mock_ocr_result
    assert result.processing_time is not None
    assert result.processing_time >= 0
    assert result.skipped_reason is None


@pytest.mark.anyio
async def test_process_images_with_ocr_disabled() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig())
    extractor = MockExtractor(mime_type="test/mock", config=config)

    test_images = [ExtractedImage(data=b"image_data", format="png", filename="test.png")]

    results = await extractor._process_images_with_ocr(test_images)
    assert results == []


@pytest.mark.anyio
async def test_process_images_with_ocr_skipped_images() -> None:
    from unittest.mock import patch

    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(1, 1), ocr_allowed_formats=frozenset({"png"})),
        ocr=TesseractConfig(),
    )
    extractor = MockExtractor(mime_type="test/mock", config=config)

    unsupported_image = ExtractedImage(data=b"image_data", format="bmp", filename="test.bmp")

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        from unittest.mock import Mock

        mock_backend = Mock()
        mock_get_backend.return_value = mock_backend

        results = await extractor._process_images_with_ocr([unsupported_image])

        assert len(results) == 1
        assert results[0].skipped_reason == "Unsupported format: bmp"
        assert results[0].image == unsupported_image


@pytest.mark.anyio
async def test_process_images_with_ocr_no_tasks() -> None:
    from unittest.mock import patch

    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(1, 1), ocr_allowed_formats=frozenset({"png"})),
        ocr=TesseractConfig(),
    )
    extractor = MockExtractor(mime_type="test/mock", config=config)

    skipped_images = [
        ExtractedImage(data=b"data1", format="bmp", filename="test1.bmp"),
        ExtractedImage(data=b"data2", format="gif", filename="test2.gif"),
    ]

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        from unittest.mock import Mock

        mock_backend = Mock()
        mock_get_backend.return_value = mock_backend

        results = await extractor._process_images_with_ocr(skipped_images)

        assert len(results) == 2
        assert all(r.skipped_reason is not None for r in results)
