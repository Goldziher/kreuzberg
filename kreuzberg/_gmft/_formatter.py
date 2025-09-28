"""Table structure formatting using Table Transformer.

Adapted from GMFT's TATRFormatter with Kreuzberg patterns:
- Proper dependency handling
- Polars DataFrames instead of pandas
- Slots and functional design for performance
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

from kreuzberg._types import GMFTConfig
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

# Import ML dependencies conditionally
try:
    from transformers import AutoImageProcessor, TableTransformerForObjectDetection  # type: ignore[attr-defined]
except ImportError:
    AutoImageProcessor = None
    TableTransformerForObjectDetection = None

if TYPE_CHECKING:
    from PIL import Image

    from ._base import BBox
else:
    # Runtime import for actual use
    BBox = tuple  # Use tuple as type alias for runtime

logger = logging.getLogger(__name__)


class TableFormatter:
    """Table structure formatting using Table Transformer (TATR).

    Extracts row/column structure from detected table regions and converts
    to structured Polars DataFrames.
    """

    __slots__ = ("_device", "_model", "_processor", "config")

    def __init__(self, config: GMFTConfig | None = None) -> None:
        """Initialize table formatter with optional ML model loading."""
        self.config = config or GMFTConfig()
        self._model: Any = None
        self._processor: Any = None
        self._device: str = self._resolve_device(self.config.structure_device)

        # Try to load ML model, but don't fail if dependencies missing
        self._try_load_model()

    def _resolve_device(self, device_config: str) -> str:
        """Resolve device configuration."""
        return resolve_device(device_config)

    def _try_load_model(self) -> None:
        """Attempt to load the Table Transformer model."""
        if AutoImageProcessor is None or TableTransformerForObjectDetection is None:
            # Dependencies not available
            return

        try:
            # Setup cache directory using unified model cache management
            cache_dir = setup_huggingface_cache(self.config.model_cache_dir)

            # HuggingFace handles caching automatically - it will:
            # 1. Check cache first
            # 2. Download if not cached
            # 3. Cache for future use
            logger.info("Loading Table Transformer structure model (cache: %s)", cache_dir or "default")

            self._processor = AutoImageProcessor.from_pretrained(
                self.config.structure_model,
                cache_dir=cache_dir,
            )
            self._model = TableTransformerForObjectDetection.from_pretrained(
                self.config.structure_model,
                cache_dir=cache_dir,
            )

            # Move to appropriate device
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
        """Check if the formatter is ready for use."""
        return self._model is not None and self._processor is not None

    def format_table(self, cropped_table: CroppedTable, image: Image.Image) -> FormattedTable:
        """Extract table structure from a cropped table region.

        Args:
            cropped_table: Detected table region information
            image: PIL Image of the table region

        Returns:
            FormattedTable with structure predictions and DataFrame

        Raises:
            MissingDependencyError: If ML dependencies are not available
        """
        if not self.is_available():
            raise MissingDependencyError(
                "Table formatting requires 'transformers' and 'torch' packages. "
                "Install with: pip install 'kreuzberg[gmft]'"
            )

        return self._format_table_with_model(cropped_table, image)

    def _format_table_with_model(self, cropped_table: CroppedTable, image: Image.Image) -> FormattedTable:
        """Perform table structure formatting using the loaded model."""
        require_torch("table structure formatting using TATR models")

        # Prepare inputs
        inputs = self._processor(images=image, return_tensors="pt")

        # Move to device
        if hasattr(inputs, "to"):
            inputs = {k: v.to(self._device) if hasattr(v, "to") else v for k, v in inputs.items()}

        # Run inference
        with with_no_grad():
            outputs = self._model(**inputs)

        # Process results for structure
        target_sizes = tensor([image.size[::-1]])  # (height, width)
        results = self._processor.post_process_object_detection(
            outputs, threshold=self.config.structure_threshold, target_sizes=target_sizes
        )[0]

        # Extract structure predictions
        predictions = self._extract_structure_predictions(results)

        # Convert to DataFrame using structure algorithm
        dataframe = extract_table_dataframe(image, predictions, self.config)

        return FormattedTable(
            cropped_table=cropped_table,
            dataframe=dataframe,
            predictions=predictions,
            confidence_scores=self._calculate_confidence_scores(results),
            metadata={"formatter_config": self.config.to_dict()},
        )

    def _extract_structure_predictions(self, results: dict[str, Any]) -> TablePredictions:
        """Extract and categorize structure predictions by type."""
        # Group predictions by type
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
        """Calculate aggregate confidence scores for the table structure."""
        if not results["scores"]:
            return {"overall": 0.0, "structure": 0.0}

        scores = [float(s) for s in results["scores"]]
        return {
            "overall": sum(scores) / len(scores),
            "structure": min(scores) if scores else 0.0,
            "max_confidence": max(scores) if scores else 0.0,
            "min_confidence": min(scores) if scores else 0.0,
        }

    async def format_table_async(self, cropped_table: CroppedTable, image: Image.Image) -> FormattedTable:
        """Async version of table formatting."""
        return await run_sync(self.format_table, cropped_table, image)
