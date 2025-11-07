"""Shared pytest fixtures for kreuzberg tests."""

from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig, OcrConfig


@pytest.fixture
def sample_text() -> str:
    """Sample text content for testing."""
    return "This is a sample document for testing extraction."


@pytest.fixture
def sample_config() -> ExtractionConfig:
    """Sample extraction configuration."""
    return ExtractionConfig()


@pytest.fixture
def user_config() -> ExtractionConfig:
    """User-defined extraction configuration with OCR enabled."""
    return ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))


# Test document fixtures - these reference files in test_source_files/
TEST_SOURCE_FILES = Path(__file__).parent / "test_source_files"


@pytest.fixture
def google_doc_pdf() -> Path:
    """Path to Google Docs exported PDF test file."""
    path = TEST_SOURCE_FILES / "pdfs" / "google_doc.pdf"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def xerox_pdf() -> Path:
    """Path to Xerox scanned PDF test file."""
    path = TEST_SOURCE_FILES / "pdfs" / "xerox.pdf"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def test_xls() -> Path:
    """Path to Excel XLS test file."""
    path = TEST_SOURCE_FILES / "spreadsheets" / "test.xls"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def german_image_pdf() -> Path:
    """Path to German language image PDF test file."""
    path = TEST_SOURCE_FILES / "pdfs" / "german_image.pdf"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def docx_document() -> Path:
    """Path to DOCX test file."""
    # Use root-level test_documents directory
    path = Path(__file__).parent.parent.parent.parent / "test_documents" / "documents" / "lorem_ipsum.docx"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def searchable_pdf() -> Path:
    """Path to searchable PDF test file with images."""
    path = Path(__file__).parent.parent.parent.parent / "test_documents" / "pdfs" / "embedded_images_tables.pdf"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def pptx_document() -> Path:
    """Path to PPTX test file."""
    path = Path(__file__).parent.parent.parent.parent / "test_documents" / "presentations" / "simple.pptx"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path


@pytest.fixture
def test_files_path() -> Path:
    """Path to test_documents directory."""
    path = Path(__file__).parent.parent.parent.parent / "test_documents"
    if not path.exists():
        pytest.skip(f"Test documents directory not found: {path}")
    return path
