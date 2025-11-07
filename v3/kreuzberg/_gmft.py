from __future__ import annotations

import io
import multiprocessing as mp
import os
import queue
import signal
import time
import traceback
from io import StringIO
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import anyio
import msgspec
import polars as pl
from PIL import Image

from kreuzberg._types import GMFTConfig, TableData
from kreuzberg._utils._cache import get_table_cache
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError, ParsingError

if TYPE_CHECKING:
    from os import PathLike

    from gmft.detectors.base import CroppedTable


def _pandas_to_polars(pandas_df: Any) -> pl.DataFrame:
    if pandas_df is None:
        return pl.DataFrame()

    try:
        return pl.from_pandas(pandas_df)
    except (TypeError, ValueError, AttributeError):
        if hasattr(pandas_df, "columns") and hasattr(pandas_df.columns, "duplicated"):
            mask = ~pandas_df.columns.duplicated()
            pandas_df = pandas_df.loc[:, mask]
            return pl.from_pandas(pandas_df)
        return pl.DataFrame()


def _dataframe_to_markdown(df: Any) -> str:
    if df is None:
        return ""

    if isinstance(df, pl.DataFrame):
        if df.is_empty():
            return ""
        return str(df)

    if hasattr(df, "to_markdown"):
        return cast("str", df.to_markdown())

    return str(df)


def _dataframe_to_csv(df: Any) -> str:
    if df is None:
        return ""

    if isinstance(df, pl.DataFrame):
        if df.is_empty():
            return ""
        return df.write_csv()

    if hasattr(df, "to_csv"):
        return cast("str", df.to_csv(index=False))

    return ""


def _is_dataframe_empty(df: Any) -> bool:
    if df is None:
        return True

    if isinstance(df, pl.DataFrame):
        return df.is_empty()

    if hasattr(df, "empty"):
        return cast("bool", df.empty)

    return True


async def extract_tables(
    file_path: str | PathLike[str], config: GMFTConfig | None = None, use_isolated_process: bool | None = None
) -> list[TableData]:
    # Determine if we should use isolated process  # ~keep
    if use_isolated_process is None:
        use_isolated_process = os.environ.get("KREUZBERG_GMFT_ISOLATED", "true").lower() in ("true", "1", "yes")

    path = Path(file_path)
    try:
        stat = path.stat()
        file_info = {
            "path": str(path.resolve()),
            "size": stat.st_size,
            "mtime": stat.st_mtime,
        }
    except OSError:  # pragma: no cover
        file_info = {
            "path": str(path),
            "size": 0,
            "mtime": 0,
        }

    config = config or GMFTConfig()
    cache_kwargs = {
        "file_info": str(sorted(file_info.items())),
        "extractor": "gmft",
        "config": str(sorted(msgspec.to_builtins(config).items())),
    }

    table_cache = get_table_cache()
    cached_result = await table_cache.aget(**cache_kwargs)
    if cached_result is not None:
        return cached_result  # type: ignore[no-any-return]

    if table_cache.is_processing(**cache_kwargs):
        event = table_cache.mark_processing(**cache_kwargs)
        await anyio.to_thread.run_sync(event.wait)

        # Try cache again after waiting for other process to complete  # ~keep
        cached_result = await table_cache.aget(**cache_kwargs)
        if cached_result is not None:
            return cached_result  # type: ignore[no-any-return]

    table_cache.mark_processing(**cache_kwargs)

    try:
        if use_isolated_process:
            result = await _extract_tables_isolated_async(file_path, config)

            await table_cache.aset(result, **cache_kwargs)

            return result

        try:
            from gmft.auto import (  # type: ignore[attr-defined]  # noqa: PLC0415
                AutoTableDetector,
                AutoTableFormatter,
            )
            from gmft.detectors.tatr import TATRDetectorConfig  # type: ignore[attr-defined]  # noqa: PLC0415
            from gmft.formatters.tatr import TATRFormatConfig  # noqa: PLC0415
            from gmft.pdf_bindings.pdfium import PyPDFium2Document  # noqa: PLC0415

            formatter: Any = AutoTableFormatter(  # type: ignore[no-untyped-call]
                config=TATRFormatConfig(
                    verbosity=config.verbosity,
                    formatter_base_threshold=config.formatter_base_threshold,
                    cell_required_confidence=config.cell_required_confidence,
                    remove_null_rows=config.remove_null_rows,
                    enable_multi_header=config.enable_multi_header,
                    semantic_spanning_cells=config.semantic_spanning_cells,
                    semantic_hierarchical_left_fill=config.semantic_hierarchical_left_fill,
                    large_table_if_n_rows_removed=config.large_table_if_n_rows_removed,
                    large_table_threshold=config.large_table_threshold,
                    large_table_row_overlap_threshold=config.large_table_row_overlap_threshold,
                    large_table_maximum_rows=config.large_table_maximum_rows,
                    force_large_table_assumption=config.force_large_table_assumption,
                )
            )
            detector: Any = AutoTableDetector(  # type: ignore[no-untyped-call]
                config=TATRDetectorConfig(detector_base_threshold=config.detector_base_threshold)
            )
            doc = await run_sync(PyPDFium2Document, str(file_path))
            cropped_tables: list[CroppedTable] = []
            dataframes: list[pl.DataFrame] = []
            try:
                for page in doc:
                    cropped_tables.extend(await run_sync(detector.extract, page))

                for cropped_table in cropped_tables:
                    formatted_table = await run_sync(formatter.extract, cropped_table)
                    dataframes.append(await run_sync(formatted_table.df))

                result = [
                    TableData(
                        cropped_image=cropped_table.image(),
                        page_number=cropped_table.page.page_number,
                        text=_dataframe_to_markdown(data_frame),
                        df=_pandas_to_polars(data_frame),
                    )
                    for data_frame, cropped_table in zip(dataframes, cropped_tables, strict=False)
                ]

                await table_cache.aset(result, **cache_kwargs)

                return result
            finally:
                await run_sync(doc.close)

        except ImportError as e:  # pragma: no cover
            error = MissingDependencyError.create_for_package(
                dependency_group="gmft", functionality="table extraction", package_name="gmft"
            )
            error.context = {
                "file_path": str(Path(file_path)),
                "error_message": str(e),
            }
            raise error from e
    finally:
        table_cache.mark_complete(**cache_kwargs)


def extract_tables_sync(
    file_path: str | PathLike[str], config: GMFTConfig | None = None, use_isolated_process: bool | None = None
) -> list[TableData]:
    # Determine if we should use isolated process  # ~keep
    if use_isolated_process is None:
        use_isolated_process = os.environ.get("KREUZBERG_GMFT_ISOLATED", "true").lower() in ("true", "1", "yes")

    path = Path(file_path)
    try:
        stat = path.stat()
        file_info = {
            "path": str(path.resolve()),
            "size": stat.st_size,
            "mtime": stat.st_mtime,
        }
    except OSError:  # pragma: no cover
        file_info = {
            "path": str(path),
            "size": 0,
            "mtime": 0,
        }

    config = config or GMFTConfig()
    cache_kwargs = {
        "file_info": str(sorted(file_info.items())),
        "extractor": "gmft",
        "config": str(sorted(msgspec.to_builtins(config).items())),
    }

    table_cache = get_table_cache()
    cached_result = table_cache.get(**cache_kwargs)
    if cached_result is not None:
        return cached_result  # type: ignore[no-any-return]

    if use_isolated_process:
        result = _extract_tables_isolated(file_path, config)

        table_cache.set(result, **cache_kwargs)

        return result

    try:
        from gmft.auto import AutoTableDetector, AutoTableFormatter  # type: ignore[attr-defined]  # noqa: PLC0415
        from gmft.detectors.tatr import TATRDetectorConfig  # type: ignore[attr-defined]  # noqa: PLC0415
        from gmft.formatters.tatr import TATRFormatConfig  # noqa: PLC0415
        from gmft.pdf_bindings.pdfium import PyPDFium2Document  # noqa: PLC0415

        formatter: Any = AutoTableFormatter(  # type: ignore[no-untyped-call]
            config=TATRFormatConfig(
                verbosity=config.verbosity,
                formatter_base_threshold=config.formatter_base_threshold,
                cell_required_confidence=config.cell_required_confidence,
                remove_null_rows=config.remove_null_rows,
                enable_multi_header=config.enable_multi_header,
                semantic_spanning_cells=config.semantic_spanning_cells,
                semantic_hierarchical_left_fill=config.semantic_hierarchical_left_fill,
                large_table_if_n_rows_removed=config.large_table_if_n_rows_removed,
                large_table_threshold=config.large_table_threshold,
                large_table_row_overlap_threshold=config.large_table_row_overlap_threshold,
                large_table_maximum_rows=config.large_table_maximum_rows,
                force_large_table_assumption=config.force_large_table_assumption,
            )
        )
        detector: Any = AutoTableDetector(  # type: ignore[no-untyped-call]
            config=TATRDetectorConfig(detector_base_threshold=config.detector_base_threshold)
        )
        doc = PyPDFium2Document(str(file_path))
        cropped_tables: list[Any] = []
        dataframes: list[Any] = []
        try:
            for page in doc:
                cropped_tables.extend(detector.extract(page))

            for cropped_table in cropped_tables:
                formatted_table = formatter.extract(cropped_table)
                dataframes.append(formatted_table.df())

            result = [
                TableData(
                    cropped_image=cropped_table.image(),
                    page_number=cropped_table.page.page_number,
                    text=_dataframe_to_markdown(data_frame),
                    df=data_frame,
                )
                for data_frame, cropped_table in zip(dataframes, cropped_tables, strict=False)
            ]

            table_cache.set(result, **cache_kwargs)

            return result
        finally:
            doc.close()  # type: ignore[no-untyped-call]

    except ImportError as e:  # pragma: no cover
        error = MissingDependencyError.create_for_package(
            dependency_group="gmft", functionality="table extraction", package_name="gmft"
        )
        error.context = {
            "file_path": str(Path(file_path)),
            "error_message": str(e),
        }
        raise error from e


def _extract_tables_in_process(
    file_path: str | PathLike[str],
    config_dict: dict[str, Any],
    result_queue: queue.Queue[tuple[bool, Any]],
) -> None:
    signal.signal(signal.SIGINT, signal.SIG_IGN)

    try:
        from gmft.auto import AutoTableDetector, AutoTableFormatter  # type: ignore[attr-defined]  # noqa: PLC0415
        from gmft.detectors.tatr import TATRDetectorConfig  # type: ignore[attr-defined]  # noqa: PLC0415
        from gmft.formatters.tatr import TATRFormatConfig  # noqa: PLC0415
        from gmft.pdf_bindings.pdfium import PyPDFium2Document  # noqa: PLC0415

        if "cell_required_confidence" in config_dict:
            cell_config = config_dict["cell_required_confidence"]
            if isinstance(cell_config, dict) and cell_config:
                config_dict["cell_required_confidence"] = {int(k): v for k, v in cell_config.items()}

        config = GMFTConfig(**config_dict)

        formatter = AutoTableFormatter(  # type: ignore[no-untyped-call]
            config=TATRFormatConfig(
                verbosity=config.verbosity,
                formatter_base_threshold=config.formatter_base_threshold,
                cell_required_confidence=config.cell_required_confidence,
                remove_null_rows=config.remove_null_rows,
                enable_multi_header=config.enable_multi_header,
                semantic_spanning_cells=config.semantic_spanning_cells,
                semantic_hierarchical_left_fill=config.semantic_hierarchical_left_fill,
                large_table_if_n_rows_removed=config.large_table_if_n_rows_removed,
                large_table_threshold=config.large_table_threshold,
                large_table_row_overlap_threshold=config.large_table_row_overlap_threshold,
                large_table_maximum_rows=config.large_table_maximum_rows,
                force_large_table_assumption=config.force_large_table_assumption,
            )
        )
        detector = AutoTableDetector(config=TATRDetectorConfig(detector_base_threshold=config.detector_base_threshold))  # type: ignore[no-untyped-call]

        doc = PyPDFium2Document(str(file_path))
        cropped_tables = []
        dataframes = []

        try:
            for page in doc:
                cropped_tables.extend(detector.extract(page))  # type: ignore[attr-defined]

            for cropped_table in cropped_tables:
                formatted_table = formatter.extract(cropped_table)  # type: ignore[attr-defined]
                dataframes.append(formatted_table.df())

            results = []
            for data_frame, cropped_table in zip(dataframes, cropped_tables, strict=False):
                img_bytes = io.BytesIO()
                cropped_image = cropped_table.image()
                cropped_image.save(img_bytes, format="PNG")
                img_bytes.seek(0)

                csv_data = _dataframe_to_csv(data_frame) if not _is_dataframe_empty(data_frame) else ""
                results.append(
                    {
                        "cropped_image_bytes": img_bytes.getvalue(),
                        "page_number": cropped_table.page.page_number,
                        "text": _dataframe_to_markdown(data_frame),
                        "df_columns": data_frame.columns,
                        "df_csv": csv_data if csv_data else None,
                    }
                )

            result_queue.put((True, results))

        finally:
            doc.close()  # type: ignore[no-untyped-call]

    except Exception as e:  # noqa: BLE001
        error_info = {"error": str(e), "type": type(e).__name__, "traceback": traceback.format_exc()}
        result_queue.put((False, error_info))


def _extract_tables_isolated(
    file_path: str | PathLike[str],
    config: GMFTConfig | None = None,
    timeout: float = 300.0,
) -> list[TableData]:
    config = config or GMFTConfig()
    config_dict = msgspec.to_builtins(config)

    ctx = mp.get_context("spawn")
    result_queue = ctx.Queue()

    process = ctx.Process(
        target=_extract_tables_in_process,
        args=(str(file_path), config_dict, result_queue),
    )

    process.start()

    try:
        # Wait for result with timeout, checking for process death  # ~keep

        start_time = time.time()
        while True:
            try:
                success, result = result_queue.get_nowait()
                break
            except queue.Empty:
                if time.time() - start_time > timeout:
                    raise

                if not process.is_alive():
                    # Process died without putting result  # ~keep
                    if process.exitcode == -signal.SIGSEGV:
                        raise ParsingError(
                            "GMFT process crashed with segmentation fault",
                            context={
                                "file_path": str(file_path),
                                "exit_code": process.exitcode,
                            },
                        ) from None
                    raise ParsingError(
                        f"GMFT process died unexpectedly with exit code {process.exitcode}",
                        context={
                            "file_path": str(file_path),
                            "exit_code": process.exitcode,
                        },
                    ) from None

                time.sleep(0.1)

        if success:
            tables = []
            for table_dict in result:
                img = Image.open(io.BytesIO(table_dict["cropped_image_bytes"]))

                if table_dict["df_csv"] is None or table_dict["df_csv"] == "":
                    df = pl.DataFrame()
                else:
                    df = pl.read_csv(StringIO(table_dict["df_csv"]), truncate_ragged_lines=True)

                tables.append(
                    TableData(
                        cropped_image=img,
                        page_number=table_dict["page_number"],
                        text=table_dict["text"],
                        df=df,
                    )
                )

            return tables

        error_info = result
        if error_info.get("type") == "ImportError":
            error = MissingDependencyError.create_for_package(
                dependency_group="gmft", functionality="table extraction", package_name="gmft"
            )
            error.context = {
                "file_path": str(Path(file_path)),
                "error_message": error_info["error"],
                "traceback": error_info.get("traceback"),
            }
            raise error from ImportError(error_info["error"])

        raise ParsingError(
            f"GMFT table extraction failed: {error_info['error']}",
            context={
                "file_path": str(file_path),
                "error_type": error_info["type"],
                "traceback": error_info["traceback"],
            },
        )

    except queue.Empty as e:
        raise ParsingError(
            "GMFT table extraction timed out",
            context={
                "file_path": str(file_path),
                "timeout": timeout,
            },
        ) from e
    finally:
        if process.is_alive():
            process.terminate()
            process.join(timeout=5)
            if process.is_alive():
                process.kill()
                process.join()


async def _extract_tables_isolated_async(
    file_path: str | PathLike[str],
    config: GMFTConfig | None = None,
    timeout: float = 300.0,  # noqa: ASYNC109
) -> list[TableData]:
    config = config or GMFTConfig()
    config_dict = msgspec.to_builtins(config)

    ctx = mp.get_context("spawn")
    result_queue = ctx.Queue()

    process = ctx.Process(
        target=_extract_tables_in_process,
        args=(str(file_path), config_dict, result_queue),
    )

    process.start()

    try:

        def get_result_sync() -> tuple[bool, Any]:
            while True:
                try:
                    return result_queue.get(timeout=0.1)  # type: ignore[no-any-return]
                except queue.Empty:  # noqa: PERF203
                    if not process.is_alive():
                        if process.exitcode == -signal.SIGSEGV:
                            raise ParsingError(
                                "GMFT process crashed with segmentation fault",
                                context={"file_path": str(file_path), "exit_code": process.exitcode},
                            ) from None
                        raise ParsingError(
                            f"GMFT process died unexpectedly with exit code {process.exitcode}",
                            context={"file_path": str(file_path), "exit_code": process.exitcode},
                        ) from None

        with anyio.fail_after(timeout):
            success, result = await anyio.to_thread.run_sync(get_result_sync)

        if success:
            tables = []
            for table_dict in result:
                img = Image.open(io.BytesIO(table_dict["cropped_image_bytes"]))

                if table_dict["df_csv"] is None or table_dict["df_csv"] == "":
                    df = pl.DataFrame()
                else:
                    df = pl.read_csv(StringIO(table_dict["df_csv"]), truncate_ragged_lines=True)

                tables.append(
                    TableData(
                        cropped_image=img,
                        page_number=table_dict["page_number"],
                        text=table_dict["text"],
                        df=df,
                    )
                )

            return tables

        error_info = result
        if error_info.get("type") == "ImportError":
            error = MissingDependencyError.create_for_package(
                dependency_group="gmft", functionality="table extraction", package_name="gmft"
            )
            error.context = {
                "file_path": str(Path(file_path)),
                "error_message": error_info["error"],
                "traceback": error_info.get("traceback"),
            }
            raise error from ImportError(error_info["error"])

        raise ParsingError(
            f"GMFT table extraction failed: {error_info['error']}",
            context={
                "file_path": str(file_path),
                "error_type": error_info["type"],
                "traceback": error_info["traceback"],
            },
        )

    except TimeoutError as e:
        raise ParsingError(
            "GMFT table extraction timed out",
            context={
                "file_path": str(file_path),
                "timeout": timeout,
            },
        ) from e
    finally:
        if process.is_alive():
            process.terminate()
            await anyio.to_thread.run_sync(lambda: process.join(timeout=5))
            if process.is_alive():
                process.kill()
                await anyio.to_thread.run_sync(process.join)
