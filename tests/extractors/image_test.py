from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from kreuzberg._extractors._image import ImageExtractor
from kreuzberg._types import (
    EasyOCRConfig,
    ExtractionConfig,
    ExtractionResult,
    PaddleOCRConfig,
    TesseractConfig,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from collections.abc import Generator

VALID_PNG_BYTES = (
    b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01"
    b"\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xcf\xc0"
    b"\x00\x00\x03\x01\x01\x00\xc9\xfe\x92\xef\x00\x00\x00\x00IEND\xaeB`\x82"
)


@pytest.fixture(scope="session")
def extractor() -> ImageExtractor:
    config = ExtractionConfig(ocr=TesseractConfig())
    return ImageExtractor(mime_type="image/png", config=config)


@pytest.fixture
def mock_ocr_backend() -> Generator[MagicMock, None, None]:
    backend = MagicMock()
    backend.process_file = AsyncMock()
    backend.process_image_sync = MagicMock()
    backend.process_image_sync = MagicMock()
    backend.process_batch_sync = MagicMock()
    backend.process_image = AsyncMock()

    with patch("kreuzberg._extractors._image.get_ocr_backend", return_value=backend):
        yield backend


@pytest.mark.anyio
async def test_extract_path_async_no_ocr_backend() -> None:
    config = ExtractionConfig(ocr=None)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError) as excinfo:
        await extractor.extract_path_async(Path("dummy_path"))

    assert "OCR is not configured" in str(excinfo.value)


@pytest.mark.anyio
async def test_extract_path_async(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(
        content="extracted text", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )
    mock_ocr_backend.process_image.return_value = expected_result

    result = await extractor.extract_path_async(image_path)

    mock_ocr_backend.process_image.assert_called_once()
    assert result == expected_result


def test_extract_path_sync(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(
        content="extracted text", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )
    mock_ocr_backend.process_image_sync.return_value = expected_result

    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result == expected_result


def test_extract_bytes_sync(mock_ocr_backend: MagicMock) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(
        content="extracted text", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )

    with patch.object(extractor, "extract_path_sync") as mock_extract_path:
        mock_extract_path.return_value = expected_result
        result = extractor.extract_bytes_sync(
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01"
            b"\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xcf\xc0"
            b"\x00\x00\x00\x03\x00\x01\x8e\xb4`\xd1\x00\x00\x00\x00IEND\xaeB`\x82"
        )

        mock_extract_path.assert_called_once()
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
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type=mime_type, config=config)

    extension = extractor._get_extension_from_mime_type(mime_type)
    assert extension == expected_extension


def test_get_extension_from_partial_mime_type() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/jpeg", config=config)

    extension = extractor._get_extension_from_mime_type("image")
    assert extension == "bmp"


def test_get_extension_from_unsupported_mime_type() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError) as excinfo:
        extractor._get_extension_from_mime_type("application/unsupported")

    assert "unsupported mimetype" in str(excinfo.value)
    assert "application/unsupported" in str(excinfo.value)


@pytest.mark.anyio
async def test_extract_bytes_async(mock_ocr_backend: MagicMock) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    png_bytes = VALID_PNG_BYTES

    expected_result = ExtractionResult(
        content="extracted text from bytes", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )
    mock_ocr_backend.process_image.return_value = expected_result

    result = await extractor.extract_bytes_async(png_bytes)

    mock_ocr_backend.process_image.assert_called_once()
    assert result.content == "extracted text from bytes"
    assert result.metadata["quality_score"] == 1.0


def test_ocr_backend_none_validation_error() -> None:
    config = ExtractionConfig(ocr=None)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError, match="OCR is not configured"):
        extractor.extract_path_sync(Path("dummy.png"))


@pytest.mark.anyio
async def test_ocr_backend_none_validation_error_async() -> None:
    config = ExtractionConfig(ocr=None)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError, match="OCR is not configured"):
        await extractor.extract_path_async(Path("dummy.png"))


def test_extract_bytes_temp_file_cleanup_on_error() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with patch("tempfile.mkstemp") as mock_mkstemp:
        mock_fd = 42
        mock_temp_path = "/tmp/test_image.png"
        mock_mkstemp.return_value = (mock_fd, mock_temp_path)

        with patch("os.fdopen") as mock_fdopen:
            mock_file = MagicMock()
            mock_fdopen.return_value.__enter__.return_value = mock_file

            with patch.object(extractor, "extract_path_sync") as mock_extract:
                mock_extract.side_effect = Exception("Processing failed")

                with patch("pathlib.Path.unlink") as mock_unlink:
                    with pytest.raises(Exception, match="Processing failed"):
                        extractor.extract_bytes_sync(b"image data")

                    mock_unlink.assert_called_once()


def test_extract_path_sync_no_ocr_backend() -> None:
    config = ExtractionConfig(ocr=None)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError) as excinfo:
        extractor.extract_path_sync(Path("dummy_path"))

    assert "OCR is not configured" in str(excinfo.value)


def test_extract_bytes_with_different_mime_types() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())

    mime_types = ["image/png", "image/jpeg", "image/tiff", "image/webp"]

    for mime_type in mime_types:
        extractor = ImageExtractor(mime_type=mime_type, config=config)
        assert extractor.mime_type == mime_type
        assert mime_type in extractor.SUPPORTED_MIME_TYPES


def test_extract_bytes_sync_with_ocr_config() -> None:
    from kreuzberg._types import PSMMode

    tesseract_config = TesseractConfig(
        language="fra",
        psm=PSMMode.SINGLE_BLOCK,
    )
    config = ExtractionConfig(ocr=tesseract_config)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with patch("tempfile.mkstemp") as mock_mkstemp:
        mock_fd = 42
        mock_temp_path = "/tmp/test_image.png"
        mock_mkstemp.return_value = (mock_fd, mock_temp_path)

        with patch("os.fdopen") as mock_fdopen:
            mock_file = MagicMock()
            mock_fdopen.return_value.__enter__.return_value = mock_file

            with patch.object(extractor, "extract_path_sync") as mock_extract:
                expected_result = ExtractionResult(
                    content="extracted French text",
                    mime_type="text/plain",
                    metadata={},
                )
                mock_extract.return_value = expected_result

                result = extractor.extract_bytes_sync(b"fake image data")

                assert result.content == "extracted French text"
                mock_file.write.assert_called_once_with(b"fake image data")


def test_extract_bytes_sync_temp_file_creation() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with patch("tempfile.mkstemp") as mock_mkstemp:
        mock_fd = 42
        mock_temp_path = "/tmp/test_image.png"
        mock_mkstemp.return_value = (mock_fd, mock_temp_path)

        with patch("os.fdopen") as mock_fdopen:
            mock_file = MagicMock()
            mock_fdopen.return_value.__enter__.return_value = mock_file

            with patch.object(extractor, "extract_path_sync") as mock_extract:
                expected_result = ExtractionResult(
                    content="extracted text",
                    mime_type="text/plain",
                    metadata={},
                )
                mock_extract.return_value = expected_result

                with patch("pathlib.Path.unlink") as mock_unlink:
                    result = extractor.extract_bytes_sync(b"fake image data")

                    assert result.content == "extracted text"
                    mock_mkstemp.assert_called_once_with(suffix=".png")
                    mock_file.write.assert_called_once_with(b"fake image data")
                    mock_unlink.assert_called_once()


@pytest.mark.anyio
async def test_extract_bytes_async_delegation() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with patch.object(extractor, "extract_path_async") as mock_async:
        expected_result = ExtractionResult(
            content="async extracted text",
            mime_type="text/plain",
            metadata={},
        )
        mock_async.return_value = expected_result

        result = await extractor.extract_bytes_async(b"fake image data")

        assert result.content == "async extracted text"
        mock_async.assert_called_once()
        assert isinstance(mock_async.call_args[0][0], Path)


@pytest.mark.anyio
async def test_extract_path_async_delegation(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    test_path = tmp_path / "test_image.png"
    test_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(
        content="async path extracted text",
        mime_type="text/plain",
        metadata={},
    )
    mock_ocr_backend.process_image.return_value = expected_result

    result = await extractor.extract_path_async(test_path)

    assert result.content == "async path extracted text"
    mock_ocr_backend.process_image.assert_called_once()


def test_extract_path_sync_with_tesseract_config(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    tesseract_config = TesseractConfig()
    config = ExtractionConfig(ocr=tesseract_config)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(
        content="extracted text", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )
    mock_ocr_backend.process_image_sync.return_value = expected_result

    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result == expected_result


def test_extract_path_sync_with_paddleocr_config(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    paddle_config = PaddleOCRConfig()
    config = ExtractionConfig(ocr=paddle_config)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(
        content="extracted text", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )
    mock_ocr_backend.process_image_sync.return_value = expected_result

    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result == expected_result


def test_extract_path_sync_with_easyocr_config(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    easy_config = EasyOCRConfig()
    config = ExtractionConfig(ocr=easy_config)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(
        content="extracted text", chunks=[], mime_type="text/plain", metadata={"quality_score": 1.0}
    )
    mock_ocr_backend.process_image_sync.return_value = expected_result

    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result == expected_result


@pytest.mark.anyio
async def test_extract_real_image_integration(test_files_path: Path) -> None:
    test_image_path = test_files_path / "images" / "test_hello_world.png"
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    result = await extractor.extract_path_async(test_image_path)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0


def test_extract_real_image_sync_integration(test_files_path: Path) -> None:
    test_image_path = test_files_path / "images" / "test_hello_world.png"
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    result = extractor.extract_path_sync(test_image_path)

    assert isinstance(result, ExtractionResult)
    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0


@pytest.mark.parametrize(
    "mime_type,expected_extension",
    [
        ("image/bmp", "bmp"),
        ("image/x-bmp", "bmp"),
        ("image/x-ms-bmp", "bmp"),
        ("image/gif", "gif"),
        ("image/jpeg", "jpg"),
        ("image/pjpeg", "jpg"),
        ("image/png", "png"),
        ("image/tiff", "tiff"),
        ("image/x-tiff", "tiff"),
        ("image/jp2", "jp2"),
        ("image/jpx", "jpx"),
        ("image/jpm", "jpm"),
        ("image/mj2", "mj2"),
        ("image/webp", "webp"),
        ("image/x-portable-anymap", "pnm"),
        ("image/x-portable-bitmap", "pbm"),
        ("image/x-portable-graymap", "pgm"),
        ("image/x-portable-pixmap", "ppm"),
    ],
)
def test_image_mime_types_all_mappings(mime_type: str, expected_extension: str) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type=mime_type, config=config)

    extension = extractor._get_extension_from_mime_type(mime_type)
    assert extension == expected_extension


def test_image_mime_types_partial_matching() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    extension = extractor._get_extension_from_mime_type("image")
    assert extension == "bmp"

    extension = extractor._get_extension_from_mime_type("image/x")
    assert extension in ["bmp", "tiff", "pnm", "pbm", "pgm", "ppm"]

    extension = extractor._get_extension_from_mime_type("image/x-portable")
    assert extension in ["pnm", "pbm", "pgm", "ppm"]


def test_image_mime_types_case_sensitivity() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError, match="unsupported mimetype"):
        extractor._get_extension_from_mime_type("IMAGE/PNG")

    with pytest.raises(ValidationError, match="unsupported mimetype"):
        extractor._get_extension_from_mime_type("Image/Png")


def test_image_sync_path_extraction_unknown_backend(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    mock_ocr_backend.process_image_sync.side_effect = NotImplementedError(
        "Sync OCR not implemented for unknown_backend"
    )

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)
    with pytest.raises(NotImplementedError, match="Sync OCR not implemented for unknown_backend"):
        extractor.extract_path_sync(image_path)


def test_image_sync_path_extraction_default_tesseract(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})
    mock_ocr_backend.process_image_sync.return_value = expected_result

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)
    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result.content == expected_result.content
    assert result.mime_type == expected_result.mime_type


def test_image_sync_path_extraction_default_paddleocr(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=PaddleOCRConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})
    mock_ocr_backend.process_image_sync.return_value = expected_result

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)
    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result.content == expected_result.content
    assert result.mime_type == expected_result.mime_type


def test_image_sync_path_extraction_default_easyocr(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=EasyOCRConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(content="extracted text", chunks=[], mime_type="text/plain", metadata={})
    mock_ocr_backend.process_image_sync.return_value = expected_result

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)
    result = extractor.extract_path_sync(image_path)

    mock_ocr_backend.process_image_sync.assert_called_once()
    assert result.content == expected_result.content
    assert result.mime_type == expected_result.mime_type


def test_image_sync_path_extraction_custom_configs(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    from kreuzberg._types import PSMMode

    tesseract_config = TesseractConfig(
        language="deu+fra",
        psm=PSMMode.SINGLE_COLUMN,
        tessedit_char_whitelist="0123456789",
        tessedit_enable_dict_correction=False,
        language_model_ngram_on=True,
    )
    config = ExtractionConfig(ocr=tesseract_config)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    expected_result = ExtractionResult(content="German text", mime_type="text/plain", metadata={})
    mock_ocr_backend.process_image_sync.return_value = expected_result

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)
    result = extractor.extract_path_sync(image_path)
    assert result.content == "German text"

    call_args = mock_ocr_backend.process_image_sync.call_args[1]
    assert call_args["language"] == "deu+fra"
    assert call_args["psm"] == PSMMode.SINGLE_COLUMN.value
    assert call_args["tessedit_char_whitelist"] == "0123456789"
    assert call_args["tessedit_enable_dict_correction"] is False
    assert call_args["language_model_ngram_on"] is True


def test_image_temp_file_handling_fd_close_error() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with patch("tempfile.mkstemp") as mock_mkstemp:
        mock_fd = 42
        mock_temp_path = "/tmp/test_image.png"
        mock_mkstemp.return_value = (mock_fd, mock_temp_path)

        with patch("os.fdopen") as mock_fdopen:
            mock_fdopen.side_effect = OSError("Cannot open file descriptor")

            with patch("pathlib.Path.unlink") as mock_unlink:
                with pytest.raises(OSError, match="Cannot open file descriptor"):
                    extractor.extract_bytes_sync(b"image data")

                mock_unlink.assert_called_once()


def test_image_temp_file_handling_unlink_error_suppressed() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with patch("tempfile.mkstemp") as mock_mkstemp:
        mock_fd = 42
        mock_temp_path = "/tmp/test_image.png"
        mock_mkstemp.return_value = (mock_fd, mock_temp_path)

        with patch("os.fdopen") as mock_fdopen:
            mock_file = MagicMock()
            mock_fdopen.return_value.__enter__.return_value = mock_file

            with patch.object(extractor, "extract_path_sync") as mock_extract:
                expected_result = ExtractionResult(content="text", mime_type="text/plain", metadata={})
                mock_extract.return_value = expected_result

                with patch("pathlib.Path.unlink") as mock_unlink:
                    mock_unlink.side_effect = OSError("Cannot unlink")

                    result = extractor.extract_bytes_sync(b"image data")
                    assert result == expected_result


@pytest.mark.anyio
async def test_image_temp_file_handling_async_cleanup() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    mock_path = Path("/tmp/test.png")
    mock_unlink = AsyncMock()

    with patch("kreuzberg._extractors._image.create_temp_file") as mock_create_temp:
        mock_create_temp.return_value = (mock_path, mock_unlink)

        # Mock AsyncPath.write_bytes for extract_bytes_async cleanup test - legitimately needed to test temp file cleanup ~keep
        with patch("kreuzberg._extractors._image.AsyncPath") as mock_async_path:
            mock_async_path_instance = MagicMock()
            mock_async_path_instance.write_bytes = AsyncMock()
            mock_async_path.return_value = mock_async_path_instance

            with patch.object(extractor, "extract_path_async") as mock_extract:
                mock_extract.side_effect = Exception("OCR failed")

                with pytest.raises(Exception, match="OCR failed"):
                    await extractor.extract_bytes_async(b"image data")

                mock_unlink.assert_called_once()


def test_image_edge_cases_supported_mime_types_constant() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    for mime_type in extractor.IMAGE_MIME_TYPE_EXT_MAP:
        assert mime_type in extractor.SUPPORTED_MIME_TYPES


def test_image_edge_cases_extract_bytes_all_mime_types() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())

    for mime_type in ImageExtractor.IMAGE_MIME_TYPE_EXT_MAP:
        extractor = ImageExtractor(mime_type=mime_type, config=config)

        with patch.object(extractor, "extract_path_sync") as mock_extract:
            expected_result = ExtractionResult(
                content=f"Extracted from {mime_type}", mime_type="text/plain", metadata={}
            )
            mock_extract.return_value = expected_result

            result = extractor.extract_bytes_sync(b"fake image data")
            assert result.content == f"Extracted from {mime_type}"


def test_image_edge_cases_mime_type_validation_context() -> None:
    config = ExtractionConfig(ocr=TesseractConfig())
    extractor = ImageExtractor(mime_type="image/png", config=config)

    with pytest.raises(ValidationError) as exc_info:
        extractor._get_extension_from_mime_type("video/mp4")

    assert "unsupported mimetype" in str(exc_info.value)
    assert exc_info.value.context == {"mime_type": "video/mp4"}


def test_image_edge_cases_quality_processing_applied(mock_ocr_backend: MagicMock, tmp_path: Path) -> None:
    config = ExtractionConfig(ocr=TesseractConfig(), enable_quality_processing=True)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    raw_result = ExtractionResult(content="Low quality text with ████ artifacts", mime_type="text/plain", metadata={})
    mock_ocr_backend.process_image_sync.return_value = raw_result

    image_path = tmp_path / "test.png"
    image_path.write_bytes(VALID_PNG_BYTES)
    result = extractor.extract_path_sync(image_path)

    assert result != raw_result


@pytest.mark.anyio
async def test_image_edge_cases_async_path_delegation_preserves_config(
    mock_ocr_backend: MagicMock, tmp_path: Path
) -> None:
    from kreuzberg._types import PSMMode

    tesseract_config = TesseractConfig(language="jpn", psm=PSMMode.SINGLE_WORD, textord_space_size_is_variable=True)
    config = ExtractionConfig(ocr=tesseract_config, enable_quality_processing=True)
    extractor = ImageExtractor(mime_type="image/png", config=config)

    image_path = tmp_path / "japanese.png"
    image_path.write_bytes(VALID_PNG_BYTES)

    expected_result = ExtractionResult(content="日本語", mime_type="text/plain", metadata={})
    mock_ocr_backend.process_image.return_value = expected_result

    await extractor.extract_path_async(image_path)

    mock_ocr_backend.process_image.assert_called_once()
    call_kwargs = mock_ocr_backend.process_image.call_args[1]
    assert "language" in call_kwargs
    assert call_kwargs["language"] == "jpn"
    assert "psm" in call_kwargs
    assert call_kwargs["psm"] == PSMMode.SINGLE_WORD.value
    assert "textord_space_size_is_variable" in call_kwargs
    assert call_kwargs["textord_space_size_is_variable"] is True
