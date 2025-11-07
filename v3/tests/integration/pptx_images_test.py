from __future__ import annotations

from typing import Any

import pytest
from kreuzberg import ExtractionConfig
from kreuzberg.extraction import extract_file


@pytest.mark.anyio
async def test_extract_images_from_pptx_smoke(pptx_document: Any) -> None:
    cfg = ExtractionConfig(extract_images=True)
    result = await extract_file(str(pptx_document), config=cfg)
    assert isinstance(result.images, list)
    for img in result.images:
        assert isinstance(img.data, (bytes, bytearray))
        assert isinstance(img.format, str)
