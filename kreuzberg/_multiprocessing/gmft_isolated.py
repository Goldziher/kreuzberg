"""Isolated GMFT table extraction to handle segmentation faults."""

from __future__ import annotations

import multiprocessing as mp
import pickle
import queue
import signal
import traceback
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from os import PathLike

    from kreuzberg._gmft import GMFTConfig
    from kreuzberg._types import TableData


def _extract_tables_in_process(
    file_path: str | PathLike[str],
    config_dict: dict[str, Any],
    result_queue: queue.Queue[tuple[bool, Any]],
) -> None:
    """Extract tables in an isolated process to handle potential segfaults.

    Args:
        file_path: Path to the PDF file
        config_dict: Serialized GMFTConfig as a dict
        result_queue: Queue to put results or errors
    """
    signal.signal(signal.SIGINT, signal.SIG_IGN)

    try:
        from gmft.auto import AutoTableDetector, AutoTableFormatter  # type: ignore[attr-defined]
        from gmft.detectors.tatr import TATRDetectorConfig  # type: ignore[attr-defined]
        from gmft.formatters.tatr import TATRFormatConfig
        from gmft.pdf_bindings.pdfium import PyPDFium2Document

        from kreuzberg._gmft import GMFTConfig

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
        detector = AutoTableDetector(  # type: ignore[no-untyped-call]
            config=TATRDetectorConfig(detector_base_threshold=config.detector_base_threshold)
        )

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
                import io

                img_bytes = io.BytesIO()
                cropped_image = cropped_table.image()
                cropped_image.save(img_bytes, format="PNG")
                img_bytes.seek(0)

                results.append(
                    {
                        "cropped_image_bytes": img_bytes.getvalue(),
                        "page_number": cropped_table.page.page_number,
                        "text": data_frame.to_markdown(),
                        "df_pickle": pickle.dumps(data_frame),
                    }
                )

            result_queue.put((True, results))

        finally:
            doc.close()  # type: ignore[no-untyped-call]

    except Exception as e:  # noqa: BLE001
        error_info = {"error": str(e), "type": type(e).__name__, "traceback": traceback.format_exc()}
        result_queue.put((False, error_info))


def extract_tables_isolated(
    file_path: str | PathLike[str],
    config: GMFTConfig | None = None,
    timeout: float = 300.0,
) -> list[TableData]:
    """Extract tables using an isolated process to handle segfaults.

    Args:
        file_path: Path to the PDF file
        config: GMFT configuration
        timeout: Maximum time to wait for extraction

    Returns:
        List of extracted tables

    Raises:
        RuntimeError: If extraction fails or times out
    """
    from kreuzberg._gmft import GMFTConfig
    from kreuzberg._types import TableData
    from kreuzberg.exceptions import ParsingError

    config = config or GMFTConfig()
    config_dict = config.__dict__.copy()

    ctx = mp.get_context("spawn")
    result_queue = ctx.Queue()

    process = ctx.Process(
        target=_extract_tables_in_process,
        args=(str(file_path), config_dict, result_queue),
    )

    process.start()

    try:
        # Wait for result with timeout, checking for process death  # ~keep
        import time

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
                import io
                import pickle

                from PIL import Image

                img = Image.open(io.BytesIO(table_dict["cropped_image_bytes"]))
                df = pickle.loads(table_dict["df_pickle"])  # noqa: S301

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


async def extract_tables_isolated_async(
    file_path: str | PathLike[str],
    config: GMFTConfig | None = None,
    timeout: float = 300.0,
) -> list[TableData]:
    """Async version of extract_tables_isolated using asyncio.

    Args:
        file_path: Path to the PDF file
        config: GMFT configuration
        timeout: Maximum time to wait for extraction

    Returns:
        List of extracted tables

    Raises:
        RuntimeError: If extraction fails or times out
    """
    import anyio

    from kreuzberg._gmft import GMFTConfig
    from kreuzberg._types import TableData
    from kreuzberg.exceptions import ParsingError

    config = config or GMFTConfig()
    config_dict = config.__dict__.copy()

    ctx = mp.get_context("spawn")
    result_queue = ctx.Queue()

    process = ctx.Process(
        target=_extract_tables_in_process,
        args=(str(file_path), config_dict, result_queue),
    )

    process.start()

    try:

        async def wait_for_result() -> tuple[bool, Any]:
            while True:
                try:
                    return result_queue.get_nowait()  # type: ignore[no-any-return]
                except queue.Empty:  # noqa: PERF203
                    await anyio.sleep(0.1)
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

        with anyio.fail_after(timeout):
            success, result = await wait_for_result()

        if success:
            tables = []
            for table_dict in result:
                import io
                import pickle

                from PIL import Image

                img = Image.open(io.BytesIO(table_dict["cropped_image_bytes"]))
                df = pickle.loads(table_dict["df_pickle"])  # noqa: S301

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
