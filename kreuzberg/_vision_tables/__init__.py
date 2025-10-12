from __future__ import annotations

import logging
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

from kreuzberg._constants import PDF_POINTS_PER_INCH
from kreuzberg._internal_bindings import calculate_optimal_dpi
from kreuzberg._types import TableExtractionConfig
from kreuzberg._utils._model_cache import (
    setup_huggingface_cache,
    setup_huggingface_cache_async,
)
from kreuzberg._utils._resource_managers import image_resources, pdf_document_sync, pdf_resources_sync
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._table import enhance_table_markdown
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    import polars as pl

    from kreuzberg._types import TableData

from ._algorithm import _build_dataframe_from_ocr
from ._base import Rect
from ._detector import TableDetector
from ._formatter import TableFormatter
from ._types import CroppedTable, FormattedTable

__all__ = [
    "CroppedTable",
    "FormattedTable",
    "Rect",
    "TableDetector",
    "TableFormatter",
    "clear_table_extraction_caches",
    "extract_tables_async",
    "extract_tables_sync",
]

logger = logging.getLogger(__name__)


def _dataframe_has_content(df: pl.DataFrame) -> bool:
    if df.is_empty():
        return False

    for row in df.iter_rows():
        for value in row:
            if isinstance(value, str):
                if value.strip():
                    return True
            elif value is not None:
                return True

    return False


@lru_cache(maxsize=2)
def _get_cached_detector(
    detection_model: str, detection_threshold: float, cache_dir: str | None = None
) -> TableDetector:
    if cache_dir:
        setup_huggingface_cache(cache_dir)

    config = TableExtractionConfig(
        detection_model=detection_model,
        detection_threshold=detection_threshold,
        model_cache_dir=cache_dir,
    )

    return TableDetector(config)


async def _get_cached_detector_async(
    detection_model: str, detection_threshold: float, cache_dir: str | None = None
) -> TableDetector:
    if cache_dir:
        await setup_huggingface_cache_async(cache_dir)

    config = TableExtractionConfig(
        detection_model=detection_model,
        detection_threshold=detection_threshold,
        model_cache_dir=cache_dir,
    )

    return TableDetector(config)


@lru_cache(maxsize=2)
def _get_cached_formatter(
    structure_model: str, structure_threshold: float, cache_dir: str | None = None
) -> TableFormatter:
    if cache_dir:
        setup_huggingface_cache(cache_dir)

    config = TableExtractionConfig(
        structure_model=structure_model,
        structure_threshold=structure_threshold,
        model_cache_dir=cache_dir,
    )
    return TableFormatter(config)


async def _get_cached_formatter_async(
    structure_model: str, structure_threshold: float, cache_dir: str | None = None
) -> TableFormatter:
    if cache_dir:
        await setup_huggingface_cache_async(cache_dir)

    config = TableExtractionConfig(
        structure_model=structure_model,
        structure_threshold=structure_threshold,
        model_cache_dir=cache_dir,
    )
    return TableFormatter(config)


def clear_table_extraction_caches() -> None:
    _get_cached_detector.cache_clear()
    _get_cached_formatter.cache_clear()


async def extract_tables_async(file_path: str | Path, config: TableExtractionConfig | None = None) -> list[TableData]:
    return await run_sync(extract_tables_sync, file_path, config)


def extract_tables_sync(file_path: str | Path, config: TableExtractionConfig | None = None) -> list[TableData]:
    pdf_path = Path(file_path)

    if config is None:
        config = TableExtractionConfig()

    detector = _get_cached_detector(config.detection_model, config.detection_threshold, config.model_cache_dir)
    formatter: TableFormatter | None = None
    if not config.extract_from_ocr:
        formatter = _get_cached_formatter(config.structure_model, config.structure_threshold, config.model_cache_dir)

    tables = []

    with pdf_document_sync(pdf_path) as document:
        for page_idx in range(len(document)):
            page = document[page_idx]
            width, height = page.get_size()

            optimal_dpi = calculate_optimal_dpi(
                page_width=width,
                page_height=height,
                target_dpi=150,
                max_dimension=25000,
                min_dpi=72,
                max_dpi=600,
            )

            scale = optimal_dpi / PDF_POINTS_PER_INCH

            bitmap = page.render(scale=scale)
            page_image = bitmap.to_pil()

            with pdf_resources_sync(bitmap), image_resources(page_image):
                detected_tables = detector.detect_tables_in_page_region(page_image, page_idx)

                for cropped_table in detected_tables:
                    rect = cropped_table.rect
                    table_image = page_image.crop((rect.xmin, rect.ymin, rect.xmax, rect.ymax))

                    with image_resources(table_image):
                        temp_table_data: TableData
                        table_data: TableData

                        if config.extract_from_ocr:
                            dataframe = _build_dataframe_from_ocr(table_image)
                            temp_table_data = {
                                "cropped_image": table_image.copy(),
                                "df": dataframe if not dataframe.is_empty() else None,
                                "page_number": page_idx + 1,
                                "text": "",
                            }
                            table_text = enhance_table_markdown(temp_table_data) if not dataframe.is_empty() else ""
                            table_data = {
                                "cropped_image": table_image.copy(),
                                "df": dataframe if not dataframe.is_empty() else None,
                                "page_number": page_idx + 1,
                                "text": table_text,
                            }
                        else:
                            if formatter is None:
                                raise MissingDependencyError(
                                    "Table formatting requires 'transformers' and 'torch' packages. "
                                    "Install with: pip install 'kreuzberg[vision-tables]'"
                                )

                            formatted_table = formatter.format_table(cropped_table, table_image)
                            dataframe = formatted_table.dataframe

                            if not _dataframe_has_content(dataframe):
                                dataframe = _build_dataframe_from_ocr(table_image)

                            temp_table_data = {
                                "cropped_image": table_image.copy(),
                                "df": dataframe if not dataframe.is_empty() else None,
                                "page_number": page_idx + 1,
                                "text": "",
                            }

                            table_text = enhance_table_markdown(temp_table_data) if not dataframe.is_empty() else ""

                            table_data = {
                                "cropped_image": table_image.copy(),
                                "df": dataframe if not dataframe.is_empty() else None,
                                "page_number": page_idx + 1,
                                "text": table_text,
                            }

                        tables.append(table_data)

    logger.info("Extracted %d tables from %s", len(tables), pdf_path)
    return tables


extract_tables = extract_tables_async
