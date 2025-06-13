"""Integration tests for OCR backends with GPU support."""

from unittest.mock import MagicMock, patch

import pytest
import torch
from PIL import Image

from kreuzberg._ocr._easyocr import EasyOCRBackend, EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRBackend, PaddleOCRConfig
from kreuzberg._utils._device import validate_device
from kreuzberg.exceptions import DeviceNotAvailableError


@pytest.fixture
def sample_image() -> Image.Image:
    return Image.new("RGB", (100, 100), color="white")


@pytest.mark.skipif(not torch.cuda.is_available(), reason="CUDA not available")
@pytest.mark.anyio
async def test_easyocr_gpu(sample_image: Image.Image) -> None:
    config = EasyOCRConfig(use_gpu=True, device="cuda", gpu_memory_limit=4.0)

    backend = EasyOCRBackend(config)

    mock_reader = MagicMock()
    mock_reader.readtext.return_value = [("text", 0.9)]

    with patch("easyocr.Reader", return_value=mock_reader):
        result = await backend.process_image(sample_image)
        assert result.content == "text"

        # Verify GPU settings were passed correctly
        mock_reader.readtext.assert_called_once()
        assert config.use_gpu
        assert config.device == "cuda"


@pytest.mark.skipif(not torch.cuda.is_available(), reason="CUDA not available")
@pytest.mark.anyio
async def test_paddleocr_gpu(sample_image: Image.Image) -> None:
    config = PaddleOCRConfig(use_gpu=True, device="cuda", gpu_memory_limit=4.0)

    backend = PaddleOCRBackend(config)

    mock_ocr = MagicMock()
    mock_ocr.ocr.return_value = [[[None, ["text", 0.9]]]]

    with patch("paddleocr.PaddleOCR", return_value=mock_ocr):
        result = await backend.process_image(sample_image)
        assert result.content == "text"

        mock_ocr.ocr.assert_called_once()
        assert config.use_gpu
        assert config.device == "cuda"


@pytest.mark.anyio
async def test_gpu_fallback(sample_image: Image.Image) -> None:
    config = EasyOCRConfig(use_gpu=True, device="auto")

    with (
        patch("torch.cuda.is_available", return_value=False),
        patch("torch.backends.mps.is_available", return_value=False),
        patch("easyocr.Reader", return_value=MagicMock()),
        patch("kreuzberg._ocr._easyocr.run_sync", side_effect=lambda func, *args, **kwargs: func(*args, **kwargs)),
    ):
        backend = EasyOCRBackend(config)
        mock_reader = MagicMock()
        mock_reader.readtext.return_value = [("text", 0.9)]

        await backend._init_easyocr()
        backend._reader = mock_reader  # type: ignore[assignment]
        result = await backend.process_image(sample_image, beam_width=5)
        assert result.content == "text"

        mock_reader.readtext.assert_called_once()
        assert backend._device == "cpu"


def test_invalid_gpu_device() -> None:
    with pytest.raises(DeviceNotAvailableError):
        # Test validation of invalid device directly
        validate_device("invalid", "easyocr")


def test_gpu_memory_limit() -> None:
    config = EasyOCRConfig(use_gpu=True, device="cuda", gpu_memory_limit=4.0)

    with (
        patch("torch.cuda.is_available", return_value=True),
        patch("torch.cuda.get_device_properties") as mock_props,
        patch("torch.cuda.set_per_process_memory_fraction") as mock_set,
    ):
        mock_props.return_value.total_memory = 8 * 1024**3  # 8GB
        EasyOCRBackend(config)
        mock_set.assert_called_once()
        assert mock_set.call_args[0][0] == 0.5
