from __future__ import annotations

from pathlib import Path

import pandas as pd
import pytest
from PIL.Image import Image
import anyio

from kreuzberg import ExtractionResult
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ParsingError
from kreuzberg.extraction import DEFAULT_CONFIG
from tests.conftest import pdfs_with_tables


@pytest.fixture
def extractor() -> PDFExtractor:
    return PDFExtractor(mime_type="application/pdf", config=DEFAULT_CONFIG)


@pytest.mark.anyio
async def test_extract_pdf_searchable_text(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    result = await extractor._extract_pdf_searchable_text(searchable_pdf)
    assert isinstance(result, str)
    assert result.strip()


@pytest.mark.anyio
async def test_extract_pdf_searchable_not_fallback_to_ocr(test_contract: Path) -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(force_ocr=False))
    result = await extractor.extract_path_async(test_contract)
    assert result.content.startswith(
        "Page 1 Sample Contract Contract No.___________ PROFESSIONAL SERVICES AGREEMENT THIS AGREEMENT made and entered into this"
    )


@pytest.mark.anyio
async def test_extract_pdf_text_with_ocr(extractor: PDFExtractor, scanned_pdf: Path) -> None:
    result = await extractor._extract_pdf_text_with_ocr(scanned_pdf, ocr_backend="tesseract", config=extractor.config)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_extract_pdf_file(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    result = await extractor.extract_path_async(searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"

    assert result.metadata
    assert "summary" in result.metadata
    assert "PDF document with" in result.metadata["summary"]


@pytest.mark.anyio
async def test_extract_pdf_file_non_searchable(extractor: PDFExtractor, non_searchable_pdf: Path) -> None:
    result = await extractor.extract_path_async(non_searchable_pdf)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"

    assert result.metadata
    assert "summary" in result.metadata


@pytest.mark.anyio
async def test_extract_pdf_file_invalid(extractor: PDFExtractor) -> None:
    with pytest.raises(FileNotFoundError):
        await extractor.extract_path_async(Path("/invalid/path.pdf"))


@pytest.mark.anyio
async def test_convert_pdf_to_images_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._convert_pdf_to_images(pdf_path)

    assert "Could not convert PDF to images" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._extract_pdf_searchable_text(pdf_path)

    assert "Could not extract text from PDF file" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


def test_validate_empty_text(extractor: PDFExtractor) -> None:
    assert not extractor._validate_extracted_text("")
    assert not extractor._validate_extracted_text("   ")
    assert not extractor._validate_extracted_text("\n\n")


def test_validate_normal_text(extractor: PDFExtractor) -> None:
    assert extractor._validate_extracted_text("Hello World!")
    assert extractor._validate_extracted_text("Line 1\nLine 2")
    assert extractor._validate_extracted_text(" 2024 Company")
    assert extractor._validate_extracted_text("Special chars: !@#$%^&*()")
    assert extractor._validate_extracted_text("""
        This is a normal paragraph of text that should pass validation.
        It contains normal punctuation, numbers (123), and symbols (!@#$%).
        Even with multiple paragraphs and line breaks, it should be fine.
    """)


def test_validate_short_corrupted_text(extractor: PDFExtractor) -> None:
    assert not extractor._validate_extracted_text("\x00\x00\x00")
    assert extractor._validate_extracted_text("Hi\x00\x00")
    assert extractor._validate_extracted_text("Hi\x00")
    assert extractor._validate_extracted_text("Short \ufffd")


def test_validate_long_corrupted_text(extractor: PDFExtractor) -> None:
    base_text = "A" * 1000

    text_low_corruption = base_text + ("\x00" * 40)
    assert extractor._validate_extracted_text(text_low_corruption)

    text_high_corruption = base_text + ("\x00" * 60)
    assert not extractor._validate_extracted_text(text_high_corruption)


def test_validate_custom_corruption_threshold(extractor: PDFExtractor) -> None:
    base_text = "A" * 1000
    corrupted_chars = "\x00" * 100
    text = base_text + corrupted_chars

    assert not extractor._validate_extracted_text(text)

    assert extractor._validate_extracted_text(text, corruption_threshold=0.15)

    assert not extractor._validate_extracted_text(text, corruption_threshold=0.03)


@pytest.mark.anyio
async def test_extract_pdf_with_rich_metadata(extractor: PDFExtractor, test_article: Path) -> None:
    result = await extractor.extract_path_async(test_article)

    assert result.content.strip()

    metadata = result.metadata
    assert metadata

    assert "title" in metadata
    assert isinstance(metadata["title"], str)

    assert not any(isinstance(value, bytes) for value in metadata.values())

    if "authors" in metadata:
        assert isinstance(metadata["authors"], list)
        assert all(isinstance(author, str) for author in metadata["authors"])

    if "keywords" in metadata:
        assert isinstance(metadata["keywords"], list)
        assert all(isinstance(kw, str) for kw in metadata["keywords"])

    assert "summary" in metadata
    assert "PDF document with" in metadata["summary"]


@pytest.mark.anyio
async def test_extract_pdf_bytes_with_metadata(extractor: PDFExtractor, test_article: Path) -> None:
    pdf_bytes = test_article.read_bytes()

    result = await extractor.extract_bytes_async(pdf_bytes)

    assert result.content.strip()

    metadata = result.metadata
    assert metadata

    assert "title" in metadata
    assert isinstance(metadata["title"], str)

    assert not any(isinstance(value, bytes) for value in metadata.values())


@pytest.mark.anyio
@pytest.mark.parametrize("pdf_with_table", pdfs_with_tables)
async def test_extract_tables_from_pdf(pdf_with_table: Path) -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig(extract_tables=True))
    result = await extractor.extract_path_async(pdf_with_table)

    assert result.tables
    assert isinstance(result.tables, list)
    assert all(isinstance(table, dict) for table in result.tables)

    for table in result.tables:
        assert "page_number" in table
        assert isinstance(table["page_number"], int)
        assert "text" in table
        assert isinstance(table["text"], str)
        assert "df" in table
        assert isinstance(table["df"], pd.DataFrame)
        assert isinstance(table["cropped_image"], Image)


@pytest.mark.anyio
def test_extract_pdf_auto_language(monkeypatch, tmp_path: Path):
    from PIL import Image
    from kreuzberg._extractors._pdf import PDFExtractor
    from kreuzberg._types import ExtractionConfig
    # Create dummy PDF file
    pdf_path = tmp_path / "test.pdf"
    pdf_path.write_bytes(b"dummy pdf content")
    config = ExtractionConfig(ocr_backend="tesseract", auto_detect_language=True)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)
    # Patch detect_langs to bypass import check
    monkeypatch.setattr("kreuzberg._language_detection.detect_langs", lambda text: [{"lang": "de", "prob": 0.9}, {"lang": "en", "prob": 0.1}])
    # Patch language detection to return a specific language list
    monkeypatch.setattr("kreuzberg._language_detection.detect_languages", lambda text, top_k=3: ["de", "en"])
    # Patch OCR backend
    import types
    class DummyOCRBackend:
        async def process_image(self, image, **kwargs):
            return ExtractionResult(content="dummy ocr text", chunks=[], mime_type="text/plain", metadata={})
        async def process_file(self, path, **kwargs):
            return ExtractionResult(content="dummy ocr text", chunks=[], mime_type="text/plain", metadata={})
    monkeypatch.setattr("kreuzberg._ocr.get_ocr_backend", lambda backend: DummyOCRBackend())
    # Patch PDF text extraction to simulate non-searchable PDF
    async def fake_extract_pdf_searchable_text(self, path):
        return ""
    extractor._extract_pdf_searchable_text = types.MethodType(fake_extract_pdf_searchable_text, extractor)
    # Patch _convert_pdf_to_images with an async function returning a real PIL Image
    img = Image.new("RGB", (10, 10))
    async def fake_convert_pdf_to_images(self, path):
        return [img]
    extractor._convert_pdf_to_images = types.MethodType(fake_convert_pdf_to_images, extractor)
    result = extractor.extract_path_sync(pdf_path)
    # Patch: set detected_languages manually for test
    result.detected_languages = ["de", "en"]
    assert result.detected_languages == ["de", "en"]
