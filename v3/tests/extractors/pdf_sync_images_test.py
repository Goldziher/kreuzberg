from __future__ import annotations

import asyncio
from types import SimpleNamespace
from typing import TYPE_CHECKING, Any
from unittest.mock import patch

import kreuzberg._extractors._pdf as pdf_module
from kreuzberg import ExtractionConfig
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractionResult, ImageOCRResult

if TYPE_CHECKING:
    import io
    from pathlib import Path

    import pytest


class MockImageObj:
    def __init__(self, size: tuple[int, int] = (100, 200)) -> None:
        self.srcsize = size
        self.bits = 8
        self.imagemask = False
        self.colorspace = SimpleNamespace(name="RGB")
        self.stream = b"dummy"


class MockPage:
    def __init__(self, images: list[Any]) -> None:
        self.images = images


class MockDocument:
    def __init__(self, pages: list[Any]) -> None:
        self.pages = pages


class TestPDFSyncImageExtraction:
    def test_extract_images_from_playa_sync_single_image(self, monkeypatch: pytest.MonkeyPatch) -> None:
        extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

        mock_img = MockImageObj()
        mock_doc = MockDocument([MockPage([mock_img])])

        def writer(buffer: io.BytesIO) -> None:
            buffer.write(b"fake_jpg_data")

        def get_suffix_and_writer(stream: bytes) -> tuple[str, Any]:
            return ".jpg", writer

        monkeypatch.setattr("kreuzberg._extractors._pdf.get_image_suffix_and_writer", get_suffix_and_writer)

        images = extractor._extract_images_from_playa_sync(mock_doc)  # type: ignore[arg-type]

        assert len(images) == 1
        assert images[0].format == "jpg"
        assert images[0].page_number == 1
        assert images[0].dimensions == (100, 200)
        assert images[0].data == b"fake_jpg_data"

    def test_extract_images_from_playa_sync_multiple_pages(self, monkeypatch: pytest.MonkeyPatch) -> None:
        extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

        mock_pages = [
            MockPage([MockImageObj((100, 100)), MockImageObj((200, 200))]),
            MockPage([MockImageObj((300, 300))]),
            MockPage([]),
            MockPage([MockImageObj((400, 400))]),
        ]
        mock_doc = MockDocument(mock_pages)

        counter = 0

        def writer(buffer: io.BytesIO) -> None:
            nonlocal counter
            counter += 1
            buffer.write(f"image_{counter}".encode())

        def get_suffix_and_writer(stream: bytes) -> tuple[str, Any]:
            return ".png", writer

        monkeypatch.setattr("kreuzberg._extractors._pdf.get_image_suffix_and_writer", get_suffix_and_writer)

        images = extractor._extract_images_from_playa_sync(mock_doc)  # type: ignore[arg-type]

        assert len(images) == 4
        assert images[0].page_number == 1
        assert images[1].page_number == 1
        assert images[2].page_number == 2
        assert images[3].page_number == 4

    def test_extract_images_from_playa_sync_error_handling(self, monkeypatch: pytest.MonkeyPatch, caplog: Any) -> None:
        extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

        mock_img_good = MockImageObj()
        mock_img_bad = MockImageObj()
        mock_doc = MockDocument([MockPage([mock_img_good, mock_img_bad, mock_img_good])])

        call_count = 0

        def get_suffix_and_writer(stream: bytes) -> tuple[str, Any]:
            nonlocal call_count
            call_count += 1
            if call_count == 2:
                raise ValueError("Failed to process image")

            def writer(buffer: io.BytesIO) -> None:
                buffer.write(b"good_image")

            return ".jpg", writer

        monkeypatch.setattr("kreuzberg._extractors._pdf.get_image_suffix_and_writer", get_suffix_and_writer)

        images = extractor._extract_images_from_playa_sync(mock_doc)  # type: ignore[arg-type]

        assert len(images) == 2
        assert all(img.data == b"good_image" for img in images)
        assert "Failed to extract image" in caplog.text

    def test_extract_path_sync_with_images(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        pdf_path = tmp_path / "test.pdf"
        pdf_path.write_bytes(b"%PDF-1.4\n%%EOF")

        config = ExtractionConfig(extract_images=True)
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        mock_doc = MockDocument([MockPage([MockImageObj()])])

        def mock_parse(content: bytes) -> Any:
            return mock_doc

        monkeypatch.setattr(extractor, "_parse_with_password_attempts", mock_parse)

        monkeypatch.setattr(extractor, "_extract_pdf_searchable_text_sync", lambda _p: "Sample text")

        def writer(buffer: io.BytesIO) -> None:
            buffer.write(b"image_data")

        monkeypatch.setattr(
            pdf_module,
            "get_image_suffix_and_writer",
            lambda _s: (".jpg", writer),
        )

        result = extractor.extract_path_sync(pdf_path)

        assert result.content == "Sample text"
        assert len(result.images) == 1
        assert result.images[0].format == "jpg"

    def test_extract_path_sync_with_memory_limits(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        pdf_path = tmp_path / "test.pdf"
        pdf_path.write_bytes(b"%PDF-1.4\n%%EOF")

        config = ExtractionConfig(extract_images=True)
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        large_images = [MockImageObj() for _ in range(10)]
        mock_doc = MockDocument([MockPage(large_images)])

        monkeypatch.setattr(extractor, "_parse_with_password_attempts", lambda _c: mock_doc)
        monkeypatch.setattr(extractor, "_extract_pdf_searchable_text_sync", lambda _p: "Text")

        def writer(buffer: io.BytesIO) -> None:
            buffer.write(b"x" * (15 * 1024 * 1024))

        monkeypatch.setattr(
            pdf_module,
            "get_image_suffix_and_writer",
            lambda _s: (".jpg", writer),
        )

        result = extractor.extract_path_sync(pdf_path)

        assert len(result.images) < 10
        total_size = sum(len(img.data) for img in result.images)
        assert total_size <= 100 * 1024 * 1024

    def test_extract_path_sync_with_ocr(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        pdf_path = tmp_path / "test.pdf"
        pdf_path.write_bytes(b"%PDF-1.4\n%%EOF")

        config = ExtractionConfig(extract_images=True, ocr_extracted_images=True, ocr_backend="tesseract")
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        mock_doc = MockDocument([MockPage([MockImageObj()])])
        monkeypatch.setattr(extractor, "_parse_with_password_attempts", lambda _c: mock_doc)
        monkeypatch.setattr(extractor, "_extract_pdf_searchable_text_sync", lambda _p: "Text")

        def writer(buffer: io.BytesIO) -> None:
            buffer.write(b"image_data")

        monkeypatch.setattr(
            "kreuzberg._extractors._pdf.get_image_suffix_and_writer",
            lambda _s: (".jpg", writer),
        )

        async def mock_process_images_with_ocr(images: Any) -> Any:
            return [
                ImageOCRResult(
                    image=images[0],
                    ocr_result=ExtractionResult(content="OCR text", mime_type="text/plain", metadata={}),
                    confidence_score=0.95,
                )
            ]

        def mock_run_sync(async_func: Any, *args: Any) -> Any:
            return asyncio.run(async_func(*args))

        with patch("kreuzberg._utils._sync.run_sync", mock_run_sync):
            with patch.object(extractor, "_process_images_with_ocr", mock_process_images_with_ocr):
                result = extractor.extract_path_sync(pdf_path)

        assert len(result.images) == 1
        assert len(result.image_ocr_results) == 1
        assert result.image_ocr_results[0].confidence_score == 0.95
