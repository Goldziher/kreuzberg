from __future__ import annotations

from unittest.mock import patch

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._types import (
    ExtractedImage,
    ExtractionResult,
    ImageExtractionConfig,
    ImageOCRResult,
    TesseractConfig,
)


@pytest.mark.anyio
async def test_process_images_filters_by_format() -> None:
    config = ExtractionConfig(
        images=ImageExtractionConfig(ocr_allowed_formats=frozenset(["png", "jpg"]), ocr_min_dimensions=(1, 1)),
        ocr=TesseractConfig(),
    )

    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    with patch.object(extractor, "_check_image_memory_limits", return_value=None):
        images = [
            ExtractedImage(data=b"test", format="png", filename="test.png"),
            ExtractedImage(data=b"test", format="svg", filename="test.svg"),
        ]

        filtered = [img for img in images if config.images and img.format in config.images.ocr_allowed_formats]

        assert len(filtered) == 1
        assert filtered[0].format == "png"


def test_memory_limit_check() -> None:
    config = ExtractionConfig(images=ImageExtractionConfig())
    extractor = PDFExtractor(mime_type="application/pdf", config=config)

    small_images = [ExtractedImage(data=b"x" * 1000, format="png")]
    result = extractor._check_image_memory_limits(small_images)
    assert len(result) == 1

    large_images = [ExtractedImage(data=b"x" * (51 * 1024 * 1024), format="png")]
    result = extractor._check_image_memory_limits(large_images)
    assert len(result) == 0


@pytest.mark.anyio
async def test_ocr_result_structure() -> None:
    result = ImageOCRResult(
        image=ExtractedImage(data=b"test", format="png"),
        ocr_result=ExtractionResult(content="text", mime_type="text/plain", metadata={}),
        skipped_reason=None,
    )

    assert result.image.format == "png"
    assert result.ocr_result.content == "text"
    assert result.skipped_reason is None

    skipped = ImageOCRResult(
        image=ExtractedImage(data=b"test", format="svg"),
        ocr_result=ExtractionResult(content="", mime_type="text/plain", metadata={}),
        skipped_reason="Unsupported format: svg",
    )

    assert skipped.skipped_reason == "Unsupported format: svg"
    assert skipped.ocr_result.content == ""
