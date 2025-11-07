from __future__ import annotations

import csv
from io import StringIO
from typing import TYPE_CHECKING

import numpy as np

from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from kreuzberg._types import TSVWord


def extract_words(tsv_data: str, *, min_confidence: float = 30.0) -> list[TSVWord]:
    try:
        reader = csv.DictReader(StringIO(tsv_data), delimiter="\t")
        words: list[TSVWord] = []

        for row in reader:
            if row.get("level") == "5" and row.get("text", "").strip():
                try:
                    conf = float(row["conf"])
                    if conf < min_confidence:
                        continue

                    words.append(
                        {
                            "level": int(row["level"]),
                            "page_num": int(row["page_num"]),
                            "block_num": int(row["block_num"]),
                            "par_num": int(row["par_num"]),
                            "line_num": int(row["line_num"]),
                            "word_num": int(row["word_num"]),
                            "left": int(row["left"]),
                            "top": int(row["top"]),
                            "width": int(row["width"]),
                            "height": int(row["height"]),
                            "conf": conf,
                            "text": row["text"],
                        }
                    )
                except (ValueError, KeyError):
                    continue

        return words

    except Exception as e:
        raise ParsingError("Failed to parse TSV data", context={"error": str(e)}) from e


def detect_columns(words: list[TSVWord], *, column_threshold: int = 20) -> list[int]:
    if not words:
        return []

    x_positions = sorted({w["left"] for w in words})

    if len(x_positions) == 1:
        return x_positions

    columns = []
    current_group = [x_positions[0]]

    for x in x_positions[1:]:
        if x - current_group[-1] <= column_threshold:
            current_group.append(x)
        else:
            columns.append(int(np.median(current_group)))
            current_group = [x]

    columns.append(int(np.median(current_group)))
    return columns


def detect_rows(words: list[TSVWord], *, row_threshold_ratio: float = 0.5) -> list[int]:
    if not words:
        return []

    y_centers = sorted(w["top"] + w["height"] / 2 for w in words)

    if len(y_centers) == 1:
        return [int(y_centers[0])]

    mean_height = np.mean([w["height"] for w in words])
    threshold = mean_height * row_threshold_ratio

    rows = []
    current_group = [y_centers[0]]

    for y in y_centers[1:]:
        if y - np.mean(current_group) <= threshold:
            current_group.append(y)
        else:
            rows.append(int(np.median(current_group)))
            current_group = [y]

    rows.append(int(np.median(current_group)))
    return rows


def _find_closest_index(value: float, positions: list[int]) -> int:
    if not positions:
        return 0

    distances = [abs(value - pos) for pos in positions]
    return distances.index(min(distances))


def _remove_empty_rows_cols(table: list[list[str]]) -> list[list[str]]:
    if not table:
        return table

    table = [row for row in table if any(cell.strip() for cell in row)]

    if not table:
        return []

    non_empty_cols = [
        col_idx for col_idx in range(len(table[0])) if any(row[col_idx].strip() for row in table if col_idx < len(row))
    ]

    if not non_empty_cols:
        return []

    return [[row[col_idx] if col_idx < len(row) else "" for col_idx in non_empty_cols] for row in table]


def reconstruct_table(
    words: list[TSVWord], *, column_threshold: int = 20, row_threshold_ratio: float = 0.5
) -> list[list[str]]:
    if not words:
        return []

    col_positions = detect_columns(words, column_threshold=column_threshold)
    row_positions = detect_rows(words, row_threshold_ratio=row_threshold_ratio)

    if not col_positions or not row_positions:
        return []

    table: list[list[str]] = [[""] * len(col_positions) for _ in range(len(row_positions))]

    for word in words:
        col_idx = _find_closest_index(word["left"], col_positions)

        y_center = word["top"] + word["height"] / 2
        row_idx = _find_closest_index(y_center, row_positions)

        if table[row_idx][col_idx]:
            table[row_idx][col_idx] += " " + word["text"]
        else:
            table[row_idx][col_idx] = word["text"]

    return _remove_empty_rows_cols(table)


def to_markdown(table: list[list[str]]) -> str:
    if not table or not table[0]:
        return ""

    lines = []

    lines.append("| " + " | ".join(str(cell) for cell in table[0]) + " |")

    lines.append("| " + " | ".join(["---"] * len(table[0])) + " |")

    for row in table[1:]:
        padded_row = list(row) + [""] * (len(table[0]) - len(row))
        lines.append("| " + " | ".join(str(cell) for cell in padded_row[: len(table[0])]) + " |")

    return "\n".join(lines)


def extract_table_from_tsv(
    tsv_data: str, *, column_threshold: int = 20, row_threshold_ratio: float = 0.5, min_confidence: float = 30.0
) -> str:
    words = extract_words(tsv_data, min_confidence=min_confidence)
    if not words:
        return ""

    table = reconstruct_table(words, column_threshold=column_threshold, row_threshold_ratio=row_threshold_ratio)
    if not table:
        return ""

    return to_markdown(table)
