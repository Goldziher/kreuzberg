"""Memory-optimized image preprocessing utilities."""

from __future__ import annotations

import gc
import tempfile
import weakref
from contextlib import contextmanager, suppress
from pathlib import Path
from typing import TYPE_CHECKING, Any

from PIL import Image

from kreuzberg._constants import PDF_POINTS_PER_INCH
from kreuzberg._types import ExtractionConfig, ImagePreprocessingMetadata

if TYPE_CHECKING:
    from collections.abc import Generator

    from PIL.Image import Image as PILImage

MAX_IMAGE_MEMORY_MB = 500
LARGE_IMAGE_THRESHOLD_MP = 20
TEMP_FILE_THRESHOLD_MB = 100


class ImageMemoryManager:
    """Manages image memory usage and cleanup."""

    def __init__(self, max_memory_mb: float = MAX_IMAGE_MEMORY_MB) -> None:
        self.max_memory_mb = max_memory_mb
        self._tracked_images: list[weakref.ReferenceType] = []
        self._temp_files: list[Path] = []

    def register_image(self, image: PILImage) -> None:
        """Register an image for memory tracking."""
        self._tracked_images.append(weakref.ref(image, self._cleanup_weakref))

    def _cleanup_weakref(self, ref: weakref.ReferenceType) -> None:
        """Called when a tracked image is garbage collected."""
        with suppress(ValueError):
            self._tracked_images.remove(ref)

    def estimate_image_memory_mb(self, width: int, height: int, mode: str = "RGB") -> float:
        """Estimate memory usage of an image in MB."""
        bytes_per_pixel = {"L": 1, "RGB": 3, "RGBA": 4, "CMYK": 4}.get(mode, 3)
        return (width * height * bytes_per_pixel) / (1024 * 1024)

    def check_memory_limit(self, width: int, height: int, mode: str = "RGB") -> None:
        """Check if image would exceed memory limits."""
        estimated_mb = self.estimate_image_memory_mb(width, height, mode)
        if estimated_mb > self.max_memory_mb:
            raise MemoryError(f"Image would use {estimated_mb:.1f}MB, exceeding limit of {self.max_memory_mb}MB")

    @contextmanager
    def temp_image_file(self, image: PILImage, format: str = "PNG") -> Generator[Path, None, None]:
        """Create a temporary file for large images."""
        temp_file = None
        try:
            temp_file = tempfile.NamedTemporaryFile(suffix=f".{format.lower()}", delete=False)
            temp_path = Path(temp_file.name)
            temp_file.close()

            image.save(temp_path, format=format)
            self._temp_files.append(temp_path)

            yield temp_path

        finally:
            if temp_file:
                try:
                    temp_path.unlink(missing_ok=True)
                    self._temp_files.remove(temp_path)
                except (OSError, ValueError):
                    pass

    def cleanup(self) -> None:
        """Clean up all tracked resources."""
        for ref in self._tracked_images[:]:
            img = ref()
            if img:
                with suppress(AttributeError):
                    img.close()

        self._tracked_images.clear()

        for temp_file in self._temp_files[:]:
            with suppress(OSError):
                temp_file.unlink(missing_ok=True)
        self._temp_files.clear()

        gc.collect()


_memory_manager = ImageMemoryManager()


@contextmanager
def managed_image(image: PILImage) -> Generator[PILImage, None, None]:
    """Context manager for automatic image cleanup."""
    _memory_manager.register_image(image)
    try:
        yield image
    finally:
        with suppress(AttributeError):
            image.close()


def calculate_optimal_dpi_memory_aware(
    page_width: float,
    page_height: float,
    target_dpi: int,
    max_dimension: int,
    max_memory_mb: float = MAX_IMAGE_MEMORY_MB,
    min_dpi: int = 72,
    max_dpi: int = 600,
) -> int:
    """Calculate optimal DPI with memory constraints."""
    width_inches = page_width / PDF_POINTS_PER_INCH
    height_inches = page_height / PDF_POINTS_PER_INCH

    target_width_pixels = int(width_inches * target_dpi)
    target_height_pixels = int(height_inches * target_dpi)

    max_pixel_dimension = max(target_width_pixels, target_height_pixels)

    estimated_memory = _memory_manager.estimate_image_memory_mb(target_width_pixels, target_height_pixels, "RGB")

    if estimated_memory > max_memory_mb:
        memory_scale = (max_memory_mb / estimated_memory) ** 0.5
        target_dpi = int(target_dpi * memory_scale)

    if max_pixel_dimension <= max_dimension:
        return max(min_dpi, min(target_dpi, max_dpi))

    max_dpi_for_width = max_dimension / width_inches if width_inches > 0 else max_dpi
    max_dpi_for_height = max_dimension / height_inches if height_inches > 0 else max_dpi
    constrained_dpi = int(min(max_dpi_for_width, max_dpi_for_height))

    return max(min_dpi, min(constrained_dpi, max_dpi))


def normalize_image_dpi_memory_optimized(
    image: PILImage,
    config: ExtractionConfig,
) -> tuple[PILImage, ImagePreprocessingMetadata]:
    """Memory-optimized version of normalize_image_dpi."""
    original_width, original_height = image.size
    original_mode = image.mode
    megapixels = (original_width * original_height) / 1_000_000

    try:
        _memory_manager.check_memory_limit(original_width, original_height, original_mode)
    except MemoryError as e:
        return image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=(72, 72),
            target_dpi=config.target_dpi,
            scale_factor=1.0,
            auto_adjusted=False,
            final_dpi=config.target_dpi,
            resize_error=str(e),
            skipped_resize=True,
        )

    current_dpi_info = image.info.get("dpi", (PDF_POINTS_PER_INCH, PDF_POINTS_PER_INCH))
    if isinstance(current_dpi_info, (list, tuple)):
        original_dpi = (float(current_dpi_info[0]), float(current_dpi_info[1]))
        current_dpi = float(current_dpi_info[0])
    else:
        current_dpi = float(current_dpi_info)
        original_dpi = (current_dpi, current_dpi)

    max_current_dimension = max(original_width, original_height)
    current_matches_target = abs(current_dpi - config.target_dpi) < 1.0

    if not config.auto_adjust_dpi and current_matches_target and max_current_dimension <= config.max_image_dimension:
        return image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=original_dpi,
            target_dpi=config.target_dpi,
            scale_factor=1.0,
            auto_adjusted=False,
            final_dpi=config.target_dpi,
            skipped_resize=True,
        )

    calculated_dpi = None
    if config.auto_adjust_dpi:
        approx_width_points = original_width * PDF_POINTS_PER_INCH / current_dpi
        approx_height_points = original_height * PDF_POINTS_PER_INCH / current_dpi

        optimal_dpi = calculate_optimal_dpi_memory_aware(
            approx_width_points,
            approx_height_points,
            config.target_dpi,
            config.max_image_dimension,
            max_memory_mb=MAX_IMAGE_MEMORY_MB,
            min_dpi=config.min_dpi,
            max_dpi=config.max_dpi,
        )
        calculated_dpi = optimal_dpi
        auto_adjusted = optimal_dpi != config.target_dpi
        target_dpi = optimal_dpi
    else:
        auto_adjusted = False
        target_dpi = config.target_dpi

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

    dimension_clamped = False
    max_new_dimension = max(new_width, new_height)
    if max_new_dimension > config.max_image_dimension:
        dimension_scale = config.max_image_dimension / max_new_dimension
        new_width = int(new_width * dimension_scale)
        new_height = int(new_height * dimension_scale)
        scale_factor *= dimension_scale
        dimension_clamped = True

    use_temp_file = (
        megapixels > LARGE_IMAGE_THRESHOLD_MP
        or _memory_manager.estimate_image_memory_mb(new_width, new_height, original_mode) > TEMP_FILE_THRESHOLD_MB
    )

    try:
        if use_temp_file:
            with _memory_manager.temp_image_file(image) as temp_path, Image.open(temp_path) as temp_image:
                normalized_image = _resize_image_progressive(temp_image, new_width, new_height, scale_factor)
        else:
            normalized_image = _resize_image_progressive(image, new_width, new_height, scale_factor)

        normalized_image.info["dpi"] = (target_dpi, target_dpi)

        return normalized_image, ImagePreprocessingMetadata(
            original_dimensions=(original_width, original_height),
            original_dpi=original_dpi,
            target_dpi=config.target_dpi,
            scale_factor=scale_factor,
            auto_adjusted=auto_adjusted,
            final_dpi=target_dpi,
            new_dimensions=(new_width, new_height),
            resample_method="PROGRESSIVE",
            dimension_clamped=dimension_clamped,
            calculated_dpi=calculated_dpi,
        )

    except (OSError, MemoryError) as e:
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


def _resize_image_progressive(image: PILImage, target_width: int, target_height: int, scale_factor: float) -> PILImage:
    """Resize image progressively to reduce memory usage."""
    current_image = image

    if scale_factor > 2.0 or scale_factor < 0.5:
        steps = max(2, int(abs(scale_factor - 1.0) * 2))
        step_factor = scale_factor ** (1.0 / steps)

        current_width, current_height = image.size

        for i in range(steps):
            if i == steps - 1:
                new_width, new_height = target_width, target_height
            else:
                new_width = int(current_width * step_factor)
                new_height = int(current_height * step_factor)

            resample_method = Image.Resampling.LANCZOS if step_factor < 1.0 else Image.Resampling.BICUBIC

            try:
                new_image = current_image.resize((new_width, new_height), resample_method)

                if current_image != image:
                    current_image.close()

                current_image = new_image
                current_width, current_height = new_width, new_height

            except AttributeError:
                resample_method = getattr(Image, "LANCZOS", 1) if step_factor < 1.0 else getattr(Image, "BICUBIC", 3)
                new_image = current_image.resize((new_width, new_height), resample_method)

                if current_image != image:
                    current_image.close()

                current_image = new_image
                current_width, current_height = new_width, new_height

            if current_width * current_height > 10_000_000:
                gc.collect()

        return current_image

    resample_method = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC

    try:
        return image.resize((target_width, target_height), resample_method)
    except AttributeError:
        resample_method = getattr(Image, "LANCZOS", 1) if scale_factor < 1.0 else getattr(Image, "BICUBIC", 3)
        return image.resize((target_width, target_height), resample_method)


def cleanup_image_memory() -> None:
    """Manually trigger image memory cleanup."""
    _memory_manager.cleanup()


def get_image_memory_stats() -> dict[str, Any]:
    """Get current image memory usage statistics."""
    tracked_count = len([ref for ref in _memory_manager._tracked_images if ref() is not None])
    temp_files_count = len(_memory_manager._temp_files)

    return {
        "tracked_images": tracked_count,
        "temp_files": temp_files_count,
        "max_memory_mb": _memory_manager.max_memory_mb,
        "large_image_threshold_mp": LARGE_IMAGE_THRESHOLD_MP,
        "temp_file_threshold_mb": TEMP_FILE_THRESHOLD_MB,
    }
