from __future__ import annotations

from pathlib import Path as SyncPath
from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionResult
from kreuzberg._extractors._spread_sheet import SpreadSheetExtractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def extractor() -> SpreadSheetExtractor:
    return SpreadSheetExtractor(
        mime_type="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", config=DEFAULT_CONFIG
    )


@pytest.mark.anyio
async def test_extract_xlsx_file(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    result = await extractor.extract_path_async(excel_document)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"


@pytest.mark.anyio
async def test_extract_xlsx_multi_sheet_file(excel_multi_sheet_document: Path, extractor: SpreadSheetExtractor) -> None:
    result = await extractor.extract_path_async(excel_multi_sheet_document)
    assert isinstance(result, ExtractionResult)
    assert result.mime_type == MARKDOWN_MIME_TYPE

    assert "## first_sheet" in result.content
    assert "## second_sheet" in result.content
    assert "Column 1" in result.content
    assert "Column 2" in result.content
    assert "Product" in result.content
    assert "Value" in result.content

    assert "| --- | --- |" in result.content
    assert "1.0" in result.content
    assert "2.0" in result.content


def test_extract_bytes_sync(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    content = SyncPath(excel_document).read_bytes()
    result = extractor.extract_bytes_sync(content)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert result.content
    assert "##" in result.content


def test_extract_path_sync(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    result = extractor.extract_path_sync(excel_document)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert result.content
    assert "##" in result.content


def test_extract_path_sync_with_metadata(excel_multi_sheet_document: Path, extractor: SpreadSheetExtractor) -> None:
    result = extractor.extract_path_sync(excel_multi_sheet_document)

    assert isinstance(result, ExtractionResult)
    assert result.metadata

    assert "sheet_count" in result.metadata
    assert result.metadata["sheet_count"] == "2"
    assert "total_cells" in result.metadata
    assert "description" in result.metadata
    assert "2 sheets" in result.metadata["description"]
    assert "summary" in result.metadata


def test_bytes_and_path_extraction_consistency(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    path_result = extractor.extract_path_sync(excel_document)

    content = SyncPath(excel_document).read_bytes()
    bytes_result = extractor.extract_bytes_sync(content)

    assert path_result.content == bytes_result.content
    assert path_result.mime_type == bytes_result.mime_type
    assert "sheet_count" in path_result.metadata
    assert "sheet_count" in bytes_result.metadata


@pytest.mark.anyio
async def test_async_sync_consistency(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    async_result = await extractor.extract_path_async(excel_document)
    sync_result = extractor.extract_path_sync(excel_document)

    assert async_result.content == sync_result.content
    assert async_result.metadata == sync_result.metadata


def test_extract_path_sync_with_invalid_file(extractor: SpreadSheetExtractor, tmp_path: Path) -> None:
    invalid_file = tmp_path / "not_an_excel_file.txt"
    invalid_file.write_text("This is not Excel data")

    with pytest.raises(OSError, match="Cannot detect file format"):
        extractor.extract_path_sync(invalid_file)


def test_extract_bytes_sync_with_invalid_data(extractor: SpreadSheetExtractor) -> None:
    invalid_data = b"This is not Excel data"

    with pytest.raises(OSError, match=r"(Zip error|Cannot detect file format)"):
        extractor.extract_bytes_sync(invalid_data)


def test_get_file_extension_mapping(extractor: SpreadSheetExtractor) -> None:
    test_cases = [
        ("application/vnd.ms-excel", ".xls"),
        ("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", ".xlsx"),
        ("application/vnd.ms-excel.sheet.macroEnabled.12", ".xlsm"),
        ("application/vnd.ms-excel.sheet.binary.macroEnabled.12", ".xlsb"),
        ("application/vnd.ms-excel.addin.macroEnabled.12", ".xlam"),
        ("application/vnd.ms-excel.template.macroEnabled.12", ".xltm"),
        ("application/vnd.oasis.opendocument.spreadsheet", ".ods"),
        ("unknown/mime-type", ".xlsx"),
    ]

    for mime_type, expected_ext in test_cases:
        extractor.mime_type = mime_type
        assert extractor._get_file_extension() == expected_ext


def test_rust_excel_implementation_performance(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    import time

    start_time = time.time()
    result = extractor.extract_path_sync(excel_document)
    end_time = time.time()

    processing_time = end_time - start_time
    assert processing_time < 1.0

    assert isinstance(result.content, str)
    assert result.content.strip()

    if "##" in result.content:
        assert "|" in result.content
        assert "---" in result.content


def test_quality_processing_preserves_table_separators(
    excel_multi_sheet_document: Path, extractor: SpreadSheetExtractor
) -> None:
    result = extractor.extract_path_sync(excel_multi_sheet_document)

    assert "| --- | --- |" in result.content
    assert "| ... | ... |" not in result.content


@pytest.mark.parametrize(
    "mime_type",
    [
        "application/vnd.ms-excel",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.ms-excel.sheet.macroEnabled.12",
        "application/vnd.oasis.opendocument.spreadsheet",
    ],
)
def test_supported_mime_types(mime_type: str) -> None:
    extractor = SpreadSheetExtractor(mime_type=mime_type, config=DEFAULT_CONFIG)
    assert extractor.mime_type == mime_type
    assert extractor._get_file_extension().startswith(".")


def test_invalid_file_error_handling(extractor: SpreadSheetExtractor) -> None:
    from pathlib import Path

    non_existent = Path("/tmp/does_not_exist.xlsx")

    with pytest.raises(OSError, match="No such file or directory"):
        extractor.extract_path_sync(non_existent)
