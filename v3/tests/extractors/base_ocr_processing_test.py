from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock, patch

import pytest
from kreuzberg import ExtractionConfig
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractedImage, ExtractionResult


@pytest.mark.anyio
class TestImageOCRProcessing:
    async def test_process_images_with_ocr_disabled(self) -> None:
        config = ExtractionConfig(extract_images=True, ocr_extracted_images=False)
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [
            ExtractedImage(data=b"test", format="png"),
            ExtractedImage(data=b"test2", format="jpg"),
        ]

        results = await extractor._process_images_with_ocr(images)
        assert results == []

    async def test_process_images_with_ocr_empty_list(self) -> None:
        config = ExtractionConfig(extract_images=True, ocr_extracted_images=True)
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        results = await extractor._process_images_with_ocr([])
        assert results == []

    async def test_process_images_with_ocr_format_filtering(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
            image_ocr_formats=frozenset({"png", "jpg"}),
        )
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [
            ExtractedImage(data=b"png_data", format="png", filename="test.png"),
            ExtractedImage(data=b"svg_data", format="svg", filename="test.svg"),
            ExtractedImage(data=b"jpg_data", format="jpg", filename="test.jpg"),
            ExtractedImage(data=b"bmp_data", format="bmp", filename="test.bmp"),
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

        bmp_result = next(r for r in results if r.image.filename == "test.bmp")
        assert bmp_result.skipped_reason
        assert "Unsupported format" in bmp_result.skipped_reason

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

    async def test_process_images_with_ocr_size_filtering(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
            image_ocr_min_dimensions=(100, 100),
            image_ocr_max_dimensions=(1000, 1000),
        )
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [
            ExtractedImage(data=b"tiny", format="png", dimensions=(50, 50), filename="tiny.png"),
            ExtractedImage(data=b"ok", format="png", dimensions=(500, 500), filename="ok.png"),
            ExtractedImage(data=b"huge", format="png", dimensions=(2000, 2000), filename="huge.png"),
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

    async def test_process_images_with_ocr_memory_limits_applied(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
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

    async def test_process_images_with_ocr_parallel_processing(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
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

    async def test_process_images_with_ocr_error_handling(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
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

    async def test_process_images_with_different_backends(self) -> None:
        for backend_name in ["tesseract", "easyocr", "paddleocr"]:
            config = ExtractionConfig(
                extract_images=True,
                ocr_extracted_images=True,
                ocr_backend=backend_name,  # type: ignore[arg-type]
                image_ocr_backend=backend_name,  # type: ignore[arg-type]
            )
            extractor = PDFExtractor(mime_type="application/pdf", config=config)

            images = [ExtractedImage(data=b"test", format="png")]

            from kreuzberg._ocr import get_ocr_backend

            get_ocr_backend.cache_clear()

            def make_mock_process(name: str) -> Any:
                async def mock_process(*args: Any, **kwargs: Any) -> ExtractionResult:
                    return ExtractionResult(content=f"{name} OCR", mime_type="text/plain", metadata={})

                return mock_process

            with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
                mock_backend = MagicMock()
                mock_backend.process_image = make_mock_process(backend_name)
                mock_get_backend.return_value = mock_backend

                with patch("PIL.Image.open"):
                    results = await extractor._process_images_with_ocr(images)

                mock_get_backend.assert_called_once_with(backend_name)
                assert results[0].ocr_result.content == f"{backend_name} OCR"
