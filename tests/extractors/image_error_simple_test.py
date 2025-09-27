from __future__ import annotations

from unittest.mock import MagicMock, patch

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._html import HTMLExtractor
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg._types import ExtractedImage


def test_html_handles_invalid_base64() -> None:
    config = ExtractionConfig(extract_images=True)
    extractor = HTMLExtractor(mime_type="text/html", config=config)

    html = b'<html><body><p>Test content</p><img src="data:image/png;base64,invalid!@#$"/></body></html>'
    result = extractor.extract_bytes_sync(html)

    assert "Test content" in result.content
    assert len(result.images) == 0


def test_pdf_image_extraction_methods() -> None:
    config = ExtractionConfig(extract_images=True)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    assert hasattr(extractor, "_extract_images_from_playa_sync")

    mock_doc = MagicMock()
    mock_doc.pages = []

    images = extractor._extract_images_from_playa_sync(mock_doc)
    assert images == []


def test_presentation_image_extraction() -> None:
    config = ExtractionConfig(extract_images=True)
    extractor = PresentationExtractor(
        mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=config
    )

    # Test that the extractor has the Rust backend
    assert hasattr(extractor, "_extractor")

    # Test with real PPTX file to ensure images are extracted
    from pathlib import Path

    pptx_path = Path("tests/test_source_files/pitch-deck-presentation.pptx")
    if pptx_path.exists():
        result = extractor.extract_path_sync(pptx_path)
        assert len(result.images) > 0  # This file has images


def test_extractors_respect_extract_images_flag() -> None:
    config = ExtractionConfig(extract_images=False)

    pdf_extractor = PDFExtractor(mime_type="application/pdf", config=config)
    with patch.object(pdf_extractor, "_extract_pdf_searchable_text_sync", return_value="text"):
        with patch.object(pdf_extractor, "_extract_images_from_playa_sync"):
            result = pdf_extractor.extract_bytes_sync(b"%PDF-1.4\n%%EOF")
            if not config.extract_images:
                assert result.images == []


def test_memory_limit_enforcement() -> None:
    config = ExtractionConfig(extract_images=True)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    large_image = ExtractedImage(data=b"x" * (51 * 1024 * 1024), format="png", filename="large.png")

    result = extractor._check_image_memory_limits([large_image])
    assert len(result) == 0

    images = [ExtractedImage(data=b"x" * (30 * 1024 * 1024), format="png", filename=f"img{i}.png") for i in range(4)]

    result = extractor._check_image_memory_limits(images)
    assert len(result) < 4
    assert sum(len(img.data) for img in result) <= 100 * 1024 * 1024
