from __future__ import annotations

import sys
from datetime import date, datetime, time, timedelta, timezone
from pathlib import Path as SyncPath
from typing import TYPE_CHECKING, Any

import pytest
from kreuzberg import ExtractionResult, ParsingError
from kreuzberg._extractors._spread_sheet import SpreadSheetExtractor
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg.extraction import DEFAULT_CONFIG
from python_calamine import CalamineWorkbook

if sys.version_info < (3, 11):  # pragma: no cover
    from exceptiongroup import ExceptionGroup  # type: ignore[import-not-found]

if TYPE_CHECKING:
    from collections.abc import AsyncGenerator
    from pathlib import Path

    from kreuzberg._types import Metadata
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


def test_extract_path_sync_with_exception(
    extractor: SpreadSheetExtractor, excel_document: Path, mocker: MockerFixture
) -> None:
    mock_error = ValueError("Sync test error")
    mocker.patch.object(CalamineWorkbook, "from_path", side_effect=mock_error)

    with pytest.raises(ParsingError) as exc_info:
        extractor.extract_path_sync(excel_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert "error" in exc_info.value.context
    assert str(mock_error) in exc_info.value.context["error"]
    assert str(excel_document) in exc_info.value.context["file"]


def test_extract_path_sync_parsing_error_wrapping(
    extractor: SpreadSheetExtractor, excel_document: Path, mocker: MockerFixture
) -> None:
    original_error = ParsingError("Original sync parsing error")
    mocker.patch.object(CalamineWorkbook, "from_path", side_effect=original_error)

    with pytest.raises(ParsingError) as exc_info:
        extractor.extract_path_sync(excel_document)

    assert "Failed to extract file data" in str(exc_info.value)
    assert "Original sync parsing error" in str(exc_info.value.context["error"])


@pytest.mark.anyio
async def test_extract_bytes_async_exception_cleanup(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    from contextlib import asynccontextmanager
    from pathlib import Path

    cleanup_called = False

    async def mock_cleanup() -> None:
        nonlocal cleanup_called
        cleanup_called = True

    @asynccontextmanager
    async def mock_temp_file(extension: str, content: bytes | None = None) -> AsyncGenerator[Path, None]:
        try:
            yield Path("/tmp/test_excel.xlsx")
        finally:
            await mock_cleanup()

    mocker.patch("kreuzberg._extractors._spread_sheet.temporary_file", side_effect=mock_temp_file)

    mock_error = ValueError("Test extraction error")
    mocker.patch.object(extractor, "extract_path_async", side_effect=mock_error)

    test_content = b"fake excel content"

    with pytest.raises(ValueError, match="Test extraction error"):
        await extractor.extract_bytes_async(test_content)

    assert cleanup_called


def test_convert_sheet_to_text_sync_empty_rows(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()

    mock_sheet.to_python.return_value = [
        ["Header1", "Header2", "Header3"],
        ["Value1"],
        ["Value2", "Value3"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._convert_sheet_to_text_sync(mock_workbook, "test_sheet")

    assert "## test_sheet" in result
    assert "Header1 | Header2 | Header3" in result

    assert "Value1 | |" in result
    assert "Value2 | Value3" in result


def test_convert_sheet_to_text_sync_no_rows(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()

    mock_sheet.to_python.return_value = []
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._convert_sheet_to_text_sync(mock_workbook, "empty_sheet")

    assert result == "## empty_sheet\n\n"


def test_extract_spreadsheet_metadata_comprehensive(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1", "Sheet2", "Summary"]

    mock_metadata = mocker.Mock()
    mock_metadata.title = "Test Spreadsheet"
    mock_metadata.author = "Test Author"
    mock_metadata.subject = "Test Subject"
    mock_metadata.comments = "Test Comments"
    mock_metadata.keywords = "keyword1, keyword2; keyword3"
    mock_metadata.category = "Test Category"
    mock_metadata.company = "Test Company"
    mock_metadata.manager = "Test Manager"

    from datetime import datetime, timezone

    mock_metadata.created = datetime(2023, 1, 1, 12, 0, 0, tzinfo=timezone.utc)
    mock_metadata.modified = datetime(2023, 2, 1, 14, 30, 0, tzinfo=timezone.utc)

    mock_workbook.metadata = mock_metadata

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        ["Header1", "Header2", "Formula"],
        ["Value1", "Value2", "=A1+B1"],
        ["Value3", "Value4", "=SUM(A:A)"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._extract_spreadsheet_metadata(mock_workbook)

    assert result["title"] == "Test Spreadsheet"
    assert result["authors"] == ["Test Author"]
    assert result["subject"] == "Test Subject"
    assert result["comments"] == "Test Comments"
    assert result["keywords"] == ["keyword1", "keyword2", "keyword3"]
    assert result["categories"] == ["Test Category"]
    assert result["organization"] == "Test Company"
    assert result["modified_by"] == "Test Manager"
    assert result["created_at"] == "2023-01-01T12:00:00+00:00"
    assert result["modified_at"] == "2023-02-01T14:30:00+00:00"
    assert result["description"] == "Spreadsheet with 3 sheets: Sheet1, Sheet2, Summary"
    assert "includes formulas" in result["summary"]


def test_extract_spreadsheet_metadata_no_metadata(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1"]
    mock_workbook.metadata = None

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Data1", "Data2"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._extract_spreadsheet_metadata(mock_workbook)

    assert result["description"] == "Spreadsheet with 1 sheet: Sheet1"
    assert "summary" in result


def test_extract_spreadsheet_metadata_many_sheets(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1", "Sheet2", "Sheet3", "Sheet4", "Sheet5", "Sheet6", "Sheet7"]
    mock_workbook.metadata = None

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Data"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._extract_spreadsheet_metadata(mock_workbook)

    assert result["description"] == "Spreadsheet with 7 sheets"
    assert "Sheet1" not in result["description"]


def test_extract_document_properties_minimal(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_metadata = mocker.Mock()
    mock_metadata.title = "Simple Title"

    mock_metadata.author = None
    mock_metadata.subject = None
    mock_metadata.comments = None
    mock_metadata.keywords = None
    mock_metadata.category = None
    mock_metadata.company = None
    mock_metadata.manager = None
    mock_metadata.created = None
    mock_metadata.modified = None

    mock_workbook.metadata = mock_metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    assert metadata["title"] == "Simple Title"
    assert len(metadata) == 1


def test_extract_date_properties_string_dates(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_props = mocker.Mock()
    mock_props.created = "2023-01-01"
    mock_props.modified = "2023-02-01"

    metadata: Metadata = {}
    extractor._extract_date_properties(mock_props, metadata)

    assert metadata["created_at"] == "2023-01-01"
    assert metadata["modified_at"] == "2023-02-01"


def test_analyze_content_complexity_no_formulas(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Data"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        ["Header1", "Header2"],
        ["Data1", "Data2"],
        ["Data3", "Data4"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "Contains" in metadata["summary"]
    assert "formulas" not in metadata["summary"]


def test_analyze_content_complexity_empty_sheets(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Empty"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [[None, None], ["", ""]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "summary" not in metadata


def test_enhance_sheet_with_table_data_pandas_available(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Name", "Age", "City"], ["Alice", 25, "New York"], ["Bob", 30, "Chicago"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    mock_df = mocker.Mock()
    mock_df.empty = False
    mock_df.dropna.return_value = mock_df

    mock_enhance = mocker.patch("kreuzberg._extractors._spread_sheet.enhance_table_markdown")
    mock_enhance.return_value = "Enhanced table markdown"

    mocker.patch("pandas.DataFrame", return_value=mock_df)
    result = extractor._enhance_sheet_with_table_data(mock_workbook, "TestSheet")

    assert result == "## TestSheet\n\nEnhanced table markdown"


def test_enhance_sheet_with_table_data_empty_sheet(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = []
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._enhance_sheet_with_table_data(mock_workbook, "EmptySheet")

    assert result == "## EmptySheet\n\n*Empty sheet*"


def test_enhance_sheet_with_table_data_no_data_after_cleanup(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [[None, None], [None, None]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    mock_df = mocker.Mock()
    mock_df.empty = True
    mock_df.dropna.return_value = mock_df

    mocker.patch("pandas.DataFrame", return_value=mock_df)
    result = extractor._enhance_sheet_with_table_data(mock_workbook, "CleanedSheet")

    assert result == "## CleanedSheet\n\n*No data*"


def test_enhance_sheet_with_table_data_error_fallback(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Header"], ["Data"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    mocker.patch(
        "kreuzberg._extractors._spread_sheet.enhance_table_markdown", side_effect=ValueError("Enhancement error")
    )
    mocker.patch.object(extractor, "_convert_sheet_to_text_sync", return_value="Fallback content")

    result = extractor._enhance_sheet_with_table_data(mock_workbook, "FallbackSheet")

    assert result == "Fallback content"


def test_extract_document_properties_no_hasattr(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    del mock_workbook.metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    assert len(metadata) == 0


def test_extract_document_properties_exception_handling(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_metadata = mocker.Mock()

    mock_metadata.title = mocker.PropertyMock(side_effect=Exception("Property access error"))
    mock_workbook.metadata = mock_metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    assert isinstance(metadata, dict)


def test_extract_date_properties_invalid_dates(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_props = mocker.Mock()
    mock_props.created = mocker.Mock()
    mock_props.created.isoformat = mocker.Mock(side_effect=Exception("Invalid date"))
    mock_props.modified = None

    metadata: Metadata = {}
    extractor._extract_date_properties(mock_props, metadata)

    assert "created_at" not in metadata
    assert "modified_at" not in metadata


def test_add_structure_info_no_sheet_names(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    del mock_workbook.sheet_names

    metadata: Metadata = {}
    extractor._add_structure_info(mock_workbook, metadata)

    assert "description" not in metadata


def test_add_structure_info_empty_sheet_names(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = []

    metadata: Metadata = {}
    extractor._add_structure_info(mock_workbook, metadata)

    assert "description" not in metadata


def test_analyze_content_complexity_exception_handling(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["ErrorSheet"]

    mock_workbook.get_sheet_by_name.side_effect = Exception("Sheet access error")

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert isinstance(metadata, dict)


def test_analyze_content_complexity_with_empty_rows(extractor: SpreadSheetExtractor, mocker: MockerFixture) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["MixedSheet"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        [],
        ["Header1", "Header2"],
        [None, None],
        ["", ""],
        ["Data1", "Data2"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "summary" in metadata
    assert "Contains" in metadata["summary"]


def test_spread_sheet_extractor_comprehensive_cell_conversion_convert_cell_to_str_complex_numeric_types(
    extractor: SpreadSheetExtractor,
) -> None:
    complex_num = complex(3, 4)
    result = extractor._convert_cell_to_str(complex_num)
    assert result == "(3+4j)"

    large_int = 10**20
    result = extractor._convert_cell_to_str(large_int)
    assert result == "100000000000000000000"

    precise_float = 3.141592653589793
    result = extractor._convert_cell_to_str(precise_float)
    assert result == "3.141592653589793"


def test_spread_sheet_extractor_comprehensive_cell_conversion_convert_cell_to_str_edge_case_objects(
    extractor: SpreadSheetExtractor,
) -> None:
    test_list = [1, 2, 3]
    result = extractor._convert_cell_to_str(test_list)
    assert result == "[1, 2, 3]"

    test_dict = {"key": "value"}
    result = extractor._convert_cell_to_str(test_dict)
    assert result == "{'key': 'value'}"

    class CustomObject:
        def __str__(self) -> str:
            return "custom_object"

    custom_obj = CustomObject()
    result = extractor._convert_cell_to_str(custom_obj)
    assert result == "custom_object"


def test_spread_sheet_extractor_comprehensive_cell_conversion_convert_cell_to_str_datetime_variants(
    extractor: SpreadSheetExtractor,
) -> None:
    from datetime import datetime, time, timedelta, timezone

    dt_with_microseconds = datetime(2023, 1, 1, 12, 30, 45, 123456, tzinfo=timezone.utc)
    result = extractor._convert_cell_to_str(dt_with_microseconds)
    assert result == "2023-01-01T12:30:45.123456+00:00"

    time_with_microseconds = time(12, 30, 45, 123456)
    result = extractor._convert_cell_to_str(time_with_microseconds)
    assert result == "12:30:45.123456"

    negative_delta = timedelta(days=-1, seconds=-3600)
    result = extractor._convert_cell_to_str(negative_delta)
    assert result == "-90000.0 seconds"

    zero_delta = timedelta(0)
    result = extractor._convert_cell_to_str(zero_delta)
    assert result == "0.0 seconds"


def test_spread_sheet_extractor_sync_extensive_error_handling_extract_bytes_sync_file_write_error(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    test_content = b"fake excel data"

    mock_fd = 5
    mock_temp_path = "/tmp/test.xlsx"
    mocker.patch("tempfile.mkstemp", return_value=(mock_fd, mock_temp_path))

    mock_fdopen = mocker.patch("os.fdopen")
    mock_file = mocker.Mock()
    mock_file.write.side_effect = OSError("Write error")
    mock_fdopen.return_value.__enter__.return_value = mock_file

    mock_unlink = mocker.patch("pathlib.Path.unlink")

    with pytest.raises(OSError, match="Write error"):
        extractor.extract_bytes_sync(test_content)

    mock_unlink.assert_called_once()


def test_spread_sheet_extractor_sync_extensive_error_handling_extract_bytes_sync_cleanup_error_suppressed(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    test_content = b"fake excel data"

    mock_fd = 5
    mock_temp_path = "/tmp/test.xlsx"
    mocker.patch("tempfile.mkstemp", return_value=(mock_fd, mock_temp_path))

    mock_fdopen = mocker.patch("os.fdopen")
    mock_file = mocker.Mock()
    mock_fdopen.return_value.__enter__.return_value = mock_file

    mock_result = ExtractionResult(content="test", mime_type=MARKDOWN_MIME_TYPE, metadata={})
    mocker.patch.object(extractor, "extract_path_sync", return_value=mock_result)

    mock_unlink = mocker.patch("pathlib.Path.unlink", side_effect=OSError("Cleanup error"))

    result = extractor.extract_bytes_sync(test_content)
    assert result == mock_result
    mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_spreadsheet_convert_sheet_to_text_csv_processing_edge_cases(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()

    mock_sheet.to_python.return_value = [
        ["Header1", "Header,with,commas", 'Header"with"quotes'],
        ["Value1", "Value,with,commas", 'Value"with"quotes'],
        ["Value\nwith\nnewlines", "Value\twith\ttabs", "Value'with'apostrophes"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    temp_path = "/tmp/test.csv"
    mock_unlink = mocker.AsyncMock()
    mocker.patch("kreuzberg._extractors._spread_sheet.create_temp_file", return_value=(temp_path, mock_unlink))

    # Mock AsyncPath.write_text for CSV conversion test - legitimately needed to test sheet-to-CSV conversion ~keep
    mock_write_text = mocker.AsyncMock()
    mocker.patch("kreuzberg._extractors._spread_sheet.AsyncPath.write_text", mock_write_text)

    result = await extractor._convert_sheet_to_text(mock_workbook, "test_sheet")

    assert "## test_sheet" in result
    assert "Header1" in result
    assert "Value1" in result
    mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_spreadsheet_convert_sheet_to_text_uneven_rows(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()

    mock_sheet.to_python.return_value = [
        ["A", "B", "C", "D", "E"],
        ["1"],
        ["2", "3"],
        ["4", "5", "6", "7", "8", "9", "10"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    temp_path = "/tmp/test.csv"
    mock_unlink = mocker.AsyncMock()
    mocker.patch("kreuzberg._extractors._spread_sheet.create_temp_file", return_value=(temp_path, mock_unlink))
    # Mock AsyncPath.write_text for uneven sheet test - legitimately needed to test sheet-to-CSV conversion ~keep
    mocker.patch("kreuzberg._extractors._spread_sheet.AsyncPath.write_text", mocker.AsyncMock())

    result = await extractor._convert_sheet_to_text(mock_workbook, "uneven_sheet")

    assert "## uneven_sheet" in result
    assert "A | B | C | D | E" in result
    assert "1 | | | |" in result
    assert "2 | 3 | | |" in result
    mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_spreadsheet_convert_sheet_to_text_all_none_cells(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()

    mock_sheet.to_python.return_value = [
        [None, None, None],
        [None, None, None],
        [None, None, None],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    temp_path = "/tmp/test.csv"
    mock_unlink = mocker.AsyncMock()
    mocker.patch("kreuzberg._extractors._spread_sheet.create_temp_file", return_value=(temp_path, mock_unlink))
    # Mock AsyncPath.write_text for None sheet test - legitimately needed to test sheet-to-CSV conversion ~keep
    mocker.patch("kreuzberg._extractors._spread_sheet.AsyncPath.write_text", mocker.AsyncMock())

    result = await extractor._convert_sheet_to_text(mock_workbook, "none_sheet")

    assert "## none_sheet" in result
    assert " | | " in result
    mock_unlink.assert_called_once()


def test_spreadsheet_metadata_extract_document_properties_partial_metadata(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_metadata = mocker.Mock()

    mock_metadata.title = "Partial Title"
    mock_metadata.author = "Partial Author"
    for attr in ["subject", "comments", "keywords", "category", "company", "manager", "created", "modified"]:
        setattr(mock_metadata, attr, None)

    mock_workbook.metadata = mock_metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    assert metadata["title"] == "Partial Title"
    assert metadata["authors"] == ["Partial Author"]
    assert len(metadata) == 2


def test_spreadsheet_metadata_extract_document_properties_empty_strings(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_metadata = mocker.Mock()

    mock_metadata.title = ""
    mock_metadata.author = ""
    mock_metadata.subject = ""
    mock_metadata.keywords = ""
    mock_metadata.comments = "Valid comment"
    mock_metadata.category = ""
    mock_metadata.company = ""
    mock_metadata.manager = ""
    mock_metadata.created = None
    mock_metadata.modified = None

    mock_workbook.metadata = mock_metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    assert metadata["comments"] == "Valid comment"
    assert len(metadata) == 1


def test_spreadsheet_metadata_extract_document_properties_complex_keywords(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_metadata = mocker.Mock()

    mock_metadata.keywords = "  keyword1,  keyword2;keyword3  ,, ; keyword4;  ,keyword5  "
    mock_workbook.metadata = mock_metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    expected_keywords = ["keyword1", "keyword2", "keyword3", "keyword4", "keyword5"]
    assert metadata["keywords"] == expected_keywords


def test_spreadsheet_metadata_extract_document_properties_keywords_empty_after_processing(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_metadata = mocker.Mock()

    mock_metadata.keywords = "  ,, ; ;;  , ,  "
    mock_workbook.metadata = mock_metadata

    metadata: Metadata = {}
    extractor._extract_document_properties(mock_workbook, metadata)

    assert "keywords" not in metadata


def test_spreadsheet_metadata_extract_date_properties_non_datetime_objects(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_props = mocker.Mock()

    mock_date_like = mocker.Mock()
    mock_date_like.isoformat.return_value = "2023-12-25T10:30:00"
    mock_props.created = mock_date_like

    mock_props.modified = "string_date"

    metadata: Metadata = {}
    extractor._extract_date_properties(mock_props, metadata)

    assert metadata["created_at"] == "2023-12-25T10:30:00"
    assert metadata["modified_at"] == "string_date"


def test_spreadsheet_structure_add_structure_info_single_sheet(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["OnlySheet"]

    metadata: Metadata = {}
    extractor._add_structure_info(mock_workbook, metadata)

    assert metadata["description"] == "Spreadsheet with 1 sheet: OnlySheet"
    assert "sheets" not in metadata["description"]


def test_spreadsheet_structure_add_structure_info_exactly_max_sheets(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1", "Sheet2", "Sheet3", "Sheet4", "Sheet5"]

    metadata: Metadata = {}
    extractor._add_structure_info(mock_workbook, metadata)

    expected_desc = "Spreadsheet with 5 sheets: Sheet1, Sheet2, Sheet3, Sheet4, Sheet5"
    assert metadata["description"] == expected_desc


def test_spreadsheet_structure_add_structure_info_one_over_max_sheets(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1", "Sheet2", "Sheet3", "Sheet4", "Sheet5", "Sheet6"]

    metadata: Metadata = {}
    extractor._add_structure_info(mock_workbook, metadata)

    assert metadata["description"] == "Spreadsheet with 6 sheets"
    assert "Sheet1" not in metadata["description"]


def test_spreadsheet_complexity_analyze_content_complexity_formula_detection_edge_cases(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["FormulaSheet"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        ["Header", "Not Formula", "Formula"],
        ["Data", "=not a formula at start", "=SUM(A:A)"],
        ["More", "Text with = sign", "=IF(A1>0,1,0)"],
        ["Values", "String=value", "=VLOOKUP(A1,B:C,2,FALSE)"],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "includes formulas" in metadata["summary"]
    assert "Contains" in metadata["summary"]


def test_spreadsheet_complexity_analyze_content_complexity_no_meaningful_cells(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["EmptyDataSheet"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        [None, "", "   "],
        ["", None, "  \t  "],
        [None, None, None],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "summary" not in metadata


def test_spreadsheet_complexity_analyze_content_complexity_row_processing_limits(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["LargeSheet"]

    large_data = [["Header"]] + [[f"Row{i}"] for i in range(100)]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = large_data
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "summary" in metadata
    assert "Contains" in metadata["summary"]


def test_spreadsheet_complexity_analyze_content_complexity_sheet_processing_limits(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["Sheet1", "Sheet2", "Sheet3", "Sheet4", "Sheet5"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Header"], ["Data"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert "summary" in metadata
    assert "Contains" in metadata["summary"]


def test_spreadsheet_complexity_analyze_content_complexity_existing_summary_preserved(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_workbook.sheet_names = ["DataSheet"]

    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Header"], ["Data"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    metadata: Metadata = {"summary": "Existing summary"}
    extractor._analyze_content_complexity(mock_workbook, metadata)

    assert metadata["summary"] == "Existing summary"


def test_spreadsheet_table_enhance_sheet_with_table_data_pandas_dataframe_operations(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        ["Name", "Age", None, "City"],
        ["Alice", 25, None, "New York"],
        [None, None, None, None],
        ["Bob", 30, None, "Chicago"],
        [None, None, None, None],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    mock_df_initial = mocker.Mock()
    mock_df_filtered = mocker.Mock()
    mock_df_final = mocker.Mock()

    mock_df_initial.filter.return_value = mock_df_filtered

    mock_df_filtered.columns = ["column_0", "column_1", "column_2", "column_3"]
    mock_col = mocker.Mock()
    mock_col.is_null.return_value.all.return_value = False
    mock_df_filtered.__getitem__ = mocker.Mock(return_value=mock_col)
    mock_df_filtered.select.return_value = mock_df_final

    mock_df_final.is_empty.return_value = False

    mock_enhance = mocker.patch("kreuzberg._extractors._spread_sheet.enhance_table_markdown")
    mock_enhance.return_value = "Enhanced table with cleaned data"

    mocker.patch("PIL.Image.new")

    mocker.patch("polars.DataFrame", return_value=mock_df_initial)
    result = extractor._enhance_sheet_with_table_data(mock_workbook, "CleanedSheet")

    assert result == "## CleanedSheet\n\nEnhanced table with cleaned data"
    assert mock_df_initial.filter.call_count == 1
    assert mock_df_filtered.select.call_count == 1


def test_spreadsheet_table_enhance_sheet_with_table_data_value_error_fallback(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Header"], ["Data"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    mocker.patch("polars.DataFrame", side_effect=ValueError("DataFrame creation error"))
    mocker.patch.object(extractor, "_convert_sheet_to_text_sync", return_value="Fallback after ValueError")

    result = extractor._enhance_sheet_with_table_data(mock_workbook, "ErrorSheet")

    assert result == "Fallback after ValueError"


def test_spreadsheet_table_enhance_sheet_with_table_data_attribute_error_fallback(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [["Header"], ["Data"]]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    mock_df = mocker.Mock()
    mock_df.filter.side_effect = AttributeError("filter method not found")

    mocker.patch("polars.DataFrame", return_value=mock_df)
    mocker.patch.object(extractor, "_convert_sheet_to_text_sync", return_value="Fallback after AttributeError")

    result = extractor._enhance_sheet_with_table_data(mock_workbook, "AttributeErrorSheet")

    assert result == "Fallback after AttributeError"


def test_spreadsheet_table_enhance_sheet_with_table_data_contains_only_empty_rows(
    extractor: SpreadSheetExtractor, mocker: MockerFixture
) -> None:
    mock_workbook = mocker.Mock(spec=CalamineWorkbook)
    mock_sheet = mocker.Mock()
    mock_sheet.to_python.return_value = [
        [],
        [None, None, None],
        ["", "", ""],
    ]
    mock_workbook.get_sheet_by_name.return_value = mock_sheet

    result = extractor._enhance_sheet_with_table_data(mock_workbook, "OnlyEmptyRows")

    assert "## OnlyEmptyRows" in result
    assert "|" in result
