from __future__ import annotations

import stat
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from PIL import Image, ImageDraw, ImageFont

from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import ExtractionResult
from kreuzberg._utils._cache import get_ocr_cache

if TYPE_CHECKING:
    from typing import Any


def create_single_column_table_image() -> Image.Image:
    img = Image.new("RGB", (400, 300), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 16)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    y_pos = 30
    for item in ["Header", "Row 1", "Row 2", "Row 3"]:
        draw.text((50, y_pos), item, fill="black", font=font)
        y_pos += 50

    return img


def create_single_row_table_image() -> Image.Image:
    img = Image.new("RGB", (600, 100), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 16)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    headers = ["Col1", "Col2", "Col3", "Col4"]
    x_pos = 30
    for header in headers:
        draw.text((x_pos, 40), header, fill="black", font=font)
        x_pos += 130

    return img


def create_irregular_table_image() -> Image.Image:
    img = Image.new("RGB", (500, 250), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 14)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    data = [
        (30, 30, "Name"),
        (120, 35, "Age"),
        (250, 30, "City"),
        (30, 80, "Alice"),
        (125, 85, "25"),
        (250, 80, "NYC"),
        (30, 130, "Bob"),
        (120, 135, "30"),
        (255, 130, "LA"),
    ]

    for x, y, text in data:
        draw.text((x, y), text, fill="black", font=font)

    return img


def create_empty_table_image() -> Image.Image:
    img = Image.new("RGB", (400, 100), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 16)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    headers = ["Col1", "Col2", "Col3"]
    x_pos = 30
    for header in headers:
        draw.text((x_pos, 40), header, fill="black", font=font)
        x_pos += 120

    return img


@pytest.mark.anyio
async def test_table_detection_single_column() -> None:
    backend = TesseractBackend()
    image = create_single_column_table_image()

    result = await backend.process_image(image, enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_table_detection_single_row() -> None:
    backend = TesseractBackend()
    image = create_single_row_table_image()

    result = await backend.process_image(image, enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_table_detection_irregular_spacing() -> None:
    backend = TesseractBackend()
    image = create_irregular_table_image()

    result = await backend.process_image(image, enable_table_detection=True, table_column_threshold=30)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_table_detection_empty_table() -> None:
    backend = TesseractBackend()
    image = create_empty_table_image()

    result = await backend.process_image(image, enable_table_detection=True)

    assert isinstance(result, ExtractionResult)
    assert len(result.content) > 0


@pytest.mark.anyio
async def test_table_detection_varying_thresholds() -> None:
    backend = TesseractBackend()
    image = create_irregular_table_image()

    result1 = await backend.process_image(image, enable_table_detection=True, table_column_threshold=20)
    assert isinstance(result1, ExtractionResult)

    result2 = await backend.process_image(image, enable_table_detection=True, table_column_threshold=80)
    assert isinstance(result2, ExtractionResult)

    assert len(result1.content) > 0
    assert len(result2.content) > 0


@pytest.mark.anyio
async def test_table_detection_varying_row_threshold_ratio() -> None:
    backend = TesseractBackend()
    image = create_irregular_table_image()

    result1 = await backend.process_image(image, enable_table_detection=True, table_row_threshold_ratio=0.3)
    assert isinstance(result1, ExtractionResult)

    result2 = await backend.process_image(image, enable_table_detection=True, table_row_threshold_ratio=0.8)
    assert isinstance(result2, ExtractionResult)


@pytest.mark.anyio
async def test_table_min_confidence_filtering() -> None:
    backend = TesseractBackend()
    image = create_irregular_table_image()

    result1 = await backend.process_image(image, enable_table_detection=True, table_min_confidence=0.0)
    content1_len = len(result1.content)

    result2 = await backend.process_image(image, enable_table_detection=True, table_min_confidence=90.0)
    content2_len = len(result2.content)

    assert content1_len >= content2_len


def test_cache_read_only_directory(tmp_path: Path) -> None:
    backend = TesseractBackend()
    get_ocr_cache()

    cache_dir = tmp_path / ".kreuzberg" / "ocr"
    cache_dir.mkdir(parents=True, exist_ok=True)
    cache_dir.chmod(stat.S_IRUSR | stat.S_IXUSR)

    try:
        img = Image.new("RGB", (200, 100), "white")
        draw = ImageDraw.Draw(img)
        draw.text((50, 40), "Test", fill="black")
        img_path = tmp_path / "test.png"
        img.save(img_path)

        result = backend.process_file_sync(img_path)
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0

    finally:
        cache_dir.chmod(stat.S_IRWXU)


def test_cache_corrupted_file(tmp_path: Path) -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()
    cache = get_ocr_cache()
    cache.clear()

    img = Image.new("RGB", (200, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((50, 40), "Test", fill="black")
    img_path = tmp_path / "test.png"
    img.save(img_path)

    result1 = backend.process_file_sync(img_path)
    assert isinstance(result1, ExtractionResult)

    cache_dir = Path.home() / ".kreuzberg" / "ocr"
    if cache_dir.exists():
        cache_files = list(cache_dir.glob("*.msgpack"))
        if cache_files:
            with cache_files[0].open("wb") as f:
                f.write(b"corrupted data")

    result2 = backend.process_file_sync(img_path)
    assert isinstance(result2, ExtractionResult)
    assert len(result2.content) > 0


def test_cache_clear_success() -> None:
    from kreuzberg._utils._cache import get_ocr_cache

    backend = TesseractBackend()
    cache = get_ocr_cache()

    img = Image.new("RGB", (200, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((50, 40), "Cache Test", fill="black")

    result = backend.process_image_sync(img)
    assert isinstance(result, ExtractionResult)

    cache.clear()

    result2 = backend.process_image_sync(img)
    assert isinstance(result2, ExtractionResult)


def test_batch_processing_mixed_valid_invalid(tmp_path: Path) -> None:
    backend = TesseractBackend()

    paths = []

    for i in range(3):
        img = Image.new("RGB", (200, 100), "white")
        draw = ImageDraw.Draw(img)
        draw.text((50, 40), f"Valid {i}", fill="black")
        img_path = tmp_path / f"valid_{i}.png"
        img.save(img_path)
        paths.append(img_path)

    paths.append(tmp_path / "nonexistent1.png")
    paths.append(tmp_path / "nonexistent2.png")

    corrupted_path = tmp_path / "corrupted.png"
    corrupted_path.write_text("not an image")
    paths.append(corrupted_path)

    results = backend.process_batch_sync(paths)

    assert len(results) == 6

    for i in range(3):
        assert isinstance(results[i], ExtractionResult)
        assert len(results[i].content) > 0


def test_batch_processing_all_invalid(tmp_path: Path) -> None:
    backend = TesseractBackend()

    paths = [
        tmp_path / "missing1.png",
        tmp_path / "missing2.png",
        tmp_path / "missing3.png",
    ]

    results = backend.process_batch_sync(paths)
    assert len(results) == 3


def test_batch_processing_empty_list() -> None:
    backend = TesseractBackend()

    results = backend.process_batch_sync([])

    assert isinstance(results, list)
    assert len(results) == 0


def test_config_validation_invalid_output_format() -> None:
    from kreuzberg._internal_bindings import TesseractConfigDTO

    with pytest.raises(ValueError, match="Invalid output_format"):
        TesseractConfigDTO(output_format="invalid_format")


def test_config_validation_invalid_psm_mode() -> None:
    from kreuzberg._internal_bindings import TesseractConfigDTO

    config = TesseractConfigDTO(psm=11)
    assert config.psm == 11


def test_config_validation_negative_psm() -> None:
    from kreuzberg._internal_bindings import TesseractConfigDTO

    with pytest.raises((ValueError, OverflowError)):
        TesseractConfigDTO(psm=-1)


def test_config_validation_confidence_range_boundary() -> None:
    backend = TesseractBackend()
    img = Image.new("RGB", (200, 100), "white")

    result1 = backend.process_image_sync(img, table_min_confidence=0.0)
    assert isinstance(result1, ExtractionResult)

    result2 = backend.process_image_sync(img, table_min_confidence=100.0)
    assert isinstance(result2, ExtractionResult)


def test_config_all_output_formats() -> None:
    backend = TesseractBackend()
    img = Image.new("RGB", (200, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((50, 40), "Test", fill="black")

    for output_format in ["text", "markdown", "hocr", "tsv"]:
        result = backend.process_image_sync(img, output_format=output_format)
        assert isinstance(result, ExtractionResult)
        assert len(result.content) > 0


def test_config_language_variations() -> None:
    backend = TesseractBackend()
    img = Image.new("RGB", (200, 100), "white")
    draw = ImageDraw.Draw(img)
    draw.text((50, 40), "Test", fill="black")

    result = backend.process_image_sync(img, language="eng")
    assert isinstance(result, ExtractionResult)

    result = backend.process_image_sync(img, language="ENG")
    assert isinstance(result, ExtractionResult)


def test_config_extreme_threshold_values() -> None:
    backend = TesseractBackend()
    img = Image.new("RGB", (200, 100), "white")

    result1 = backend.process_image_sync(img, enable_table_detection=True, table_column_threshold=1)
    assert isinstance(result1, ExtractionResult)

    result2 = backend.process_image_sync(img, enable_table_detection=True, table_column_threshold=1000)
    assert isinstance(result2, ExtractionResult)

    result3 = backend.process_image_sync(img, enable_table_detection=True, table_row_threshold_ratio=0.01)
    assert isinstance(result3, ExtractionResult)

    result4 = backend.process_image_sync(img, enable_table_detection=True, table_row_threshold_ratio=0.99)
    assert isinstance(result4, ExtractionResult)


def test_large_image_memory_handling() -> None:
    backend = TesseractBackend()

    img = Image.new("RGB", (5000, 5000), "white")
    draw = ImageDraw.Draw(img)

    try:
        font: Any = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 32)
    except (OSError, AttributeError):
        font = ImageFont.load_default()

    for i in range(10):
        draw.text((100, 100 + i * 200), f"Large image line {i}", fill="black", font=font)

    result = backend.process_image_sync(img)
    assert isinstance(result, ExtractionResult)


def test_multiple_sequential_processes() -> None:
    backend = TesseractBackend()

    for i in range(20):
        img = Image.new("RGB", (200, 100), "white")
        draw = ImageDraw.Draw(img)
        draw.text((50, 40), f"Image {i}", fill="black")

        result = backend.process_image_sync(img)
        assert isinstance(result, ExtractionResult)


def test_concurrent_cache_access() -> None:
    import concurrent.futures

    backend = TesseractBackend()

    def process_image(i: int) -> ExtractionResult:
        img = Image.new("RGB", (200, 100), "white")
        draw = ImageDraw.Draw(img)
        draw.text((50, 40), f"Concurrent {i}", fill="black")
        return backend.process_image_sync(img)

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(process_image, i) for i in range(10)]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]

    assert len(results) == 10
    for result in results:
        assert isinstance(result, ExtractionResult)
