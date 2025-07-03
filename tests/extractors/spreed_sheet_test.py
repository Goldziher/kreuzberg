from __future__ import annotations

import sys
from datetime import date, datetime, time, timedelta, timezone
from pathlib import Path as SyncPath
from typing import TYPE_CHECKING, Any

import pytest
from python_calamine import CalamineWorkbook

from kreuzberg import ExtractionResult, ParsingError
from kreuzberg._extractors._spread_sheet import SpreadSheetExtractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg.extraction import DEFAULT_CONFIG

if sys.version_info < (3, 11):  # pragma: no cover
    from exceptiongroup import ExceptionGroup  # type: ignore[import-not-found]

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


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

    sheets = result.content.split("\n\n")
    assert len(sheets) == 4

    assert sheets[0] == "## first_sheet"
    first_sheet_content = sheets[1]
    assert "Column 1" in first_sheet_content
    assert "Column 2" in first_sheet_content
    assert "a" in first_sheet_content
    assert "1.0" in first_sheet_content
    assert "b" in first_sheet_content
    assert "2.0" in first_sheet_content
    assert "c" in first_sheet_content
    assert "3.0" in first_sheet_content

    assert sheets[2] == "## second_sheet"
    second_sheet_content = sheets[3]
    assert "Product" in second_sheet_content
    assert "Value" in second_sheet_content
    assert "Tomato" in second_sheet_content
    assert "Potato" in second_sheet_content
    assert "Beetroot" in second_sheet_content
    assert "1.0" in second_sheet_content
    assert "2.0" in second_sheet_content


@pytest.mark.anyio
async def test_extract_xlsx_file_exception_group(
    mocker: MockerFixture, excel_multi_sheet_document: Path, extractor: SpreadSheetExtractor
) -> None:
    mock_err = ParsingError(
        "Failed to extract file data",
        context={"file": str(excel_multi_sheet_document), "errors": [ValueError("Error 1"), ValueError("Error 2")]},
    )
    mocker.patch.object(extractor, "extract_path_async", side_effect=mock_err)

    with pytest.raises(ParsingError) as exc_info:
        await extractor.extract_path_async(excel_multi_sheet_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert len(exc_info.value.context["errors"]) == 2


@pytest.mark.anyio
async def test_extract_xlsx_file_general_exception(
    mocker: MockerFixture, excel_document: Path, extractor: SpreadSheetExtractor
) -> None:
    mock_error = ValueError("Test error")
    mocker.patch.object(CalamineWorkbook, "from_path", side_effect=mock_error)

    with pytest.raises(ParsingError) as exc_info:
        await extractor.extract_path_async(excel_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert str(mock_error) in str(exc_info.value.context["error"])


@pytest.mark.anyio
async def test_extract_xlsx_file_parsing_error_passthrough(
    mocker: MockerFixture, excel_document: Path, extractor: SpreadSheetExtractor
) -> None:
    original_error = ParsingError("Original parsing error")
    mocker.patch.object(CalamineWorkbook, "from_path", side_effect=original_error)

    with pytest.raises(ParsingError) as exc_info:
        await extractor.extract_path_async(excel_document)

    assert exc_info.value is original_error


def test_extract_bytes_sync(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    content = SyncPath(excel_document).read_bytes()
    result = extractor.extract_bytes_sync(content)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert result.content


def test_extract_path_sync(excel_document: Path, extractor: SpreadSheetExtractor) -> None:
    result = extractor.extract_path_sync(excel_document)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == MARKDOWN_MIME_TYPE
    assert result.content


@pytest.mark.parametrize(
    "value,expected",
    [
        (None, ""),
        (True, "true"),
        (False, "false"),
        (date(2023, 1, 1), "2023-01-01"),
        (time(12, 30, 45), "12:30:45"),
        (datetime(2023, 1, 1, 12, 30, 45, tzinfo=timezone.utc), "2023-01-01T12:30:45+00:00"),
        (timedelta(seconds=3600), "3600.0 seconds"),
        (123, "123"),
        ("test", "test"),
    ],
)
def test_convert_cell_to_str(extractor: SpreadSheetExtractor, value: Any, expected: str) -> None:
    result = extractor._convert_cell_to_str(value)
    assert result == expected


@pytest.mark.anyio
async def test_convert_sheet_to_text_with_missing_cells(mocker: MockerFixture, extractor: SpreadSheetExtractor) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        ["Header1", "Header2", "Header3"],
        ["Value1", "Value2"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = await extractor._convert_sheet_to_text(mock_workbook, "test_sheet")

    assert "## test_sheet" in result
    assert "Header1 | Header2 | Header3" in result
    assert "Value1 | Value2 |" in result


@pytest.mark.anyio
async def test_convert_sheet_to_text_empty_sheet(mocker: MockerFixture, extractor: SpreadSheetExtractor) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = []
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = await extractor._convert_sheet_to_text(mock_workbook, "empty_sheet")

    assert "## empty_sheet" in result
    assert result.strip() == "## empty_sheet"


@pytest.mark.anyio
async def test_exception_group_handling(
    mocker: MockerFixture, excel_document: Path, extractor: SpreadSheetExtractor
) -> None:
    exceptions = [ValueError("Error 1"), RuntimeError("Error 2")]
    eg = ExceptionGroup("test errors", exceptions)

    async def mock_run_taskgroup(*args: Any) -> None:
        raise eg

    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1", "Sheet2"]

    mocker.patch("kreuzberg._extractors._spread_sheet.run_taskgroup", side_effect=mock_run_taskgroup)
    mocker.patch.object(CalamineWorkbook, "from_path", return_value=mock_workbook)

    with pytest.raises(ParsingError) as exc_info:
        await extractor.extract_path_async(excel_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert "errors" in exc_info.value.context

    errors = exc_info.value.context["errors"]
    assert len(errors) == 2
    assert any(isinstance(err, ValueError) and "Error 1" in str(err) for err in errors)
    assert any(isinstance(err, RuntimeError) and "Error 2" in str(err) for err in errors)


@pytest.mark.anyio
async def test_extract_path_async_with_regular_exception(
    mocker: MockerFixture, excel_document: Path, extractor: SpreadSheetExtractor
) -> None:
    mock_error = ValueError("Test error")

    mocker.patch.object(CalamineWorkbook, "from_path", side_effect=mock_error)

    with pytest.raises(ParsingError) as exc_info:
        await extractor.extract_path_async(excel_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert "error" in exc_info.value.context
    assert str(mock_error) in exc_info.value.context["error"]


@pytest.mark.anyio
async def test_extract_path_async_parsing_error_passthrough(
    mocker: MockerFixture, excel_document: Path, extractor: SpreadSheetExtractor
) -> None:
    original_error = ParsingError("Original parsing error")

    mocker.patch.object(CalamineWorkbook, "from_path", side_effect=original_error)

    with pytest.raises(ParsingError) as exc_info:
        await extractor.extract_path_async(excel_document)

    assert exc_info.value is original_error
