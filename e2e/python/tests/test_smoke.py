# Auto-generated tests for smoke fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_smoke_docx_basic() -> None:
    """Smoke test: DOCX with formatted text"""

    document_path = helpers.resolve_document("documents/fake.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_docx_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)
    helpers.assert_content_contains_any(result, ["Lorem", "ipsum", "document", "text"])


def test_smoke_html_basic() -> None:
    """Smoke test: HTML converted to Markdown"""

    document_path = helpers.resolve_document("web/simple_table.html")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_html_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["text/html"])
    helpers.assert_min_content_length(result, 10)
    helpers.assert_content_contains_any(result, ["#", "**", "simple", "HTML"])


def test_smoke_image_png() -> None:
    """Smoke test: PNG image (without OCR, metadata only)"""

    document_path = helpers.resolve_document("images/sample.png")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_image_png: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["image/png"])
    helpers.assert_metadata_expectation(result, "format", {"eq": "PNG"})


def test_smoke_json_basic() -> None:
    """Smoke test: JSON file extraction"""

    document_path = helpers.resolve_document("data_formats/simple.json")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_json_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/json"])
    helpers.assert_min_content_length(result, 5)


def test_smoke_pdf_basic() -> None:
    """Smoke test: PDF with simple text extraction"""

    document_path = helpers.resolve_document("pdfs/fake_memo.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_pdf_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 50)
    helpers.assert_content_contains_any(result, ["May 5, 2023", "To Whom it May Concern"])


def test_smoke_txt_basic() -> None:
    """Smoke test: Plain text file"""

    document_path = helpers.resolve_document("text/report.txt")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_txt_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["text/plain"])
    helpers.assert_min_content_length(result, 5)


def test_smoke_xlsx_basic() -> None:
    """Smoke test: XLSX with basic spreadsheet data including tables"""

    document_path = helpers.resolve_document("spreadsheets/stanley_cups.xlsx")
    if not document_path.exists():
        pytest.skip(f"Skipping smoke_xlsx_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"])
    helpers.assert_min_content_length(result, 100)
    helpers.assert_content_contains_all(
        result, ["Team", "Location", "Stanley Cups", "Blues", "Flyers", "Maple Leafs", "STL", "PHI", "TOR"]
    )
    helpers.assert_table_count(result, 1, None)
    helpers.assert_metadata_expectation(result, "sheet_count", {"gte": 2})
    helpers.assert_metadata_expectation(result, "sheet_names", {"contains": ["Stanley Cups"]})
