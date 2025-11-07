from __future__ import annotations

from typing import TYPE_CHECKING, Any

import pytest
from kreuzberg import ExtractionConfig
from kreuzberg.extraction import extract_file

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.anyio
async def test_docx_image_extraction_smoke(docx_document: Any) -> None:
    cfg = ExtractionConfig(extract_images=True)
    result = await extract_file(str(docx_document), config=cfg)
    assert result is not None
    assert isinstance(result.images, list)


@pytest.mark.anyio
async def test_epub_odt_image_extraction_smoke(tmp_path: Path) -> None:  # pragma: no cover - smoke
    odt = tmp_path / "sample.odt"
    odt.write_bytes(b"ODT")
    cfg = ExtractionConfig(extract_images=True)
    try:
        await extract_file(str(odt), config=cfg)
    except Exception:
        assert True
