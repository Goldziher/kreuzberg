"""Table detection using Table Transformer.

Adapted from GMFT's TATRDetector with:
- Proper dependency handling following Kreuzberg patterns
- Async/sync support
- Kreuzberg patterns
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

from kreuzberg._types import GMFTConfig
from kreuzberg._utils._model_cache import setup_huggingface_cache
from kreuzberg._utils._sync import run_sync
from kreuzberg._utils._torch import require_torch, resolve_device, tensor, with_no_grad
from kreuzberg.exceptions import MissingDependencyError

from ._base import BBox, Rect
from ._types import CroppedTable

# Import ML dependencies conditionally
try:
    import transformers
    from transformers import AutoImageProcessor, TableTransformerForObjectDetection
except ImportError:
    transformers = None
    AutoImageProcessor = None
    TableTransformerForObjectDetection = None

if TYPE_CHECKING:
    from PIL import Image

logger = logging.getLogger(__name__)


class TableDetector:
    """Table detection using Table Transformer (TATR).

    Gracefully handles missing ML dependencies by falling back to
    alternative detection methods or raising helpful errors.
    """

    __slots__ = ("_device", "_model", "_processor", "config")

    def __init__(self, config: GMFTConfig | None = None) -> None:
        """Initialize table detector with optional ML model loading."""
        self.config = config or GMFTConfig()
        self._model: Any = None
        self._processor: Any = None
        self._device: str = self._resolve_device(self.config.detection_device)

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
            logger.info("Loading Table Transformer detector (cache: %s)", cache_dir or "default")

            self._processor = AutoImageProcessor.from_pretrained(
                self.config.detection_model,
                cache_dir=cache_dir,
            )
            self._model = TableTransformerForObjectDetection.from_pretrained(
                self.config.detection_model,
                cache_dir=cache_dir,
            )

            # Move to appropriate device
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
        """Check if the detector is ready for use."""
        return self._model is not None and self._processor is not None

    def detect_tables_in_image(self, image: Image.Image) -> list[CroppedTable]:
        """Detect tables in a PIL Image.

        Args:
            image: PIL Image to analyze

        Returns:
            List of detected table regions

        Raises:
            MissingDependencyError: If ML dependencies are not available
        """
        if not self.is_available():
            raise MissingDependencyError(
                "Table detection requires 'transformers' and 'torch' packages. "
                "Install with: pip install 'kreuzberg[gmft]'"
            )

        return self._detect_tables_with_model(image)

    def _detect_tables_with_model(self, image: Image.Image) -> list[CroppedTable]:
        """Perform table detection using the loaded model."""
        require_torch("table detection using TATR models")

        # Prepare inputs
        inputs = self._processor(images=image, return_tensors="pt")

        # Move to device
        if hasattr(inputs, "to"):
            inputs = {k: v.to(self._device) if hasattr(v, "to") else v for k, v in inputs.items()}

        # Run inference
        with with_no_grad():
            outputs = self._model(**inputs)

        # Process results
        target_sizes = tensor([image.size[::-1]])  # (height, width)
        results = self._processor.post_process_object_detection(
            outputs, threshold=self.config.detection_threshold, target_sizes=target_sizes
        )[0]

        # Convert to CroppedTable objects
        tables = []
        for score, _label_id, box in zip(results["scores"], results["labels"], results["boxes"], strict=True):
            # Convert box from tensor to tuple
            bbox: BBox = tuple(box.cpu().numpy())

            # Create table object
            table = CroppedTable(
                rect=Rect(bbox),
                confidence_score=float(score),
                page_number=0,  # Will be set by caller
                label="table",
            )
            tables.append(table)

        return tables

    def detect_tables_in_page_region(self, image: Image.Image, page_number: int = 0) -> list[CroppedTable]:
        """Detect tables in a specific page region.

        Args:
            image: PIL Image of the page region
            page_number: Page number for metadata

        Returns:
            List of detected tables with page numbers set
        """
        tables = self.detect_tables_in_image(image)

        # Update page numbers
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
        """Async version of table detection."""
        # For now, just run sync version in thread pool
        # Could be enhanced with async model inference in the future
        return await run_sync(self.detect_tables_in_image, image)

    async def detect_tables_in_page_region_async(self, image: Image.Image, page_number: int = 0) -> list[CroppedTable]:
        """Async version of page region detection."""
        return await run_sync(self.detect_tables_in_page_region, image, page_number)
