"""Aggressive memory-optimized image preprocessing."""

from __future__ import annotations

import gc
import tempfile
from contextlib import contextmanager, suppress
from pathlib import Path
from typing import TYPE_CHECKING

from PIL import Image

from kreuzberg._constants import PDF_POINTS_PER_INCH
from kreuzberg._types import ExtractionConfig, ImagePreprocessingMetadata

if TYPE_CHECKING:
    from collections.abc import Generator

    from PIL.Image import Image as PILImage

MAX_IMAGE_MEMORY_MB = 200
MAX_PIXELS_IN_MEMORY = 50_000_000
ALWAYS_USE_DISK_THRESHOLD_MB = 50


class AggressiveMemoryManager:
    """Aggressive memory management with disk-based operations."""

    def __init__(self) -> None:
        self._temp_dir = None
        self._temp_files: list[Path] = []

    @contextmanager
    def temp_directory(self) -> Generator[Path, None, None]:
        """Create temporary directory for operations."""
        if self._temp_dir is None:
            self._temp_dir = tempfile.mkdtemp(prefix="kreuzberg_img_")

        temp_path = Path(self._temp_dir)
        try:
            yield temp_path
        finally:
            pass

    def estimate_memory_mb(self, width: int, height: int, channels: int = 3) -> float:
        """Estimate memory usage."""
        return (width * height * channels) / (1024 * 1024)

    def should_use_disk(self, width: int, height: int, channels: int = 3) -> bool:
        """Determine if operation should use disk instead of memory."""
        estimated_mb = self.estimate_memory_mb(width, height, channels)
        return estimated_mb > ALWAYS_USE_DISK_THRESHOLD_MB

    def cleanup(self) -> None:
        """Aggressive cleanup of all resources."""
        for temp_file in self._temp_files:
            with suppress(OSError):
                temp_file.unlink(missing_ok=True)
        self._temp_files.clear()

        if self._temp_dir:
            try:
                import shutil

                shutil.rmtree(self._temp_dir, ignore_errors=True)
            except OSError:
                pass
            self._temp_dir = None

        for _ in range(3):
            gc.collect()


_aggressive_manager = AggressiveMemoryManager()


def calculate_smart_dpi(
    page_width: float,
    page_height: float,
    target_dpi: int,
    max_dimension: int,
    max_memory_mb: float = MAX_IMAGE_MEMORY_MB,
) -> int:
    """Calculate DPI that respects memory constraints."""
    width_inches = page_width / PDF_POINTS_PER_INCH
    height_inches = page_height / PDF_POINTS_PER_INCH

    max_width_pixels = int((max_memory_mb * 1024 * 1024 / 3) ** 0.5)
    max_height_pixels = max_width_pixels

    max_dpi_for_memory_width = max_width_pixels / width_inches if width_inches > 0 else target_dpi
    max_dpi_for_memory_height = max_height_pixels / height_inches if height_inches > 0 else target_dpi

    memory_constrained_dpi = int(min(max_dpi_for_memory_width, max_dpi_for_memory_height))

    target_width_pixels = int(width_inches * target_dpi)
    target_height_pixels = int(height_inches * target_dpi)
    max_pixel_dimension = max(target_width_pixels, target_height_pixels)

    if max_pixel_dimension > max_dimension:
        max_dpi_for_width = max_dimension / width_inches if width_inches > 0 else target_dpi
        max_dpi_for_height = max_dimension / height_inches if height_inches > 0 else target_dpi
        dimension_constrained_dpi = int(min(max_dpi_for_width, max_dpi_for_height))
    else:
        dimension_constrained_dpi = target_dpi

    final_dpi = min(target_dpi, memory_constrained_dpi, dimension_constrained_dpi)
    return max(72, final_dpi)


def resize_with_disk_fallback(
    image: PILImage, target_width: int, target_height: int, scale_factor: float, use_disk: bool = False
) -> PILImage:
    """Resize image with disk-based fallback for large operations."""
    if not use_disk:
        try:
            resample = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC

            return image.resize((target_width, target_height), resample)

        except (MemoryError, OSError):
            pass

    with _aggressive_manager.temp_directory() as temp_dir:
        input_path = temp_dir / "input.png"
        image.save(input_path, "PNG", compress_level=1)

        del image
        gc.collect()

        with Image.open(input_path) as disk_image:
            resample = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC

            resized = disk_image.resize((target_width, target_height), resample)

            output_path = temp_dir / "output.png"
            resized.save(output_path, "PNG", compress_level=1)

            del resized
            gc.collect()

            final_image = Image.open(output_path)

            final_copy = final_image.copy()
            final_image.close()

            return final_copy


def normalize_image_dpi_aggressive(
    image: PILImage,
    config: ExtractionConfig,
) -> tuple[PILImage, ImagePreprocessingMetadata]:
    """Aggressively memory-optimized image normalization."""
    original_width, original_height = image.size
    original_width * original_height

    original_memory_mb = _aggressive_manager.estimate_memory_mb(original_width, original_height)
    if original_memory_mb > MAX_IMAGE_MEMORY_MB:
        return image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=(72, 72),
            target_dpi=config.target_dpi,
            scale_factor=1.0,
            auto_adjusted=False,
            final_dpi=72,
            resize_error=f"Image too large: {original_memory_mb:.1f}MB > {MAX_IMAGE_MEMORY_MB}MB",
            skipped_resize=True,
        )

    current_dpi_info = image.info.get("dpi", (PDF_POINTS_PER_INCH, PDF_POINTS_PER_INCH))
    if isinstance(current_dpi_info, (list, tuple)):
        original_dpi = (float(current_dpi_info[0]), float(current_dpi_info[1]))
        current_dpi = float(current_dpi_info[0])
    else:
        current_dpi = float(current_dpi_info)
        original_dpi = (current_dpi, current_dpi)

    if config.auto_adjust_dpi:
        approx_width_points = original_width * PDF_POINTS_PER_INCH / current_dpi
        approx_height_points = original_height * PDF_POINTS_PER_INCH / current_dpi

        target_dpi = calculate_smart_dpi(
            approx_width_points,
            approx_height_points,
            config.target_dpi,
            config.max_image_dimension,
            MAX_IMAGE_MEMORY_MB,
        )
        auto_adjusted = target_dpi != config.target_dpi
        calculated_dpi = target_dpi
    else:
        approx_width_points = original_width * PDF_POINTS_PER_INCH / current_dpi
        approx_height_points = original_height * PDF_POINTS_PER_INCH / current_dpi

        memory_safe_dpi = calculate_smart_dpi(
            approx_width_points,
            approx_height_points,
            config.target_dpi,
            config.max_image_dimension,
            MAX_IMAGE_MEMORY_MB,
        )

        target_dpi = min(config.target_dpi, memory_safe_dpi)
        auto_adjusted = target_dpi != config.target_dpi
        calculated_dpi = target_dpi

    scale_factor = target_dpi / current_dpi

    if abs(scale_factor - 1.0) < 0.05:
        return image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=original_dpi,
            target_dpi=config.target_dpi,
            scale_factor=scale_factor,
            auto_adjusted=auto_adjusted,
            final_dpi=target_dpi,
            calculated_dpi=calculated_dpi,
            skipped_resize=True,
        )

    new_width = int(original_width * scale_factor)
    new_height = int(original_height * scale_factor)
    new_pixels = new_width * new_height

    dimension_clamped = False
    max_new_dimension = max(new_width, new_height)
    if max_new_dimension > config.max_image_dimension:
        dimension_scale = config.max_image_dimension / max_new_dimension
        new_width = int(new_width * dimension_scale)
        new_height = int(new_height * dimension_scale)
        scale_factor *= dimension_scale
        new_pixels = new_width * new_height
        dimension_clamped = True

    result_memory_mb = _aggressive_manager.estimate_memory_mb(new_width, new_height)
    total_memory_needed = original_memory_mb + result_memory_mb

    use_disk = (
        total_memory_needed > MAX_IMAGE_MEMORY_MB
        or new_pixels > MAX_PIXELS_IN_MEMORY
        or _aggressive_manager.should_use_disk(new_width, new_height)
    )

    try:
        normalized_image = resize_with_disk_fallback(image, new_width, new_height, scale_factor, use_disk)

        normalized_image.info["dpi"] = (target_dpi, target_dpi)

        _aggressive_manager.cleanup()

        return normalized_image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=original_dpi,
            target_dpi=config.target_dpi,
            scale_factor=scale_factor,
            auto_adjusted=auto_adjusted,
            final_dpi=target_dpi,
            new_dimensions=(new_width, new_height),
            resample_method="DISK_FALLBACK" if use_disk else "MEMORY",
            dimension_clamped=dimension_clamped,
            calculated_dpi=calculated_dpi,
        )

    except Exception as e:
        _aggressive_manager.cleanup()
        return image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=original_dpi,
            target_dpi=config.target_dpi,
            scale_factor=scale_factor,
            auto_adjusted=auto_adjusted,
            final_dpi=target_dpi,
            calculated_dpi=calculated_dpi,
            resize_error=str(e),
        )


def cleanup_aggressive_memory() -> None:
    """Clean up all aggressive memory management resources."""
    _aggressive_manager.cleanup()
