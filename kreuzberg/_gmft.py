from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Literal

from kreuzberg._types import TableData
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from os import PathLike

    from gmft.detectors.base import CroppedTable
    from pandas import DataFrame


@dataclass(unsafe_hash=True)
class GMFTConfig:
    """Configuration options for GMFT.

    This class encapsulates the configuration options for GMFT, providing a way to customize its behavior.
    """

    verbosity: int = 0
    """
    Verbosity level for logging.

    0: errors only
    1: print warnings
    2: print warnings and info
    3: print warnings, info, and debug
    """
    formatter_base_threshold: float = 0.3
    """
    Base threshold for the confidence demanded of a table feature (row/column).

    Note that a low threshold is actually better, because overzealous rows means that generally, numbers are still aligned and there are just many empty rows (having fewer rows than expected merges cells, which is bad).
    """
    cell_required_confidence: dict[Literal[0, 1, 2, 3, 4, 5, 6], float] = field(
        default_factory=lambda: {
            0: 0.3,
            1: 0.3,
            2: 0.3,
            3: 0.3,
            4: 0.5,
            5: 0.5,
            6: 99,
        },
        hash=False,
    )
    """
    Confidences required (>=) for a row/column feature to be considered good. See TATRFormattedTable.id2label

    But low confidences may be better than too high confidence (see formatter_base_threshold)
    """
    detector_base_threshold: float = 0.9
    """Minimum confidence score required for a table"""
    remove_null_rows: bool = True
    """
    Flag to remove rows with no text.
    """
    enable_multi_header: bool = False
    """
    Enable multi-indices in the dataframe.

    If false, then multiple headers will be merged column-wise.
    """
    semantic_spanning_cells: bool = False
    """
    [Experimental] Enable semantic spanning cells, which often encode hierarchical multi-level indices.
    """
    semantic_hierarchical_left_fill: str | None = "algorithm"
    """
    [Experimental] When semantic spanning cells is enabled, when a left header is detected which might represent a group of rows, that same value is reduplicated for each row.

    Possible values: 'algorithm', 'deep', None.

    'algorithm': assumes that the higher-level header is always the first row followed by several empty rows.
    'deep': merges headers according to the spanning cells detected by the Table Transformer.
    None: headers are not duplicated.
    """
    large_table_if_n_rows_removed: int = 8
    """
    If >= n rows are removed due to non-maxima suppression (NMS), then this table is classified as a large table.
    """
    large_table_threshold: int = 10
    """
    With large tables, table transformer struggles with placing too many overlapping rows. Luckily, with more rows, we have more info on the usual size of text, which we can use to make a guess on the height such that no rows are merged or overlapping.

    Large table assumption is only applied when (# of rows > large_table_threshold) AND (total overlap > large_table_row_overlap_threshold). Set 9999 to disable; set 0 to force large table assumption to run every time.
    """
    large_table_row_overlap_threshold: float = 0.2
    """
    With large tables, table transformer struggles with placing too many overlapping rows. Luckily, with more rows, we have more info on the usual size of text, which we can use to make a guess on the height such that no rows are merged or overlapping.

    Large table assumption is only applied when (# of rows > large_table_threshold) AND (total overlap > large_table_row_overlap_threshold).
    """
    large_table_maximum_rows: int = 1000
    """
    Maximum number of rows allowed for a large table.
    """
    force_large_table_assumption: bool | None = None
    """
    Force the large table assumption to be applied, regardless of the number of rows and overlap.
    """


async def extract_tables(file_path: str | PathLike[str], config: GMFTConfig | None = None) -> list[TableData]:
    """Extracts tables from a PDF file.

    This function takes a file path to a PDF file, and an optional configuration object.
    It returns a list of strings, where each string is a markdown-formatted table.

    Args:
        file_path: The path to the PDF file.
        config: An optional configuration object.

    Raises:
        MissingDependencyError: Raised when the required dependencies are not installed.

    Returns:
        A list of table data dictionaries.
    """
    try:
        from gmft.auto import AutoTableDetector, AutoTableFormatter
        from gmft.detectors.tatr import TATRDetectorConfig
        from gmft.formatters.tatr import TATRFormatConfig
        from gmft.pdf_bindings.pdfium import PyPDFium2Document

        config = config or GMFTConfig()
        formatter = AutoTableFormatter(
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
        detector = AutoTableDetector(config=TATRDetectorConfig(detector_base_threshold=config.detector_base_threshold))
        doc = await run_sync(PyPDFium2Document, str(file_path))
        cropped_tables: list[CroppedTable] = []
        dataframes: list[DataFrame] = []
        try:
            for page in doc:
                cropped_tables.extend(await run_sync(detector.extract, page))

            for cropped_table in cropped_tables:
                formatted_table = await run_sync(formatter.extract, cropped_table)
                dataframes.append(await run_sync(formatted_table.df))

            return [
                TableData(
                    cropped_image=cropped_table.image(),
                    page_number=cropped_table.page.page_number,
                    text=data_frame.to_markdown(),
                    df=data_frame,
                )
                for data_frame, cropped_table in zip(dataframes, cropped_tables)
            ]
        finally:
            await run_sync(doc.close)

    except ImportError as e:
        raise MissingDependencyError.create_for_package(
            dependency_group="gmft", functionality="table extraction", package_name="gmft"
        ) from e
