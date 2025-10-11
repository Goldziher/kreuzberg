from __future__ import annotations

import gc
import tracemalloc
from typing import TYPE_CHECKING, Any

import pytest
from PIL import Image, ImageDraw, ImageFont

from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import ExtractionResult

if TYPE_CHECKING:
    from pathlib import Path

    from PIL.Image import Image as PILImage
    from pytest_benchmark.fixture import BenchmarkFixture  # type: ignore[import-untyped]


def create_test_image_small() -> PILImage:
    img = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 20)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    draw.text((20, 30), "Small Test Image", fill="black", font=font)
    return img


def create_test_image_medium() -> PILImage:
    img = Image.new("RGB", (1200, 800), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 24)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    y_pos = 50
    for i in range(10):
        draw.text((50, y_pos), f"Line {i + 1}: This is test content for benchmarking", fill="black", font=font)
        y_pos += 70

    return img


def create_test_image_large() -> PILImage:
    img = Image.new("RGB", (2400, 1600), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 28)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    y_pos = 50
    for i in range(20):
        draw.text((100, y_pos), f"Row {i + 1}: Large image benchmark test content goes here", fill="black", font=font)
        y_pos += 70

    return img


@pytest.mark.anyio
async def test_benchmark_async_process_image_small(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_small()

    async def process() -> ExtractionResult:
        return await backend.process_image(image)

    result = await benchmark.pedantic(process, rounds=5, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_benchmark_async_process_image_medium(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_medium()

    async def process() -> ExtractionResult:
        return await backend.process_image(image)

    result = await benchmark.pedantic(process, rounds=3, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_benchmark_async_process_image_large(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_large()

    async def process() -> ExtractionResult:
        return await backend.process_image(image)

    result = await benchmark.pedantic(process, rounds=2, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_benchmark_async_process_file(benchmark: BenchmarkFixture, tmp_path: Path) -> None:
    backend = TesseractBackend()
    image = create_test_image_small()
    img_path = tmp_path / "benchmark.png"
    image.save(img_path)

    async def process() -> ExtractionResult:
        return await backend.process_file(img_path)

    result = await benchmark.pedantic(process, rounds=5, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_benchmark_sync_process_image_small(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_small()

    result = benchmark.pedantic(backend.process_image_sync, args=(image,), kwargs={}, rounds=5, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_benchmark_sync_process_image_medium(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_medium()

    result = benchmark.pedantic(backend.process_image_sync, args=(image,), kwargs={}, rounds=3, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_benchmark_sync_process_image_large(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_large()

    result = benchmark.pedantic(backend.process_image_sync, args=(image,), kwargs={}, rounds=2, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_benchmark_sync_process_file(benchmark: BenchmarkFixture, tmp_path: Path) -> None:
    backend = TesseractBackend()
    image = create_test_image_small()
    img_path = tmp_path / "benchmark_sync.png"
    image.save(img_path)

    result = benchmark.pedantic(backend.process_file_sync, args=(img_path,), kwargs={}, rounds=5, iterations=1)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


def test_benchmark_sync_batch_processing_10_images(benchmark: BenchmarkFixture, tmp_path: Path) -> None:
    backend = TesseractBackend()

    paths = []
    for i in range(10):
        img = create_test_image_small()
        img_path = tmp_path / f"batch_{i}.png"
        img.save(img_path)
        paths.append(img_path)

    def process_batch() -> list[ExtractionResult]:
        return backend.process_batch_sync(paths)

    results = benchmark.pedantic(process_batch, rounds=2, iterations=1)

    assert len(results) == 10
    for result in results:
        assert isinstance(result, ExtractionResult)


def test_benchmark_cache_hit(benchmark: BenchmarkFixture, tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()
    cache = get_ocr_cache()
    cache.clear()

    image = create_test_image_small()
    img_path = tmp_path / "cache_hit_test.png"
    image.save(img_path)

    backend.process_file_sync(img_path)

    def process_with_cache() -> ExtractionResult:
        return backend.process_file_sync(img_path)

    result = benchmark.pedantic(process_with_cache, rounds=20, iterations=1)

    assert isinstance(result, ExtractionResult)


def test_benchmark_cache_miss(benchmark: BenchmarkFixture, tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()
    cache = get_ocr_cache()

    def process_unique_image() -> ExtractionResult:
        cache.clear()
        image = create_test_image_small()
        img_path = tmp_path / "cache_miss_test.png"
        image.save(img_path)
        return backend.process_file_sync(img_path)

    result = benchmark.pedantic(process_unique_image, rounds=3, iterations=1)

    assert isinstance(result, ExtractionResult)


def test_benchmark_memory_small_image(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_small()

    def process_with_memory_tracking() -> tuple[ExtractionResult, float]:
        tracemalloc.start()
        gc.collect()

        result = backend.process_image_sync(image)

        _current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()

        peak_mb = peak / 1024 / 1024
        return result, peak_mb

    result, peak_mb = benchmark(process_with_memory_tracking)

    assert isinstance(result, ExtractionResult)
    assert peak_mb > 0


def test_benchmark_memory_medium_image(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_medium()

    def process_with_memory_tracking() -> tuple[ExtractionResult, float]:
        tracemalloc.start()
        gc.collect()

        result = backend.process_image_sync(image)

        _current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()

        peak_mb = peak / 1024 / 1024
        return result, peak_mb

    result, peak_mb = benchmark(process_with_memory_tracking)

    assert isinstance(result, ExtractionResult)
    assert peak_mb > 0


def test_benchmark_memory_large_image(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_large()

    def process_with_memory_tracking() -> tuple[ExtractionResult, float]:
        tracemalloc.start()
        gc.collect()

        result = backend.process_image_sync(image)

        _current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()

        peak_mb = peak / 1024 / 1024
        return result, peak_mb

    result, peak_mb = benchmark(process_with_memory_tracking)

    assert isinstance(result, ExtractionResult)
    assert peak_mb > 0


def test_benchmark_memory_batch_10_images(benchmark: BenchmarkFixture, tmp_path: Path) -> None:
    backend = TesseractBackend()

    paths = []
    for i in range(10):
        img = create_test_image_small()
        img_path = tmp_path / f"memory_batch_{i}.png"
        img.save(img_path)
        paths.append(img_path)

    def process_with_memory_tracking() -> tuple[list[ExtractionResult], float]:
        tracemalloc.start()
        gc.collect()

        results = backend.process_batch_sync(paths)

        _current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()

        peak_mb = peak / 1024 / 1024
        return results, peak_mb

    results, peak_mb = benchmark(process_with_memory_tracking)

    assert len(results) == 10
    assert peak_mb > 0


@pytest.mark.parametrize("output_format", ["text", "markdown", "hocr", "tsv"])
def test_benchmark_output_format(benchmark: BenchmarkFixture, output_format: str) -> None:
    backend = TesseractBackend()
    image = create_test_image_small()

    result = benchmark.pedantic(
        backend.process_image_sync,
        args=(image,),
        kwargs={"output_format": output_format},
        rounds=5,
        iterations=1,
    )

    assert isinstance(result, ExtractionResult)


def test_benchmark_with_table_detection(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_medium()

    result = benchmark.pedantic(
        backend.process_image_sync,
        args=(image,),
        kwargs={"enable_table_detection": True},
        rounds=3,
        iterations=1,
    )

    assert isinstance(result, ExtractionResult)


def test_benchmark_without_table_detection(benchmark: BenchmarkFixture) -> None:
    backend = TesseractBackend()
    image = create_test_image_medium()

    result = benchmark.pedantic(
        backend.process_image_sync,
        args=(image,),
        kwargs={"enable_table_detection": False, "output_format": "text"},
        rounds=3,
        iterations=1,
    )

    assert isinstance(result, ExtractionResult)
