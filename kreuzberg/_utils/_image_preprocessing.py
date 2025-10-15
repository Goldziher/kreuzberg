from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
from PIL import Image

from kreuzberg._internal_bindings import ExtractionConfigDTO, normalize_image_dpi as rust_normalize_image_dpi
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

    # Convert grayscale to RGB
    if len(image_array.shape) == 2:
        image_array = np.stack([image_array] * 3, axis=-1)
    # Convert RGBA to RGB
    elif image_array.shape[-1] == 4:
        image_array = image_array[:, :, :3]

    # Ensure uint8 dtype
    if image_array.dtype != np.uint8:
        image_array = image_array.astype(np.uint8)

    # Create DTO config for Rust
    config_dto = ExtractionConfigDTO(
        target_dpi=config.target_dpi,
        max_image_dimension=config.max_image_dimension,
        auto_adjust_dpi=config.auto_adjust_dpi,
        min_dpi=config.min_dpi,
        max_dpi=config.max_dpi,
    )

    # Call Rust implementation
    result_array, metadata_dto = rust_normalize_image_dpi(image_array, config_dto, dpi_info)

    result_image = Image.fromarray(result_array)

    # Convert DTO to dataclass
    metadata = ImagePreprocessingMetadata(
        original_dimensions=metadata_dto.original_dimensions,
        original_dpi=metadata_dto.original_dpi,
        target_dpi=metadata_dto.target_dpi,
        scale_factor=metadata_dto.scale_factor,
        auto_adjusted=metadata_dto.auto_adjusted,
        final_dpi=metadata_dto.final_dpi,
        new_dimensions=metadata_dto.new_dimensions,
        resample_method=metadata_dto.resample_method,
        dimension_clamped=metadata_dto.dimension_clamped,
        calculated_dpi=metadata_dto.calculated_dpi,
        skipped_resize=metadata_dto.skipped_resize,
        resize_error=metadata_dto.resize_error,
    )

    return result_image, metadata
