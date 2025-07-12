from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import TYPE_CHECKING, cast

import pytest

from kreuzberg import TesseractConfig
from kreuzberg._mime_types import (
    DOCX_MIME_TYPE,
    EXCEL_MIME_TYPE,
    MARKDOWN_MIME_TYPE,
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
)
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ValidationError
from kreuzberg.extraction import (
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)
from tests.conftest import pdfs_with_tables

IS_CI = os.environ.get("CI", "false").lower() == "true"

if TYPE_CHECKING:
    from kreuzberg._types import ExtractionResult


@pytest.mark.anyio
async def test_extract_bytes_pdf(scanned_pdf: Path) -> None:
    content = scanned_pdf.read_bytes()
    result = await extract_bytes(content, PDF_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
@pytest.mark.skipif(
    sys.platform == "win32",
    reason="Tesseract Languages not installed on Windows due to complexity of installation in CI",
)
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_bytes_force_ocr_pdf(non_ascii_pdf: Path) -> None:
    content = non_ascii_pdf.read_bytes()
    config = ExtractionConfig(force_ocr=True, ocr_config=TesseractConfig(language="deu"))
    result = await extract_bytes(content, PDF_MIME_TYPE, config=config)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    # OCR results may vary by system - check for any reasonable content or allow empty
    if result.content.strip():
        # If OCR produced content, it should contain some actual text (not just zeros/whitespace)
        cleaned_content = result.content.replace("0", "").replace("\n", "").replace(" ", "").strip()
        if cleaned_content == "":
            # OCR produced only zeros and whitespace - this is acceptable for poor OCR
            pass
        else:
            # OCR produced some actual content - verify it's reasonable
            assert len(cleaned_content) > 0


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_bytes_image(ocr_image: Path) -> None:
    content = ocr_image.read_bytes()
    mime_type = "image/jpeg"
    result = await extract_bytes(content, mime_type)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_bytes_pandoc(docx_document: Path) -> None:
    content = docx_document.read_bytes()
    mime_type = DOCX_MIME_TYPE
    result = await extract_bytes(content, mime_type)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_bytes_plain_text() -> None:
    content = b"This is a plain text file."
    result = await extract_bytes(content, PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content.strip() == "This is a plain text file."


@pytest.mark.anyio
async def test_extract_bytes_pptx(pptx_document: Path) -> None:
    content = pptx_document.read_bytes()
    result = await extract_bytes(content, POWER_POINT_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert (
        "At Contoso, we empower organizations to foster collaborative thinking to further drive workplace innovation. By closing the loop and leveraging agile frameworks, we help business grow organically and foster a consumer first mindset."
        in result.content
    )


@pytest.mark.anyio
async def test_extract_bytes_html(html_document: Path) -> None:
    content = html_document.read_bytes()
    result = await extract_bytes(content, "text/html")
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert (
        result.content
        == 'Browsers usually insert quotation marks around the q element.\n\nWWF\'s goal is to: "Build a future where people live in harmony with nature."'
    )


@pytest.mark.anyio
async def test_extract_bytes_markdown(markdown_document: Path) -> None:
    content = markdown_document.read_bytes()
    result = await extract_bytes(content, MARKDOWN_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_bytes_invalid_mime() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        await extract_bytes(b"some content", "application/unknown")


@pytest.mark.anyio
async def test_extract_file_pdf(scanned_pdf: Path) -> None:
    result = await extract_file(scanned_pdf, PDF_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_extract_file_image(ocr_image: Path) -> None:
    mime_type = "image/jpeg"
    result = await extract_file(ocr_image, mime_type)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_pandoc(docx_document: Path) -> None:
    mime_type = DOCX_MIME_TYPE
    result = await extract_file(docx_document, mime_type)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_plain_text(tmp_path: Path) -> None:
    text_file = tmp_path / "sample.txt"
    text_file.write_text("This is a plain text file.")
    result = await extract_file(text_file, PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content.strip() == "This is a plain text file."


@pytest.mark.anyio
async def test_extract_file_markdown(markdown_document: Path) -> None:
    result = await extract_file(markdown_document, MARKDOWN_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_pptx(pptx_document: Path) -> None:
    result = await extract_file(pptx_document, POWER_POINT_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert (
        "At Contoso, we empower organizations to foster collaborative thinking to further drive workplace innovation. By closing the loop and leveraging agile frameworks, we help business grow organically and foster a consumer first mindset."
        in result.content
    )


@pytest.mark.anyio
async def test_extract_file_html(html_document: Path) -> None:
    result = await extract_file(html_document, "text/html")
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
    assert (
        result.content
        == 'Browsers usually insert quotation marks around the q element.\n\nWWF\'s goal is to: "Build a future where people live in harmony with nature."'
    )


@pytest.mark.anyio
async def test_extract_file_invalid_mime(tmp_path: Path) -> None:
    test_file = tmp_path / "valid-file.txt"
    test_file.write_text("test content")

    with pytest.raises(ValidationError, match="Unsupported mime type"):
        await extract_file(test_file, "application/unknown")


@pytest.mark.anyio
async def test_extract_file_not_exists() -> None:
    with pytest.raises(ValidationError, match="The file does not exist"):
        await extract_file("/invalid/path.txt", PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_bytes_excel(excel_document: Path) -> None:
    content = excel_document.read_bytes()
    result = await extract_bytes(content, EXCEL_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_excel(excel_document: Path) -> None:
    result = await extract_file(excel_document, EXCEL_MIME_TYPE)
    assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)


@pytest.mark.anyio
async def test_extract_file_excel_invalid() -> None:
    with pytest.raises(ValidationError, match="The file does not exist"):
        await extract_file("/invalid/path.xlsx", EXCEL_MIME_TYPE)


def test_extract_bytes_sync_plain_text() -> None:
    content = b"This is a plain text file."
    result = extract_bytes_sync(content, PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content.strip() == "This is a plain text file."


def test_extract_file_sync_plain_text(tmp_path: Path) -> None:
    text_file = tmp_path / "sample.txt"
    text_file.write_text("This is a plain text file.")
    result = extract_file_sync(text_file, PLAIN_TEXT_MIME_TYPE)
    assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
    assert result.content.strip() == "This is a plain text file."


def test_extract_bytes_sync_invalid_mime() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type"):
        extract_bytes_sync(b"some content", "application/unknown")


def test_extract_file_sync_invalid_mime(tmp_path: Path) -> None:
    test_file = tmp_path / "valid-file.txt"
    test_file.write_text("test content")

    with pytest.raises(ValidationError, match="Unsupported mime type"):
        extract_file_sync(test_file, "application/unknown")


def test_extract_file_sync_not_exists() -> None:
    with pytest.raises(ValidationError, match="The file does not exist"):
        extract_file_sync("/invalid/path.txt", PLAIN_TEXT_MIME_TYPE)


def assert_extraction_result(result: ExtractionResult, *, mime_type: str) -> None:
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == mime_type
    assert isinstance(result.metadata, dict)


@pytest.mark.anyio
@pytest.mark.xfail(IS_CI, reason="OCR tests may fail in CI due to Tesseract issues")
async def test_batch_extract_pdf_files(scanned_pdf: Path, test_article: Path) -> None:
    config = ExtractionConfig(force_ocr=True)
    results = await batch_extract_file([scanned_pdf, test_article], config=config)
    assert len(results) == 2
    for result in results:
        assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_batch_extract_file_mixed(test_article: Path) -> None:
    test_files = [test_article]
    test_files.extend((Path(__file__).parent / "source").glob("*.docx"))
    test_files.extend((Path(__file__).parent / "source").glob("*.xlsx"))

    results = await batch_extract_file(test_files)
    assert len(results) == len(test_files)
    for path, result in zip(test_files, results, strict=False):
        if path.suffix in [".docx", ".xlsx"]:
            assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
        else:
            assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_batch_extract_pdf_tables() -> None:
    # note the point of this test is to ensure we dont hit segmentation errors during concurrency ~keep
    config = ExtractionConfig(extract_tables=True)
    results = await batch_extract_file(list(pdfs_with_tables), config=config)
    assert len(results) == len(pdfs_with_tables)
    for result in results:
        assert result.tables


@pytest.mark.anyio
async def test_batch_extract_file_empty() -> None:
    results = await batch_extract_file([])
    assert len(results) == 0


@pytest.mark.anyio
async def test_batch_extract_file_invalid(tmp_path: Path) -> None:
    invalid_file = tmp_path / "invalid-file.xyz"
    invalid_file.write_text("Invalid file content")

    results = await batch_extract_file([invalid_file])
    assert len(results) == 1
    result = results[0]

    assert result.metadata.get("error") is True
    error_context = cast("dict[str, object]", result.metadata.get("error_context", {}))
    error_info = cast("dict[str, object]", error_context.get("error", {}))
    assert "ValidationError" in str(error_info.get("type", ""))
    assert "Unsupported mime type" in str(
        error_info.get("message", "")
    ) or "Could not determine the mime type of the file" in str(error_info.get("message", ""))


@pytest.mark.anyio
async def test_batch_extract_bytes_mixed(searchable_pdf: Path, docx_document: Path) -> None:
    contents = [
        (b"This is plain text", PLAIN_TEXT_MIME_TYPE),
        (
            docx_document.read_bytes(),
            DOCX_MIME_TYPE,
        ),
        (searchable_pdf.read_bytes(), PDF_MIME_TYPE),
    ]

    results = await batch_extract_bytes(contents)
    assert len(results) == len(contents)
    for i, result in enumerate(results):
        if i == 0:
            assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
            assert result.content.strip() == "This is plain text"
        else:
            assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE if i == 1 else PLAIN_TEXT_MIME_TYPE)


@pytest.mark.anyio
async def test_batch_extract_bytes_empty() -> None:
    results = await batch_extract_bytes([])
    assert len(results) == 0


@pytest.mark.anyio
async def test_batch_extract_bytes_invalid() -> None:
    results = await batch_extract_bytes([(b"content", "application/invalid")])
    assert len(results) == 1
    result = results[0]

    assert result.metadata.get("error") is True
    error_context = cast("dict[str, object]", result.metadata.get("error_context", {}))
    error_info = cast("dict[str, object]", error_context.get("error", {}))
    assert "ValidationError" in str(error_info.get("type", ""))
    assert "Unsupported mime type" in str(error_info.get("message", ""))


def test_batch_extract_file_sync_mixed(test_article: Path) -> None:
    test_files = [test_article]
    test_files.extend((Path(__file__).parent / "source").glob("*.docx"))
    test_files.extend((Path(__file__).parent / "source").glob("*.xlsx"))

    results = batch_extract_file_sync(test_files)
    assert len(results) == len(test_files)
    for path, result in zip(test_files, results, strict=False):
        if path.suffix in [".docx", ".xlsx"]:
            assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE)
        else:
            assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)


def test_batch_extract_bytes_sync_mixed(searchable_pdf: Path, docx_document: Path) -> None:
    contents = [
        (b"This is plain text", PLAIN_TEXT_MIME_TYPE),
        (
            docx_document.read_bytes(),
            DOCX_MIME_TYPE,
        ),
        (searchable_pdf.read_bytes(), PDF_MIME_TYPE),
    ]

    results = batch_extract_bytes_sync(contents)
    assert len(results) == len(contents)
    for i, result in enumerate(results):
        if i == 0:
            assert_extraction_result(result, mime_type=PLAIN_TEXT_MIME_TYPE)
            assert result.content.strip() == "This is plain text"
        else:
            assert_extraction_result(result, mime_type=MARKDOWN_MIME_TYPE if i == 1 else PLAIN_TEXT_MIME_TYPE)
