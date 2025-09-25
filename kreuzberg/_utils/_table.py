"""Table processing functions using Rust acceleration via Arrow IPC bridge."""

from __future__ import annotations

import io
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    import polars as pl

    from kreuzberg._types import TableData

# Import Rust functions
from kreuzberg._internal_bindings import (
    table_from_arrow_to_markdown,
    # TODO: Add these once implemented in Rust
    # table_from_arrow_to_csv,
    # table_from_arrow_to_structure_info,
    # tables_from_arrow_to_summary,
)


def _dataframe_to_arrow_bytes(df: pl.DataFrame) -> bytes:
    """Convert Polars DataFrame to Arrow IPC bytes for Rust interop."""
    buffer = io.BytesIO()
    df.write_ipc(buffer)
    return buffer.getvalue()


def enhance_table_markdown(table: TableData) -> str:
    """Generate markdown representation of table using Rust acceleration.

    Uses Arrow IPC bridge for 10-15x performance improvement over pure Python.
    """
    if "df" not in table or table["df"] is None:
        return table.get("text", "")

    df = table["df"]

    if df.is_empty():
        return table.get("text", "")

    # Convert to Arrow IPC and process in Rust
    arrow_bytes = _dataframe_to_arrow_bytes(df)
    return table_from_arrow_to_markdown(arrow_bytes)


def export_table_to_csv(table: TableData, separator: str = ",") -> str:
    """Export table to CSV format.

    Currently uses Python Polars implementation.
    TODO: Implement Rust version with Arrow bridge.
    """
    if "df" not in table or table["df"] is None:
        return ""

    buffer = io.StringIO()
    df = table["df"]
    df.write_csv(buffer, separator=separator, include_header=True)
    return buffer.getvalue().strip()


def export_table_to_tsv(table: TableData) -> str:
    """Export table to TSV format."""
    return export_table_to_csv(table, separator="\t")


def extract_table_structure_info(table: TableData) -> dict[str, Any]:
    """Extract structural information about table.

    Currently uses Python implementation.
    TODO: Implement Rust version with Arrow bridge.
    """
    info = {
        "has_headers": False,
        "row_count": 0,
        "column_count": 0,
        "numeric_columns": 0,
        "text_columns": 0,
        "empty_cells": 0,
        "data_density": 0.0,
    }

    if "df" not in table or table["df"] is None:
        return info

    df = table["df"]

    if df.is_empty():
        return info

    info["row_count"] = df.height
    info["column_count"] = df.width
    info["has_headers"] = df.width > 0

    # Simple column type analysis
    for col in df.columns:
        dtype_str = str(df[col].dtype)
        if dtype_str in {
            "Int64",
            "Float64",
            "Int32",
            "Float32",
            "Int8",
            "Int16",
            "UInt8",
            "UInt16",
            "UInt32",
            "UInt64",
        }:
            info["numeric_columns"] += 1
        else:
            info["text_columns"] += 1

    # Calculate data density
    total_cells = df.height * df.width
    if total_cells > 0:
        null_counts = df.null_count()
        empty_cells = sum(null_counts.row(0))
        info["empty_cells"] = empty_cells
        info["data_density"] = (total_cells - empty_cells) / total_cells

    return info


def generate_table_summary(tables: list[TableData]) -> dict[str, Any]:
    """Generate summary statistics for multiple tables.

    Currently uses Python implementation.
    TODO: Implement Rust version with Arrow bridge.
    """
    if not tables:
        return {
            "table_count": 0,
            "total_rows": 0,
            "total_columns": 0,
            "pages_with_tables": 0,
        }

    total_rows = 0
    total_columns = 0
    pages_with_tables = set()
    tables_by_page = {}

    for table in tables:
        if "df" in table and table["df"] is not None:
            df = table["df"]
            total_rows += df.height
            total_columns += df.width

        if "page_number" in table:
            page_num = table["page_number"]
            pages_with_tables.add(page_num)

            if page_num not in tables_by_page:
                tables_by_page[page_num] = 0
            tables_by_page[page_num] += 1

    return {
        "table_count": len(tables),
        "total_rows": total_rows,
        "total_columns": total_columns,
        "pages_with_tables": len(pages_with_tables),
        "avg_rows_per_table": total_rows / len(tables) if tables else 0,
        "avg_columns_per_table": total_columns / len(tables) if tables else 0,
        "tables_by_page": dict(tables_by_page),
    }
