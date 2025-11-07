from __future__ import annotations

import io
import queue
import signal
from typing import Any
from unittest.mock import AsyncMock, Mock, patch

import numpy as np
import polars as pl
import pytest
from kreuzberg._gmft import (
    _dataframe_to_csv,
    _dataframe_to_markdown,
    _extract_tables_in_process,
    _extract_tables_isolated,
    _extract_tables_isolated_async,
    _is_dataframe_empty,
    _pandas_to_polars,
    extract_tables,
    extract_tables_sync,
)
from kreuzberg._types import GMFTConfig, TableData
from kreuzberg.exceptions import MissingDependencyError, ParsingError
from PIL import Image


def test_pandas_to_polars_with_none() -> None:
    result = _pandas_to_polars(None)
    assert isinstance(result, pl.DataFrame)
    assert result.is_empty()


def test_pandas_to_polars_with_valid_dataframe() -> None:
    mock_pandas_df = Mock()
    mock_polars_df = Mock(spec=pl.DataFrame)

    with patch("polars.from_pandas", return_value=mock_polars_df) as mock_from_pandas:
        result = _pandas_to_polars(mock_pandas_df)

    assert result == mock_polars_df
    mock_from_pandas.assert_called_once_with(mock_pandas_df)


def test_pandas_to_polars_with_conversion_error_and_duplicate_columns() -> None:
    mock_pandas_df = Mock()
    mock_pandas_df.columns = Mock()
    mock_pandas_df.columns.duplicated.return_value = np.array([False, True, False])

    mock_filtered_df = Mock()
    mock_pandas_df.loc = Mock()
    mock_pandas_df.loc.__getitem__ = Mock(return_value=mock_filtered_df)

    mock_polars_df = Mock(spec=pl.DataFrame)

    with patch("polars.from_pandas") as mock_from_pandas:
        mock_from_pandas.side_effect = [ValueError("Duplicate columns"), mock_polars_df]

        result = _pandas_to_polars(mock_pandas_df)

    assert result == mock_polars_df
    assert mock_from_pandas.call_count == 2
    mock_pandas_df.columns.duplicated.assert_called_once()


def test_pandas_to_polars_with_conversion_error_no_columns_attr() -> None:
    mock_pandas_df = Mock()
    del mock_pandas_df.columns

    with patch("polars.from_pandas", side_effect=TypeError("Conversion failed")):
        result = _pandas_to_polars(mock_pandas_df)

    assert isinstance(result, pl.DataFrame)
    assert result.is_empty()


def test_pandas_to_polars_with_attribute_error() -> None:
    mock_pandas_df = Mock()
    del mock_pandas_df.columns

    with patch("polars.from_pandas", side_effect=AttributeError("Missing attribute")):
        result = _pandas_to_polars(mock_pandas_df)

    assert isinstance(result, pl.DataFrame)
    assert result.is_empty()


def test_dataframe_to_markdown_with_none() -> None:
    result = _dataframe_to_markdown(None)
    assert result == ""


def test_dataframe_to_markdown_with_empty_polars_dataframe() -> None:
    empty_df = pl.DataFrame()
    result = _dataframe_to_markdown(empty_df)
    assert result == ""


def test_dataframe_to_markdown_with_polars_dataframe() -> None:
    df = pl.DataFrame({"col1": [1, 2], "col2": [3, 4]})
    result = _dataframe_to_markdown(df)
    assert "col1" in result
    assert "col2" in result
    assert str(df) in result


def test_dataframe_to_markdown_with_pandas_dataframe() -> None:
    mock_df = Mock()
    mock_df.to_markdown.return_value = "| col1 | col2 |\n|------|------|\n| 1    | 2    |"

    result = _dataframe_to_markdown(mock_df)

    assert result == "| col1 | col2 |\n|------|------|\n| 1    | 2    |"
    mock_df.to_markdown.assert_called_once()


def test_dataframe_to_markdown_with_object_without_to_markdown() -> None:
    mock_df = Mock()
    del mock_df.to_markdown

    result = _dataframe_to_markdown(mock_df)

    assert result == str(mock_df)


def test_dataframe_to_csv_with_none() -> None:
    result = _dataframe_to_csv(None)
    assert result == ""


def test_dataframe_to_csv_with_empty_polars_dataframe() -> None:
    empty_df = pl.DataFrame()
    result = _dataframe_to_csv(empty_df)
    assert result == ""


def test_dataframe_to_csv_with_polars_dataframe() -> None:
    df = pl.DataFrame({"col1": [1, 2], "col2": [3, 4]})
    result = _dataframe_to_csv(df)

    assert "col1,col2" in result
    assert "1,3" in result
    assert "2,4" in result


def test_dataframe_to_csv_with_pandas_dataframe() -> None:
    mock_df = Mock()
    mock_df.to_csv.return_value = "col1,col2\n1,2\n3,4\n"

    result = _dataframe_to_csv(mock_df)

    assert result == "col1,col2\n1,2\n3,4\n"
    mock_df.to_csv.assert_called_once_with(index=False)


def test_dataframe_to_csv_with_object_without_to_csv() -> None:
    mock_df = Mock()
    del mock_df.to_csv

    result = _dataframe_to_csv(mock_df)

    assert result == ""


def test_is_dataframe_empty_with_none() -> None:
    result = _is_dataframe_empty(None)
    assert result is True


def test_is_dataframe_empty_with_empty_polars_dataframe() -> None:
    empty_df = pl.DataFrame()
    result = _is_dataframe_empty(empty_df)
    assert result is True


def test_is_dataframe_empty_with_nonempty_polars_dataframe() -> None:
    df = pl.DataFrame({"col1": [1, 2]})
    result = _is_dataframe_empty(df)
    assert result is False


def test_is_dataframe_empty_with_pandas_dataframe_empty() -> None:
    mock_df = Mock()
    mock_df.empty = True

    result = _is_dataframe_empty(mock_df)

    assert result is True


def test_is_dataframe_empty_with_pandas_dataframe_nonempty() -> None:
    mock_df = Mock()
    mock_df.empty = False

    result = _is_dataframe_empty(mock_df)

    assert result is False


def test_is_dataframe_empty_with_object_without_empty_attr() -> None:
    mock_df = Mock()
    del mock_df.empty

    result = _is_dataframe_empty(mock_df)

    assert result is True


@pytest.mark.anyio
async def test_extract_tables_file_stat_os_error() -> None:
    file_path = "/nonexistent/file.pdf"

    with (
        patch("pathlib.Path.stat", side_effect=OSError("File not found")),
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "true"}),
        patch("kreuzberg._gmft._extract_tables_isolated_async") as mock_isolated,
    ):
        mock_table_data = [TableData(cropped_image=Mock(), page_number=1, text="Sample table", df=pl.DataFrame())]
        mock_isolated.return_value = mock_table_data

        with patch("kreuzberg._gmft.get_table_cache") as mock_cache:
            mock_cache_instance = Mock()
            mock_cache.return_value = mock_cache_instance
            mock_cache_instance.aget = AsyncMock(return_value=None)
            mock_cache_instance.aset = AsyncMock()
            mock_cache_instance.is_processing.return_value = False
            mock_cache_instance.mark_processing.return_value = Mock()

            result = await extract_tables(file_path)

        assert result == mock_table_data
        mock_isolated.assert_called_once()


def test_extract_tables_sync_file_stat_os_error() -> None:
    file_path = "/nonexistent/file.pdf"

    with (
        patch("pathlib.Path.stat", side_effect=OSError("File not found")),
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "true"}),
        patch("kreuzberg._gmft._extract_tables_isolated") as mock_isolated,
    ):
        mock_table_data = [TableData(cropped_image=Mock(), page_number=1, text="Sample table", df=pl.DataFrame())]
        mock_isolated.return_value = mock_table_data

        with patch("kreuzberg._gmft.get_table_cache") as mock_cache:
            mock_cache_instance = Mock()
            mock_cache.return_value = mock_cache_instance
            mock_cache_instance.get.return_value = None

            result = extract_tables_sync(file_path)

        assert result == mock_table_data
        mock_isolated.assert_called_once()


@pytest.mark.anyio
async def test_extract_tables_with_cache_hit() -> None:
    file_path = "/path/to/file.pdf"
    cached_tables = [TableData(cropped_image=Mock(), page_number=1, text="Cached table", df=pl.DataFrame())]

    with patch("pathlib.Path.stat") as mock_stat, patch("kreuzberg._gmft.get_table_cache") as mock_cache:
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.aget = AsyncMock(return_value=cached_tables)

        result = await extract_tables(file_path)

    assert result == cached_tables
    mock_cache_instance.aget.assert_called_once()


def test_extract_tables_sync_with_cache_hit() -> None:
    file_path = "/path/to/file.pdf"
    cached_tables = [TableData(cropped_image=Mock(), page_number=1, text="Cached table", df=pl.DataFrame())]

    with patch("pathlib.Path.stat") as mock_stat, patch("kreuzberg._gmft.get_table_cache") as mock_cache:
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.get.return_value = cached_tables

        result = extract_tables_sync(file_path)

    assert result == cached_tables
    mock_cache_instance.get.assert_called_once()


@pytest.mark.anyio
async def test_extract_tables_with_processing_wait() -> None:
    file_path = "/path/to/file.pdf"

    with (
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
        patch("anyio.to_thread.run_sync") as mock_run_sync,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance

        cached_result = [TableData(cropped_image=Mock(), page_number=1, text="Processed table", df=pl.DataFrame())]
        mock_cache_instance.aget = AsyncMock(side_effect=[None, cached_result])
        mock_cache_instance.is_processing.return_value = True

        mock_event = Mock()
        mock_cache_instance.mark_processing.return_value = mock_event

        result = await extract_tables(file_path)

        assert result == cached_result
        mock_run_sync.assert_called_once_with(mock_event.wait)
        assert mock_cache_instance.aget.call_count == 2


@pytest.mark.anyio
async def test_extract_tables_wait_then_no_cache() -> None:
    file_path = "/path/to/file.pdf"

    with (
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "true"}),
        patch("kreuzberg._gmft._extract_tables_isolated_async") as mock_isolated,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance

        mock_cache_instance.aget = AsyncMock(side_effect=[None, None])
        mock_cache_instance.is_processing.return_value = True
        mock_cache_instance.aset = AsyncMock()
        mock_cache_instance.mark_complete = Mock()

        mock_event = Mock()
        mock_cache_instance.mark_processing.return_value = mock_event

        mock_table_data = [TableData(cropped_image=Mock(), page_number=1, text="New table", df=pl.DataFrame())]
        mock_isolated.return_value = mock_table_data

        result = await extract_tables(file_path)

        assert result == mock_table_data
        mock_run_sync.assert_called_once_with(mock_event.wait)
        assert mock_cache_instance.aget.call_count == 2
        mock_isolated.assert_called_once()
        mock_cache_instance.mark_complete.assert_called_once()


@pytest.mark.anyio
async def test_extract_tables_environment_variable_isolated_false() -> None:
    file_path = "/path/to/file.pdf"

    with (
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "false"}),
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
        patch("kreuzberg._gmft._extract_tables_isolated_async") as mock_isolated,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.aget = AsyncMock(return_value=None)
        mock_cache_instance.aset = AsyncMock()
        mock_cache_instance.is_processing.return_value = False
        mock_cache_instance.mark_processing.return_value = Mock()

        with patch.dict("sys.modules", {"gmft": None, "gmft.auto": None}):
            with pytest.raises(MissingDependencyError):
                await extract_tables(file_path)

        mock_isolated.assert_not_called()


def test_extract_tables_sync_environment_variable_variations() -> None:
    file_path = "/path/to/file.pdf"

    test_cases = [
        ("true", True),
        ("1", True),
        ("yes", True),
        ("TRUE", True),
        ("false", False),
        ("0", False),
        ("no", False),
        ("", False),
        ("random", False),
    ]

    for env_value, expected_isolated in test_cases:
        with (
            patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": env_value}),
            patch("pathlib.Path.stat") as mock_stat,
            patch("kreuzberg._gmft.get_table_cache") as mock_cache,
            patch("kreuzberg._gmft._extract_tables_isolated") as mock_isolated,
        ):
            mock_stat.return_value.st_size = 1000
            mock_stat.return_value.st_mtime = 123456789

            mock_cache_instance = Mock()
            mock_cache.return_value = mock_cache_instance
            mock_cache_instance.get.return_value = None

            if expected_isolated:
                mock_isolated.return_value = []
                extract_tables_sync(file_path)
                mock_isolated.assert_called_once()
            else:
                with patch.dict("sys.modules", {"gmft": None, "gmft.auto": None}):
                    with pytest.raises(MissingDependencyError):
                        extract_tables_sync(file_path)
                mock_isolated.assert_not_called()


@pytest.mark.anyio
async def test_extract_tables_config_serialization() -> None:
    file_path = "/path/to/file.pdf"
    custom_config = GMFTConfig(verbosity=2, detector_base_threshold=0.8, formatter_base_threshold=0.9)

    with (
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "true"}),
        patch("kreuzberg._gmft._extract_tables_isolated_async") as mock_isolated,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.aget = AsyncMock(return_value=None)
        mock_cache_instance.aset = AsyncMock()
        mock_cache_instance.is_processing.return_value = False
        mock_cache_instance.mark_processing.return_value = Mock()

        mock_isolated.return_value = []

        await extract_tables(file_path, config=custom_config)

        cache_call_kwargs = mock_cache_instance.aget.call_args[1]
        assert "config" in cache_call_kwargs
        assert "verbosity" in cache_call_kwargs["config"]
        assert "detector_base_threshold" in cache_call_kwargs["config"]


def test_extract_tables_sync_missing_gmft_dependency() -> None:
    file_path = "/path/to/file.pdf"

    with (
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "false"}),
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.get.return_value = None

        with patch.dict("sys.modules", {"gmft": None, "gmft.auto": None}):
            with pytest.raises(MissingDependencyError) as exc_info:
                extract_tables_sync(file_path)

        error = exc_info.value
        assert "gmft" in str(error).lower()
        assert "table extraction" in str(error)


@pytest.mark.anyio
async def test_extract_tables_default_config() -> None:
    file_path = "/path/to/file.pdf"

    with (
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "true"}),
        patch("kreuzberg._gmft._extract_tables_isolated_async") as mock_isolated,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.aget = AsyncMock(return_value=None)
        mock_cache_instance.aset = AsyncMock()
        mock_cache_instance.is_processing.return_value = False
        mock_cache_instance.mark_processing.return_value = Mock()

        mock_isolated.return_value = []

        await extract_tables(file_path)

        mock_isolated.assert_called_once()
        call_args = mock_isolated.call_args
        config_arg = call_args[0][1]
        assert isinstance(config_arg, GMFTConfig)


def test_extract_tables_sync_override_isolated_process_parameter() -> None:
    file_path = "/path/to/file.pdf"

    with (
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "true"}),
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.get.return_value = None

        with patch.dict("sys.modules", {"gmft": None, "gmft.auto": None}):
            with pytest.raises(MissingDependencyError):
                extract_tables_sync(file_path, use_isolated_process=False)


@pytest.mark.anyio
async def test_extract_tables_non_isolated_with_gmft_finally_block() -> None:
    file_path = "/path/to/file.pdf"

    mock_image = Mock(spec=Image.Image)
    mock_cropped_table = Mock()
    mock_cropped_table.image.return_value = mock_image
    mock_cropped_table.page.page_number = 1

    mock_df = Mock()
    mock_df.to_markdown.return_value = "| col1 | col2 |"

    Mock()

    mock_detector = Mock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = Mock()

    mock_doc = Mock()
    mock_doc.__iter__ = Mock(return_value=iter([Mock()]))
    mock_doc_close = Mock()
    mock_doc.close = mock_doc_close

    with (
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "false"}),
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.aget = AsyncMock(return_value=None)
        mock_cache_instance.aset = AsyncMock()
        mock_cache_instance.is_processing.return_value = False
        mock_cache_instance.mark_processing.return_value = Mock()
        mock_cache_instance.mark_complete = Mock()

        with (
            patch.dict(
                "sys.modules",
                {
                    "gmft": Mock(),
                    "gmft.auto": Mock(
                        AutoTableDetector=Mock(return_value=mock_detector),
                        AutoTableFormatter=Mock(return_value=mock_formatter),
                    ),
                    "gmft.detectors.tatr": Mock(TATRDetectorConfig=Mock),
                    "gmft.formatters.tatr": Mock(TATRFormatConfig=Mock),
                    "gmft.pdf_bindings.pdfium": Mock(PyPDFium2Document=Mock(return_value=mock_doc)),
                },
            ),
            patch("kreuzberg._gmft.run_sync") as mock_run_sync,
            patch("kreuzberg._gmft._pandas_to_polars") as mock_pandas_to_polars,
        ):
            import polars as pl

            mock_pandas_to_polars.return_value = pl.DataFrame()

            async def run_sync_error(*args: Any, **kwargs: Any) -> Any:
                if args and hasattr(args[0], "__name__") and args[0].__name__ == "PyPDFium2Document":
                    return mock_doc
                if args and args[0] == mock_doc_close:
                    return None
                raise RuntimeError("Test error")

            mock_run_sync.side_effect = run_sync_error

            with pytest.raises(RuntimeError):
                await extract_tables(file_path, use_isolated_process=False)

            mock_cache_instance.mark_complete.assert_called_once()


@pytest.mark.anyio
async def test_extract_tables_non_isolated_with_gmft() -> None:
    file_path = "/path/to/file.pdf"

    mock_image = Mock(spec=Image.Image)
    mock_cropped_table = Mock()
    mock_cropped_table.image.return_value = mock_image
    mock_cropped_table.page.page_number = 1

    mock_df = Mock()
    mock_df.to_markdown.return_value = "| col1 | col2 |"

    mock_formatted_table = Mock()
    mock_formatted_table.df.return_value = mock_df

    mock_detector = Mock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = Mock()
    mock_formatter.extract.return_value = mock_formatted_table

    mock_doc = Mock()
    mock_doc.__iter__ = Mock(return_value=iter([Mock()]))
    mock_doc.close = Mock()

    with (
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "false"}),
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
        patch("kreuzberg._gmft.run_sync") as mock_run_sync,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.aget = AsyncMock(return_value=None)
        mock_cache_instance.aset = AsyncMock()
        mock_cache_instance.is_processing.return_value = False
        mock_cache_instance.mark_processing.return_value = Mock()
        mock_cache_instance.mark_complete = Mock()

        with (
            patch.dict(
                "sys.modules",
                {
                    "gmft": Mock(),
                    "gmft.auto": Mock(
                        AutoTableDetector=Mock(return_value=mock_detector),
                        AutoTableFormatter=Mock(return_value=mock_formatter),
                    ),
                    "gmft.detectors.tatr": Mock(TATRDetectorConfig=Mock),
                    "gmft.formatters.tatr": Mock(TATRFormatConfig=Mock),
                    "gmft.pdf_bindings.pdfium": Mock(PyPDFium2Document=Mock(return_value=mock_doc)),
                },
            ),
            patch("kreuzberg._gmft._pandas_to_polars") as mock_pandas_to_polars,
        ):
            import polars as pl

            mock_pandas_to_polars.return_value = pl.DataFrame()

            async def run_sync_side_effect(func: Any, *args: Any) -> Any:
                if hasattr(func, "__name__") and func.__name__ == "PyPDFium2Document":
                    return mock_doc
                if hasattr(func, "__self__") and hasattr(func.__self__, "extract"):
                    return func(*args)
                if callable(func):
                    return func()
                if func.__name__ == "close":
                    return func()
                return None

            mock_run_sync.side_effect = run_sync_side_effect

            result = await extract_tables(file_path, use_isolated_process=False)

            assert len(result) == 1
            assert result[0]["page_number"] == 1
            assert "col1" in result[0]["text"]
            mock_cache_instance.mark_complete.assert_called_once()
            mock_doc.close.assert_called_once()


def test_extract_tables_sync_non_isolated_with_gmft() -> None:
    file_path = "/path/to/file.pdf"

    mock_image = Mock(spec=Image.Image)
    mock_cropped_table = Mock()
    mock_cropped_table.image.return_value = mock_image
    mock_cropped_table.page.page_number = 2

    mock_df = Mock()
    mock_df.to_markdown.return_value = "| data1 | data2 |"

    mock_formatted_table = Mock()
    mock_formatted_table.df.return_value = mock_df

    mock_detector = Mock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = Mock()
    mock_formatter.extract.return_value = mock_formatted_table

    mock_doc = Mock()
    mock_doc.__iter__ = Mock(return_value=iter([Mock()]))
    mock_doc.close = Mock()

    with (
        patch.dict("os.environ", {"KREUZBERG_GMFT_ISOLATED": "false"}),
        patch("pathlib.Path.stat") as mock_stat,
        patch("kreuzberg._gmft.get_table_cache") as mock_cache,
    ):
        mock_stat.return_value.st_size = 1000
        mock_stat.return_value.st_mtime = 123456789

        mock_cache_instance = Mock()
        mock_cache.return_value = mock_cache_instance
        mock_cache_instance.get.return_value = None
        mock_cache_instance.set = Mock()

        with patch.dict(
            "sys.modules",
            {
                "gmft": Mock(),
                "gmft.auto": Mock(
                    AutoTableDetector=Mock(return_value=mock_detector),
                    AutoTableFormatter=Mock(return_value=mock_formatter),
                ),
                "gmft.detectors.tatr": Mock(TATRDetectorConfig=Mock),
                "gmft.formatters.tatr": Mock(TATRFormatConfig=Mock),
                "gmft.pdf_bindings.pdfium": Mock(PyPDFium2Document=Mock(return_value=mock_doc)),
            },
        ):
            result = extract_tables_sync(file_path, use_isolated_process=False)

            assert len(result) == 1
            assert result[0]["page_number"] == 2
            assert "data1" in result[0]["text"]
            mock_cache_instance.set.assert_called_once()
            mock_doc.close.assert_called_once()


def test_extract_tables_in_process_success() -> None:
    result_queue: queue.Queue[Any] = queue.Queue()
    file_path = "/path/to/file.pdf"
    config_dict = {"verbosity": 0}

    mock_image = Mock(spec=Image.Image)
    mock_image.save = Mock()

    mock_cropped_table = Mock()
    mock_cropped_table.image.return_value = mock_image
    mock_cropped_table.page.page_number = 3

    mock_df = Mock()
    mock_df.to_csv.return_value = "col1,col2\nval1,val2"
    mock_df.to_markdown.return_value = "| col1 | col2 |"
    mock_df.columns = ["col1", "col2"]
    mock_df.empty = False

    mock_formatted_table = Mock()
    mock_formatted_table.df.return_value = mock_df

    mock_detector = Mock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = Mock()
    mock_formatter.extract.return_value = mock_formatted_table

    mock_doc = Mock()
    mock_doc.__iter__ = Mock(return_value=iter([Mock()]))
    mock_doc.close = Mock()

    with (
        patch("signal.signal"),
        patch.dict(
            "sys.modules",
            {
                "gmft": Mock(),
                "gmft.auto": Mock(
                    AutoTableDetector=Mock(return_value=mock_detector),
                    AutoTableFormatter=Mock(return_value=mock_formatter),
                ),
                "gmft.detectors.tatr": Mock(TATRDetectorConfig=Mock),
                "gmft.formatters.tatr": Mock(TATRFormatConfig=Mock),
                "gmft.pdf_bindings.pdfium": Mock(PyPDFium2Document=Mock(return_value=mock_doc)),
            },
        ),
    ):
        _extract_tables_in_process(file_path, config_dict, result_queue)

        success, result = result_queue.get_nowait()
        assert success is True
        assert len(result) == 1
        assert result[0]["page_number"] == 3
        assert "col1" in result[0]["text"]
        assert result[0]["df_columns"] == ["col1", "col2"]
        assert "val1,val2" in result[0]["df_csv"]
        mock_doc.close.assert_called_once()


def test_extract_tables_in_process_with_string_cell_config() -> None:
    result_queue: queue.Queue[Any] = queue.Queue()
    file_path = "/path/to/file.pdf"
    config_dict = {"verbosity": 0, "cell_required_confidence": {"0": 0.8, "1": 0.9}}

    mock_image = Mock(spec=Image.Image)
    mock_image.save = Mock()

    mock_cropped_table = Mock()
    mock_cropped_table.image.return_value = mock_image
    mock_cropped_table.page.page_number = 1

    mock_df = Mock()
    mock_df.to_csv.return_value = "data"
    mock_df.to_markdown.return_value = "| data |"
    mock_df.columns = ["data"]
    mock_df.empty = False

    mock_formatted_table = Mock()
    mock_formatted_table.df.return_value = mock_df

    mock_detector = Mock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = Mock()
    mock_formatter.extract.return_value = mock_formatted_table

    mock_doc = Mock()
    mock_doc.__iter__ = Mock(return_value=iter([Mock()]))
    mock_doc.close = Mock()

    with (
        patch("signal.signal"),
        patch.dict(
            "sys.modules",
            {
                "gmft": Mock(),
                "gmft.auto": Mock(
                    AutoTableDetector=Mock(return_value=mock_detector),
                    AutoTableFormatter=Mock(return_value=mock_formatter),
                ),
                "gmft.detectors.tatr": Mock(TATRDetectorConfig=Mock),
                "gmft.formatters.tatr": Mock(TATRFormatConfig=Mock),
                "gmft.pdf_bindings.pdfium": Mock(PyPDFium2Document=Mock(return_value=mock_doc)),
            },
        ),
    ):
        _extract_tables_in_process(file_path, config_dict, result_queue)

        success, result = result_queue.get_nowait()
        assert success is True
        assert len(result) == 1


def test_extract_tables_in_process_with_cell_config() -> None:
    result_queue: queue.Queue[Any] = queue.Queue()
    file_path = "/path/to/file.pdf"
    config_dict = {"verbosity": 0, "cell_required_confidence": {"0": 0.8, "1": 0.9}}

    mock_image = Mock(spec=Image.Image)
    mock_image.save = Mock()

    mock_cropped_table = Mock()
    mock_cropped_table.image.return_value = mock_image
    mock_cropped_table.page.page_number = 1

    mock_df = Mock()
    mock_df.to_csv.return_value = "data"
    mock_df.to_markdown.return_value = "| data |"
    mock_df.columns = ["data"]
    mock_df.empty = False

    mock_formatted_table = Mock()
    mock_formatted_table.df.return_value = mock_df

    mock_detector = Mock()
    mock_detector.extract.return_value = [mock_cropped_table]

    mock_formatter = Mock()
    mock_formatter.extract.return_value = mock_formatted_table

    mock_doc = Mock()
    mock_doc.__iter__ = Mock(return_value=iter([Mock()]))
    mock_doc.close = Mock()

    with (
        patch("signal.signal"),
        patch.dict(
            "sys.modules",
            {
                "gmft": Mock(),
                "gmft.auto": Mock(
                    AutoTableDetector=Mock(return_value=mock_detector),
                    AutoTableFormatter=Mock(return_value=mock_formatter),
                ),
                "gmft.detectors.tatr": Mock(TATRDetectorConfig=Mock),
                "gmft.formatters.tatr": Mock(TATRFormatConfig=Mock),
                "gmft.pdf_bindings.pdfium": Mock(PyPDFium2Document=Mock(return_value=mock_doc)),
            },
        ),
    ):
        _extract_tables_in_process(file_path, config_dict, result_queue)

        success, result = result_queue.get_nowait()
        assert success is True
        assert len(result) == 1


def test_extract_tables_in_process_exception() -> None:
    result_queue: queue.Queue[Any] = queue.Queue()
    file_path = "/path/to/file.pdf"
    config_dict = {"verbosity": 0}

    with (
        patch("signal.signal"),
        patch.dict(
            "sys.modules",
            {
                "gmft": Mock(),
                "gmft.auto": Mock(
                    AutoTableDetector=Mock(side_effect=RuntimeError("GMFT error")),
                    AutoTableFormatter=Mock(side_effect=RuntimeError("GMFT error")),
                ),
                "gmft.detectors.tatr": Mock(),
                "gmft.formatters.tatr": Mock(),
                "gmft.pdf_bindings.pdfium": Mock(),
            },
        ),
    ):
        _extract_tables_in_process(file_path, config_dict, result_queue)

        success, error_info = result_queue.get_nowait()
        assert success is False
        assert error_info["type"] == "RuntimeError"
        assert "GMFT error" in error_info["error"]
        assert "traceback" in error_info


def test_extract_tables_isolated_success() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig(verbosity=1)

    test_img = Image.new("RGB", (100, 100), color="white")
    img_bytes = io.BytesIO()
    test_img.save(img_bytes, format="PNG")
    img_bytes.seek(0)

    mock_result = [
        {
            "cropped_image_bytes": img_bytes.getvalue(),
            "page_number": 1,
            "text": "| test |",
            "df_columns": ["test"],
            "df_csv": "test\nvalue",
        }
    ]

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.side_effect = [queue.Empty(), (True, mock_result)]

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("time.time", side_effect=[0, 0.1]),
        patch("time.sleep"),
    ):
        result = _extract_tables_isolated(file_path, config, timeout=10.0)

        assert len(result) == 1
        assert result[0]["page_number"] == 1
        assert "test" in result[0]["text"]
        assert isinstance(result[0]["cropped_image"], Image.Image)
        assert isinstance(result[0]["df"], pl.DataFrame)
        mock_process.start.assert_called_once()
        mock_process.terminate.assert_called_once()


def test_extract_tables_isolated_process_died_sigsegv() -> None:
    file_path = "/path/to/file.pdf"
    with pytest.warns(FutureWarning):
        config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.return_value = False
    mock_process.exitcode = -signal.SIGSEGV
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.side_effect = queue.Empty

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("time.time", side_effect=[0, 0.1]),
    ):
        with pytest.raises(ParsingError) as exc_info:
            _extract_tables_isolated(file_path, config)

        assert "segmentation fault" in str(exc_info.value)
        assert exc_info.value.context["exit_code"] == -signal.SIGSEGV


def test_extract_tables_isolated_process_died_other() -> None:
    file_path = "/path/to/file.pdf"
    with pytest.warns(FutureWarning):
        config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.return_value = False
    mock_process.exitcode = 1
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.side_effect = queue.Empty

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("time.time", side_effect=[0, 0.1]),
    ):
        with pytest.raises(ParsingError) as exc_info:
            _extract_tables_isolated(file_path, config)

        assert "died unexpectedly with exit code 1" in str(exc_info.value)
        assert exc_info.value.context["exit_code"] == 1


def test_extract_tables_isolated_timeout() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.kill = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.side_effect = queue.Empty

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("time.time", side_effect=[0, 1.1, 2.1]),
        patch("time.sleep"),
    ):
        with pytest.raises(ParsingError) as exc_info:
            _extract_tables_isolated(file_path, config, timeout=1.0)

        assert "timed out" in str(exc_info.value)
        assert exc_info.value.context["timeout"] == 1.0
        mock_process.terminate.assert_called_once()


def test_extract_tables_isolated_error_from_process() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_error_info = {
        "error": "Processing failed",
        "type": "ValueError",
        "traceback": "Traceback...",
    }

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.return_value = (False, mock_error_info)

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with patch("multiprocessing.get_context", return_value=mock_ctx):
        with pytest.raises(ParsingError) as exc_info:
            _extract_tables_isolated(file_path, config)

        assert "Processing failed" in str(exc_info.value)
        assert exc_info.value.context["error_type"] == "ValueError"


def test_extract_tables_isolated_empty_csv() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    test_img = Image.new("RGB", (50, 50), color="blue")
    img_bytes = io.BytesIO()
    test_img.save(img_bytes, format="PNG")
    img_bytes.seek(0)

    mock_result = [
        {
            "cropped_image_bytes": img_bytes.getvalue(),
            "page_number": 2,
            "text": "",
            "df_columns": [],
            "df_csv": "",
        }
    ]

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.return_value = (True, mock_result)

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with patch("multiprocessing.get_context", return_value=mock_ctx):
        result = _extract_tables_isolated(file_path, config)

        assert len(result) == 1
        assert result[0]["page_number"] == 2
        assert result[0]["text"] == ""
        assert result[0]["df"] is not None
        assert result[0]["df"].is_empty()


def test_extract_tables_isolated_process_needs_kill() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.side_effect = [True, True, True]
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.kill = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.side_effect = queue.Empty

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("time.time", side_effect=[0, 1.1]),
        patch("time.sleep"),
    ):
        with pytest.raises(ParsingError):
            _extract_tables_isolated(file_path, config, timeout=1.0)

        mock_process.terminate.assert_called_once()
        mock_process.kill.assert_called_once()


def test_extract_tables_isolated_missing_dependency() -> None:
    file_path = "/path/to/file.pdf"
    with pytest.warns(FutureWarning):
        config = GMFTConfig()

    error_info = {
        "error": "libcublasLt.so.12: failed to map segment from shared object",
        "type": "ImportError",
        "traceback": "Traceback (most recent call last): ...",
    }

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get_nowait.side_effect = [(False, error_info)]

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with patch("multiprocessing.get_context", return_value=mock_ctx):
        with pytest.raises(MissingDependencyError) as exc_info:
            _extract_tables_isolated(file_path, config, timeout=1.0)

    assert "kreuzberg['gmft']" in str(exc_info.value)
    assert exc_info.value.context["file_path"] == file_path
    assert exc_info.value.context["error_message"] == error_info["error"]


@pytest.mark.anyio
async def test_extract_tables_isolated_async_success() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig(verbosity=2)

    test_img = Image.new("RGB", (100, 100), color="red")
    img_bytes = io.BytesIO()
    test_img.save(img_bytes, format="PNG")
    img_bytes.seek(0)

    mock_result = [
        {
            "cropped_image_bytes": img_bytes.getvalue(),
            "page_number": 5,
            "text": "| async | test |",
            "df_columns": ["async", "test"],
            "df_csv": "async,test\nval1,val2",
        }
    ]

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get.return_value = (True, mock_result)

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after"),
    ):

        async def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        result = await _extract_tables_isolated_async(file_path, config, timeout=10.0)

        assert len(result) == 1
        assert result[0]["page_number"] == 5
        assert "async" in result[0]["text"]
        assert isinstance(result[0]["cropped_image"], Image.Image)
        assert isinstance(result[0]["df"], pl.DataFrame)
        mock_process.start.assert_called_once()
        mock_process.terminate.assert_called_once()


@pytest.mark.anyio
async def test_extract_tables_isolated_async_process_died_sigsegv() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.return_value = False
    mock_process.exitcode = -signal.SIGSEGV
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()

    def get_with_timeout(timeout: Any) -> None:
        raise queue.Empty

    mock_queue.get.side_effect = get_with_timeout

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after"),
    ):

        def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        with pytest.raises(ParsingError) as exc_info:
            await _extract_tables_isolated_async(file_path, config)

        assert "segmentation fault" in str(exc_info.value)
        assert exc_info.value.context["exit_code"] == -signal.SIGSEGV


@pytest.mark.anyio
async def test_extract_tables_isolated_async_process_died_other() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.return_value = False
    mock_process.exitcode = 42
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()

    def get_with_timeout(timeout: Any) -> None:
        raise queue.Empty

    mock_queue.get.side_effect = get_with_timeout

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after"),
    ):

        def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        with pytest.raises(ParsingError) as exc_info:
            await _extract_tables_isolated_async(file_path, config)

        assert "died unexpectedly with exit code 42" in str(exc_info.value)
        assert exc_info.value.context["exit_code"] == 42


@pytest.mark.anyio
async def test_extract_tables_isolated_async_timeout() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync"),
        patch("anyio.fail_after", side_effect=TimeoutError()),
    ):
        with pytest.raises(ParsingError) as exc_info:
            await _extract_tables_isolated_async(file_path, config, timeout=2.0)

        assert "timed out" in str(exc_info.value)
        assert exc_info.value.context["timeout"] == 2.0
        mock_process.terminate.assert_called_once()


@pytest.mark.anyio
async def test_extract_tables_isolated_async_error_from_process() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_error_info = {
        "error": "Async processing failed",
        "type": "RuntimeError",
        "traceback": "Async traceback...",
    }

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get.return_value = (False, mock_error_info)

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after"),
    ):

        async def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        with pytest.raises(ParsingError) as exc_info:
            await _extract_tables_isolated_async(file_path, config)

        assert "Async processing failed" in str(exc_info.value)
        assert exc_info.value.context["error_type"] == "RuntimeError"


@pytest.mark.anyio
async def test_extract_tables_isolated_async_missing_dependency() -> None:
    file_path = "/path/to/file.pdf"
    with pytest.warns(FutureWarning):
        config = GMFTConfig()

    error_info = {
        "error": "libcublasLt.so.12: failed to map segment from shared object",
        "type": "ImportError",
        "traceback": "Traceback (most recent call last): ...",
    }

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.kill = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get.return_value = (False, error_info)

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after"),
    ):

        async def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        with pytest.raises(MissingDependencyError) as exc_info:
            await _extract_tables_isolated_async(file_path, config, timeout=1.0)

    assert "kreuzberg['gmft']" in str(exc_info.value)
    assert exc_info.value.context["file_path"] == file_path
    assert exc_info.value.context["error_message"] == error_info["error"]


@pytest.mark.anyio
async def test_extract_tables_isolated_async_empty_csv() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    test_img = Image.new("RGB", (50, 50), color="green")
    img_bytes = io.BytesIO()
    test_img.save(img_bytes, format="PNG")
    img_bytes.seek(0)

    mock_result = [
        {
            "cropped_image_bytes": img_bytes.getvalue(),
            "page_number": 7,
            "text": "",
            "df_columns": [],
            "df_csv": None,
        }
    ]

    mock_process = Mock()
    mock_process.is_alive.return_value = True
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()
    mock_queue.get.return_value = (True, mock_result)

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after"),
    ):

        async def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        result = await _extract_tables_isolated_async(file_path, config)

        assert len(result) == 1
        assert result[0]["page_number"] == 7
        assert result[0]["text"] == ""
        assert result[0]["df"] is not None
        assert result[0]["df"].is_empty()


@pytest.mark.anyio
async def test_extract_tables_isolated_async_process_needs_kill() -> None:
    file_path = "/path/to/file.pdf"
    config = GMFTConfig()

    mock_process = Mock()
    mock_process.is_alive.side_effect = [True, True]
    mock_process.start = Mock()
    mock_process.terminate = Mock()
    mock_process.kill = Mock()
    mock_process.join = Mock()

    mock_queue = Mock()

    mock_ctx = Mock()
    mock_ctx.Queue.return_value = mock_queue
    mock_ctx.Process.return_value = mock_process

    with (
        patch("multiprocessing.get_context", return_value=mock_ctx),
        patch("anyio.to_thread.run_sync") as mock_run_sync,
        patch("anyio.fail_after", side_effect=TimeoutError()),
    ):

        async def run_sync_side_effect(func: Any) -> Any:
            if callable(func):
                return func()
            return None

        mock_run_sync.side_effect = run_sync_side_effect

        with pytest.raises(ParsingError):
            await _extract_tables_isolated_async(file_path, config, timeout=1.0)

        mock_process.terminate.assert_called_once()
        mock_process.kill.assert_called_once()
