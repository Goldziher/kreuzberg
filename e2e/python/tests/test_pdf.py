# Auto-generated tests for pdf fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_pdf_assembly_technical() -> None:
    """Assembly language technical manual with large body of text."""

    document_path = helpers.resolve_document("pdfs/assembly_language_for_beginners_al4_b_en.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_assembly_technical: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 5000)
    helpers.assert_content_contains_any(result, ["assembly", "register", "instruction"])
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_bayesian_data_analysis() -> None:
    """Bayesian data analysis textbook PDF with large content volume."""

    document_path = helpers.resolve_document("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_bayesian_data_analysis: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 10000)
    helpers.assert_content_contains_any(result, ["Bayesian", "probability", "distribution"])
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_code_and_formula() -> None:
    """PDF containing code snippets and formulas should retain substantial content."""

    document_path = helpers.resolve_document("pdfs/code_and_formula.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_code_and_formula: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 100)

def test_pdf_deep_learning() -> None:
    """Deep learning textbook PDF to ensure long-form extraction quality."""

    document_path = helpers.resolve_document("pdfs/fundamentals_of_deep_learning_2014.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_deep_learning: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 1000)
    helpers.assert_content_contains_any(result, ["neural", "network", "deep learning"])
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_embedded_images() -> None:
    """PDF with embedded images should extract text and tables when present."""

    document_path = helpers.resolve_document("pdfs/embedded_images_tables.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_embedded_images: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 50)
    helpers.assert_table_count(result, 0, None)

def test_pdf_google_doc() -> None:
    """Google Docs exported PDF to verify conversion fidelity."""

    document_path = helpers.resolve_document("pdfs/google_doc_document.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_google_doc: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 50)
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_large_ciml() -> None:
    """Large machine learning textbook PDF to stress extraction length."""

    document_path = helpers.resolve_document("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_large_ciml: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 10000)
    helpers.assert_content_contains_any(result, ["machine learning", "algorithm", "training"])
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_non_english_german() -> None:
    """German technical PDF to ensure non-ASCII content extraction."""

    document_path = helpers.resolve_document("pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_non_english_german: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 100)
    helpers.assert_content_contains_any(result, ["Intel", "paging"])
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_right_to_left() -> None:
    """Right-to-left language PDF to verify RTL extraction."""

    document_path = helpers.resolve_document("pdfs/right_to_left_01.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_right_to_left: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 50)
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

def test_pdf_simple_text() -> None:
    """Simple text-heavy PDF should extract content without OCR or tables."""

    document_path = helpers.resolve_document("pdfs/fake_memo.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_simple_text: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 50)
    helpers.assert_content_contains_any(result, ["May 5, 2023", "To Whom it May Concern", "Mallori"])

def test_pdf_tables_large() -> None:
    """Large PDF with extensive tables to stress table extraction."""

    document_path = helpers.resolve_document("pdfs_with_tables/large.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_tables_large: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 500)

def test_pdf_tables_medium() -> None:
    """Medium-sized PDF with multiple tables."""

    document_path = helpers.resolve_document("pdfs_with_tables/medium.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_tables_medium: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 100)

def test_pdf_tables_small() -> None:
    """Small PDF containing tables to validate table extraction."""

    document_path = helpers.resolve_document("pdfs_with_tables/tiny.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_tables_small: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 10)

def test_pdf_technical_stat_learning() -> None:
    """Technical statistical learning PDF requiring substantial extraction."""

    document_path = helpers.resolve_document("pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf")
    if not document_path.exists():
        pytest.skip(f"Skipping pdf_technical_stat_learning: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/pdf"])
    helpers.assert_min_content_length(result, 10000)
    helpers.assert_content_contains_any(result, ["statistical", "regression", "learning"])
    helpers.assert_metadata_expectation(result, "format_type", {"eq": "pdf"})

