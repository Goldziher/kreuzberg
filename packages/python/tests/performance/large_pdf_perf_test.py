from __future__ import annotations

import os
from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig
from kreuzberg import extract_file

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.anyio
@pytest.mark.skipif(os.getenv("RUN_PERF_TESTS") != "1", reason="Set RUN_PERF_TESTS=1 to run performance tests")
async def test_large_pdf_image_extraction_performance(test_files_path: Path) -> None:
    candidate = None
    for name in ("sharable_web_guide.pdf", "xerox_alta_link_series_mfp_sag_en_us_2.pdf"):
        p = test_files_path / "pdfs" / name
        if p.exists():
            candidate = p
            break
    if candidate is None:
        pytest.skip("No large PDF available for performance test")

    cfg = ExtractionConfig(images=ImageExtractionConfig())
    result = await extract_file(str(candidate), config=cfg)
    assert isinstance(result.images, list)
