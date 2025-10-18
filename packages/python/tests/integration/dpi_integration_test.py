from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from PIL import Image

from kreuzberg import ExtractionConfig, OcrConfig, extract_file

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def large_test_image(tmp_path: Path) -> Path:
    """Create a large test image with high DPI."""
    image = Image.new("RGB", (5000, 7000), "white")

    from PIL import ImageDraw, ImageFont

    draw = ImageDraw.Draw(image)

    try:
        font = ImageFont.load_default()
    except OSError:
        font = None

    if font:
        for i in range(20):
            y_pos = 100 + (i * 300)
            draw.text((100, y_pos), f"Test text line {i + 1} with some content", fill="black", font=font)
    else:
        for i in range(20):
            y_pos = 100 + (i * 300)
            draw.rectangle([100, y_pos, 2000, y_pos + 50], fill="black")
            draw.rectangle([110, y_pos + 10, 1990, y_pos + 40], fill="white")

    image.info["dpi"] = (300, 300)

    image_path = tmp_path / "large_test_image.png"
    image.save(str(image_path), format="PNG", dpi=(300, 300))
    return image_path


@pytest.fixture
def small_test_image(tmp_path: Path) -> Path:
    """Create a small test image with low DPI."""
    image = Image.new("RGB", (400, 600), "white")

    from PIL import ImageDraw, ImageFont

    draw = ImageDraw.Draw(image)

    try:
        font = ImageFont.load_default()
    except OSError:
        font = None

    if font:
        draw.text((50, 50), "Small test image", fill="black", font=font)
        draw.text((50, 100), "With some text", fill="black", font=font)
    else:
        draw.rectangle([50, 50, 350, 80], fill="black")
        draw.rectangle([60, 60, 340, 70], fill="white")

    image.info["dpi"] = (72, 72)

    image_path = tmp_path / "small_test_image.png"
    image.save(str(image_path), format="PNG", dpi=(72, 72))
    return image_path


@pytest.mark.asyncio
async def test_large_image_dpi_adjustment(large_test_image: Path) -> None:
    """Test that large images get DPI adjusted for OCR processing."""
    config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=300,
        max_image_dimension=10000,
        auto_adjust_dpi=True,
        min_dpi=72,
        max_dpi=600,
    )

    result = await extract_file(str(large_test_image), config=config)

    assert result is not None
    assert result.content is not None
    assert len(result.content.strip()) > 0

    assert "image_preprocessing" in result.metadata
    preprocessing = result.metadata["image_preprocessing"]

    assert preprocessing.auto_adjusted or preprocessing.scale_factor != 1.0


@pytest.mark.asyncio
async def test_small_image_no_adjustment(small_test_image: Path) -> None:
    """Test that small images don't get excessive upscaling."""
    config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=150,
        max_image_dimension=25000,
        auto_adjust_dpi=True,
    )

    result = await extract_file(str(small_test_image), config=config)

    assert result is not None
    assert result.content is not None

    if "image_preprocessing" in result.metadata:
        preprocessing = result.metadata["image_preprocessing"]
        scale_factor = preprocessing.scale_factor
        assert 0.3 <= scale_factor <= 3.0


@pytest.mark.asyncio
async def test_dpi_disabled_auto_adjust(large_test_image: Path) -> None:
    """Test that disabling auto-adjust prevents DPI changes."""
    config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=72,
        max_image_dimension=25000,
        auto_adjust_dpi=False,
    )

    result = await extract_file(str(large_test_image), config=config)

    assert result is not None
    assert result.content is not None

    if "image_preprocessing" in result.metadata:
        preprocessing = result.metadata["image_preprocessing"]
        assert not preprocessing.auto_adjusted


@pytest.mark.asyncio
async def test_different_dpi_targets(small_test_image: Path) -> None:
    """Test extraction with different target DPI values."""
    low_dpi_config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=72,
        auto_adjust_dpi=False,
    )

    high_dpi_config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=300,
        auto_adjust_dpi=False,
    )

    low_result = await extract_file(str(small_test_image), config=low_dpi_config)
    high_result = await extract_file(str(small_test_image), config=high_dpi_config)

    assert low_result is not None
    assert high_result is not None
    assert low_result.content is not None
    assert high_result.content is not None


@pytest.mark.asyncio
async def test_pdf_dpi_integration(google_doc_pdf: Path) -> None:
    """Test DPI handling when forcing OCR on PDFs."""
    config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=100,
        max_image_dimension=20000,
        auto_adjust_dpi=True,
        force_ocr=True,
    )

    result = await extract_file(str(google_doc_pdf), config=config)

    assert result is not None
    assert result.content is not None
    assert len(result.content) > 100

    content_lower = result.content.lower()
    assert any(word in content_lower for word in ["page", "web", "guide", "the", "and"])


@pytest.mark.asyncio
async def test_extreme_dpi_values(small_test_image: Path) -> None:
    """Test handling of extreme DPI constraints."""
    very_low_config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=72,
        min_dpi=72,
        max_dpi=100,
        max_image_dimension=5000,
        auto_adjust_dpi=True,
    )

    result = await extract_file(str(small_test_image), config=very_low_config)
    assert result is not None
    assert result.content is not None


@pytest.mark.asyncio
async def test_metadata_preservation(small_test_image: Path) -> None:
    """Test that preprocessing metadata is preserved in results."""
    config = ExtractionConfig(
        ocr=OcrConfig(),
        target_dpi=144,
        auto_adjust_dpi=True,
    )

    result = await extract_file(str(small_test_image), config=config)

    assert result is not None
    assert result.metadata is not None

    if "image_preprocessing" in result.metadata:
        preprocessing = result.metadata["image_preprocessing"]

        assert hasattr(preprocessing, "original_dimensions")
        assert hasattr(preprocessing, "scale_factor")
        assert hasattr(preprocessing, "target_dpi")

        orig_dims = preprocessing.original_dimensions
        assert isinstance(orig_dims, (list, tuple))
        assert len(orig_dims) == 2
        assert all(isinstance(d, int) and d > 0 for d in orig_dims)
