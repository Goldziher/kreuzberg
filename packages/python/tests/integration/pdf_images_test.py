# mypy: ignore-errors
from __future__ import annotations

from typing import Any

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig, extract_file


@pytest.mark.skip(
    reason="Image extraction removed from general ExtractionResult in v4 - images are now format-specific"
)
@pytest.mark.asyncio
async def test_extract_images_from_pdf_smoke(searchable_pdf: Any) -> None:
    cfg = ExtractionConfig(images=ImageExtractionConfig())
    result = await extract_file(str(searchable_pdf), config=cfg)
    assert isinstance(result.images, list)
    for img in result.images:
        assert isinstance(img.data, (bytes, bytearray))
        assert isinstance(img.format, str)
