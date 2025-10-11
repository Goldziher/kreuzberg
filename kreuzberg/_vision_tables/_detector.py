from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

from kreuzberg._types import TableExtractionConfig
from kreuzberg._utils._model_cache import setup_huggingface_cache
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._torch import require_torch, resolve_device, tensor, with_no_grad
from kreuzberg.exceptions import MissingDependencyError

from ._base import BBox, Rect
from ._types import CroppedTable

if TYPE_CHECKING:
    from PIL import Image

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


class TableDetector:
    __slots__ = ("_device", "_model", "_processor", "config")

    def __init__(self, config: TableExtractionConfig | None = None) -> None:
        self.config = config or TableExtractionConfig()
        self._model: Any = None
        self._processor: Any = None
        self._device: str = self._resolve_device(self.config.detection_device)

        self._try_load_model()

    def _resolve_device(self, device_config: str) -> str:
        return resolve_device(device_config)

    def _try_load_model(self) -> None:
        AutoImageProcessor, TableTransformerForObjectDetection = _import_transformers()  # noqa: N806
        if AutoImageProcessor is None or TableTransformerForObjectDetection is None:
            return

        try:
            cache_dir = setup_huggingface_cache(self.config.model_cache_dir)

            logger.info("Loading Table Transformer detector (cache: %s)", cache_dir or "default")

            self._processor = AutoImageProcessor.from_pretrained(
                self.config.detection_model,
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
                self.config.detection_model,
                cache_dir=cache_dir,
            )

            if hasattr(self._model, "to"):
                self._model.to(self._device)

            logger.info("Table Transformer detector ready on %s", self._device)

        except ImportError as e:
            logger.warning("ML dependencies not available for table detection: %s", e)
            self._model = None
            self._processor = None
        except (OSError, RuntimeError) as e:
            logger.error("Failed to load Table Transformer model: %s", e)
            self._model = None
            self._processor = None

    def is_available(self) -> bool:
        return self._model is not None and self._processor is not None

    def detect_tables_in_image(self, image: Image.Image) -> list[CroppedTable]:
        if not self.is_available():
            raise MissingDependencyError(
                "Table detection requires 'transformers' and 'torch' packages. "
                "Install with: pip install 'kreuzberg[vision-tables]'"
            )

        return self._detect_tables_with_model(image)

    def _detect_tables_with_model(self, image: Image.Image) -> list[CroppedTable]:
        require_torch("table detection using TATR models")

        inputs = self._processor(images=image, return_tensors="pt")

        if hasattr(inputs, "to"):
            inputs = {k: v.to(self._device) if hasattr(v, "to") else v for k, v in inputs.items()}

        with with_no_grad():
            outputs = self._model(**inputs)

        target_sizes = tensor([image.size[::-1]])
        results = self._processor.post_process_object_detection(
            outputs, threshold=self.config.detection_threshold, target_sizes=target_sizes
        )[0]

        tables = []
        for score, _label_id, box in zip(results["scores"], results["labels"], results["boxes"], strict=True):
            bbox: BBox = tuple(box.cpu().numpy())

            table = CroppedTable(
                rect=Rect(bbox),
                confidence_score=float(score),
                page_number=0,
                label="table",
            )
            tables.append(table)

        return tables

    def detect_tables_in_page_region(self, image: Image.Image, page_number: int = 0) -> list[CroppedTable]:
        tables = self.detect_tables_in_image(image)

        return [
            CroppedTable(
                rect=table.rect,
                confidence_score=table.confidence_score,
                page_number=page_number,
                angle=table.angle,
                label=table.label,
            )
            for table in tables
        ]

    async def detect_tables_in_image_async(self, image: Image.Image) -> list[CroppedTable]:
        return await run_sync(self.detect_tables_in_image, image)

    async def detect_tables_in_page_region_async(self, image: Image.Image, page_number: int = 0) -> list[CroppedTable]:
        return await run_sync(self.detect_tables_in_page_region, image, page_number)
