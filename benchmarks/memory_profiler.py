"""Deep memory profiling of image preprocessing operations."""

from __future__ import annotations

import gc
import os
import tracemalloc

from PIL import Image

from kreuzberg._types import ExtractionConfig


def get_process_memory_mb() -> float:
    """Get current process memory usage in MB."""
    try:
        import psutil

        process = psutil.Process(os.getpid())
        return process.memory_info().rss / (1024 * 1024)
    except ImportError:
        return 0.0


def create_test_image(width: int, height: int) -> Image.Image:
    """Create a test image."""
    return Image.new("RGB", (width, height), color="white")


def detailed_memory_trace(width: int, height: int) -> None:
    """Trace memory usage step by step."""

    config = ExtractionConfig(target_dpi=300, max_image_dimension=4096, auto_adjust_dpi=False)

    # Start tracing
    tracemalloc.start()
    gc.collect()

    step = 0

    def trace_step(name: str) -> None:
        nonlocal step
        step += 1
        _current, _peak = tracemalloc.get_traced_memory()
        get_process_memory_mb()

    trace_step("Initial state")

    # Create image
    image = create_test_image(width, height)
    trace_step("Image created")

    # Extract dimensions
    original_width, original_height = image.size
    trace_step("Dimensions extracted")

    # Extract DPI
    current_dpi_info = image.info.get("dpi", (72, 72))
    if isinstance(current_dpi_info, (list, tuple)):
        (float(current_dpi_info[0]), float(current_dpi_info[1]))
        current_dpi = float(current_dpi_info[0])
    else:
        current_dpi = float(current_dpi_info)
    trace_step("DPI extracted")

    # Calculate scale factor
    target_dpi = config.target_dpi
    scale_factor = target_dpi / current_dpi
    trace_step("Scale factor calculated")

    # Calculate new dimensions
    new_width = int(original_width * scale_factor)
    new_height = int(original_height * scale_factor)
    trace_step("New dimensions calculated")

    # Check if resize needed
    if abs(scale_factor - 1.0) < 0.05:
        tracemalloc.stop()
        return

    # Choose resampling method
    try:
        resample_method = Image.Resampling.LANCZOS if scale_factor < 1.0 else Image.Resampling.BICUBIC
    except AttributeError:
        resample_method = getattr(Image, "LANCZOS", 1) if scale_factor < 1.0 else getattr(Image, "BICUBIC", 3)

    trace_step("Resampling method chosen")

    # THE CRITICAL OPERATION: Resize
    normalized_image = image.resize((new_width, new_height), resample_method)
    trace_step("Image resized")

    # Set DPI
    normalized_image.info["dpi"] = (target_dpi, target_dpi)
    trace_step("DPI set")

    # Get final memory
    _current, _peak = tracemalloc.get_traced_memory()

    # Cleanup
    image.close()
    trace_step("Original image closed")

    normalized_image.close()
    trace_step("Normalized image closed")

    tracemalloc.stop()
    gc.collect()
    get_process_memory_mb()


def analyze_memory_patterns() -> None:
    """Analyze memory usage patterns."""

    # Test different scenarios
    scenarios = [
        (1000, 1000, "small_square"),
        (2000, 1000, "wide_rectangle"),
        (1000, 2000, "tall_rectangle"),
        (2000, 2000, "medium_square"),
        (3000, 3000, "large_square"),
        (4000, 3000, "large_wide"),
    ]

    for width, height, _scenario_name in scenarios:
        (width * height * 3) / (1024 * 1024)
        # Estimate resized image size (assuming 300 DPI target vs 72 DPI current)
        scale_factor = 300 / 72
        new_width = int(width * scale_factor)
        new_height = int(height * scale_factor)
        (new_width * new_height * 3) / (1024 * 1024)

        detailed_memory_trace(width, height)


def find_memory_hotspots() -> None:
    """Identify the biggest memory hotspots."""

    # Test the resize operation in isolation
    image = create_test_image(3000, 2000)  # ~17.2 MB

    tracemalloc.start()
    gc.collect()

    _initial_current, _ = tracemalloc.get_traced_memory()
    get_process_memory_mb()

    # Test different scale factors
    scale_factors = [0.5, 1.0, 2.0, 4.0]

    for scale_factor in scale_factors:
        gc.collect()
        _before_current, _ = tracemalloc.get_traced_memory()
        get_process_memory_mb()

        new_width = int(3000 * scale_factor)
        new_height = int(2000 * scale_factor)

        try:
            resized = image.resize((new_width, new_height), Image.Resampling.LANCZOS)

            _after_current, _peak = tracemalloc.get_traced_memory()
            get_process_memory_mb()

            resized.close()

        except Exception:
            pass

    image.close()
    tracemalloc.stop()


def main() -> None:
    """Run deep memory analysis."""

    analyze_memory_patterns()
    find_memory_hotspots()


if __name__ == "__main__":
    main()
