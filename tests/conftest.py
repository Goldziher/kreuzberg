from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

test_source_files_folder = Path(__file__).parent / "test_source_files"


@pytest.fixture(scope="session")
def searchable_pdf() -> Path:
    return test_source_files_folder / "searchable.pdf"


@pytest.fixture(scope="session")
def scanned_pdf() -> Path:
    return test_source_files_folder / "scanned.pdf"


@pytest.fixture(scope="session")
def non_searchable_pdf() -> Path:
    return test_source_files_folder / "non-searchable.pdf"


@pytest.fixture(scope="session")
def non_ascii_pdf() -> Path:
    return test_source_files_folder / "non-ascii-text.pdf"


@pytest.fixture(scope="session")
def test_article() -> Path:
    return test_source_files_folder / "test-article.pdf"


@pytest.fixture(scope="session")
def test_contract() -> Path:
    return test_source_files_folder / "sample-contract.pdf"


@pytest.fixture(scope="session")
def ocr_image() -> Path:
    return test_source_files_folder / "ocr-image.jpg"


@pytest.fixture(scope="session")
def docx_document() -> Path:
    return test_source_files_folder / "document.docx"


@pytest.fixture(scope="session")
def markdown_document() -> Path:
    return test_source_files_folder / "markdown.md"


@pytest.fixture(scope="session")
def pptx_document() -> Path:
    return test_source_files_folder / "pitch-deck-presentation.pptx"


@pytest.fixture(scope="session")
def html_document() -> Path:
    return test_source_files_folder / "html.html"


@pytest.fixture(scope="session")
def excel_document() -> Path:
    return test_source_files_folder / "excel.xlsx"


@pytest.fixture(scope="session")
def excel_multi_sheet_document() -> Path:
    return test_source_files_folder / "excel-multi-sheet.xlsx"


@pytest.fixture(scope="session")
def test_files_path() -> Path:
    return test_source_files_folder


@pytest.fixture(scope="session")
def tiny_pdf_with_tables() -> Path:
    return test_source_files_folder / "pdfs_with_tables" / "tiny.pdf"


pdfs_with_tables = sorted((test_source_files_folder / "pdfs_with_tables").glob("*.pdf"))


@pytest.fixture
def clear_cache() -> Generator[None, None, None]:
    """Fixture to clear all caches before each test that requests it."""
    from kreuzberg._utils._cache import clear_all_caches

    clear_all_caches()
    yield

    clear_all_caches()


@pytest.fixture(autouse=False)
def fresh_cache() -> None:
    """Fixture to ensure fresh cache state for each test function.

    Use this fixture for tests that need to test caching behavior
    or ensure they start with a clean cache state.
    """
    from kreuzberg._utils._cache import clear_all_caches

    clear_all_caches()


@pytest.fixture(scope="session", autouse=True)
def clean_cache_session() -> Generator[None, None, None]:
    """Automatically clear caches at the start and end of test session."""
    from kreuzberg._utils._cache import clear_all_caches

    clear_all_caches()
    yield
    clear_all_caches()
