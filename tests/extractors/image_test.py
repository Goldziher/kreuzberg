from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from kreuzberg._extractors._image import ImageExtractor
from kreuzberg._types import ExtractionConfig, ExtractionResult
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture
def extractor() -> ImageExtractor:
    config = ExtractionConfig(ocr_backend="tesseract")
    return ImageExtractor(mime_type="image/png", config=config)


@pytest.fixture
def mock_ocr_backend() -> Generator[MagicMock, None, None]:
    with patch("kreuzberg._extractors._image.get_ocr_backend") as mock:
        backend = MagicMock()

        backend.process_file = AsyncMock()
        mock.return_value = backend
        yield backend


@pytest.mark.anyio
async def test_extract_path_async_no_ocr_backend() -> None:
    config = ExtractionConfig(ocr_backend=None)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError) as excinfo:
        await extractor.extract_path_async(Path("dummy_path"))

    assert "ocr_backend is None" in str(excinfo.value)


@pytest.mark.anyio
async def test_extract_path_async(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(b"dummy image content")

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})
    mock_ocr_backend.process_file.return_value = expected_result

    result = await extractor.extract_path_async(image_path)

    mock_ocr_backend.process_file.assert_called_once()
    assert result == expected_result


def test_extract_path_sync(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(b"dummy image content")

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})
    mock_ocr_backend.process_file.return_value = expected_result

    with patch("kreuzberg._extractors._image.anyio.run") as mock_run:
        mock_run.return_value = expected_result
        result = extractor.extract_path_sync(image_path)

        mock_run.assert_called_once()
        assert result == expected_result


def test_extract_bytes_sync(mock_ocr_backend: MagicMock) -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})

    with patch("kreuzberg._extractors._image.anyio.run") as mock_run:
        mock_run.return_value = expected_result
        result = extractor.extract_bytes_sync(b"dummy image content")

        mock_run.assert_called_once()
        assert result == expected_result


@pytest.mark.parametrize(
    "mime_type,expected_extension",
    [
        ("image/png", "png"),
        ("image/jpeg", "jpg"),
        ("image/gif", "gif"),
        ("image/bmp", "bmp"),
        ("image/tiff", "tiff"),
        ("image/webp", "webp"),
    ],
)
def test_get_extension_from_mime_type(mime_type: str, expected_extension: str) -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type=mime_type, config=config)

    extension = extractor._get_extension_from_mime_type(mime_type)
    assert extension == expected_extension


def test_get_extension_from_partial_mime_type() -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type="image/jpeg", config=config)

    extension = extractor._get_extension_from_mime_type("image")
    assert extension == "bmp"


def test_get_extension_from_unsupported_mime_type() -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError) as excinfo:
        extractor._get_extension_from_mime_type("application/unsupported")

    assert "unsupported mimetype" in str(excinfo.value)
    assert "application/unsupported" in str(excinfo.value)


@pytest.mark.anyio
async def test_extract_bytes_async(mock_ocr_backend: MagicMock) -> None:
    config = ExtractionConfig(ocr_backend="tesseract")
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})
    mock_ocr_backend.process_file.return_value = expected_result

    mock_path = MagicMock()
    mock_unlink = AsyncMock()

    with patch("kreuzberg._extractors._image.create_temp_file") as mock_create_temp:
        mock_create_temp.return_value = (mock_path, mock_unlink)

        with patch("kreuzberg._extractors._image.AsyncPath") as mock_async_path:
            mock_async_path_instance = MagicMock()
            mock_async_path_instance.write_bytes = AsyncMock()
            mock_async_path.return_value = mock_async_path_instance

            result = await extractor.extract_bytes_async(b"dummy image content")

            mock_create_temp.assert_called_once_with(".png")

            mock_async_path_instance.write_bytes.assert_called_once_with(b"dummy image content")

            mock_ocr_backend.process_file.assert_called_once_with(mock_path, **config.get_config_dict())

            mock_unlink.assert_called_once()

            assert result == expected_result
