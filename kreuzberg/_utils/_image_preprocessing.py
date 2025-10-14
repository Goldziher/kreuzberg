from __future__ import annotations

from typing import TYPE_CHECKING

import msgpack  # type: ignore[import-not-found]
import numpy as np
from PIL import Image

from kreuzberg._internal_bindings import normalize_image_dpi_msgpack  # type: ignore[attr-defined]
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

    # Serialize config to MessagePack
    config_dict = {
        "target_dpi": config.target_dpi,
        "max_image_dimension": config.max_image_dimension,
        "auto_adjust_dpi": config.auto_adjust_dpi,
        "min_dpi": config.min_dpi,
        "max_dpi": config.max_dpi,
    }
    config_msgpack = msgpack.packb(config_dict)

    # Call Rust implementation
    result_array, metadata_msgpack = normalize_image_dpi_msgpack(image_array, config_msgpack, dpi_info)

    result_image = Image.fromarray(result_array)

    # Deserialize metadata from MessagePack
    metadata_dict = msgpack.unpackb(metadata_msgpack, raw=False)

    metadata = ImagePreprocessingMetadata(
        original_dimensions=tuple(metadata_dict["original_dimensions"]),
        original_dpi=tuple(metadata_dict["original_dpi"]),
        target_dpi=metadata_dict["target_dpi"],
        scale_factor=metadata_dict["scale_factor"],
        auto_adjusted=metadata_dict["auto_adjusted"],
        final_dpi=metadata_dict["final_dpi"],
        new_dimensions=tuple(metadata_dict["new_dimensions"]) if metadata_dict["new_dimensions"] else None,
        resample_method=metadata_dict["resample_method"],
        dimension_clamped=metadata_dict["dimension_clamped"],
        calculated_dpi=metadata_dict["calculated_dpi"],
        skipped_resize=metadata_dict["skipped_resize"],
        resize_error=metadata_dict["resize_error"],
    )

    return result_image, metadata
