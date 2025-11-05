# Auto-generated tests for ocr fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_ocr_image_hello_world() -> None:
    """PNG image with visible English text for OCR validation."""

    document_path = helpers.resolve_document("images/test_hello_world.png")
    if not document_path.exists():
        pytest.skip(f"Skipping ocr_image_hello_world: missing document at {document_path}")

    config = helpers.build_config({"force_ocr": True, "ocr": {"backend": "tesseract", "language": "eng"}})

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["image/png"])
    helpers.assert_min_content_length(result, 5)
    helpers.assert_content_contains_any(result, ["hello", "world"])


def test_ocr_image_no_text() -> None:
    """Image with no text to ensure OCR handles empty results gracefully."""

    document_path = helpers.resolve_document("images/flower_no_text.jpg")
    if not document_path.exists():
        pytest.skip(f"Skipping ocr_image_no_text: missing document at {document_path}")

    config = helpers.build_config({"force_ocr": True, "ocr": {"backend": "tesseract", "language": "eng"}})

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["image/jpeg"])
    helpers.assert_max_content_length(result, 200)


def test_ocr_pdf_image_only_german() -> None:
    """Image-only German PDF requiring OCR to extract text."""

    document_path = helpers.resolve_document("pdfs/image_only_german_pdf.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping ocr_pdf_image_only_german: missing document at {document_path}")

    config = helpers.build_config({"force_ocr": True, "ocr": {"backend": "tesseract", "language": "eng"}})

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 20)
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})


def test_ocr_pdf_rotated_90() -> None:
    """Rotated page PDF requiring OCR to verify orientation handling."""

    document_path = helpers.resolve_document("pdfs/ocr_test_rotated_90.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping ocr_pdf_rotated_90: missing document at {document_path}")

    config = helpers.build_config({"force_ocr": True, "ocr": {"backend": "tesseract", "language": "eng"}})

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 10)


def test_ocr_pdf_tesseract() -> None:
    """Scanned PDF requires OCR to extract text."""

    document_path = helpers.resolve_document("pdfs/ocr_test.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping ocr_pdf_tesseract: missing document at {document_path}")

    config = helpers.build_config({"force_ocr": True, "ocr": {"backend": "tesseract", "language": "eng"}})

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 20)
    helpers.assert_content_contains_any(result, ["Docling", "Markdown", "JSON"])
