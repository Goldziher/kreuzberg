# mypy: ignore-errors
from __future__ import annotations

from typing import Any

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig, extract_file


@pytest.mark.asyncio
async def test_pptx_complex_images_smoke(pptx_document: Any) -> None:
    cfg = ExtractionConfig(images=ImageExtractionConfig())
    result = await extract_file(str(pptx_document), config=cfg)

    # Images may be None or empty list if file has no images
    assert result.images is None or isinstance(result.images, list)
    if result.images:
        for img in result.images:
            assert isinstance(img["data"], (bytes, bytearray))
            assert isinstance(img["format"], str)
            if img.get("page_number") is not None:
                assert isinstance(img["page_number"], int)
                assert img["page_number"] > 0
