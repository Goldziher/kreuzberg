from __future__ import annotations

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._base import MAX_SINGLE_IMAGE_SIZE, MAX_TOTAL_IMAGE_SIZE
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import ExtractedImage


def test_memory_constants_defined() -> None:
    assert MAX_TOTAL_IMAGE_SIZE == 100 * 1024 * 1024
    assert MAX_SINGLE_IMAGE_SIZE == 50 * 1024 * 1024


def test_check_image_memory_limits_empty_list() -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())
    result = extractor._check_image_memory_limits([])
    assert result == []


def test_check_image_memory_limits_within_limits() -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

    images = [
        ExtractedImage(data=b"x" * 1024, format="png", filename="img1.png"),
        ExtractedImage(data=b"y" * 2048, format="jpg", filename="img2.jpg"),
    ]

    result = extractor._check_image_memory_limits(images)
    assert len(result) == 2
    assert result == images


def test_check_image_memory_limits_single_image_too_large() -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

    large_data = b"x" * (MAX_SINGLE_IMAGE_SIZE + 1)
    small_data = b"y" * 1024

    images = [
        ExtractedImage(data=large_data, format="png", filename="large.png"),
        ExtractedImage(data=small_data, format="jpg", filename="small.jpg"),
    ]

    result = extractor._check_image_memory_limits(images)
    assert len(result) == 1
    assert result[0].filename == "small.jpg"


def test_check_image_memory_limits_total_exceeds_limit() -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

    image_size = 30 * 1024 * 1024
    images = [
        ExtractedImage(data=b"a" * image_size, format="png", filename="img1.png"),
        ExtractedImage(data=b"b" * image_size, format="jpg", filename="img2.jpg"),
        ExtractedImage(data=b"c" * image_size, format="gif", filename="img3.gif"),
        ExtractedImage(data=b"d" * image_size, format="bmp", filename="img4.bmp"),
    ]

    result = extractor._check_image_memory_limits(images)
    assert len(result) == 3
    assert sum(len(img.data) for img in result) <= MAX_TOTAL_IMAGE_SIZE


def test_check_image_memory_limits_prioritizes_smaller_images() -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

    images = [
        ExtractedImage(data=b"a" * (40 * 1024 * 1024), format="png", filename="large.png"),
        ExtractedImage(data=b"b" * (30 * 1024 * 1024), format="jpg", filename="medium.jpg"),
        ExtractedImage(data=b"c" * (20 * 1024 * 1024), format="gif", filename="small1.gif"),
        ExtractedImage(data=b"d" * (15 * 1024 * 1024), format="bmp", filename="small2.bmp"),
    ]

    result = extractor._check_image_memory_limits(images)

    assert len(result) <= 3
    assert sum(len(img.data) for img in result) <= MAX_TOTAL_IMAGE_SIZE

    result_filenames = {img.filename for img in result}
    assert "small2.bmp" in result_filenames
    assert "small1.gif" in result_filenames


def test_check_image_memory_limits_mixed_scenario() -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=ExtractionConfig())

    images = [
        ExtractedImage(data=b"a" * (60 * 1024 * 1024), format="png", filename="huge.png"),
        ExtractedImage(data=b"b" * (10 * 1024 * 1024), format="jpg", filename="small1.jpg"),
        ExtractedImage(data=b"c" * (45 * 1024 * 1024), format="gif", filename="medium.gif"),
        ExtractedImage(data=b"d" * (20 * 1024 * 1024), format="bmp", filename="small2.bmp"),
        ExtractedImage(data=b"e" * (30 * 1024 * 1024), format="tiff", filename="medium2.tiff"),
    ]

    result = extractor._check_image_memory_limits(images)

    assert all(len(img.data) <= MAX_SINGLE_IMAGE_SIZE for img in result)
    assert sum(len(img.data) for img in result) <= MAX_TOTAL_IMAGE_SIZE
    assert "huge.png" not in {img.filename for img in result}
