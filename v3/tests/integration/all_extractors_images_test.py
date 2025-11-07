from __future__ import annotations

from typing import TYPE_CHECKING, Any
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from kreuzberg import ExtractionConfig
from kreuzberg._extractors._email import EmailExtractor
from kreuzberg._extractors._html import HTMLExtractor
from kreuzberg._extractors._pandoc import PandocExtractor
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg._types import ExtractedImage

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.anyio
class TestAllExtractorsImageIntegration:
    async def test_html_extractor_with_base64_images(self) -> None:
        html_content = """
        <html>
        <body>
            <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==" alt="Red dot">
            <img src="data:image/jpeg;base64,/9j/4AAQSkZJRgABAQEAYABgAAD/2wBDAP/bAEMAAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB/8AAEQgAAQABAwEiAAIRAQMRAf/EABQAAQAAAAAAAAAAAAAAAAAAAAD/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/8QAFQEBAQAAAAAAAAAAAAAAAAAAAAX/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIRAxEAPwCwAA//2Q==" alt="Test">
            <svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg>
            <img src="https://example.com/external.jpg" alt="External">
        </body>
        </html>
        """

        config = ExtractionConfig(extract_images=True)
        extractor = HTMLExtractor(mime_type="text/html", config=config)

        result = await extractor.extract_bytes_async(html_content.encode())

        assert len(result.images) == 3
        formats = {img.format for img in result.images}
        assert "png" in formats
        assert "jpeg" in formats
        assert "svg" in formats

        png_img = next(img for img in result.images if img.format == "png")
        assert png_img.description == "Red dot"

    def test_html_extractor_sync_with_ocr(self) -> None:
        html_content = """
        <html>
        <body>
            <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==">
        </body>
        </html>
        """

        config = ExtractionConfig(
            extract_images=True, ocr_extracted_images=True, ocr_backend="tesseract", image_ocr_min_dimensions=(1, 1)
        )
        extractor = HTMLExtractor(mime_type="text/html", config=config)

        with patch.object(extractor, "_process_images_with_ocr") as mock_process_ocr:
            from kreuzberg._types import ExtractionResult, ImageOCRResult

            def mock_ocr_processing(images: list[ExtractedImage]) -> list[ImageOCRResult]:
                if images:
                    return [
                        ImageOCRResult(
                            image=images[0],
                            ocr_result=ExtractionResult(content="OCR text", mime_type="text/plain", metadata={}),
                            confidence_score=0.95,
                            processing_time=0.1,
                        )
                    ]
                return []

            mock_process_ocr.side_effect = mock_ocr_processing

            result = extractor.extract_bytes_sync(html_content.encode())

        assert len(result.images) == 1
        assert len(result.image_ocr_results) == 1
        assert result.image_ocr_results[0].ocr_result.content == "OCR text"

    async def test_presentation_extractor_with_images(self) -> None:
        config = ExtractionConfig(extract_images=True)
        extractor = PresentationExtractor(
            mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=config
        )

        mock_image = MagicMock()
        mock_image.blob = b"fake_image_data"
        mock_image.ext = "png"

        mock_shape = MagicMock()
        mock_shape.shape_type = 13
        mock_shape.image = mock_image

        mock_slide = MagicMock()
        mock_slide.shapes = [mock_shape]

        mock_presentation = MagicMock()
        mock_presentation.slides = [mock_slide, mock_slide]

        with patch("pptx.Presentation") as mock_pptx:
            mock_pptx.return_value = mock_presentation

            result = await extractor.extract_bytes_async(b"fake_pptx_data")

        assert len(result.images) == 2
        assert all(img.format == "png" for img in result.images)
        assert result.images[0].page_number == 1
        assert result.images[1].page_number == 2

    async def test_pandoc_extractor_with_images(self, tmp_path: Path) -> None:
        config = ExtractionConfig(extract_images=True)
        extractor = PandocExtractor(
            mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document", config=config
        )

        docx_path = tmp_path / "test.docx"
        docx_path.write_bytes(b"fake_docx_content")

        with patch("kreuzberg._extractors._pandoc.run_process") as mock_run_process:

            async def mock_process_call(command: list[str], **kwargs: Any) -> Any:
                mock_result = MagicMock()
                if command[0] == "pandoc" and "--version" in command:
                    mock_result.stdout = b"pandoc 2.19\n"
                    mock_result.returncode = 0
                else:
                    mock_result.stdout = b"mocked output\n"
                    mock_result.stderr = b""
                    mock_result.returncode = 0
                return mock_result

            mock_run_process.side_effect = mock_process_call

            with patch("kreuzberg._extractors._pandoc.AsyncPath") as mock_async_path:

                async def mock_read_text(encoding: str | None = None) -> str:
                    path_str = str(mock_async_path.call_args[0][0])
                    if path_str.endswith(".json"):
                        return '{"title": "Test Document", "author": "Test Author"}'
                    return "Document content"

                mock_path_instance = MagicMock()
                mock_path_instance.read_text = mock_read_text
                mock_path_instance.read_bytes = AsyncMock(return_value=b"image_data")
                mock_async_path.return_value = mock_path_instance

                with patch("pathlib.Path.rglob") as mock_rglob:
                    mock_img_path = MagicMock()
                    mock_img_path.is_file.return_value = True
                    mock_img_path.suffix = ".png"
                    mock_img_path.name = "image1.png"
                    mock_rglob.return_value = [mock_img_path]

                    with patch("pathlib.Path.exists") as mock_exists:
                        mock_exists.return_value = True

                        result = await extractor.extract_path_async(docx_path)

        assert len(result.images) == 1
        assert result.images[0].format == "png"
        assert result.images[0].filename == "image1.png"

    async def test_email_extractor_with_attached_images(self) -> None:
        config = ExtractionConfig(extract_images=True)
        extractor = EmailExtractor(mime_type="message/rfc822", config=config)

        mock_attachment = {
            "content_type": "image/jpeg",
            "filename": "photo.jpg",
            "content": b"fake_jpeg_data",
        }

        mock_email = {
            "headers": {"from": "test@example.com", "subject": "Test"},
            "body": {"plain": "Email body"},
            "attachments": [mock_attachment],
        }

        with patch("mailparse.EmailDecode.load") as mock_load:
            mock_load.return_value = mock_email

            result = await extractor.extract_bytes_async(b"fake_email_data")

        assert len(result.images) == 1
        assert result.images[0].format == "jpg"
        assert result.images[0].filename == "photo.jpg"
        assert result.images[0].data == b"fake_jpeg_data"

    async def test_pdf_extractor_complete_pipeline(self, searchable_pdf: Path) -> None:
        pdf_path = searchable_pdf

        config = ExtractionConfig(
            extract_images=True,
            ocr_extracted_images=True,
            ocr_backend="tesseract",
        )
        extractor = PDFExtractor(mime_type="application/pdf", config=config)

        from kreuzberg._types import ExtractionResult, ImageOCRResult

        mock_img = ExtractedImage(
            data=b"image_data",
            format="jpg",
            filename="test.jpg",
            page_number=1,
            dimensions=(200, 300),
        )

        with patch.object(extractor, "_parse_with_password_attempts"):
            with patch.object(extractor, "_extract_images_from_playa") as mock_extract:
                mock_extract.return_value = [mock_img]

                with patch.object(extractor, "_process_images_with_ocr") as mock_ocr:
                    mock_ocr_result = ImageOCRResult(
                        image=mock_img,
                        ocr_result=ExtractionResult(content="Text in image", mime_type="text/plain", metadata={}),
                        confidence_score=0.88,
                        processing_time=0.5,
                    )
                    mock_ocr.return_value = [mock_ocr_result]

                    result = await extractor.extract_path_async(pdf_path)

        assert len(result.images) == 1
        assert result.images[0].format == "jpg"
        assert len(result.image_ocr_results) == 1
        assert result.image_ocr_results[0].confidence_score == 0.88
        assert result.image_ocr_results[0].ocr_result.content == "Text in image"

    def test_all_extractors_sync_methods_exist(self) -> None:
        extractors = [
            PDFExtractor(mime_type="application/pdf", config=ExtractionConfig()),
            HTMLExtractor(mime_type="text/html", config=ExtractionConfig()),
            PresentationExtractor(
                mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation",
                config=ExtractionConfig(),
            ),
            EmailExtractor(mime_type="message/rfc822", config=ExtractionConfig()),
            PandocExtractor(
                mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                config=ExtractionConfig(),
            ),
        ]

        for extractor in extractors:
            assert hasattr(extractor, "extract_bytes_sync")
            assert hasattr(extractor, "extract_path_sync")
            assert callable(extractor.extract_bytes_sync)
            assert callable(extractor.extract_path_sync)
