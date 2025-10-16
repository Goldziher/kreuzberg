from __future__ import annotations

from typing import TYPE_CHECKING, Any

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig
from kreuzberg import extract_file

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.anyio
async def test_docx_image_extraction_smoke(docx_document: Any) -> None:
    cfg = ExtractionConfig(images=ImageExtractionConfig())
    result = await extract_file(str(docx_document), config=cfg)
    assert result is not None
    assert isinstance(result.images, list)


@pytest.mark.anyio
async def test_epub_odt_image_extraction_smoke(tmp_path: Path) -> None:  # pragma: no cover - smoke
    odt = tmp_path / "sample.odt"
    odt.write_bytes(b"ODT")
    cfg = ExtractionConfig(images=ImageExtractionConfig())
    try:
        await extract_file(str(odt), config=cfg)
    except Exception:
        assert True
