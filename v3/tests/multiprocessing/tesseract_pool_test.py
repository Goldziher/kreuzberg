from __future__ import annotations

import io
from typing import TYPE_CHECKING, Any
from unittest.mock import patch

import pytest
from kreuzberg._ocr._tesseract import (
    TesseractProcessPool,
    _process_image_bytes_with_tesseract,
    _process_image_with_tesseract,
)
from kreuzberg._types import TesseractConfig
from PIL import Image

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def tesseract_config() -> dict[str, Any]:
    return {
        "language": "eng",
        "psm": 3,
        "oem": 3,
        "dpi": 300,
        "tesseract_cmd": "tesseract",
    }


@pytest.fixture
def test_image_path(tmp_path: Path) -> Path:
    img_path = tmp_path / "test_image.png"
    img = Image.new("RGB", (100, 100), color="white")
    img.save(img_path)
    return img_path


class _MockSubprocessResult:
    def __init__(self, returncode: int, stdout: str = "", stderr: str = ""):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr


def test_process_image_with_tesseract_success(test_image_path: Path, tesseract_config: dict[str, Any]) -> None:
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = "Test OCR output"

            result = _process_image_with_tesseract(str(test_image_path), tesseract_config)

            assert result["success"] is True
            assert result["text"] == "Test OCR output"
            assert result["confidence"] is None
            assert result["error"] is None


def test_process_image_with_tesseract_error(test_image_path: Path, tesseract_config: dict[str, Any]) -> None:
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=1, stderr="Tesseract error")

        result = _process_image_with_tesseract(str(test_image_path), tesseract_config)

        assert result["success"] is False
        assert result["text"] == ""
        assert "Tesseract failed" in result["error"]


def test_process_image_with_tesseract_exception(test_image_path: Path, tesseract_config: dict[str, Any]) -> None:
    with patch("subprocess.run", side_effect=Exception("Unexpected error")):
        result = _process_image_with_tesseract(str(test_image_path), tesseract_config)

        assert result["success"] is False
        assert result["text"] == ""
        assert "Unexpected error" in result["error"]


def test_process_image_with_tesseract_custom_params(test_image_path: Path) -> None:
    config = {
        "language": "fra",
        "psm": 6,
        "oem": 1,
        "dpi": 600,
        "tesseract_cmd": "/usr/local/bin/tesseract",
        "config_string": "--tessdata-dir /custom/path",
    }

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = _MockSubprocessResult(returncode=0)

        with patch("pathlib.Path.open") as mock_open:
            mock_file = mock_open.return_value.__enter__.return_value
            mock_file.read.return_value = "French text"

            _process_image_with_tesseract(str(test_image_path), config)

            args = mock_run.call_args[0][0]
            assert args[0] == "tesseract"
            assert "-l" in args
            assert "fra" in args
            assert "--psm" in args
            assert "6" in args
            assert "--oem" in args
            assert "1" in args


def test_process_image_bytes_with_tesseract(tesseract_config: dict[str, Any]) -> None:
    img = Image.new("RGB", (100, 100), color="white")
    img_bytes = io.BytesIO()
    img.save(img_bytes, format="PNG")
    image_bytes = img_bytes.getvalue()

    with patch("kreuzberg._ocr._tesseract._process_image_with_tesseract") as mock_process:
        mock_process.return_value = {
            "success": True,
            "text": "Bytes OCR output",
            "confidence": None,
            "error": None,
        }

        result = _process_image_bytes_with_tesseract(image_bytes, tesseract_config)

        assert result["success"] is True
        assert result["text"] == "Bytes OCR output"
        mock_process.assert_called_once()


def test_tesseract_process_pool_init_default() -> None:
    pool = TesseractProcessPool()
    assert pool.config is not None
    assert pool.process_manager is not None

    assert isinstance(pool.config, TesseractConfig)


def test_tesseract_process_pool_init_custom_processes() -> None:
    pool = TesseractProcessPool(max_processes=4)
    assert pool.process_manager.max_processes == 4


def test_tesseract_process_pool_init_custom_config() -> None:
    config = TesseractConfig(language="fra", psm=6)  # type: ignore[arg-type]
    pool = TesseractProcessPool(config=config)
    assert pool.config == config


def test_tesseract_process_pool_config_to_dict() -> None:
    config = TesseractConfig(language="fra", psm=6)  # type: ignore[arg-type]
    pool = TesseractProcessPool(config=config)

    config_dict = pool._config_to_dict()

    assert config_dict["language"] == "fra"
    assert config_dict["psm"] == 6

    override_config = TesseractConfig(language="eng", psm=3)  # type: ignore[arg-type]
    override_dict = pool._config_to_dict(override_config)
    assert override_dict["language"] == "eng"
    assert override_dict["psm"] == 3


@pytest.mark.anyio
async def test_tesseract_process_pool_process_batch_images_empty_list() -> None:
    pool = TesseractProcessPool(max_processes=2)

    results = await pool.process_batch_images([])

    assert results == []


def test_tesseract_process_pool_result_from_dict_error() -> None:
    pool = TesseractProcessPool()

    error_result = {"success": False, "text": "", "confidence": None, "error": "Tesseract failed"}

    from kreuzberg.exceptions import OCRError

    with pytest.raises(OCRError, match="Tesseract processing failed: Tesseract failed"):
        pool._result_from_dict(error_result)


@pytest.mark.anyio
async def test_tesseract_process_pool_process_image_async(test_image_path: Path) -> None:
    pool = TesseractProcessPool(max_processes=2)

    mock_result = {
        "success": True,
        "text": "Async OCR result",
        "confidence": None,
        "error": None,
    }

    with patch.object(pool.process_manager, "submit_task", return_value=mock_result):
        result = await pool.process_image(test_image_path)

        assert result.content == "Async OCR result"
        assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_tesseract_process_pool_process_image_error(test_image_path: Path) -> None:
    pool = TesseractProcessPool(max_processes=2)

    mock_result = {
        "success": False,
        "text": "",
        "confidence": None,
        "error": "OCR failed",
    }

    with patch.object(pool.process_manager, "submit_task", return_value=mock_result):
        with pytest.raises(Exception, match="OCR failed") as exc_info:
            await pool.process_image(test_image_path)

        assert "OCR failed" in str(exc_info.value)


@pytest.mark.anyio
async def test_tesseract_process_pool_process_image_bytes_async() -> None:
    pool = TesseractProcessPool(max_processes=2)

    img = Image.new("RGB", (100, 100), color="white")
    img_bytes_io = io.BytesIO()
    img.save(img_bytes_io, format="PNG")
    image_bytes = img_bytes_io.getvalue()

    mock_result = {
        "success": True,
        "text": "Bytes OCR result",
        "confidence": None,
        "error": None,
    }

    with patch.object(pool.process_manager, "submit_task", return_value=mock_result):
        result = await pool.process_image_bytes(image_bytes)

        assert result.content == "Bytes OCR result"
        assert result.mime_type == "text/plain"


@pytest.mark.anyio
async def test_tesseract_process_pool_process_batch_images(tmp_path: Path) -> None:
    images = []
    for i in range(3):
        img_path = tmp_path / f"test_{i}.png"
        img = Image.new("RGB", (50, 50), color="white")
        img.save(img_path)
        images.append(img_path)

    pool = TesseractProcessPool(max_processes=2)

    mock_results = [
        {
            "success": True,
            "text": f"Image {i} text",
            "confidence": None,
            "error": None,
        }
        for i in range(3)
    ]

    with patch.object(pool.process_manager, "submit_batch", return_value=mock_results):
        results = await pool.process_batch_images(images)  # type: ignore[arg-type]

        assert len(results) == 3
        for i, result in enumerate(results):
            assert result.content == f"Image {i} text"


@pytest.mark.anyio
async def test_tesseract_process_pool_process_batch_bytes() -> None:
    image_bytes_list = []
    for _ in range(3):
        img = Image.new("RGB", (50, 50), color="white")
        img_bytes = io.BytesIO()
        img.save(img_bytes, format="PNG")
        image_bytes_list.append(img_bytes.getvalue())

    pool = TesseractProcessPool(max_processes=2)

    mock_results = [
        {
            "success": True,
            "text": f"Bytes {i} text",
            "confidence": None,
            "error": None,
        }
        for i in range(3)
    ]

    with patch.object(pool.process_manager, "submit_batch", return_value=mock_results):
        results = await pool.process_batch_bytes(image_bytes_list)

        assert len(results) == 3
        for i, result in enumerate(results):
            assert result.content == f"Bytes {i} text"


def test_tesseract_process_pool_shutdown() -> None:
    pool = TesseractProcessPool(max_processes=2)

    with patch.object(pool.process_manager, "shutdown") as mock_shutdown:
        pool.shutdown()
        mock_shutdown.assert_called_once_with(wait=True)


def test_tesseract_process_pool_get_system_info() -> None:
    pool = TesseractProcessPool()
    mock_info = {"cpu_count": 4, "memory_total": 8000}

    with patch.object(pool.process_manager, "get_system_info", return_value=mock_info):
        info = pool.get_system_info()
        assert info == mock_info


@pytest.mark.anyio
async def test_tesseract_process_pool_async_context_manager() -> None:
    pool = TesseractProcessPool(max_processes=2)

    async with pool as p:
        assert p is pool

    with patch.object(pool, "shutdown") as mock_shutdown:
        async with pool:
            pass
        mock_shutdown.assert_called_once()
