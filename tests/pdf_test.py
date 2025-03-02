"""Tests for PDF extraction functionality."""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import ExtractionResult
from kreuzberg._pdf import (
    _convert_pdf_to_images,
    _extract_pdf_searchable_text,
    _extract_pdf_text_with_ocr,
    _validate_extracted_text,
    extract_pdf_file,
)
from kreuzberg.exceptions import ParsingError


@pytest.mark.anyio
async def test_extract_pdf_searchable_text(searchable_pdf: Path) -> None:
    """Test extracting text from a searchable PDF."""
    result = await _extract_pdf_searchable_text(searchable_pdf)
    assert isinstance(result, str)
    assert result.strip()


@pytest.mark.anyio
async def test_extract_pdf_searchable_not_fallback_to_ocr(test_contract: Path) -> None:
    result = await extract_pdf_file(test_contract, force_ocr=False, max_processes=1)
    assert result.content.startswith(
        "Page 1 Sample Contract Contract No.___________ PROFESSIONAL SERVICES AGREEMENT THIS AGREEMENT made and entered into this"
    )


@pytest.mark.anyio
async def test_extract_pdf_text_with_ocr(scanned_pdf: Path) -> None:
    """Test extracting text from a scanned PDF using OCR."""
    result = await _extract_pdf_text_with_ocr(scanned_pdf, max_processes=1)
    assert isinstance(result, ExtractionResult)
    assert result.content.strip()


@pytest.mark.anyio
async def test_extract_pdf_file(searchable_pdf: Path) -> None:
    """Test extracting text from a PDF file."""
    result = await extract_pdf_file(searchable_pdf, force_ocr=False, max_processes=1)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_file_non_searchable(non_searchable_pdf: Path) -> None:
    """Test extracting text from a non-searchable PDF file."""
    result = await extract_pdf_file(non_searchable_pdf, force_ocr=False, max_processes=1)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_extract_pdf_file_invalid() -> None:
    """Test that attempting to extract from an invalid PDF raises an error."""
    with pytest.raises(FileNotFoundError):
        await extract_pdf_file(Path("/invalid/path.pdf"), force_ocr=False, max_processes=1)


@pytest.mark.anyio
async def test_convert_pdf_to_images_raises_parsing_error(tmp_path: Path) -> None:
    """Test that attempting to convert an invalid PDF to images raises a ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await _convert_pdf_to_images(pdf_path)

    assert "Could not convert PDF to images" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


@pytest.mark.anyio
async def test_extract_pdf_searchable_text_raises_parsing_error(tmp_path: Path) -> None:
    """Test that attempting to extract text from an invalid PDF raises a ParsingError."""
    pdf_path = tmp_path / "invalid.pdf"
    pdf_path.write_text("invalid pdf content")

    with pytest.raises(ParsingError) as exc_info:
        await _extract_pdf_searchable_text(pdf_path)

    assert "Could not extract text from PDF file" in str(exc_info.value)
    assert str(pdf_path) in str(exc_info.value.context["file_path"])


def test_validate_empty_text() -> None:
    """Test that empty text is considered invalid."""
    assert not _validate_extracted_text("")
    assert not _validate_extracted_text("   ")
    assert not _validate_extracted_text("\n\n")


def test_validate_normal_text() -> None:
    """Test that normal text passes validation."""
    assert _validate_extracted_text("Hello World!")
    assert _validate_extracted_text("Line 1\nLine 2")
    assert _validate_extracted_text("© 2024 Company")
    assert _validate_extracted_text("Special chars: !@#$%^&*()")
    assert _validate_extracted_text("""
        This is a normal paragraph of text that should pass validation.
        It contains normal punctuation, numbers (123), and symbols (!@#$%).
        Even with multiple paragraphs and line breaks, it should be fine.
    """)


def test_validate_short_corrupted_text() -> None:
    """Test validation of short text with corruption matches."""
    # Test text shorter than SHORT_TEXT_THRESHOLD with corruption matches
    assert not _validate_extracted_text("\x00\x00\x00")  # Three null bytes (> MINIMUM_CORRUPTED_RESULTS)
    assert _validate_extracted_text("Hi\x00\x00")  # Two null bytes (= MINIMUM_CORRUPTED_RESULTS)
    assert _validate_extracted_text("Hi\x00")  # One null byte (< MINIMUM_CORRUPTED_RESULTS)
    assert _validate_extracted_text("Short \ufffd")  # One replacement char (< MINIMUM_CORRUPTED_RESULTS)


def test_validate_long_corrupted_text() -> None:
    """Test validation of long text with corruption threshold."""
    # Create a long text with varying levels of corruption
    base_text = "A" * 1000  # Long text to exceed SHORT_TEXT_THRESHOLD

    # Test with corruption below threshold (5%)
    text_low_corruption = base_text + ("\x00" * 40)  # 4% corruption
    assert _validate_extracted_text(text_low_corruption)

    # Test with corruption above threshold (5%)
    text_high_corruption = base_text + ("\x00" * 60)  # 6% corruption
    assert not _validate_extracted_text(text_high_corruption)


def test_validate_custom_corruption_threshold() -> None:
    """Test validation with custom corruption threshold."""
    base_text = "A" * 1000
    corrupted_chars = "\x00" * 100  # 10% corruption
    text = base_text + corrupted_chars

    # Should fail with default threshold (5%)
    assert not _validate_extracted_text(text)

    # Should pass with higher threshold (15%)
    assert _validate_extracted_text(text, corruption_threshold=0.15)

    # Should fail with lower threshold (3%)
    assert not _validate_extracted_text(text, corruption_threshold=0.03)
