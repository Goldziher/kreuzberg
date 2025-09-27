"""Table extraction using Table Transformer (TATR) adapted from GMFT.

This module provides table detection and structure extraction capabilities
adapted from GMFT (https://github.com/conjuncts/gmft) to work with Kreuzberg's
architecture and patterns.

See ATTRIBUTION.md for proper attribution and licensing information.
"""

from __future__ import annotations

import logging
from functools import lru_cache
from pathlib import Path
from typing import TYPE_CHECKING

from kreuzberg._constants import PDF_POINTS_PER_INCH
from kreuzberg._internal_bindings import calculate_optimal_dpi
from kreuzberg._types import GMFTConfig
from kreuzberg._utils._model_cache import (
    setup_huggingface_cache,
    setup_huggingface_cache_async,
)
from kreuzberg._utils._resource_managers import image_resources, pdf_document_sync, pdf_resources_sync
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._table import enhance_table_markdown

if TYPE_CHECKING:
    from kreuzberg._types import TableData

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
    "extract_tables_async",
    "extract_tables_sync",
]

logger = logging.getLogger(__name__)


@lru_cache(maxsize=2)
def _get_cached_detector(
    detection_model: str, detection_threshold: float, cache_dir: str | None = None
) -> TableDetector:
    """Get cached table detector instance.

    HuggingFace handles model downloads automatically.
    """
    # Set up cache directory if provided
    if cache_dir:
        setup_huggingface_cache(cache_dir)

    # Create config with specific detection settings
    config = GMFTConfig(
        detection_model=detection_model,
        detection_threshold=detection_threshold,
        model_cache_dir=cache_dir,
    )

    return TableDetector(config)


async def _get_cached_detector_async(
    detection_model: str, detection_threshold: float, cache_dir: str | None = None
) -> TableDetector:
    """Get cached table detector instance (async version).

    HuggingFace handles model downloads automatically.
    """
    # Set up cache directory if provided
    if cache_dir:
        await setup_huggingface_cache_async(cache_dir)

    # Create config with specific detection settings
    config = GMFTConfig(
        detection_model=detection_model,
        detection_threshold=detection_threshold,
        model_cache_dir=cache_dir,
    )

    return TableDetector(config)


@lru_cache(maxsize=2)
def _get_cached_formatter(
    structure_model: str, structure_threshold: float, cache_dir: str | None = None
) -> TableFormatter:
    """Get cached table formatter instance."""
    # Set up cache directory if provided
    if cache_dir:
        setup_huggingface_cache(cache_dir)

    # Create config with specific structure settings
    config = GMFTConfig(
        structure_model=structure_model,
        structure_threshold=structure_threshold,
        model_cache_dir=cache_dir,
    )
    return TableFormatter(config)


async def _get_cached_formatter_async(
    structure_model: str, structure_threshold: float, cache_dir: str | None = None
) -> TableFormatter:
    """Get cached table formatter instance (async version)."""
    # Set up cache directory if provided
    if cache_dir:
        await setup_huggingface_cache_async(cache_dir)

    # Create config with specific structure settings
    config = GMFTConfig(
        structure_model=structure_model,
        structure_threshold=structure_threshold,
        model_cache_dir=cache_dir,
    )
    return TableFormatter(config)


async def extract_tables_async(file_path: str | Path, config: GMFTConfig | None = None) -> list[TableData]:
    """Extract tables from PDF documents using Table Transformer (TATR).

    Uses Microsoft's Table Transformer models for table detection and structure
    recognition, adapted from GMFT with Kreuzberg patterns.

    Args:
        file_path: Path to PDF file to extract tables from
        config: Optional GMFT configuration for customizing extraction behavior

    Returns:
        List of TableData objects containing extracted tables with:
        - cropped_image: PIL Image of the table region
        - df: Polars DataFrame with structured table data
        - page_number: Page number where table was found
        - text: Markdown representation of the table

    Raises:
        MissingDependencyError: If transformers or torch dependencies are missing
        ParsingError: If PDF processing or table extraction fails
    """
    return await run_sync(extract_tables_sync, file_path, config)


def extract_tables_sync(file_path: str | Path, config: GMFTConfig | None = None) -> list[TableData]:
    """Synchronous table extraction from PDF documents.

    Leverages Kreuzberg's existing PDF processing facilities and DPI optimization.

    Args:
        file_path: Path to PDF file to extract tables from
        config: Optional GMFT configuration for customizing extraction behavior

    Returns:
        List of TableData objects containing extracted tables

    Raises:
        MissingDependencyError: If transformers or torch dependencies are missing
        ParsingError: If PDF processing or table extraction fails
    """
    # Convert to Path object
    pdf_path = Path(file_path)

    # Use default config if none provided
    if config is None:
        config = GMFTConfig()

    # Get cached detector and formatter instances
    detector = _get_cached_detector(config.detection_model, config.detection_threshold, config.model_cache_dir)
    formatter = _get_cached_formatter(config.structure_model, config.structure_threshold, config.model_cache_dir)

    tables = []

    # Use Kreuzberg's PDF facilities
    with pdf_document_sync(pdf_path) as document:
        for page_idx in range(len(document)):
            page = document[page_idx]
            width, height = page.get_size()

            # Use Kreuzberg's DPI optimization (from Rust bindings)
            optimal_dpi = calculate_optimal_dpi(
                page_width=width,
                page_height=height,
                target_dpi=150,  # Standard DPI for table extraction
                max_dimension=25000,
                min_dpi=72,
                max_dpi=600,
            )

            scale = optimal_dpi / PDF_POINTS_PER_INCH

            # Render page to image using Kreuzberg's facilities
            bitmap = page.render(scale=scale)
            page_image = bitmap.to_pil()

            with pdf_resources_sync(bitmap), image_resources(page_image):
                # Detect tables on the page
                detected_tables = detector.detect_tables_in_page_region(page_image, page_idx)

                # Process each detected table
                for cropped_table in detected_tables:
                    # Crop the table region from the page image
                    rect = cropped_table.rect
                    table_image = page_image.crop((rect.xmin, rect.ymin, rect.xmax, rect.ymax))

                    with image_resources(table_image):
                        # Format the table structure
                        formatted_table = formatter.format_table(cropped_table, table_image)

                        # Convert to TableData format expected by Kreuzberg
                        table_text = enhance_table_markdown(
                            {
                                "df": formatted_table.dataframe,
                                "text": "",
                            }
                        )

                        table_data: TableData = {
                            "cropped_image": table_image.copy(),  # Make a copy for safe keeping
                            "df": formatted_table.dataframe,
                            "page_number": page_idx + 1,  # 1-indexed for user display
                            "text": table_text,
                        }

                        tables.append(table_data)

    logger.info("Extracted %d tables from %s", len(tables), pdf_path)
    return tables


# Backward compatibility alias
extract_tables = extract_tables_async
