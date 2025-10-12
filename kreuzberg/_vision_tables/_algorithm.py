from __future__ import annotations

import logging
from collections import defaultdict
from dataclasses import dataclass
from functools import lru_cache
from typing import TYPE_CHECKING, Any

import numpy as np
import polars as pl

from kreuzberg._ocr import get_ocr_backend
from kreuzberg.exceptions import KreuzbergError, MissingDependencyError

from ._base import BBox, rect_intersect
from ._types import BboxPredictions, TablePredictions

if TYPE_CHECKING:
    from PIL import Image

    from kreuzberg._ocr._base import OCRBackend
    from kreuzberg._types import TableExtractionConfig

logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class OCRWord:
    left: int
    top: int
    width: int
    height: int
    text: str

    @property
    def right(self) -> int:
        return self.left + self.width

    @property
    def bottom(self) -> int:
        return self.top + self.height

    @property
    def x_center(self) -> float:
        return self.left + self.width / 2.0

    @property
    def y_center(self) -> float:
        return self.top + self.height / 2.0


def extract_table_dataframe(
    image: Image.Image, predictions: TablePredictions, config: TableExtractionConfig
) -> pl.DataFrame:
    filtered_predictions = _filter_predictions_by_confidence(predictions, config)

    sorted_predictions = _sort_predictions_by_position(filtered_predictions)

    row_boxes = list(sorted_predictions.rows.boxes)
    row_scores = list(sorted_predictions.rows.scores)
    if row_boxes:
        kept_row_indices = _apply_non_maximum_suppression(row_boxes, row_scores, threshold=0.5)
        row_boxes = [row_boxes[i] for i in kept_row_indices]
        row_scores = [row_scores[i] for i in kept_row_indices]

    col_boxes = list(sorted_predictions.columns.boxes)
    col_scores = list(sorted_predictions.columns.scores)
    if col_boxes:
        kept_col_indices = _apply_non_maximum_suppression(col_boxes, col_scores, threshold=0.5)
        col_boxes = [col_boxes[i] for i in kept_col_indices]
        col_scores = [col_scores[i] for i in kept_col_indices]

    if not row_boxes or not col_boxes:
        logger.warning("No valid rows or columns found in table predictions")
        return pl.DataFrame()

    intersection_matrix = _calculate_intersection_matrix(row_boxes, col_boxes)

    return _create_grid_dataframe(len(row_boxes), len(col_boxes), intersection_matrix, image, row_boxes, col_boxes)


@lru_cache(maxsize=128)
def _filter_predictions_cached(predictions: BboxPredictions, required_conf: float) -> BboxPredictions:
    if not predictions.boxes:
        return predictions

    filtered_boxes = []
    filtered_scores = []
    filtered_labels = []

    for box, score, label in zip(predictions.boxes, predictions.scores, predictions.labels, strict=False):
        if score >= required_conf:
            filtered_boxes.append(box)
            filtered_scores.append(score)
            filtered_labels.append(label)

    return BboxPredictions.from_lists(
        boxes=filtered_boxes,
        scores=filtered_scores,
        labels=filtered_labels,
    )


def _filter_predictions_by_confidence(predictions: TablePredictions, config: TableExtractionConfig) -> TablePredictions:
    threshold = config.structure_threshold

    filtered_rows = _filter_predictions_cached(predictions.rows, threshold)
    filtered_columns = _filter_predictions_cached(predictions.columns, threshold)
    filtered_spanning = _filter_predictions_cached(predictions.spanning_cells, threshold * 1.2)

    return TablePredictions(
        rows=filtered_rows,
        columns=filtered_columns,
        spanning_cells=filtered_spanning,
    )


def _sort_predictions_by_position(predictions: TablePredictions) -> TablePredictions:
    def sort_rows(pred: BboxPredictions) -> BboxPredictions:
        if not pred.boxes:
            return pred

        sorted_indices = sorted(range(len(pred.boxes)), key=lambda i: pred.boxes[i][1])

        return BboxPredictions.from_lists(
            boxes=[pred.boxes[i] for i in sorted_indices],
            scores=[pred.scores[i] for i in sorted_indices],
            labels=[pred.labels[i] for i in sorted_indices],
        )

    def sort_columns(pred: BboxPredictions) -> BboxPredictions:
        if not pred.boxes:
            return pred

        sorted_indices = sorted(range(len(pred.boxes)), key=lambda i: pred.boxes[i][0])

        return BboxPredictions.from_lists(
            boxes=[pred.boxes[i] for i in sorted_indices],
            scores=[pred.scores[i] for i in sorted_indices],
            labels=[pred.labels[i] for i in sorted_indices],
        )

    return TablePredictions(
        rows=sort_rows(predictions.rows),
        columns=sort_columns(predictions.columns),
        spanning_cells=predictions.spanning_cells,
    )


@lru_cache(maxsize=256)
def _calculate_cell_intersection(row_box: BBox, col_box: BBox) -> float:
    intersection_rect = rect_intersect(row_box, col_box)
    intersection_area = intersection_rect.area

    row_area = (row_box[2] - row_box[0]) * (row_box[3] - row_box[1])
    col_area = (col_box[2] - col_box[0]) * (col_box[3] - col_box[1])
    union_area = row_area + col_area - intersection_area

    return intersection_area / union_area if union_area > 0 else 0.0


def _calculate_intersection_matrix(row_boxes: list[BBox], col_boxes: list[BBox]) -> list[list[float]]:
    if not row_boxes or not col_boxes:
        return []

    row_arr = np.array(row_boxes, dtype=np.float32)
    col_arr = np.array(col_boxes, dtype=np.float32)

    row_x1, row_y1, row_x2, row_y2 = row_arr.T
    col_x1, col_y1, col_x2, col_y2 = col_arr.T

    x1 = np.maximum(row_x1[:, np.newaxis], col_x1)
    y1 = np.maximum(row_y1[:, np.newaxis], col_y1)
    x2 = np.minimum(row_x2[:, np.newaxis], col_x2)
    y2 = np.minimum(row_y2[:, np.newaxis], col_y2)

    intersection = np.maximum(0, x2 - x1) * np.maximum(0, y2 - y1)

    row_areas = (row_x2 - row_x1) * (row_y2 - row_y1)
    col_areas = (col_x2 - col_x1) * (col_y2 - col_y1)

    union = row_areas[:, np.newaxis] + col_areas - intersection

    iou = np.where(union > 0, intersection / union, 0)

    return iou.tolist()  # type: ignore[no-any-return]


def _create_grid_dataframe(
    num_rows: int,
    num_cols: int,
    intersection_matrix: list[list[float]],
    image: Image.Image,
    row_boxes: list[BBox],
    col_boxes: list[BBox],
    threshold: float = 0.1,
) -> pl.DataFrame:
    if num_rows == 0 or num_cols == 0:
        return pl.DataFrame()

    ocr_cell_map = _extract_table_cells_with_tesseract(image, row_boxes, col_boxes)

    data = {}
    for col_idx in range(num_cols):
        column_data = []
        for row_idx in range(num_rows):
            if row_idx < len(intersection_matrix) and col_idx < len(intersection_matrix[row_idx]):
                intersection = intersection_matrix[row_idx][col_idx]

                if intersection > threshold and row_idx < len(row_boxes) and col_idx < len(col_boxes):
                    row_box = row_boxes[row_idx]
                    col_box = col_boxes[col_idx]

                    cell_left = max(row_box[0], col_box[0])
                    cell_top = max(row_box[1], col_box[1])
                    cell_right = min(row_box[2], col_box[2])
                    cell_bottom = min(row_box[3], col_box[3])

                    if cell_right > cell_left and cell_bottom > cell_top:
                        key = (row_idx, col_idx)
                        if key in ocr_cell_map:
                            cell_text = ocr_cell_map[key]
                        else:
                            cell_text = _extract_cell_text(image, (cell_left, cell_top, cell_right, cell_bottom))
                    else:
                        cell_text = ""
                else:
                    cell_text = ""
            else:
                cell_text = ""

            column_data.append(cell_text)

        data[f"Column_{col_idx}"] = column_data

    return pl.DataFrame(data)


def _extract_cell_text(image: Image.Image, cell_bbox: BBox) -> str:
    left, top, right, bottom = cell_bbox
    if right <= left or bottom <= top:
        return ""

    try:
        cell_image = image.crop(cell_bbox)
        width, height = cell_image.size

        if width <= 10 or height <= 10:
            cell_image.close()
            return ""

        backend = _get_cell_ocr_backend()
        if backend is None:
            cell_image.close()
            return ""

        try:
            ocr_result = backend.process_image_sync(
                cell_image,
                output_format="text",
                enable_table_detection=False,
                psm=6,
            )
        finally:
            cell_image.close()

        text = ocr_result.content.strip()
        if not text:
            return ""

        return " ".join(ch for ch in text.replace("\n", " ").split() if ch)
    except (OSError, ValueError):
        return ""


@lru_cache(maxsize=1)
def _get_cell_ocr_backend() -> OCRBackend[Any] | None:
    try:
        return get_ocr_backend("tesseract")
    except MissingDependencyError:
        logger.warning("Tesseract backend unavailable for table cell OCR; falling back to empty cells")
        return None
    except (OSError, RuntimeError):
        raise
    except KreuzbergError as exc:  # pragma: no cover - defensive fallback
        logger.debug("Failed to initialise Tesseract backend for table cell OCR: %s", exc)
        return None


def _extract_table_cells_with_tesseract(
    image: Image.Image, row_boxes: list[BBox], col_boxes: list[BBox]
) -> dict[tuple[int, int], str]:
    backend = _get_cell_ocr_backend()
    if backend is None:
        return {}

    words = _parse_tesseract_tsv(image, backend)
    if not words:
        return {}

    cell_words: dict[tuple[int, int], list[tuple[int, str]]] = defaultdict(list)

    for word in words:
        row_idx = _locate_row_index(word.y_center, row_boxes)
        col_idx = _locate_column_index(word.x_center, col_boxes)

        if row_idx is None or col_idx is None:
            continue

        cell_words[(row_idx, col_idx)].append((word.left, word.text))

    cell_text_map: dict[tuple[int, int], str] = {}
    for key, cell_entries in cell_words.items():
        cell_entries.sort(key=lambda item: item[0])
        text = " ".join(word for _, word in cell_entries).strip()
        if text:
            cleaned = " ".join(text.replace("\n", " ").split())
            cell_text_map[key] = cleaned

    return cell_text_map


def _locate_row_index(y_center: float, row_boxes: list[BBox]) -> int | None:
    best_idx = None
    best_distance = float("inf")

    for idx, (_, y1, _, y2) in enumerate(row_boxes):
        row_top = y1
        row_bottom = y2
        margin = max((row_bottom - row_top) * 0.2, 2.0)

        if row_top - margin <= y_center <= row_bottom + margin:
            distance = (
                0.0 if row_top <= y_center <= row_bottom else min(abs(y_center - row_top), abs(y_center - row_bottom))
            )
            if distance < best_distance:
                best_distance = distance
                best_idx = idx

    return best_idx


def _locate_column_index(x_center: float, col_boxes: list[BBox]) -> int | None:
    best_idx = None
    best_distance = float("inf")

    for idx, (x1, _, x2, _) in enumerate(col_boxes):
        col_left = x1
        col_right = x2
        margin = max((col_right - col_left) * 0.2, 2.0)

        if col_left - margin <= x_center <= col_right + margin:
            distance = (
                0.0 if col_left <= x_center <= col_right else min(abs(x_center - col_left), abs(x_center - col_right))
            )
            if distance < best_distance:
                best_distance = distance
                best_idx = idx

    return best_idx


def _parse_tesseract_tsv(image: Image.Image, backend: OCRBackend[Any]) -> list[OCRWord]:
    try:
        tsv_result = backend.process_image_sync(
            image,
            output_format="tsv",
            enable_table_detection=False,
            psm=6,
        )
    except (OSError, RuntimeError):
        raise
    except KreuzbergError as exc:  # pragma: no cover - defensive fallback
        logger.debug("Failed to obtain TSV text for table OCR: %s", exc)
        return []

    lines = tsv_result.content.splitlines()
    if not lines:
        return []

    words: list[OCRWord] = []
    for line in lines[1:]:
        parts = line.split("\t")
        if len(parts) < 12 or parts[0] != "5":
            continue

        text = parts[11].strip()
        if not text:
            continue

        try:
            left = int(parts[6])
            top = int(parts[7])
            width = int(parts[8])
            height = int(parts[9])
        except ValueError:
            continue

        words.append(OCRWord(left=left, top=top, width=width, height=height, text=text))

    return words


def _detect_column_positions(words: list[OCRWord]) -> tuple[list[float], float]:
    if not words:
        return ([], 0.0)

    widths = sorted(w.width for w in words if w.width > 0)
    median_width = widths[len(widths) // 2] if widths else 40
    threshold = float(max(int(median_width * 0.8), 20))

    x_positions = sorted(float(w.left) for w in words)
    columns = _group_positions(x_positions, threshold)
    return (columns, threshold)


def _detect_row_positions(words: list[OCRWord], ratio: float = 0.5) -> tuple[list[float], float]:
    if not words:
        return ([], 0.0)

    heights = sorted(w.height for w in words if w.height > 0)
    median_height = heights[len(heights) // 2] if heights else 20
    threshold = float(max(int(median_height * ratio), 10))

    y_centers = sorted(w.y_center for w in words)
    rows = _group_positions(y_centers, threshold)
    return (rows, threshold)


def _group_positions(values: list[float], threshold: float) -> list[float]:
    if not values:
        return []

    groups: list[list[float]] = []

    for value in values:
        placed = False
        for group in groups:
            if abs(value - group[0]) <= threshold:
                group.append(value)
                placed = True
                break
        if not placed:
            groups.append([value])

    medians = []
    for group in groups:
        group.sort()
        medians.append(group[len(group) // 2])

    medians.sort()
    return medians


def _nearest_index_with_tolerance(value: float, positions: list[float], tolerance: float) -> int | None:
    best_idx = None
    best_distance = tolerance

    for idx, pos in enumerate(positions):
        distance = abs(value - pos)
        if distance <= tolerance and distance < best_distance:
            best_idx = idx
            best_distance = distance

    return best_idx


def _remove_empty_rows_columns(table: list[list[str]]) -> list[list[str]]:
    if not table:
        return []

    non_empty_rows = [row for row in table if any(cell.strip() for cell in row)]
    if not non_empty_rows:
        return []

    num_cols = len(non_empty_rows[0])
    col_indices = [idx for idx in range(num_cols) if any(row[idx].strip() for row in non_empty_rows if idx < len(row))]

    return [[row[idx] for idx in col_indices] for row in non_empty_rows]


def _build_dataframe_from_ocr(image: Image.Image) -> pl.DataFrame:
    backend = _get_cell_ocr_backend()
    if backend is None:
        return pl.DataFrame()

    words = _parse_tesseract_tsv(image, backend)
    if not words:
        return pl.DataFrame()

    column_positions, column_threshold = _detect_column_positions(words)
    row_positions, row_threshold = _detect_row_positions(words)

    if not column_positions or not row_positions:
        return pl.DataFrame()

    table_cells: list[list[list[str]]] = [[[] for _ in column_positions] for _ in row_positions]

    for word in words:
        row_idx = _nearest_index_with_tolerance(word.y_center, row_positions, row_threshold * 1.5)
        col_idx = _nearest_index_with_tolerance(word.x_center, column_positions, column_threshold * 1.5)

        if row_idx is None or col_idx is None:
            continue

        table_cells[row_idx][col_idx].append(word.text)

    table = [[" ".join(cell_words).strip() for cell_words in row] for row in table_cells]

    cleaned = _remove_empty_rows_columns(table)
    if not cleaned:
        return pl.DataFrame()

    width = len(cleaned[0])
    data = {f"Column_{idx}": [row[idx] if idx < len(row) else "" for row in cleaned] for idx in range(width)}

    return pl.DataFrame(data)


def _apply_non_maximum_suppression(boxes: list[BBox], scores: list[float], threshold: float = 0.5) -> list[int]:
    if not boxes:
        return []

    boxes_arr = np.array(boxes, dtype=np.float32)
    scores_arr = np.array(scores, dtype=np.float32)

    sorted_indices = np.argsort(scores_arr)[::-1]
    kept_indices = []
    suppressed = np.zeros(len(boxes), dtype=bool)

    for idx in sorted_indices:
        if suppressed[idx]:
            continue

        kept_indices.append(int(idx))
        current_box = boxes_arr[idx]

        remaining_mask = ~suppressed
        remaining_indices = np.where(remaining_mask)[0]

        if len(remaining_indices) > 1:
            remaining_boxes = boxes_arr[remaining_indices]

            x1 = np.maximum(current_box[0], remaining_boxes[:, 0])
            y1 = np.maximum(current_box[1], remaining_boxes[:, 1])
            x2 = np.minimum(current_box[2], remaining_boxes[:, 2])
            y2 = np.minimum(current_box[3], remaining_boxes[:, 3])

            intersection = np.maximum(0, x2 - x1) * np.maximum(0, y2 - y1)
            box_areas = (remaining_boxes[:, 2] - remaining_boxes[:, 0]) * (
                remaining_boxes[:, 3] - remaining_boxes[:, 1]
            )

            iob = np.where(box_areas > 0, intersection / box_areas, 0)

            suppress_mask = iob > threshold
            suppressed[remaining_indices[suppress_mask]] = True

    return kept_indices


def _merge_close_predictions(
    boxes: list[BBox], scores: list[float], labels: list[int], merge_threshold: float = 0.6
) -> tuple[list[BBox], list[float], list[int]]:
    if not boxes:
        return boxes, scores, labels

    n = len(boxes)
    boxes_arr = np.array(boxes, dtype=np.float32)
    scores_arr = np.array(scores, dtype=np.float32)
    labels_arr = np.array(labels, dtype=np.int32)

    iob_matrix = np.zeros((n, n), dtype=np.float32)

    for i in range(n):
        if i < n - 1:
            remaining_boxes = boxes_arr[i + 1 :]
            box_i = boxes_arr[i]

            x1 = np.maximum(box_i[0], remaining_boxes[:, 0])
            y1 = np.maximum(box_i[1], remaining_boxes[:, 1])
            x2 = np.minimum(box_i[2], remaining_boxes[:, 2])
            y2 = np.minimum(box_i[3], remaining_boxes[:, 3])

            intersection = np.maximum(0, x2 - x1) * np.maximum(0, y2 - y1)

            area_i = (box_i[2] - box_i[0]) * (box_i[3] - box_i[1])
            areas_j = (remaining_boxes[:, 2] - remaining_boxes[:, 0]) * (remaining_boxes[:, 3] - remaining_boxes[:, 1])

            iob_i = intersection / area_i if area_i > 0 else 0
            iob_j = np.where(areas_j > 0, intersection / areas_j, 0)

            max_iob = np.maximum(iob_i, iob_j)
            iob_matrix[i, i + 1 :] = max_iob
            iob_matrix[i + 1 :, i] = max_iob

    merged_groups = []
    used = np.zeros(n, dtype=bool)

    for i in range(n):
        if used[i]:
            continue

        group = [i]
        to_check = [i]
        used[i] = True

        while to_check:
            current = to_check.pop()
            neighbors = np.where((iob_matrix[current] > merge_threshold) & (~used))[0]

            for neighbor in neighbors:
                group.append(int(neighbor))
                to_check.append(int(neighbor))
                used[neighbor] = True

        merged_groups.append(group)

    merged_boxes = []
    merged_scores = []
    merged_labels = []

    for group in merged_groups:
        group_boxes = boxes_arr[group]
        group_scores = scores_arr[group]
        group_labels = labels_arr[group]

        x1 = np.min(group_boxes[:, 0])
        y1 = np.min(group_boxes[:, 1])
        x2 = np.max(group_boxes[:, 2])
        y2 = np.max(group_boxes[:, 3])

        merged_boxes.append((float(x1), float(y1), float(x2), float(y2)))
        merged_scores.append(float(np.max(group_scores)))
        merged_labels.append(int(group_labels[0]))

    return merged_boxes, merged_scores, merged_labels
