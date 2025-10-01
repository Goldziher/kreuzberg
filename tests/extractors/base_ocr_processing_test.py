from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock, patch

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractedImage, ExtractionResult, ImageExtractionConfig, TesseractConfig


@pytest.mark.anyio
async def test_process_images_with_ocr_disabled() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig())
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    images = [
        ExtractedImage(data=b"test", format="png"),
        ExtractedImage(data=b"test2", format="jpg"),
    ]

    results = await extractor._process_images_with_ocr(images)
    assert results == []


@pytest.mark.anyio
async def test_process_images_with_ocr_empty_list() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)), ocr=TesseractConfig())
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    results = await extractor._process_images_with_ocr([])
    assert results == []


@pytest.mark.anyio
async def test_process_images_with_ocr_format_filtering() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)),
        ocr=TesseractConfig(),
    )
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    images = [
        ExtractedImage(data=b"png_data", format="png", filename="test.png"),
        ExtractedImage(data=b"svg_data", format="svg", filename="test.svg"),
        ExtractedImage(data=b"jpg_data", format="jpg", filename="test.jpg"),
        ExtractedImage(data=b"ico_data", format="ico", filename="test.ico"),
    ]

    async def mock_process_image(*args: Any, **kwargs: Any) -> ExtractionResult:
        return ExtractionResult(content="OCR text", mime_type="text/plain", metadata={})

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        mock_backend = MagicMock()
        mock_backend.process_image = mock_process_image
        mock_get_backend.return_value = mock_backend

        with patch("PIL.Image.open") as mock_open:
            mock_open.return_value = MagicMock()

            results = await extractor._process_images_with_ocr(images)

    assert len(results) == 4

    svg_result = next(r for r in results if r.image.filename == "test.svg")
    assert svg_result.skipped_reason
    assert "Unsupported format" in svg_result.skipped_reason

    ico_result = next(r for r in results if r.image.filename == "test.ico")
    assert ico_result.skipped_reason
    assert "Unsupported format" in ico_result.skipped_reason

    png_result = next(r for r in results if r.image.filename == "test.png")
    if png_result.skipped_reason:
        pass
    assert png_result.ocr_result.content == "OCR text"
    assert png_result.skipped_reason is None

    jpg_result = next(r for r in results if r.image.filename == "test.jpg")
    if jpg_result.skipped_reason:
        pass
    assert jpg_result.ocr_result.content == "OCR text"
    assert jpg_result.skipped_reason is None


@pytest.mark.anyio
async def test_process_images_with_ocr_size_filtering() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(50, 50), ocr_max_dimensions=(10000, 10000)),
        ocr=TesseractConfig(),
    )
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    images = [
        ExtractedImage(data=b"tiny", format="png", dimensions=(40, 40), filename="tiny.png"),
        ExtractedImage(data=b"ok", format="png", dimensions=(500, 500), filename="ok.png"),
        ExtractedImage(data=b"huge", format="png", dimensions=(15000, 15000), filename="huge.png"),
        ExtractedImage(data=b"no_dim", format="png", dimensions=None, filename="no_dim.png"),
    ]

    async def mock_process_image(*args: Any, **kwargs: Any) -> ExtractionResult:
        return ExtractionResult(content="OCR text", mime_type="text/plain", metadata={})

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        mock_backend = MagicMock()
        mock_backend.process_image = mock_process_image
        mock_get_backend.return_value = mock_backend

        with patch("PIL.Image.open") as mock_open:
            mock_open.return_value = MagicMock()

            results = await extractor._process_images_with_ocr(images)

    assert len(results) == 4

    tiny_result = next(r for r in results if r.image.filename == "tiny.png")
    assert tiny_result.skipped_reason
    assert "Too small" in tiny_result.skipped_reason

    huge_result = next(r for r in results if r.image.filename == "huge.png")
    assert huge_result.skipped_reason
    assert "Too large" in huge_result.skipped_reason

    ok_result = next(r for r in results if r.image.filename == "ok.png")
    assert ok_result.skipped_reason is None
    assert ok_result.ocr_result.content == "OCR text"

    no_dim_result = next(r for r in results if r.image.filename == "no_dim.png")
    assert no_dim_result.skipped_reason is None
    assert no_dim_result.ocr_result.content == "OCR text"


@pytest.mark.anyio
async def test_process_images_with_ocr_memory_limits_applied() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)),
        ocr=TesseractConfig(),
    )
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    images = [
        ExtractedImage(data=b"x" * (60 * 1024 * 1024), format="png", filename="huge.png"),
        ExtractedImage(data=b"y" * (10 * 1024 * 1024), format="png", filename="small.png"),
    ]

    async def mock_process_image(*args: Any, **kwargs: Any) -> ExtractionResult:
        return ExtractionResult(content="OCR text", mime_type="text/plain", metadata={})

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        mock_backend = MagicMock()
        mock_backend.process_image = mock_process_image
        mock_get_backend.return_value = mock_backend

        with patch("PIL.Image.open") as mock_open:
            mock_open.return_value = MagicMock()

            results = await extractor._process_images_with_ocr(images)

    assert len(results) == 1
    assert results[0].image.filename == "small.png"


@pytest.mark.anyio
async def test_process_images_with_ocr_parallel_processing() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)),
        ocr=TesseractConfig(),
    )
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    images = [ExtractedImage(data=f"img_{i}".encode(), format="png", filename=f"img_{i}.png") for i in range(10)]

    from kreuzberg._ocr import get_ocr_backend

    get_ocr_backend.cache_clear()

    call_count = 0

    async def mock_process_image(img: Any, **kwargs: Any) -> ExtractionResult:
        nonlocal call_count
        call_count += 1
        return ExtractionResult(content=f"OCR {call_count}", mime_type="text/plain", metadata={})

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        mock_backend = MagicMock()
        mock_backend.process_image = mock_process_image
        mock_get_backend.return_value = mock_backend

        with patch("PIL.Image.open") as mock_open:
            mock_open.return_value = MagicMock()

            results = await extractor._process_images_with_ocr(images)

    assert len(results) == 10
    assert call_count == 10
    ocr_contents = {r.ocr_result.content for r in results if r.skipped_reason is None}
    assert len(ocr_contents) == 10


@pytest.mark.anyio
async def test_process_images_with_ocr_error_handling() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)),
        ocr=TesseractConfig(),
    )
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    images = [
        ExtractedImage(data=b"good", format="png", filename="good.png"),
        ExtractedImage(data=b"bad", format="png", filename="bad.png"),
        ExtractedImage(data=b"also_good", format="png", filename="also_good.png"),
    ]

    from kreuzberg._ocr import get_ocr_backend

    get_ocr_backend.cache_clear()

    async def mock_process_image(img: Any, **kwargs: Any) -> ExtractionResult:
        if b"bad" in img.getvalue():
            raise ValueError("OCR processing failed")
        return ExtractionResult(content="OCR success", mime_type="text/plain", metadata={})

    with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
        mock_backend = MagicMock()
        mock_backend.process_image = mock_process_image
        mock_get_backend.return_value = mock_backend

        def mock_pil_open(data: Any) -> MagicMock:
            mock_img = MagicMock()
            mock_img.getvalue.return_value = data.getvalue()
            return mock_img

        with patch("PIL.Image.open", side_effect=mock_pil_open):
            results = await extractor._process_images_with_ocr(images)

    assert len(results) == 3

    good_result = next(r for r in results if r.image.filename == "good.png")
    assert good_result.ocr_result.content == "OCR success"
    assert good_result.skipped_reason is None

    bad_result = next(r for r in results if r.image.filename == "bad.png")
    assert bad_result.ocr_result.content == ""
    assert bad_result.skipped_reason
    assert "OCR failed" in bad_result.skipped_reason


@pytest.mark.anyio
async def test_process_images_with_different_backends() -> None:
    from kreuzberg._types import EasyOCRConfig, PaddleOCRConfig

    backend_configs: list[tuple[str, TesseractConfig | EasyOCRConfig | PaddleOCRConfig]] = [
        ("tesseract", TesseractConfig()),
        ("easyocr", EasyOCRConfig()),
        ("paddleocr", PaddleOCRConfig()),
    ]

    for _backend_name, backend_config in backend_configs:
        config = ExtractionConfig(
            images=ImageExtractionConfig(ocr_min_dimensions=(1, 1)),
            ocr=backend_config,
        )
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [ExtractedImage(data=b"test", format="png")]

        from kreuzberg._ocr import get_ocr_backend

        get_ocr_backend.cache_clear()

        async def mock_process(*args: Any, **kwargs: Any) -> ExtractionResult:
            return ExtractionResult(content="OCR text", mime_type="text/plain", metadata={})

        with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
            mock_backend = MagicMock()
            mock_backend.process_image = mock_process
            mock_get_backend.return_value = mock_backend

            with patch("PIL.Image.open"):
                results = await extractor._process_images_with_ocr(images)

            assert len(results) == 1
            assert results[0].ocr_result.content == "OCR text"
