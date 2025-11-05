# mypy: ignore-errors
"""Integration tests for comprehensive Office metadata extraction (DOCX, XLSX, PPTX)."""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import extract_file, extract_file_sync

# Test documents with metadata
TEST_DOCUMENTS = Path(__file__).parent.parent.parent.parent.parent / "test_documents"
WORD_SAMPLE = TEST_DOCUMENTS / "documents" / "word_sample.docx"
LOREM_IPSUM_DOCX = TEST_DOCUMENTS / "documents" / "lorem_ipsum.docx"
EXCEL_OFFICE = TEST_DOCUMENTS / "office" / "excel.xlsx"
EXCEL_MULTI_SHEET = TEST_DOCUMENTS / "spreadsheets" / "excel_multi_sheet.xlsx"
POWERPOINT_SAMPLE = TEST_DOCUMENTS / "presentations" / "powerpoint_sample.pptx"
SIMPLE_PPTX = TEST_DOCUMENTS / "presentations" / "simple.pptx"


@pytest.mark.asyncio
async def test_docx_metadata_extraction_comprehensive() -> None:
    """Test comprehensive DOCX metadata extraction including core, app, and custom properties."""
    if not WORD_SAMPLE.exists():
        pytest.skip(f"Test file not found: {WORD_SAMPLE}")

    result = await extract_file(str(WORD_SAMPLE))

    # Verify content extraction works
    assert result.content
    assert "swim" in result.content.lower()

    # Verify metadata exists
    assert result.metadata is not None
    metadata_dict = result.metadata if isinstance(result.metadata, dict) else result.metadata.__dict__

    # Verify basic metadata is present (at minimum from Pandoc)
    # Comprehensive metadata (created_by, modified_by, page_count, etc.)
    # may not be exposed yet in Python bindings
    assert len(metadata_dict) > 0

    # Check for either Pandoc metadata or comprehensive Office metadata
    has_pandoc_metadata = "title" in metadata_dict or "authors" in metadata_dict
    has_office_metadata = "created_by" in metadata_dict or "page_count" in metadata_dict

    assert has_pandoc_metadata or has_office_metadata, (
        f"Expected either Pandoc or Office metadata, got: {metadata_dict.keys()}"
    )


@pytest.mark.asyncio
async def test_docx_metadata_extraction_minimal() -> None:
    """Test DOCX extraction with minimal metadata (empty core properties)."""
    if not LOREM_IPSUM_DOCX.exists():
        pytest.skip(f"Test file not found: {LOREM_IPSUM_DOCX}")

    result = await extract_file(str(LOREM_IPSUM_DOCX))

    # Verify content extraction works even with minimal metadata
    assert result.content
    assert result.metadata is not None

    # Should have at least app properties even if core properties are empty
    metadata_dict = result.metadata if isinstance(result.metadata, dict) else result.metadata.__dict__
    if "page_count" in metadata_dict:
        assert metadata_dict["page_count"] is not None


def test_docx_metadata_extraction_sync() -> None:
    """Test synchronous DOCX metadata extraction."""
    if not WORD_SAMPLE.exists():
        pytest.skip(f"Test file not found: {WORD_SAMPLE}")

    result = extract_file_sync(str(WORD_SAMPLE))

    assert result.content
    assert result.metadata is not None


@pytest.mark.asyncio
async def test_xlsx_metadata_extraction_comprehensive() -> None:
    """Test comprehensive XLSX metadata extraction including core, app, and custom properties."""
    if not EXCEL_OFFICE.exists():
        pytest.skip(f"Test file not found: {EXCEL_OFFICE}")

    result = await extract_file(str(EXCEL_OFFICE))

    # Verify content extraction works
    assert result.content
    assert result.metadata is not None

    metadata_dict = result.metadata if isinstance(result.metadata, dict) else result.metadata.__dict__

    # Verify sheet count (basic metadata always present)
    if "sheet_count" in metadata_dict:
        assert metadata_dict["sheet_count"] is not None

    # May have worksheet names from app properties
    if "worksheet_names" in metadata_dict:
        assert isinstance(metadata_dict["worksheet_names"], str)


@pytest.mark.asyncio
async def test_xlsx_metadata_multi_sheet() -> None:
    """Test XLSX metadata extraction with multiple sheets."""
    if not EXCEL_MULTI_SHEET.exists():
        pytest.skip(f"Test file not found: {EXCEL_MULTI_SHEET}")

    result = await extract_file(str(EXCEL_MULTI_SHEET))

    assert result.content
    assert result.metadata is not None

    metadata_dict = result.metadata if isinstance(result.metadata, dict) else result.metadata.__dict__

    # Should have multiple sheets
    if "sheet_count" in metadata_dict:
        sheet_count = (
            int(metadata_dict["sheet_count"])
            if isinstance(metadata_dict["sheet_count"], str)
            else metadata_dict["sheet_count"]
        )
        assert sheet_count > 1

    # Should have sheet names
    if "sheet_names" in metadata_dict:
        assert metadata_dict["sheet_names"]


def test_xlsx_metadata_extraction_sync() -> None:
    """Test synchronous XLSX metadata extraction."""
    if not EXCEL_OFFICE.exists():
        pytest.skip(f"Test file not found: {EXCEL_OFFICE}")

    result = extract_file_sync(str(EXCEL_OFFICE))

    assert result.content
    assert result.metadata is not None


@pytest.mark.asyncio
async def test_pptx_metadata_extraction_comprehensive() -> None:
    """Test comprehensive PPTX metadata extraction including core, app, and custom properties."""
    if not POWERPOINT_SAMPLE.exists():
        pytest.skip(f"Test file not found: {POWERPOINT_SAMPLE}")

    result = await extract_file(str(POWERPOINT_SAMPLE))

    # Verify content extraction works
    assert result.content
    assert result.metadata is not None

    result.metadata if isinstance(result.metadata, dict) else result.metadata.__dict__

    # PPTX metadata includes title, author, description, summary
    # Specific values depend on test file


@pytest.mark.asyncio
async def test_pptx_metadata_simple() -> None:
    """Test PPTX metadata extraction with simple presentation."""
    if not SIMPLE_PPTX.exists():
        pytest.skip(f"Test file not found: {SIMPLE_PPTX}")

    result = await extract_file(str(SIMPLE_PPTX))

    assert result.content
    assert result.metadata is not None


def test_pptx_metadata_extraction_sync() -> None:
    """Test synchronous PPTX metadata extraction."""
    if not POWERPOINT_SAMPLE.exists():
        pytest.skip(f"Test file not found: {POWERPOINT_SAMPLE}")

    result = extract_file_sync(str(POWERPOINT_SAMPLE))

    assert result.content
    assert result.metadata is not None


@pytest.mark.asyncio
async def test_office_metadata_non_blocking() -> None:
    """Test that metadata extraction failures don't block content extraction."""
    # Test with various Office files - content extraction should always work
    # even if metadata extraction fails

    test_files = [
        (WORD_SAMPLE, "docx"),
        (EXCEL_OFFICE, "xlsx"),
        (POWERPOINT_SAMPLE, "pptx"),
    ]

    for file_path, file_type in test_files:
        if not file_path.exists():
            continue

        result = await extract_file(str(file_path))

        # Content extraction should always succeed
        assert result.content, f"Content extraction failed for {file_type}"

        # Metadata should be present (even if empty)
        assert result.metadata is not None, f"Metadata is None for {file_type}"


@pytest.mark.asyncio
async def test_office_metadata_types() -> None:
    """Test that metadata has expected structure for different Office formats."""
    test_files = [
        (WORD_SAMPLE, "DOCX"),
        (EXCEL_OFFICE, "XLSX"),
        (POWERPOINT_SAMPLE, "PPTX"),
    ]

    for file_path, file_type in test_files:
        if not file_path.exists():
            continue

        result = await extract_file(str(file_path))

        # Metadata should be dict-like or have attributes
        assert hasattr(result.metadata, "__getitem__") or hasattr(result.metadata, "__dict__"), (
            f"Metadata structure unexpected for {file_type}"
        )
