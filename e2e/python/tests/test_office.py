# Auto-generated tests for office fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_office_doc_legacy() -> None:
    """Legacy .doc document conversion via LibreOffice."""

    document_path = helpers.resolve_document("legacy_office/unit_test_lists.doc")
    if not document_path.exists():
        pytest.skip(f"Skipping office_doc_legacy: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/msword"])
    helpers.assert_min_content_length(result, 20)

def test_office_docx_basic() -> None:
    """DOCX document extraction baseline."""

    document_path = helpers.resolve_document("office/document.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 10)

def test_office_docx_equations() -> None:
    """DOCX file containing equations to validate math extraction."""

    document_path = helpers.resolve_document("documents/equations.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_equations: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)

def test_office_docx_fake() -> None:
    """Simple DOCX document to verify baseline extraction."""

    document_path = helpers.resolve_document("documents/fake.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_fake: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)

def test_office_docx_formatting() -> None:
    """DOCX document heavy on formatting for style preservation."""

    document_path = helpers.resolve_document("documents/unit_test_formatting.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_formatting: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)

def test_office_docx_headers() -> None:
    """DOCX document with complex headers."""

    document_path = helpers.resolve_document("documents/unit_test_headers.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_headers: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)

def test_office_docx_lists() -> None:
    """DOCX document emphasizing list formatting."""

    document_path = helpers.resolve_document("documents/unit_test_lists.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_lists: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)

def test_office_docx_tables() -> None:
    """DOCX document containing tables for table-aware extraction."""

    document_path = helpers.resolve_document("documents/docx_tables.docx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_docx_tables: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"])
    helpers.assert_min_content_length(result, 20)

def test_office_ppt_legacy() -> None:
    """Legacy PowerPoint .ppt file requiring LibreOffice conversion."""

    document_path = helpers.resolve_document("legacy_office/simple.ppt")
    if not document_path.exists():
        pytest.skip(f"Skipping office_ppt_legacy: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.ms-powerpoint"])
    helpers.assert_min_content_length(result, 10)

def test_office_pptx_basic() -> None:
    """PPTX deck should extract slides content."""

    document_path = helpers.resolve_document("presentations/simple.pptx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_pptx_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.presentationml.presentation"])
    helpers.assert_min_content_length(result, 50)

def test_office_pptx_images() -> None:
    """PPTX presentation containing images to ensure metadata extraction."""

    document_path = helpers.resolve_document("presentations/powerpoint_with_image.pptx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_pptx_images: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.presentationml.presentation"])
    helpers.assert_min_content_length(result, 20)

def test_office_pptx_pitch_deck() -> None:
    """Pitch deck PPTX used to validate large slide extraction."""

    document_path = helpers.resolve_document("presentations/pitch_deck_presentation.pptx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_pptx_pitch_deck: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.presentationml.presentation"])
    helpers.assert_min_content_length(result, 100)

def test_office_xls_legacy() -> None:
    """Legacy XLS spreadsheet to ensure backward compatibility."""

    document_path = helpers.resolve_document("spreadsheets/test_excel.xls")
    if not document_path.exists():
        pytest.skip(f"Skipping office_xls_legacy: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.ms-excel"])
    helpers.assert_min_content_length(result, 10)

def test_office_xlsx_basic() -> None:
    """XLSX spreadsheet should produce metadata and content."""

    document_path = helpers.resolve_document("spreadsheets/stanley_cups.xlsx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_xlsx_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"])
    helpers.assert_min_content_length(result, 10)
    helpers.assert_metadata_expectation(result, "sheet_count", {"gte": 1})

def test_office_xlsx_multi_sheet() -> None:
    """XLSX workbook with multiple sheets."""

    document_path = helpers.resolve_document("spreadsheets/excel_multi_sheet.xlsx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_xlsx_multi_sheet: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"])
    helpers.assert_min_content_length(result, 20)
    helpers.assert_metadata_expectation(result, "sheet_count", {"gte": 2})

def test_office_xlsx_office_example() -> None:
    """Simple XLSX spreadsheet shipped alongside office integration tests."""

    document_path = helpers.resolve_document("office/excel.xlsx")
    if not document_path.exists():
        pytest.skip(f"Skipping office_xlsx_office_example: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"])
    helpers.assert_min_content_length(result, 10)

