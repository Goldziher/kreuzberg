from __future__ import annotations

import io  # noqa: TC003
from types import SimpleNamespace
from typing import Any

import pytest
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg.extraction import DEFAULT_CONFIG


class _MockImageObj:
    def __init__(self) -> None:
        self.srcsize = (100, 200)
        self.bits = 8
        self.imagemask = False
        self.colorspace = SimpleNamespace(name="RGB")
        self.stream = b"dummy"


class _MockPage:
    def __init__(self, images: list[Any]) -> None:
        self.images = images


class _MockDoc:
    def __init__(self, pages: list[Any]) -> None:
        self.pages = pages


@pytest.mark.anyio
async def test_pdf_extract_images_from_playa_method(monkeypatch: pytest.MonkeyPatch) -> None:
    extractor = PDFExtractor(mime_type="application/pdf", config=DEFAULT_CONFIG)

    mock_img = _MockImageObj()
    mock_doc = _MockDoc([_MockPage([mock_img])])

    def _writer(buffer: io.BytesIO) -> None:
        buffer.write(b"fake_jpg_data")

    def _get_suffix_and_writer(stream: bytes) -> tuple[str, Any]:
        return ".jpg", _writer

    monkeypatch.setattr("kreuzberg._extractors._pdf.get_image_suffix_and_writer", _get_suffix_and_writer)

    images = await extractor._extract_images_from_playa(mock_doc)  # type: ignore[arg-type]

    assert len(images) == 1
    assert images[0].format == "jpg"
    assert images[0].page_number == 1
    assert images[0].dimensions == (100, 200)
