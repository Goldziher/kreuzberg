from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
from PIL import Image

from kreuzberg._internal_bindings import (
    ExtractionConfigDTO,
)
from kreuzberg._internal_bindings import (
    normalize_image_dpi as _normalize_image_dpi,
)
from kreuzberg._types import ImagePreprocessingMetadata

if TYPE_CHECKING:
    from kreuzberg._types import ExtractionConfig


def normalize_image_dpi(
    image: Image.Image,
    config: ExtractionConfig,
    dpi_info: dict[str, float] | None = None,
) -> tuple[Image.Image, ImagePreprocessingMetadata]:
    image.load()
    image_array = np.array(image)

    if len(image_array.shape) == 2:
        image_array = np.stack([image_array] * 3, axis=-1)
    elif image_array.shape[-1] == 4:
        image_array = image_array[:, :, :3]

    if image_array.dtype != np.uint8:
        image_array = image_array.astype(np.uint8)

    rust_config = ExtractionConfigDTO(
        target_dpi=config.target_dpi,
        max_image_dimension=config.max_image_dimension,
        auto_adjust_dpi=config.auto_adjust_dpi,
        min_dpi=config.min_dpi,
        max_dpi=config.max_dpi,
    )

    result_array, metadata_obj = _normalize_image_dpi(image_array, rust_config, dpi_info)

    result_image = Image.fromarray(result_array)

    metadata = ImagePreprocessingMetadata(
        original_dimensions=metadata_obj.original_dimensions,
        original_dpi=metadata_obj.original_dpi,
        target_dpi=metadata_obj.target_dpi,
        scale_factor=metadata_obj.scale_factor,
        auto_adjusted=metadata_obj.auto_adjusted,
        final_dpi=metadata_obj.final_dpi,
        new_dimensions=metadata_obj.new_dimensions,
        resample_method=metadata_obj.resample_method,
        skipped_resize=metadata_obj.skipped_resize,
    )

    return result_image, metadata
