from __future__ import annotations

import io
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from kreuzberg._types import TableData


def export_table_to_csv(table: TableData, separator: str = ",") -> str:
    if "df" not in table or table["df"] is None:
        return ""

    buffer = io.StringIO()
    df = table["df"]
    df.write_csv(buffer, separator=separator, include_header=True)
    return buffer.getvalue().strip()


def export_table_to_tsv(table: TableData) -> str:
    return export_table_to_csv(table, separator="\t")


def enhance_table_markdown(table: TableData) -> str:
    if "df" not in table or table["df"] is None:
        return table.get("text", "")

    df = table["df"]

    if df.is_empty():
        return table.get("text", "")

    lines = []

    headers = [str(col).strip() for col in df.columns]
    lines.append("| " + " | ".join(headers) + " |")

    lines.append(_generate_separator_row(df))

    float_col_formatting = _analyze_float_columns(df)

    for row in df.iter_rows(named=True):
        formatted_row = _format_table_row(row, df, float_col_formatting)
        lines.append("| " + " | ".join(formatted_row) + " |")

    return "\n".join(lines)


def _generate_separator_row(df: Any) -> str:
    separators = []
    for col in df.columns:
        dtype_str = str(df[col].dtype)
        if dtype_str in ["Int64", "Float64", "Int32", "Float32"] or _is_numeric_column(df[col]):
            separators.append("---:")
        else:
            separators.append("---")
    return "| " + " | ".join(separators) + " |"


def _analyze_float_columns(df: Any) -> dict[str, str]:
    float_col_formatting = {}
    for col in df.columns:
        dtype_str = str(df[col].dtype)
        if dtype_str in ["Float64", "Float32"]:
            non_null_values = df[col].drop_nulls()
            if len(non_null_values) > 0:
                try:
                    values_list = non_null_values.to_list()
                    all_integers = all(float(val).is_integer() for val in values_list if val is not None)
                    float_col_formatting[col] = "int" if all_integers else "float"
                except (ValueError, AttributeError):
                    float_col_formatting[col] = "float"
            else:
                float_col_formatting[col] = "int"
    return float_col_formatting


def _format_table_row(row: Any, df: Any, float_col_formatting: dict[str, str]) -> list[str]:
    formatted_row = []
    for col_name, value in row.items():
        if value is None:
            formatted_row.append("")
        else:
            dtype_str = str(df[col_name].dtype)
            if dtype_str in ["Int64", "Int32"]:
                formatted_row.append(str(int(value)))
            elif isinstance(value, float):
                if col_name in float_col_formatting and float_col_formatting[col_name] == "int":
                    formatted_row.append(str(int(value)))
                else:
                    formatted_row.append(f"{value:.2f}")
            elif isinstance(value, bool):
                formatted_row.append(str(value).lower())
            else:
                clean_value = str(value).strip().replace("|", "\\|")
                formatted_row.append(clean_value)
    return formatted_row


def _is_numeric_column(series: Any) -> bool:
    if len(series) == 0:
        return False

    try:
        dtype_str = str(series.dtype)
        if dtype_str in {"Int64", "Float64", "Int32", "Float32"}:
            return True

        sample_size = min(100, len(series))
        series_no_nulls = series.drop_nulls()

        if len(series_no_nulls) == 0:
            return False

        sample_series = series_no_nulls.slice(0, sample_size) if len(series_no_nulls) > 1000 else series_no_nulls

        if len(sample_series) == 0:
            return False

        numeric_count = 0
        for val in sample_series.to_list():
            val_str = str(val).replace(",", "").replace("$", "").replace("%", "")
            if val_str and all(c in "0123456789.-+eE" for c in val_str):
                try:
                    float(val_str)
                    numeric_count += 1
                except (ValueError, TypeError):
                    pass

        return (numeric_count / len(sample_series)) > 0.7

    except (ValueError, TypeError, ZeroDivisionError):
        return False


def generate_table_summary(tables: list[TableData]) -> dict[str, Any]:
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


def extract_table_structure_info(table: TableData) -> dict[str, Any]:
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

    for col in df.columns:
        if _is_numeric_column(df[col]):
            info["numeric_columns"] += 1
        else:
            info["text_columns"] += 1

    total_cells = df.height * df.width
    if total_cells > 0:
        null_counts = df.null_count()
        empty_cells = sum(null_counts.row(0))
        info["empty_cells"] = empty_cells
        info["data_density"] = (total_cells - empty_cells) / total_cells

    return info
