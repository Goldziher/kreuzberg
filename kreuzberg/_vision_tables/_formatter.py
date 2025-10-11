from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

import msgspec

from kreuzberg._types import TableExtractionConfig
from kreuzberg._utils._model_cache import setup_huggingface_cache
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._torch import require_torch, resolve_device, tensor, with_no_grad
from kreuzberg.exceptions import MissingDependencyError

from ._algorithm import extract_table_dataframe
from ._types import (
    POSSIBLE_COLUMNS,
    POSSIBLE_ROWS,
    TATR_ID_TO_LABEL,
    BboxPredictions,
    CroppedTable,
    FormattedTable,
    TablePredictions,
)

if TYPE_CHECKING:
    from PIL import Image

    from ._base import BBox

logger = logging.getLogger(__name__)


def _import_transformers() -> tuple[Any, Any]:
    try:
        from transformers import (  # noqa: PLC0415
            AutoImageProcessor,
            TableTransformerForObjectDetection,
        )

        return AutoImageProcessor, TableTransformerForObjectDetection
    except ImportError:
        return None, None


class TableFormatter:
    __slots__ = ("_device", "_model", "_processor", "config")

    def __init__(self, config: TableExtractionConfig | None = None) -> None:
        self.config = config or TableExtractionConfig()
        self._model: Any = None
        self._processor: Any = None
        self._device: str = self._resolve_device(self.config.structure_device)

        self._try_load_model()

    def _resolve_device(self, device_config: str) -> str:
        return resolve_device(device_config)

    def _try_load_model(self) -> None:
        AutoImageProcessor, TableTransformerForObjectDetection = _import_transformers()  # noqa: N806
        if AutoImageProcessor is None or TableTransformerForObjectDetection is None:
            return

        try:
            cache_dir = setup_huggingface_cache(self.config.model_cache_dir)

            logger.info("Loading Table Transformer structure model (cache: %s)", cache_dir or "default")

            self._processor = AutoImageProcessor.from_pretrained(
                self.config.structure_model,
                cache_dir=cache_dir,
            )

            if (
                hasattr(self._processor, "size")
                and isinstance(self._processor.size, dict)
                and "longest_edge" in self._processor.size
                and "shortest_edge" not in self._processor.size
            ):
                self._processor.size["shortest_edge"] = self._processor.size["longest_edge"]

            self._model = TableTransformerForObjectDetection.from_pretrained(
                self.config.structure_model,
                cache_dir=cache_dir,
            )

            if hasattr(self._model, "to"):
                self._model.to(self._device)

            logger.info("Table Transformer structure model ready on %s", self._device)

        except ImportError as e:
            logger.warning("ML dependencies not available for table formatting: %s", e)
            self._model = None
            self._processor = None
        except (OSError, RuntimeError) as e:
            logger.error("Failed to load Table Transformer formatter: %s", e)
            self._model = None
            self._processor = None

    def is_available(self) -> bool:
        return self._model is not None and self._processor is not None

    def format_table(self, cropped_table: CroppedTable, image: Image.Image) -> FormattedTable:
        if not self.is_available():
            raise MissingDependencyError(
                "Table formatting requires 'transformers' and 'torch' packages. "
                "Install with: pip install 'kreuzberg[vision-tables]'"
            )

        return self._format_table_with_model(cropped_table, image)

    def _format_table_with_model(self, cropped_table: CroppedTable, image: Image.Image) -> FormattedTable:
        require_torch("table structure formatting using TATR models")

        inputs = self._processor(images=image, return_tensors="pt")

        if hasattr(inputs, "to"):
            inputs = {k: v.to(self._device) if hasattr(v, "to") else v for k, v in inputs.items()}

        with with_no_grad():
            outputs = self._model(**inputs)

        target_sizes = tensor([image.size[::-1]])
        results = self._processor.post_process_object_detection(
            outputs, threshold=self.config.structure_threshold, target_sizes=target_sizes
        )[0]

        predictions = self._extract_structure_predictions(results)

        dataframe = extract_table_dataframe(image, predictions, self.config)

        return FormattedTable(
            cropped_table=cropped_table,
            dataframe=dataframe,
            predictions=predictions,
            confidence_scores=self._calculate_confidence_scores(results),
            metadata={"formatter_config": msgspec.to_builtins(self.config)},
        )

    def _extract_structure_predictions(self, results: dict[str, Any]) -> TablePredictions:
        rows_boxes = []
        rows_scores = []
        rows_labels = []

        columns_boxes = []
        columns_scores = []
        columns_labels = []

        spanning_boxes = []
        spanning_scores = []
        spanning_labels = []

        for score, label_id, box in zip(results["scores"], results["labels"], results["boxes"], strict=True):
            label = TATR_ID_TO_LABEL.get(int(label_id), "unknown")
            bbox: BBox = tuple(box.cpu().numpy())

            if label in POSSIBLE_ROWS:
                rows_boxes.append(bbox)
                rows_scores.append(float(score))
                rows_labels.append(int(label_id))
            elif label in POSSIBLE_COLUMNS:
                columns_boxes.append(bbox)
                columns_scores.append(float(score))
                columns_labels.append(int(label_id))
            elif label == "table spanning cell":
                spanning_boxes.append(bbox)
                spanning_scores.append(float(score))
                spanning_labels.append(int(label_id))

        return TablePredictions(
            rows=BboxPredictions.from_lists(
                boxes=rows_boxes,
                scores=rows_scores,
                labels=rows_labels,
            ),
            columns=BboxPredictions.from_lists(
                boxes=columns_boxes,
                scores=columns_scores,
                labels=columns_labels,
            ),
            spanning_cells=BboxPredictions.from_lists(
                boxes=spanning_boxes,
                scores=spanning_scores,
                labels=spanning_labels,
            ),
        )

    def _calculate_confidence_scores(self, results: dict[str, Any]) -> dict[str, float]:
        if len(results["scores"]) == 0:
            return {"overall": 0.0, "structure": 0.0}

        scores = [float(s) for s in results["scores"]]
        return {
            "overall": sum(scores) / len(scores),
            "structure": min(scores) if scores else 0.0,
            "max_confidence": max(scores) if scores else 0.0,
            "min_confidence": min(scores) if scores else 0.0,
        }

    async def format_table_async(self, cropped_table: CroppedTable, image: Image.Image) -> FormattedTable:
        return await run_sync(self.format_table, cropped_table, image)
