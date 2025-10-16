from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    import polars as pl

    from ._base import BBox, Rect


@dataclass(frozen=True, slots=True)
class BboxPredictions:
    boxes: tuple[BBox, ...]
    scores: tuple[float, ...]
    labels: tuple[int, ...]

    def __post_init__(self) -> None:
        if not (len(self.boxes) == len(self.scores) == len(self.labels)):
            raise ValueError("All prediction sequences must have same length")

    @classmethod
    def from_lists(cls, boxes: list[BBox], scores: list[float], labels: list[int]) -> BboxPredictions:
        return cls(
            boxes=tuple(boxes),
            scores=tuple(scores),
            labels=tuple(labels),
        )

    def to_dict(self) -> dict[str, list[Any]]:
        return {
            "boxes": list(self.boxes),
            "scores": list(self.scores),
            "labels": list(self.labels),
        }


@dataclass(frozen=True, slots=True)
class TablePredictions:
    rows: BboxPredictions
    columns: BboxPredictions
    spanning_cells: BboxPredictions

    @classmethod
    def from_dicts(cls, data: dict[str, dict[str, list[Any]]]) -> TablePredictions:
        return cls(
            rows=BboxPredictions.from_lists(**data["rows"]),
            columns=BboxPredictions.from_lists(**data["columns"]),
            spanning_cells=BboxPredictions.from_lists(**data["spanning_cells"]),
        )


@dataclass(frozen=True, slots=True)
class CroppedTable:
    rect: Rect
    confidence_score: float
    page_number: int
    angle: float = 0.0
    label: str = "table"

    def __hash__(self) -> int:
        return hash((self.rect, self.confidence_score, self.page_number, self.angle, self.label))


@dataclass(frozen=True, slots=True)
class FormattedTable:
    cropped_table: CroppedTable
    dataframe: pl.DataFrame
    predictions: TablePredictions | None = None
    confidence_scores: dict[str, float] | None = None
    metadata: dict[str, Any] | None = None

    def __hash__(self) -> int:
        return hash((self.cropped_table, id(self.dataframe)))

    def to_csv(self) -> str:
        return self.dataframe.write_csv()

    def to_dict(self) -> list[dict[str, Any]]:
        return self.dataframe.to_dicts()

    def to_markdown(self) -> str:
        return str(self.dataframe)


TATR_ID_TO_LABEL = {
    0: "table",
    1: "table column",
    2: "table row",
    3: "table column header",
    4: "table projected row header",
    5: "table spanning cell",
    6: "no object",
}

TATR_LABEL_TO_ID = {v: k for k, v in TATR_ID_TO_LABEL.items()}

POSSIBLE_ROWS = [
    "table row",
    "table spanning cell",
    "table projected row header",
]

POSSIBLE_PROJECTING_ROWS = [
    "table projected row header",
]

POSSIBLE_COLUMN_HEADERS = [
    "table column header",
]

POSSIBLE_COLUMNS = [
    "table column",
]
