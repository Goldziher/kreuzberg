from __future__ import annotations

import csv
from io import StringIO
from typing import TYPE_CHECKING

import numpy as np

from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from kreuzberg._types import TSVWord

try:
    from scipy.cluster.hierarchy import fclusterdata

    SCIPY_AVAILABLE = True
except ImportError:
    SCIPY_AVAILABLE = False


def extract_words(tsv_data: str, *, min_confidence: float = 30.0) -> list[TSVWord]:
    """Parse TSV output into structured word data.

    Args:
        tsv_data: Raw TSV output from Tesseract.
        min_confidence: Minimum confidence score to include a word.

    Returns:
        List of word dictionaries with position and text data.

    Raises:
        ParsingError: If TSV data cannot be parsed.
    """
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


def _detect_columns_simple(words: list[TSVWord], *, column_threshold: int = 20) -> list[int]:
    """Simple column detection without scipy.

    Args:
        words: List of word dictionaries.
        column_threshold: Pixel threshold for column clustering.

    Returns:
        List of detected column positions.
    """
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


def detect_columns(words: list[TSVWord], *, column_threshold: int = 20) -> list[int]:
    """Detect columns using hierarchical clustering on X positions.

    Args:
        words: List of word dictionaries from TSV.
        column_threshold: Pixel threshold for column clustering.

    Returns:
        Sorted list of column X positions.
    """
    if not words:
        return []

    x_positions = np.array([w["left"] for w in words]).reshape(-1, 1)

    if len(x_positions) == 1:
        return [int(x_positions[0, 0])]

    if not SCIPY_AVAILABLE:
        return _detect_columns_simple(words, column_threshold=column_threshold)

    clusters = fclusterdata(x_positions, column_threshold, criterion="distance", method="single")

    column_positions = []
    for cluster_id in np.unique(clusters):
        cluster_mask = clusters == cluster_id
        cluster_positions = x_positions[cluster_mask]
        column_positions.append(int(np.median(cluster_positions)))

    return sorted(column_positions)


def _detect_rows_simple(words: list[TSVWord], threshold: float) -> list[int]:
    """Simple row detection without scipy.

    Args:
        words: List of word dictionaries.
        threshold: Distance threshold for grouping.

    Returns:
        List of detected row positions.
    """
    if not words:
        return []

    y_centers = sorted(w["top"] + w["height"] / 2 for w in words)

    if len(y_centers) == 1:
        return [int(y_centers[0])]

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


def detect_rows(words: list[TSVWord], *, row_threshold_ratio: float = 0.5) -> list[int]:
    """Detect rows using clustering on Y positions.

    Args:
        words: List of word dictionaries from TSV.
        row_threshold_ratio: Row threshold as ratio of mean text height.

    Returns:
        Sorted list of row Y positions.
    """
    if not words:
        return []

    y_centers = np.array([w["top"] + w["height"] / 2 for w in words])
    mean_height = np.mean([w["height"] for w in words])
    threshold = mean_height * row_threshold_ratio

    y_positions = y_centers.reshape(-1, 1)

    if len(y_positions) == 1:
        return [int(y_positions[0, 0])]

    if not SCIPY_AVAILABLE:
        return _detect_rows_simple(words, float(threshold))

    clusters = fclusterdata(y_positions, threshold, criterion="distance", method="single")

    row_positions = []
    for cluster_id in np.unique(clusters):
        cluster_mask = clusters == cluster_id
        cluster_positions = y_positions[cluster_mask]
        row_positions.append(int(np.median(cluster_positions)))

    return sorted(row_positions)


def _find_closest_index(value: float, positions: list[int]) -> int:
    """Find index of closest position.

    Args:
        value: The value to match.
        positions: List of positions to search.

    Returns:
        Index of the closest position.
    """
    if not positions:
        return 0

    distances = [abs(value - pos) for pos in positions]
    return distances.index(min(distances))


def _remove_empty_rows_cols(table: list[list[str]]) -> list[list[str]]:
    """Remove completely empty rows and columns.

    Args:
        table: 2D table array.

    Returns:
        Cleaned table with empty rows/columns removed.
    """
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
    """Reconstruct table from words and detected structure.

    Args:
        words: List of word dictionaries from TSV.
        column_threshold: Pixel threshold for column clustering.
        row_threshold_ratio: Row threshold as ratio of mean text height.

    Returns:
        2D list representing the table structure.
    """
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
    """Convert table to markdown format.

    Args:
        table: 2D list representing the table.

    Returns:
        Markdown-formatted table string.
    """
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
    """Convenience function to extract table from TSV data.

    Args:
        tsv_data: Raw TSV output from Tesseract.
        column_threshold: Pixel threshold for column clustering.
        row_threshold_ratio: Row threshold as ratio of mean text height.
        min_confidence: Minimum confidence score to include a word.

    Returns:
        Markdown-formatted table string, or empty string if no table detected.
    """
    words = extract_words(tsv_data, min_confidence=min_confidence)
    if not words:
        return ""

    table = reconstruct_table(words, column_threshold=column_threshold, row_threshold_ratio=row_threshold_ratio)
    if not table:
        return ""

    return to_markdown(table)


# For backward compatibility, create a class that wraps the functions
class TesseractTableExtractor:
    """Legacy class wrapper for table extraction functions.

    This class is maintained for backward compatibility.
    New code should use the functions directly.
    """

    def __init__(
        self,
        column_threshold: int = 20,
        row_threshold_ratio: float = 0.5,
        min_confidence: float = 30.0,
    ) -> None:
        self.column_threshold = column_threshold
        self.row_threshold_ratio = row_threshold_ratio
        self.min_confidence = min_confidence

    def extract_words(self, tsv_data: str) -> list[TSVWord]:
        return extract_words(tsv_data, min_confidence=self.min_confidence)

    def detect_columns(self, words: list[TSVWord]) -> list[int]:
        return detect_columns(words, column_threshold=self.column_threshold)

    def detect_rows(self, words: list[TSVWord]) -> list[int]:
        return detect_rows(words, row_threshold_ratio=self.row_threshold_ratio)

    def reconstruct_table(self, words: list[TSVWord]) -> list[list[str]]:
        return reconstruct_table(
            words,
            column_threshold=self.column_threshold,
            row_threshold_ratio=self.row_threshold_ratio,
        )

    def to_markdown(self, table: list[list[str]]) -> str:
        return to_markdown(table)
