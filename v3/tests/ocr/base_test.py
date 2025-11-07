from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import Mock

import pytest
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult
from PIL import Image

if TYPE_CHECKING:
    from pathlib import Path


class TestOCRBackend(OCRBackend[dict[str, object]]):
    async def process_image(self, image: Image.Image, **kwargs: dict[str, object]) -> ExtractionResult:
        return ExtractionResult(content="Test OCR result", mime_type="text/plain", metadata={}, chunks=[])

    async def process_file(self, path: Path, **kwargs: dict[str, object]) -> ExtractionResult:
        return ExtractionResult(content="Test file OCR result", mime_type="text/plain", metadata={}, chunks=[])

    def process_image_sync(self, image: Image.Image, **kwargs: dict[str, object]) -> ExtractionResult:
        return ExtractionResult(content="Test OCR result", mime_type="text/plain", metadata={}, chunks=[])

    def process_file_sync(self, path: Path, **kwargs: dict[str, object]) -> ExtractionResult:
        return ExtractionResult(content="Test file OCR result", mime_type="text/plain", metadata={}, chunks=[])


def test_ocr_backend_hash() -> None:
    backend1 = TestOCRBackend()
    backend2 = TestOCRBackend()

    assert hash(backend1) == hash(backend2)
    assert hash(backend1) == hash("TestOCRBackend")


def test_ocr_backend_different_types_different_hash() -> None:
    class AnotherTestBackend(OCRBackend[dict[str, object]]):
        async def process_image(self, image: Image.Image, **kwargs: dict[str, object]) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={}, chunks=[])

        async def process_file(self, path: Path, **kwargs: dict[str, object]) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={}, chunks=[])

        def process_image_sync(self, image: Image.Image, **kwargs: dict[str, object]) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={}, chunks=[])

        def process_file_sync(self, path: Path, **kwargs: dict[str, object]) -> ExtractionResult:
            return ExtractionResult(content="", mime_type="text/plain", metadata={}, chunks=[])

    backend1 = TestOCRBackend()
    backend2 = AnotherTestBackend()

    assert hash(backend1) != hash(backend2)


@pytest.mark.anyio
async def test_ocr_backend_process_image() -> None:
    backend = TestOCRBackend()
    image = Mock(spec=Image.Image)

    result = await backend.process_image(image)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Test OCR result"
    assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_ocr_backend_process_file(tmp_path: Path) -> None:
    backend = TestOCRBackend()
    test_file = tmp_path / "test.txt"
    test_file.write_text("Test content")

    result = await backend.process_file(test_file)

    assert isinstance(result, ExtractionResult)
    assert result.content == "Test file OCR result"
    assert result.mime_type == "text/plain"
