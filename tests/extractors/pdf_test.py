from __future__ import annotations

import os
from pathlib import Path
from typing import NoReturn

import pandas as pd
import pytest
from PIL.Image import Image
from pytest import MonkeyPatch

from kreuzberg import ExtractionResult
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ParsingError
from kreuzberg.extraction import DEFAULT_CONFIG
from tests.conftest import pdfs_with_tables

IS_CI = os.environ.get("CI", "false").lower() == "true"


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
        "Page 1\nSample Contract\nContract No....\nPROFESSIONAL SERVICES AGREEMENT\nTHIS AGREEMENT made and entered into this"
    )


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_pdf_text_with_ocr(extractor: PDFExtractor, scanned_pdf: Path) -> None:
    result = await extractor._extract_pdf_text_with_ocr(scanned_pdf, ocr_backend="tesseract")
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
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
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
    assert str(pdf_path) in str(exc_info.value.context["file"]["path"])


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_raises_parsing_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await extractor._extract_pdf_searchable_text(pdf_path)

    assert "Could not extract text from PDF file" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file"]["path"])


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

        assert isinstance(table["df"], (pd.DataFrame, dict))

        assert isinstance(table["cropped_image"], (Image, type(None)))


def test_extract_pdf_bytes_sync(extractor: PDFExtractor, test_article: Path) -> None:
    """Test sync PDF extraction from bytes."""
    pdf_bytes = test_article.read_bytes()

    result = extractor.extract_bytes_sync(pdf_bytes)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert result.mime_type == "text/plain"
    assert result.metadata
    assert "title" in result.metadata


def test_extract_pdf_path_sync(extractor: PDFExtractor, searchable_pdf: Path) -> None:
    """Test sync PDF extraction from path."""
    result = extractor.extract_path_sync(searchable_pdf)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert result.mime_type == "text/plain"
    assert result.metadata == {"quality_score": 1.0}


def test_extract_pdf_path_sync_with_tables(searchable_pdf: Path) -> None:
    """Test sync PDF extraction with table extraction enabled."""
    config = ExtractionConfig(extract_tables=True)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = extractor.extract_path_sync(searchable_pdf)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert isinstance(result.tables, list)


@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
def test_extract_pdf_path_sync_force_ocr_tesseract(searchable_pdf: Path) -> None:
    """Test sync PDF extraction with forced OCR using tesseract."""
    config = ExtractionConfig(force_ocr=True, ocr_backend="tesseract")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = extractor.extract_path_sync(searchable_pdf)

    assert isinstance(result, ExtractionResult)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


def test_extract_pdf_searchable_text_sync_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    """Test sync searchable text extraction with invalid PDF raises ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError, match="Failed to extract PDF text"):
        extractor._extract_pdf_searchable_text_sync(pdf_path)


def test_extract_pdf_with_ocr_sync_error(extractor: PDFExtractor, tmp_path: Path) -> None:
    """Test sync OCR extraction with invalid PDF raises ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError, match="Failed to OCR PDF"):
        extractor._extract_pdf_with_ocr_sync(pdf_path)


@pytest.mark.anyio
async def test_extract_pdf_no_ocr_backend_fallback(non_searchable_pdf: Path) -> None:
    """Test PDF extraction falls back to empty result when no OCR backend available."""
    config = ExtractionConfig(force_ocr=False, ocr_backend=None)
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = await extractor.extract_path_async(non_searchable_pdf)

    # Should fallback to empty result since no OCR backend and PDF isn't searchable  # ~keep
    assert result.content == ""
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_partial_failure(
    extractor: PDFExtractor, tmp_path: Path, monkeypatch: MonkeyPatch
) -> None:
    """Test searchable text extraction with partial page failures."""

    def mock_page_get_textpage() -> NoReturn:
        raise Exception("Page extraction failed")


def test_validate_short_text_with_many_corrupted_chars(extractor: PDFExtractor) -> None:
    """Test validation of short text with many corrupted characters."""

    corrupted_text = "hi\x00\x01\x02"
    assert not extractor._validate_extracted_text(corrupted_text)

    semi_corrupted = "hi\x00\x01"
    assert extractor._validate_extracted_text(semi_corrupted)


def test_validate_text_unicode_replacement_chars(extractor: PDFExtractor) -> None:
    """Test validation with Unicode replacement characters."""

    text_with_replacements = "Hello " + ("\ufffd" * 20) + " World"
    assert not extractor._validate_extracted_text(text_with_replacements)

    text_with_few_replacements = "Hello \ufffd World"
    assert extractor._validate_extracted_text(text_with_few_replacements)


def test_validate_text_mixed_corruption(extractor: PDFExtractor) -> None:
    """Test validation with mixed corruption types."""
    base_text = "A" * 1000

    mixed_corruption = "\x00\x01\x02\ufffd\ufffd" * 15
    text = base_text + mixed_corruption

    # Should fail due to high corruption ratio (75/1075 = ~7%)  # ~keep
    assert not extractor._validate_extracted_text(text)

    assert extractor._validate_extracted_text(text, corruption_threshold=0.08)


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_pdf_force_ocr_when_valid_text_exists(searchable_pdf: Path) -> None:
    """Test force_ocr=True bypasses valid text extraction - covers line 52->57."""
    config = ExtractionConfig(force_ocr=True, ocr_backend="tesseract")
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    result = await extractor.extract_path_async(searchable_pdf)

    # Should still get valid content via OCR  # ~keep
    assert result.content.strip()
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_page_errors(
    extractor: PDFExtractor, tmp_path: Path, monkeypatch: MonkeyPatch
) -> None:
    """Test partial page failure handling - covers lines 255-257, 264, 267."""
    import pypdfium2

    class MockPage:
        def __init__(self, should_fail: bool = False) -> None:
            self.should_fail = should_fail

        def get_textpage(self) -> object:
            if self.should_fail:
                raise Exception("Page extraction failed")
            return MockTextPage()

    class MockTextPage:
        def get_text_bounded(self) -> str:
            return "Valid page text"

    class MockDocument:
        def __init__(self) -> None:
            self.pages = [
                MockPage(should_fail=False),
                MockPage(should_fail=True),
                MockPage(should_fail=False),
            ]

        def __iter__(self) -> object:
            return iter(self.pages)

        def close(self) -> None:
            pass

    tmp_path / "test.pdf"

    def mock_pdf_document(*args: object, **kwargs: object) -> MockDocument:
        return MockDocument()

    monkeypatch.setattr(pypdfium2, "PdfDocument", mock_pdf_document)
