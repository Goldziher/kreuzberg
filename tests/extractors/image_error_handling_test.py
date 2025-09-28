from __future__ import annotations

from pathlib import Path
from types import SimpleNamespace
from typing import TYPE_CHECKING, Any
from unittest.mock import MagicMock, patch

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._html import HTMLExtractor
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg._types import ExtractedImage
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    import io


class TestImageExtractionErrorHandling:
    def test_html_invalid_base64_handling(self) -> None:
        html_content = """
        <html>
        <body>
            <img src="data:image/png;base64,INVALID_BASE64_@#$%">
            <img src="data:image/jpeg;base64,">
            <img src="data:image/;base64,dGVzdA==">
            <img src="data:;base64,dGVzdA==">
            <img src="not_a_data_url">
        </body>
        </html>
        """

        config = ExtractionConfig(extract_images=True)
        extractor = HTMLExtractor(mime_type="text/html", config=config)

        result = extractor.extract_bytes_sync(html_content.encode())

        assert len(result.images) == 0 or all(img.data for img in result.images)

    def test_html_malformed_svg_handling(self) -> None:
        html_content = """
        <html>
        <body>
            <svg>Incomplete SVG
            <svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg>
            <svg></svg>
        </body>
        </html>
        """

        config = ExtractionConfig(extract_images=True)
        extractor = HTMLExtractor(mime_type="text/html", config=config)

        result = extractor.extract_bytes_sync(html_content.encode())

        assert any(img.format == "svg" for img in result.images)

    def test_pdf_corrupted_image_stream_sync(self, monkeypatch: pytest.MonkeyPatch) -> None:
        extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(extract_images=True))

        class MockImageObj:
            def __init__(self, should_fail: bool = False) -> None:
                self.srcsize = (100, 100)
                self.bits = 8
                self.imagemask = False
                self.colorspace = SimpleNamespace(name="RGB")
                self.stream = b"corrupted" if should_fail else b"good"
                self.should_fail = should_fail

        mock_doc = MagicMock()
        mock_doc.pages = [MagicMock(images=[MockImageObj(False), MockImageObj(True), MockImageObj(False)])]

        def get_suffix_and_writer(stream: bytes) -> tuple[str, Any]:
            if stream == b"corrupted":
                raise ValueError("Corrupted image stream")

            def writer(buf: io.BytesIO) -> None:
                buf.write(b"valid_image")

            return ".jpg", writer

        monkeypatch.setattr("kreuzberg._extractors._pdf.get_image_suffix_and_writer", get_suffix_and_writer)

        images = extractor._extract_images_from_playa_sync(mock_doc)
        assert len(images) == 2
        assert all(img.data == b"valid_image" for img in images)

    def test_presentation_with_real_file(self) -> None:
        config = ExtractionConfig(extract_images=True)
        extractor = PresentationExtractor(
            mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation",
            config=config,
        )

        pptx_path = Path("test_documents/pitch_deck_presentation.pptx")
        if not pptx_path.exists():
            pytest.skip(f"Test file not found: {pptx_path}")

        content = pptx_path.read_bytes()
        result = extractor._extract_from_bytes(content)

        assert len(result.images) > 0
        for img in result.images:
            assert img.data is not None
            assert len(img.data) > 0
            assert img.format in {"png", "jpg", "jpeg", "gif", "bmp", "tiff", "svg", "unknown"}

        with pytest.raises(ParsingError, match="PPTX extraction failed"):
            extractor._extract_from_bytes(b"invalid_pptx_data")

    @pytest.mark.anyio
    async def test_ocr_backend_not_available(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend=None,
        )
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [ExtractedImage(data=b"test", format="png")]

        results = await extractor._process_images_with_ocr(images)

        assert results == []

    @pytest.mark.anyio
    async def test_ocr_processing_exception(self) -> None:
        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
        )
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [
            ExtractedImage(data=b"test1", format="png", filename="test1.png"),
            ExtractedImage(data=b"test2", format="png", filename="test2.png"),
        ]

        with patch("kreuzberg._extractors._base.get_ocr_backend") as mock_get_backend:
            mock_backend = MagicMock()

            async def failing_process(*args: Any, **kwargs: Any) -> None:
                raise RuntimeError("OCR engine crashed")

            mock_backend.process_image = failing_process
            mock_get_backend.return_value = mock_backend

            with patch("PIL.Image.open"):
                results = await extractor._process_images_with_ocr(images)

        assert len(results) == 2
        for result in results:
            assert result.ocr_result.content == ""
            assert result.skipped_reason
            assert "Backend error: RuntimeError: OCR engine crashed" in result.skipped_reason

    def test_empty_image_data(self) -> None:
        config = ExtractionConfig(extract_images=True)
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        images = [
            ExtractedImage(data=b"", format="png", filename="empty.png"),
            ExtractedImage(data=b"valid", format="jpg", filename="valid.jpg"),
        ]

        filtered = extractor._check_image_memory_limits(images)

        assert len(filtered) == 2

    def test_image_with_none_fields(self) -> None:
        img = ExtractedImage(
            data=b"test",
            format="png",
            filename=None,
            page_number=None,
            dimensions=None,
            colorspace=None,
            bits_per_component=None,
            description=None,
        )

        assert hash(img)
        assert img.format == "png"
        assert img.data == b"test"

    def test_concurrent_extraction_errors_sync(self) -> None:
        extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(extract_images=True))

        mock_images = []
        for i in range(20):
            mock_img = MagicMock()
            mock_img.srcsize = (100, 100)
            mock_img.bits = 8
            mock_img.imagemask = False
            mock_img.colorspace = SimpleNamespace(name="RGB")
            mock_img.stream = b"fail" if i % 5 == 0 else b"good"
            mock_images.append(mock_img)

        mock_doc = MagicMock()
        mock_doc.pages = [MagicMock(images=mock_images[:10]), MagicMock(images=mock_images[10:])]

        def get_suffix_and_writer(stream: bytes) -> tuple[str, Any]:
            if stream == b"fail":
                raise ValueError("Failed to process")

            def writer(buf: io.BytesIO) -> None:
                buf.write(b"image_data")

            return ".jpg", writer

        with patch("kreuzberg._extractors._pdf.get_image_suffix_and_writer", get_suffix_and_writer):
            images = extractor._extract_images_from_playa_sync(mock_doc)
        assert len(images) == 16
        assert all(img.data == b"image_data" for img in images)

    def test_extract_images_disabled(self) -> None:
        config = ExtractionConfig(extract_images=False)

        pdf_extractor = PDFExtractor(mime_type="application/pdf", config=config)
        with patch.object(pdf_extractor, "_extract_pdf_searchable_text_sync", return_value="Text content"):
            with patch.object(pdf_extractor, "_extract_images_from_playa_sync") as mock_extract:
                result = pdf_extractor.extract_bytes_sync(b"%PDF-1.4\n%%EOF")
                mock_extract.assert_not_called()
                assert result.images == []

        html_extractor = HTMLExtractor(mime_type="text/html", config=config)
        result = html_extractor.extract_bytes_sync(
            b"<html><body><p>Text content</p><img src='data:image/png;base64,test'/></body></html>"
        )
        assert result.images == []

        ppt_extractor = PresentationExtractor(
            mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=config
        )
        pptx_path = Path("test_documents/pitch_deck_presentation.pptx")
        if pptx_path.exists():
            content = pptx_path.read_bytes()
            result = ppt_extractor.extract_bytes_sync(content)
            assert result.images == []
            assert len(result.content) > 0
