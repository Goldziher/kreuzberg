"""Aggressive memory-optimized image preprocessing."""

from __future__ import annotations

import gc
import shutil
import tempfile
from contextlib import contextmanager, suppress
from pathlib import Path
from typing import TYPE_CHECKING, Any

import psutil
from PIL import Image

from kreuzberg._constants import PDF_POINTS_PER_INCH
from kreuzberg._types import ExtractionConfig, ImagePreprocessingMetadata
from kreuzberg.exceptions import MemoryLimitError

if TYPE_CHECKING:
    from collections.abc import Generator

    from PIL.Image import Image as PILImage

# Constants for memory and disk management
_DEFAULT_TOTAL_MEMORY_MB = 8192  # 8GB fallback
_DEFAULT_AVAILABLE_MEMORY_MB = 4096  # 4GB fallback
_MEMORY_USAGE_FRACTION = 0.25  # Use 25% of available memory
_MAX_MEMORY_CAP_MB = 2048  # Never exceed 2GB
_DISK_SAFETY_MARGIN = 2.5  # 2.5x image size needed for disk operations
_MIN_DISK_SPACE_MB = 50  # Minimum disk space required
_TEST_FILE_SIZE_BYTES = 1024  # Size of test file for disk checks


def _get_system_memory_info() -> tuple[float, float]:
    """Get system memory information.

    Returns:
        Tuple of (total_memory_mb, available_memory_mb)
    """
    try:
        memory = psutil.virtual_memory()
        total_mb = memory.total / (1024 * 1024)
        available_mb = memory.available / (1024 * 1024)
        return total_mb, available_mb
    except (OSError, ValueError, AttributeError):
        # Fallback: Try to get memory info from /proc/meminfo (Linux)
        try:
            meminfo_path = Path("/proc/meminfo")
            with meminfo_path.open() as f:
                meminfo = f.read()

            total_kb = None
            available_kb = None

            for line in meminfo.split("\n"):
                if line.startswith("MemTotal:"):
                    total_kb = int(line.split()[1])
                elif line.startswith("MemAvailable:"):
                    available_kb = int(line.split()[1])

            if total_kb and available_kb:
                return total_kb / 1024, available_kb / 1024
        except (OSError, ValueError, FileNotFoundError):
            pass

    # Conservative fallback if we can't detect memory
    return _DEFAULT_TOTAL_MEMORY_MB, _DEFAULT_AVAILABLE_MEMORY_MB


def _calculate_memory_limits() -> tuple[float, float, float]:
    """Calculate dynamic memory limits based on system resources.

    Returns:
        Tuple of (max_image_memory_mb, max_pixels, disk_threshold_mb)
    """
    _total_mb, available_mb = _get_system_memory_info()

    # Use at most 25% of available memory for image operations
    max_image_memory_mb = min(available_mb * _MEMORY_USAGE_FRACTION, _MAX_MEMORY_CAP_MB)

    # Scale pixel limits based on memory
    max_pixels = int((max_image_memory_mb * 1024 * 1024) / 3)  # 3 bytes per pixel (RGB)

    # Use disk for anything above 10% of our memory limit
    disk_threshold_mb = max_image_memory_mb * 0.1

    return max_image_memory_mb, max_pixels, disk_threshold_mb


# Dynamic memory limits based on system resources
MAX_IMAGE_MEMORY_MB, MAX_PIXELS_IN_MEMORY, ALWAYS_USE_DISK_THRESHOLD_MB = _calculate_memory_limits()


class AggressiveMemoryManager:
    """Aggressive memory management with disk-based operations."""

    def __init__(self) -> None:
        self._temp_dir: str | None = None
        self._temp_files: list[Path] = []
        self._disk_available: bool | None = None  # Cache the disk availability check
        self._disk_check_error: str | None = None

    def _check_disk_availability(self) -> tuple[bool, str | None]:
        """Check if disk operations are available and safe to use.

        Returns:
            Tuple of (is_available, error_message)
        """
        if self._disk_available is not None:
            return self._disk_available, self._disk_check_error

        try:
            # Test 1: Can we create a temporary directory?
            test_temp_dir = tempfile.mkdtemp(prefix="kreuzberg_test_")
            test_temp_path = Path(test_temp_dir)

            # Test 2: Can we write to it?
            test_file = test_temp_path / "test_write.tmp"
            test_content = b"kreuzberg_disk_test_" + b"x" * _TEST_FILE_SIZE_BYTES
            test_file.write_bytes(test_content)

            # Test 3: Can we read back?
            if test_file.read_bytes() != test_content:
                raise OSError("File read/write verification failed")

            # Test 4: Comprehensive disk space analysis
            disk_usage = shutil.disk_usage(test_temp_path)
            free_space_mb = disk_usage.free / (1024 * 1024)
            total_space_mb = disk_usage.total / (1024 * 1024)

            # Calculate required space based on our memory limits
            # For disk fallback, we need space for: original image + resized image + overhead
            max_temp_space_needed = MAX_IMAGE_MEMORY_MB * _DISK_SAFETY_MARGIN

            if free_space_mb < max_temp_space_needed:
                raise OSError(
                    f"Insufficient disk space for image operations: "
                    f"{free_space_mb:.1f}MB available < {max_temp_space_needed:.1f}MB required "
                    f"(total disk: {total_space_mb:.1f}MB)"
                )

            # Test 5: Check if filesystem is read-only
            test_file2 = test_temp_path / "test_readonly.tmp"
            try:
                test_file2.write_bytes(b"readonly_test")
                test_file2.unlink()
            except (OSError, PermissionError) as err:
                raise OSError("Filesystem appears to be read-only") from err

            # Test 6: Test creating subdirectory (some operations might need this)
            test_subdir = test_temp_path / "subdir"
            test_subdir.mkdir()
            test_subdir.rmdir()

            # Cleanup test files
            test_file.unlink()
            test_temp_path.rmdir()

            self._disk_available = True
            self._disk_check_error = None
            return True, None

        except (OSError, PermissionError, FileNotFoundError) as e:
            error_msg = f"Disk operations unavailable: {e}"
            self._disk_available = False
            self._disk_check_error = error_msg
            return False, error_msg
        except (ValueError, TypeError) as e:
            error_msg = f"Unexpected disk check error: {e}"
            self._disk_available = False
            self._disk_check_error = error_msg
            return False, error_msg

    def can_use_disk(self) -> bool:
        """Check if disk-based operations are available."""
        available, _ = self._check_disk_availability()
        return available

    def get_disk_error(self) -> str | None:
        """Get the error message if disk is not available."""
        _, error = self._check_disk_availability()
        return error

    def get_resource_info(self) -> dict[str, Any]:
        """Get comprehensive resource information for error context."""
        total_mb, available_mb = _get_system_memory_info()

        disk_info = {"available": False, "error": "Not checked"}
        if self._disk_available is not None:
            disk_info["available"] = self._disk_available
            if not self._disk_available:
                disk_info["error"] = self._disk_check_error
            else:
                try:
                    temp_path = Path(tempfile.gettempdir())
                    disk_usage = shutil.disk_usage(temp_path)
                    disk_info.update(
                        {
                            "free_space_mb": disk_usage.free / (1024 * 1024),
                            "total_space_mb": disk_usage.total / (1024 * 1024),
                            "temp_directory": str(temp_path),
                        }
                    )
                except (OSError, ImportError) as e:
                    disk_info["error"] = f"Could not get disk info: {e}"

        return {
            "memory": {
                "total_system_mb": total_mb,
                "available_system_mb": available_mb,
                "max_image_limit_mb": MAX_IMAGE_MEMORY_MB,
                "disk_threshold_mb": ALWAYS_USE_DISK_THRESHOLD_MB,
                "max_pixels": MAX_PIXELS_IN_MEMORY,
            },
            "disk": disk_info,
        }

    @contextmanager
    def temp_directory(self) -> Generator[Path, None, None]:
        """Create temporary directory for operations.

        Raises:
            OSError: If disk operations are not available
        """
        available, error = self._check_disk_availability()
        if not available:
            raise OSError(f"Cannot create temp directory: {error}")

        if self._temp_dir is None:
            self._temp_dir = tempfile.mkdtemp(prefix="kreuzberg_img_")

        temp_path = Path(self._temp_dir)
        try:
            yield temp_path
        finally:
            pass  # Don't delete directory until cleanup

    def estimate_memory_mb(self, width: int, height: int, channels: int = 3) -> float:
        """Estimate memory usage."""
        return (width * height * channels) / (1024 * 1024)

    def should_use_disk(self, width: int, height: int, channels: int = 3) -> bool:
        """Determine if operation should use disk instead of memory."""
        estimated_mb = self.estimate_memory_mb(width, height, channels)
        would_benefit = estimated_mb > ALWAYS_USE_DISK_THRESHOLD_MB

        # Only use disk if it would help AND disk is available
        return would_benefit and self.can_use_disk()

    def cleanup(self) -> None:
        """Aggressive cleanup of all resources."""
        # Delete all temp files
        for temp_file in self._temp_files:
            with suppress(OSError):
                temp_file.unlink(missing_ok=True)
        self._temp_files.clear()

        # Delete temp directory
        if self._temp_dir:
            with suppress(OSError):
                shutil.rmtree(self._temp_dir, ignore_errors=True)
            self._temp_dir = None

        # Force multiple garbage collections
        for _ in range(3):
            gc.collect()


# Global aggressive manager
_aggressive_manager = AggressiveMemoryManager()


def calculate_optimal_dpi(
    page_width: float,
    page_height: float,
    target_dpi: int,
    max_dimension: int,
    min_dpi: int = 72,
    max_dpi: int = 600,
) -> int:
    """Calculate optimal DPI based on page dimensions and constraints.

    Args:
        page_width: Page width in points (1/72 inch)
        page_height: Page height in points (1/72 inch)
        target_dpi: Desired target DPI
        max_dimension: Maximum allowed pixel dimension
        min_dpi: Minimum DPI threshold
        max_dpi: Maximum DPI threshold

    Returns:
        Optimal DPI value that keeps image within max_dimension
    """
    # Calculate the DPI using smart logic
    smart_dpi = calculate_smart_dpi(page_width, page_height, target_dpi, max_dimension, MAX_IMAGE_MEMORY_MB)

    # Apply the min/max bounds that the original function expected
    return max(min_dpi, min(smart_dpi, max_dpi))


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

    # Calculate what DPI would fit in memory
    max_width_pixels = int((max_memory_mb * 1024 * 1024 / 3) ** 0.5)
    max_height_pixels = max_width_pixels

    max_dpi_for_memory_width = max_width_pixels / width_inches if width_inches > 0 else target_dpi
    max_dpi_for_memory_height = max_height_pixels / height_inches if height_inches > 0 else target_dpi

    memory_constrained_dpi = int(min(max_dpi_for_memory_width, max_dpi_for_memory_height))

    # Also respect dimension constraint
    target_width_pixels = int(width_inches * target_dpi)
    target_height_pixels = int(height_inches * target_dpi)
    max_pixel_dimension = max(target_width_pixels, target_height_pixels)

    if max_pixel_dimension > max_dimension:
        max_dpi_for_width = max_dimension / width_inches if width_inches > 0 else target_dpi
        max_dpi_for_height = max_dimension / height_inches if height_inches > 0 else target_dpi
        dimension_constrained_dpi = int(min(max_dpi_for_width, max_dpi_for_height))
    else:
        dimension_constrained_dpi = target_dpi

    # Use the most restrictive constraint
    final_dpi = min(target_dpi, memory_constrained_dpi, dimension_constrained_dpi)
    return max(72, final_dpi)  # Never go below 72 DPI


def resize_with_disk_fallback(
    image: PILImage, target_width: int, target_height: int, scale_factor: float, use_disk: bool = False
) -> PILImage:
    """Resize image with disk-based fallback for large operations."""
    if not use_disk:
        # Try memory-based resize first
        try:
            # Temporarily increase PIL safety limits for large images
            old_max_pixels = Image.MAX_IMAGE_PIXELS
            original_pixels = image.size[0] * image.size[1]
            target_pixels = target_width * target_height
            max_needed_pixels = max(original_pixels, target_pixels)

            if Image.MAX_IMAGE_PIXELS is not None and max_needed_pixels > Image.MAX_IMAGE_PIXELS:
                Image.MAX_IMAGE_PIXELS = max_needed_pixels * 2  # Add some buffer

            try:
                resample = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC

                return image.resize((target_width, target_height), resample)
            finally:
                # Restore original safety limit
                Image.MAX_IMAGE_PIXELS = old_max_pixels

        except (MemoryError, OSError, Image.DecompressionBombError):
            # Fall back to disk-based approach
            pass

    # Disk-based approach - but check if disk is available first
    if not _aggressive_manager.can_use_disk():
        # Disk not available, but we were asked to use it - raise an informative error
        context = {
            "requested_operation": "disk-based image resize",
            "image_dimensions": (image.size[0], image.size[1]),
            "target_dimensions": (target_width, target_height),
            "scale_factor": scale_factor,
            "resources": _aggressive_manager.get_resource_info(),
        }

        raise MemoryLimitError("Disk-based resize requested but disk operations not available", context=context)

    # Store image info before potential deletion for error fallback
    original_image = image
    original_pixels = image.size[0] * image.size[1]

    try:
        with _aggressive_manager.temp_directory() as temp_dir:
            # Save original to disk
            input_path = temp_dir / "input.png"
            image.save(input_path, "PNG", compress_level=1)  # Fast compression

            # Force cleanup of original from memory (but keep original_image reference)
            del image
            gc.collect()

            # Load and resize from disk
            with Image.open(input_path) as disk_image:
                resample = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC

                resized = disk_image.resize((target_width, target_height), resample)

                # Save resized to disk and reload to ensure memory is clean
                output_path = temp_dir / "output.png"
                resized.save(output_path, "PNG", compress_level=1)

                # Clean up intermediate result
                del resized
                gc.collect()

                # Load final result
                final_image = Image.open(output_path)

                # Make a copy to avoid file handle issues
                final_copy = final_image.copy()
                final_image.close()

                return final_copy

    except (OSError, PermissionError, FileNotFoundError):
        # Disk operations failed - fall back to memory-only with increased limits

        # Try one more time with memory-only approach and increased PIL limits
        old_max_pixels = Image.MAX_IMAGE_PIXELS
        try:
            target_pixels = target_width * target_height
            max_needed_pixels = max(original_pixels, target_pixels)
            Image.MAX_IMAGE_PIXELS = max_needed_pixels * 2

            resample = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC

            return original_image.resize((target_width, target_height), resample)
        finally:
            Image.MAX_IMAGE_PIXELS = old_max_pixels


def normalize_image_dpi(  # noqa: PLR0912, PLR0915
    image: PILImage,
    config: ExtractionConfig,
) -> tuple[PILImage, ImagePreprocessingMetadata]:
    """Aggressively memory-optimized image normalization."""
    original_width, original_height = image.size
    original_pixels = original_width * original_height

    # Immediate memory check - consider both size and disk availability
    original_memory_mb = _aggressive_manager.estimate_memory_mb(original_width, original_height)

    if original_memory_mb > MAX_IMAGE_MEMORY_MB:
        can_use_disk = _aggressive_manager.can_use_disk()

        # If image is huge and we have no disk access, we must reject it
        if not can_use_disk and not config.auto_adjust_dpi:
            context = {
                "image_dimensions": (original_width, original_height),
                "image_memory_mb": original_memory_mb,
                "config": {
                    "target_dpi": config.target_dpi,
                    "max_image_dimension": config.max_image_dimension,
                    "auto_adjust_dpi": config.auto_adjust_dpi,
                },
                "resources": _aggressive_manager.get_resource_info(),
                "recommendations": [
                    "Enable auto_adjust_dpi to allow automatic downscaling",
                    "Provide smaller input images",
                    "Ensure sufficient disk space and write permissions for temporary files",
                    f"Free up memory (currently using {original_memory_mb:.1f}MB > {MAX_IMAGE_MEMORY_MB}MB limit)",
                ],
            }

            raise MemoryLimitError(
                f"Image too large for memory-only processing: {original_memory_mb:.1f}MB > {MAX_IMAGE_MEMORY_MB}MB. "
                f"Disk fallback not available. Consider enabling auto_adjust_dpi or providing smaller images.",
                context=context,
            )

        # If auto_adjust is disabled but we have disk, allow processing (will use disk fallback)
        if not config.auto_adjust_dpi and can_use_disk:
            return image, ImagePreprocessingMetadata(
                original_dimensions=(original_width, original_height),
                original_dpi=(72, 72),
                target_dpi=config.target_dpi,
                scale_factor=1.0,
                auto_adjusted=False,
                final_dpi=72,
                resize_error=f"Image too large: {original_memory_mb:.1f}MB > {MAX_IMAGE_MEMORY_MB}MB (will use disk)",
                skipped_resize=True,
            )

        # If auto_adjust is enabled, continue and try to downscale aggressively

    # Extract DPI
    current_dpi_info = image.info.get("dpi", (PDF_POINTS_PER_INCH, PDF_POINTS_PER_INCH))
    if isinstance(current_dpi_info, (list, tuple)):
        original_dpi = (float(current_dpi_info[0]), float(current_dpi_info[1]))
        current_dpi = float(current_dpi_info[0])
    else:
        current_dpi = float(current_dpi_info)
        original_dpi = (current_dpi, current_dpi)

    # Calculate smart target DPI that respects memory limits
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
        # Even for non-auto-adjust, apply memory constraints
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

    # Skip if change is minimal AND no dimension constraints are violated
    max_current_dimension = max(original_width, original_height)
    needs_dimension_resize = max_current_dimension > config.max_image_dimension

    if abs(scale_factor - 1.0) < 0.05 and not needs_dimension_resize:
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

    # Calculate new dimensions
    new_width = int(original_width * scale_factor)
    new_height = int(original_height * scale_factor)
    new_pixels = new_width * new_height

    # Apply dimension constraints
    dimension_clamped = False
    max_new_dimension = max(new_width, new_height)
    if max_new_dimension > config.max_image_dimension:
        dimension_scale = config.max_image_dimension / max_new_dimension
        new_width = int(new_width * dimension_scale)
        new_height = int(new_height * dimension_scale)
        scale_factor *= dimension_scale
        new_pixels = new_width * new_height
        dimension_clamped = True

    # Check if we should use disk-based processing
    result_memory_mb = _aggressive_manager.estimate_memory_mb(new_width, new_height)
    total_memory_needed = original_memory_mb + result_memory_mb  # Both images in memory during resize

    would_benefit_from_disk = (
        total_memory_needed > MAX_IMAGE_MEMORY_MB
        or new_pixels > MAX_PIXELS_IN_MEMORY
        or result_memory_mb > ALWAYS_USE_DISK_THRESHOLD_MB
    )

    # Only use disk if it would help AND disk operations are available
    use_disk = would_benefit_from_disk and _aggressive_manager.can_use_disk()

    # If we would benefit from disk but can't use it, log the limitation
    if would_benefit_from_disk and not _aggressive_manager.can_use_disk():
        _aggressive_manager.get_disk_error()
        # Continue with memory-only operation, but this might fail or be slow

    try:
        # Perform the resize
        normalized_image = resize_with_disk_fallback(image, new_width, new_height, scale_factor, use_disk)

        # Set DPI info
        normalized_image.info["dpi"] = (target_dpi, target_dpi)

        # Force cleanup
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

    except (MemoryError, OSError, Image.DecompressionBombError, ValueError) as e:
        _aggressive_manager.cleanup()

        # If memory-optimized approach failed and auto_adjust is enabled,
        # try falling back to dimension-only optimization (like original implementation)
        if config.auto_adjust_dpi and "decompression bomb" in str(e).lower():
            try:
                # Use original DPI calculation that only considers dimensions
                approx_width_points = original_width * PDF_POINTS_PER_INCH / current_dpi
                approx_height_points = original_height * PDF_POINTS_PER_INCH / current_dpi

                # Calculate DPI using only dimension constraints (no memory constraints)
                width_inches = approx_width_points / PDF_POINTS_PER_INCH
                height_inches = approx_height_points / PDF_POINTS_PER_INCH

                target_width_pixels = int(width_inches * target_dpi)
                target_height_pixels = int(height_inches * target_dpi)
                max_pixel_dimension = max(target_width_pixels, target_height_pixels)

                if max_pixel_dimension <= config.max_image_dimension:
                    fallback_dpi = target_dpi
                else:
                    max_dpi_for_width = config.max_image_dimension / width_inches if width_inches > 0 else target_dpi
                    max_dpi_for_height = config.max_image_dimension / height_inches if height_inches > 0 else target_dpi
                    fallback_dpi = int(min(max_dpi_for_width, max_dpi_for_height))
                    fallback_dpi = max(72, min(fallback_dpi, 600))  # Apply bounds

                fallback_scale_factor = fallback_dpi / current_dpi
                fallback_width = int(original_width * fallback_scale_factor)
                fallback_height = int(original_height * fallback_scale_factor)

                # Try the fallback resize with increased PIL limits
                old_max_pixels = Image.MAX_IMAGE_PIXELS
                try:
                    needed_pixels = max(original_pixels, fallback_width * fallback_height)
                    Image.MAX_IMAGE_PIXELS = needed_pixels * 2

                    resample = Image.Resampling.LANCZOS if fallback_scale_factor < 1.0 else Image.Resampling.BICUBIC

                    fallback_image = image.resize((fallback_width, fallback_height), resample)
                    fallback_image.info["dpi"] = (fallback_dpi, fallback_dpi)

                    return fallback_image, ImagePreprocessingMetadata(
                        original_dimensions=(original_width, original_height),
                        original_dpi=original_dpi,
                        target_dpi=config.target_dpi,
                        scale_factor=fallback_scale_factor,
                        auto_adjusted=True,
                        final_dpi=fallback_dpi,
                        new_dimensions=(fallback_width, fallback_height),
                        resample_method="FALLBACK",
                        dimension_clamped=fallback_dpi != target_dpi,
                        calculated_dpi=fallback_dpi,
                    )
                finally:
                    Image.MAX_IMAGE_PIXELS = old_max_pixels

            except (MemoryError, OSError, Image.DecompressionBombError):
                pass  # If fallback also fails, continue to original error handling

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


def get_dpi_adjustment_heuristics(
    width: float,
    height: float,
    current_dpi: int,
    target_dpi: int,
    max_dimension: int,
    content_type: str = "document",
) -> dict[str, Any]:
    """Get smart DPI adjustment recommendations based on content analysis.

    Args:
        width: Image width in pixels
        height: Image height in pixels
        current_dpi: Current DPI setting
        target_dpi: Desired target DPI
        max_dimension: Maximum allowed dimension
        content_type: Type of content ("document", "photo", "mixed")

    Returns:
        Dictionary with adjustment recommendations and rationale
    """
    recommendations: list[str] = []
    heuristics = {
        "recommended_dpi": target_dpi,
        "content_analysis": {},
        "performance_impact": "medium",
        "quality_impact": "medium",
        "recommendations": recommendations,
    }

    aspect_ratio = width / height if height > 0 else 1.0
    total_pixels = width * height
    megapixels = total_pixels / 1_000_000

    heuristics["content_analysis"] = {
        "aspect_ratio": aspect_ratio,
        "megapixels": megapixels,
        "is_portrait": aspect_ratio < 0.8,
        "is_landscape": aspect_ratio > 1.2,
        "is_large": max(width, height) > max_dimension * 0.8,
    }

    if content_type == "document":
        if aspect_ratio > 2.0 or aspect_ratio < 0.5:
            recommendations.append("Consider higher DPI for narrow documents")
            if target_dpi < 200:
                heuristics["recommended_dpi"] = min(200, target_dpi * 1.3)

        if megapixels > 50:
            recommendations.append("Large document detected - consider DPI reduction")
            heuristics["performance_impact"] = "high"
            if target_dpi > 150:
                heuristics["recommended_dpi"] = max(120, target_dpi * 0.8)

    estimated_memory_mb = (width * height * 3) / (1024 * 1024)
    if estimated_memory_mb > MAX_IMAGE_MEMORY_MB:
        heuristics["performance_impact"] = "high"
        recommendations.append(f"High memory usage expected (~{estimated_memory_mb:.0f}MB)")

    scale_factor = target_dpi / current_dpi if current_dpi > 0 else 1.0
    if scale_factor < 0.7:
        heuristics["quality_impact"] = "high"
        recommendations.append("Significant downscaling may reduce OCR accuracy")
    elif scale_factor > 1.5:
        heuristics["performance_impact"] = "high"
        recommendations.append("Upscaling will increase processing time")

    return heuristics


def estimate_processing_time(
    width: int,
    height: int,
    ocr_backend: str = "tesseract",
) -> dict[str, float | str]:
    """Estimate processing time based on image dimensions and OCR backend.

    Args:
        width: Image width in pixels
        height: Image height in pixels
        ocr_backend: OCR backend name

    Returns:
        Dictionary with time estimates in seconds
    """
    total_pixels = width * height
    megapixels = total_pixels / 1_000_000

    base_times = {
        "tesseract": 2.5,
        "easyocr": 4.0,
        "paddleocr": 3.5,
    }

    base_time = base_times.get(ocr_backend, 3.0)

    scaling_factor = 1.0 + (megapixels - 10) * 0.1 if megapixels > 10 else 1.0

    estimated_time = base_time * megapixels * scaling_factor

    return {
        "estimated_seconds": estimated_time,
        "megapixels": megapixels,
        "backend": ocr_backend,
        "scaling_factor": scaling_factor,
    }


def cleanup_aggressive_memory() -> None:
    """Clean up all aggressive memory management resources."""
    _aggressive_manager.cleanup()
