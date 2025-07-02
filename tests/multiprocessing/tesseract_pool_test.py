"""Tests for Tesseract process pool."""

from __future__ import annotations

import io
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

import pytest
from PIL import Image

from kreuzberg._multiprocessing.tesseract_pool import (
    TesseractProcessPool,
    _process_image_bytes_with_tesseract,
    _process_image_with_tesseract,
)
from kreuzberg._ocr._tesseract import TesseractConfig

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def tesseract_config() -> dict[str, Any]:
    """Create a test Tesseract configuration."""
    return {
        "language": "eng",
        "psm": 3,
        "oem": 3,
        "dpi": 300,
        "tesseract_cmd": "tesseract",
    }


@pytest.fixture
def test_image_path(tmp_path: Path) -> Path:
    """Create a test image file."""
    img_path = tmp_path / "test_image.png"
    img = Image.new("RGB", (100, 100), color="white")
    img.save(img_path)
    return img_path


def test_process_image_with_tesseract_success(test_image_path: Path, tesseract_config: dict[str, Any]) -> None:
    """Test successful image processing with tesseract."""
    with patch("subprocess.run") as mock_run:
        # Mock successful tesseract execution
        mock_run.return_value = Mock(returncode=0)

        # Mock file read
        with patch("pathlib.Path.read_text", return_value="Test OCR output"):
            result = _process_image_with_tesseract(str(test_image_path), tesseract_config)

            assert result["success"] is True
            assert result["text"] == "Test OCR output"
            assert result["confidence"] is None
            assert result["error"] is None


def test_process_image_with_tesseract_error(test_image_path: Path, tesseract_config: dict[str, Any]) -> None:
    """Test image processing with tesseract error."""
    with patch("subprocess.run") as mock_run:
        # Mock tesseract failure
        mock_run.return_value = Mock(returncode=1, stderr="Tesseract error")

        result = _process_image_with_tesseract(str(test_image_path), tesseract_config)

        assert result["success"] is False
        assert result["text"] == ""
        assert "Tesseract failed" in result["error"]


def test_process_image_with_tesseract_exception(test_image_path: Path, tesseract_config: dict[str, Any]) -> None:
    """Test image processing with unexpected exception."""
    with patch("subprocess.run", side_effect=Exception("Unexpected error")):
        result = _process_image_with_tesseract(str(test_image_path), tesseract_config)

        assert result["success"] is False
        assert result["text"] == ""
        assert "Unexpected error" in result["error"]


def test_process_image_with_tesseract_custom_params(test_image_path: Path) -> None:
    """Test image processing with custom tesseract parameters."""
    config = {
        "language": "fra",
        "psm": 6,
        "oem": 1,
        "dpi": 600,
        "tesseract_cmd": "/usr/local/bin/tesseract",
        "config_string": "--tessdata-dir /custom/path",
    }

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = Mock(returncode=0)

        with patch("pathlib.Path.read_text", return_value="French text"):
            _process_image_with_tesseract(str(test_image_path), config)

            # Verify command was built correctly
            args = mock_run.call_args[0][0]
            assert args[0] == "tesseract"  # Default tesseract command
            assert "-l" in args
            assert "fra" in args
            assert "--psm" in args
            assert "6" in args
            assert "--oem" in args
            assert "1" in args


def test_process_image_bytes_with_tesseract(tesseract_config: dict[str, Any]) -> None:
    """Test image bytes processing."""
    # Create test image bytes
    img = Image.new("RGB", (100, 100), color="white")
    img_bytes = io.BytesIO()
    img.save(img_bytes, format="PNG")
    image_bytes = img_bytes.getvalue()

    with patch("kreuzberg._multiprocessing.tesseract_pool._process_image_with_tesseract") as mock_process:
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


class TestTesseractProcessPool:
    """Tests for TesseractProcessPool class."""

    def test_init_default(self) -> None:
        """Test TesseractPool initialization with defaults."""
        pool = TesseractProcessPool()
        assert pool.max_workers > 0
        assert pool._config == TesseractConfig()
        assert pool._pool is None

    def test_init_custom_workers(self) -> None:
        """Test TesseractPool initialization with custom workers."""
        pool = TesseractProcessPool(max_workers=4)
        assert pool.max_workers == 4

    def test_init_custom_config(self) -> None:
        """Test TesseractPool initialization with custom config."""
        config = TesseractConfig(language="fra", psm=6)
        pool = TesseractProcessPool(config=config)
        assert pool._config == config

    def test_enter_exit(self) -> None:
        """Test context manager functionality."""
        pool = TesseractProcessPool(max_workers=2)

        with (
            patch("kreuzberg._multiprocessing.process_manager.ProcessPoolManager.__enter__") as mock_enter,
            patch("kreuzberg._multiprocessing.process_manager.ProcessPoolManager.__exit__") as mock_exit,
        ):
            mock_enter.return_value = pool

            with pool as p:
                assert p is pool

            mock_enter.assert_called_once()
            mock_exit.assert_called_once()

    @pytest.mark.anyio
    async def test_process_image_async(self, test_image_path: Path) -> None:
        """Test async image processing."""
        pool = TesseractProcessPool(max_workers=2)

        # Mock the sync processing result
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
    async def test_process_image_error(self, test_image_path: Path) -> None:
        """Test async image processing with error."""
        pool = TesseractProcessPool(max_workers=2)

        # Mock error result
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
    async def test_process_image_bytes_async(self) -> None:
        """Test async image bytes processing."""
        pool = TesseractProcessPool(max_workers=2)

        # Create test image bytes
        img = Image.new("RGB", (100, 100), color="white")
        img_bytes_io = io.BytesIO()
        img.save(img_bytes_io, format="PNG")
        image_bytes = img_bytes_io.getvalue()

        # Mock the sync processing result
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
    async def test_process_batch_images(self, tmp_path: Path) -> None:
        """Test batch image processing."""
        # Create test images
        images = []
        for i in range(3):
            img_path = tmp_path / f"test_{i}.png"
            img = Image.new("RGB", (50, 50), color="white")
            img.save(img_path)
            images.append(img_path)

        pool = TesseractProcessPool(max_workers=2)

        # Mock results
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
            results = await pool.process_batch_images(images)

            assert len(results) == 3
            for i, result in enumerate(results):
                assert result.content == f"Image {i} text"

    @pytest.mark.anyio
    async def test_process_batch_bytes(self) -> None:
        """Test batch byte processing."""
        # Create test image bytes
        image_bytes_list = []
        for _ in range(3):
            img = Image.new("RGB", (50, 50), color="white")
            img_bytes = io.BytesIO()
            img.save(img_bytes, format="PNG")
            image_bytes_list.append(img_bytes.getvalue())

        pool = TesseractProcessPool(max_workers=2)

        # Mock results
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

    def test_shutdown(self) -> None:
        """Test pool shutdown."""
        pool = TesseractProcessPool(max_workers=2)

        with patch.object(pool.process_manager, "shutdown") as mock_shutdown:
            pool.shutdown()
            mock_shutdown.assert_called_once_with(wait=True)

    def test_get_system_info(self) -> None:
        """Test getting system info."""
        pool = TesseractProcessPool()
        mock_info = {"cpu_count": 4, "memory_total": 8000}

        with patch.object(pool.process_manager, "get_system_info", return_value=mock_info):
            info = pool.get_system_info()
            assert info == mock_info

    @pytest.mark.anyio
    async def test_async_context_manager(self) -> None:
        """Test async context manager functionality."""
        pool = TesseractProcessPool(max_workers=2)

        async with pool as p:
            assert p is pool

        # Test shutdown was called
        with patch.object(pool, "shutdown") as mock_shutdown:
            async with pool:
                pass
            mock_shutdown.assert_called_once()
