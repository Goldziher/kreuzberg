# mypy: ignore-errors
from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig, extract_file

if TYPE_CHECKING:
    from pathlib import Path

REAL_PDFS = (
    "test_article.pdf",
    "google_doc_document.pdf",
    "sharable_web_guide.pdf",
)


def _candidate_paths(base: Path) -> list[Path]:
    return [base / "pdfs" / name for name in REAL_PDFS if (base / "pdfs" / name).exists()]


@pytest.mark.asyncio
async def test_extract_images_from_real_pdfs_runs(test_files_path: Path) -> None:
    cfg = ExtractionConfig(images=ImageExtractionConfig())
    candidates = _candidate_paths(test_files_path)
    assert candidates, "No candidate real PDFs found in test_source_files."

    for pdf in candidates:
        result = await extract_file(str(pdf), config=cfg)
        assert isinstance(result.images, list)
        for img in result.images:
            assert isinstance(img.data, (bytes, bytearray))
            assert isinstance(img.format, str)


@pytest.mark.asyncio
async def test_at_least_one_real_pdf_has_images(test_files_path: Path) -> None:
    cfg = ExtractionConfig(images=ImageExtractionConfig())
    candidates = _candidate_paths(test_files_path)
    assert candidates, "No candidate real PDFs found in test_source_files."

    images_found = 0
    for pdf in candidates:
        result = await extract_file(str(pdf), config=cfg)
        images_found += len(result.images)

    if images_found == 0:
        pytest.skip("No embedded images detected in available real PDFs.")

    assert images_found > 0
