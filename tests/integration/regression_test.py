from __future__ import annotations

import os
from pathlib import Path

import pytest

from kreuzberg import batch_extract_file, extract_file
from kreuzberg._types import ExtractionConfig, PSMMode, TesseractConfig
from kreuzberg.extraction import extract_file_sync

TEST_DATA_DIR = Path(__file__).parent.parent / "test_documents"


@pytest.mark.parametrize(
    "pdf_fixture,expected_content",
    [
        ("google_doc_pdf", "Example document"),
        pytest.param(
            "xerox_pdf", "UNIVERSITIES" if os.environ.get("CI") == "true" else "AltaLink", id="xerox_pdf-AltaLink"
        ),
    ],
)
@pytest.mark.parametrize("test_mode", ["async", "sync"])
@pytest.mark.parametrize("config_type", ["default", "user_config"])
@pytest.mark.anyio
async def test_pdf_extraction_regression(
    pdf_fixture: str,
    expected_content: str,
    test_mode: str,
    config_type: str,
    google_doc_pdf: Path,
    xerox_pdf: Path,
    user_config: ExtractionConfig,
    request: pytest.FixtureRequest,
) -> None:
    pdf_path = {"google_doc_pdf": google_doc_pdf, "xerox_pdf": xerox_pdf}[pdf_fixture]

    config = user_config if config_type == "user_config" else None

    if test_mode == "async":
        if config:
            result = await extract_file(str(pdf_path), config=config)
        else:
            result = await extract_file(str(pdf_path))
    elif config:
        result = extract_file_sync(str(pdf_path), config=config)
    else:
        result = extract_file_sync(str(pdf_path))

    assert result.content is not None
    assert len(result.content) > 0
    assert expected_content in result.content


@pytest.mark.parametrize("test_mode", ["async", "sync", "bytes"])
@pytest.mark.parametrize("config_type", ["default", "user_config"])
@pytest.mark.anyio
async def test_xls_extraction_regression(
    test_mode: str, config_type: str, test_xls: Path, user_config: ExtractionConfig
) -> None:
    config = user_config if config_type == "user_config" else None

    if test_mode == "async":
        if config:
            result = await extract_file(str(test_xls), config=config)
        else:
            result = await extract_file(str(test_xls))
    elif test_mode == "sync":
        result = extract_file_sync(str(test_xls), config=config) if config else extract_file_sync(str(test_xls))
    else:
        content = test_xls.read_bytes()
        from kreuzberg import extract_bytes

        if config:
            result = await extract_bytes(content, mime_type="application/vnd.ms-excel", config=config)
        else:
            result = await extract_bytes(content, mime_type="application/vnd.ms-excel")

    assert result.content is not None
    assert len(result.content) > 0


@pytest.mark.parametrize("config_type", ["default", "user_config"])
@pytest.mark.anyio
async def test_batch_extraction_regression(
    config_type: str, google_doc_pdf: Path, xerox_pdf: Path, user_config: ExtractionConfig
) -> None:
    config = user_config if config_type == "user_config" else None

    if config:
        results = await batch_extract_file(file_paths=[str(google_doc_pdf), str(xerox_pdf)], config=config)
    else:
        results = await batch_extract_file(file_paths=[str(google_doc_pdf), str(xerox_pdf)])

    assert len(results) == 2
    for result in results:
        assert result.content is not None
        assert len(result.content) > 0


@pytest.mark.anyio
async def test_psm_mode_4_specifically(google_doc_pdf: Path) -> None:
    config = ExtractionConfig(
        ocr=TesseractConfig(psm=PSMMode.SINGLE_COLUMN),
    )

    result = await extract_file(str(google_doc_pdf), config=config)
    assert result.content is not None
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_batch_extract_bytes_regression(google_doc_pdf: Path, test_xls: Path) -> None:
    from kreuzberg import batch_extract_bytes

    pdf_content = google_doc_pdf.read_bytes()
    xls_content = test_xls.read_bytes()

    files_data = [
        (pdf_content, "application/pdf"),
        (xls_content, "application/vnd.ms-excel"),
    ]

    results = await batch_extract_bytes(files_data)

    assert len(results) == 2
    for result in results:
        assert result.content is not None
        assert len(result.content) > 0


@pytest.mark.parametrize("test_mode", ["async", "sync"])
@pytest.mark.anyio
async def test_issue_149_windows_tesseract_hocr_regression(test_mode: str, german_image_pdf: Path) -> None:
    import re

    pdf_path = german_image_pdf
    config = ExtractionConfig(force_ocr=True)

    if test_mode == "async":
        result = await extract_file(str(pdf_path), config=config)
    else:
        result = extract_file_sync(str(pdf_path), config=config)

    def normalize_whitespace(text: str) -> str:
        return re.sub(r"\s+", " ", text.strip())

    normalized_content = normalize_whitespace(result.content).lower()

    assert result.content is not None
    assert len(result.content) > 200, f"Expected substantial content, got {len(result.content)} chars"
    for expected_keyword in ("bayern", "heimatpflege", "landesverein"):
        assert expected_keyword in normalized_content
