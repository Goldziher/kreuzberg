"""Integration tests for OCR backends with GPU support."""

import pytest
from PIL import Image
import torch
from unittest.mock import patch, MagicMock

from kreuzberg._ocr._easyocr import EasyOCRConfig, EasyOCRBackend
from kreuzberg._ocr._paddleocr import PaddleOCRConfig, PaddleOCRBackend
from kreuzberg.exceptions import DeviceNotAvailableError

@pytest.fixture
def sample_image():
    return Image.new("RGB", (100, 100), color="white")

@pytest.mark.skipif(not torch.cuda.is_available(), reason="CUDA not available")
def test_easyocr_gpu(sample_image):
    config = EasyOCRConfig(
        use_gpu=True,
        device="cuda",
        gpu_memory_limit=4.0
    )
    
    backend = EasyOCRBackend(config)
    
    mock_reader = MagicMock()
    mock_reader.readtext.return_value = [("text", 0.9)]
    
    with patch("easyocr.Reader", return_value=mock_reader):
        result = backend.process_image(sample_image)
        assert result.content == "text"
        
        # Verify GPU settings were passed correctly
        mock_reader.readtext.assert_called_once()
        assert config.use_gpu
        assert config.device == "cuda"

@pytest.mark.skipif(not torch.cuda.is_available(), reason="CUDA not available")
def test_paddleocr_gpu(sample_image):
    config = PaddleOCRConfig(
        use_gpu=True,
        device="cuda",
        gpu_memory_limit=4.0
    )
    
    backend = PaddleOCRBackend(config)
    
    mock_ocr = MagicMock()
    mock_ocr.ocr.return_value = [[[None, ["text", 0.9]]]]
    
    with patch("paddleocr.PaddleOCR", return_value=mock_ocr):
        result = backend.process_image(sample_image)
        assert result.content == "text"
        
        mock_ocr.ocr.assert_called_once()
        assert config.use_gpu
        assert config.device == "cuda"

@pytest.mark.anyio
async def test_gpu_fallback(sample_image):
    config = EasyOCRConfig(
        use_gpu=True,
        device="auto"
    )
    
    with patch("torch.cuda.is_available", return_value=False):
        with patch("torch.backends.mps.is_available", return_value=False):
            backend = EasyOCRBackend(config)
            
            mock_reader = MagicMock()
            mock_reader.readtext.return_value = [("text", 0.9)]
            
            with patch("easyocr.Reader", return_value=mock_reader):
                with patch("kreuzberg._ocr._easyocr.run_sync", side_effect=lambda func, *args, **kwargs: func(*args, **kwargs)):
                    with patch.object(backend, "_init_easyocr", return_value=None):
                        backend._reader = mock_reader
                        result = await backend.process_image(sample_image, beam_width=5)
                        assert result.content == "text"
                        
                        mock_reader.readtext.assert_called_once()
                        assert backend._device == "cpu"  # Verify device fallback

def test_invalid_gpu_device():
    config = EasyOCRConfig(
        use_gpu=True,
        device="invalid"
    )
    
    with pytest.raises(DeviceNotAvailableError):
        EasyOCRBackend(config)

def test_gpu_memory_limit():

    config = EasyOCRConfig(
        use_gpu=True,
        device="cuda",
        gpu_memory_limit=4.0
    )
    
    with patch("torch.cuda.is_available", return_value=True):
        with patch("torch.cuda.get_device_properties") as mock_props:
            mock_props.return_value.total_memory = 8 * 1024**3  # 8GB
            with patch("torch.cuda.set_per_process_memory_fraction") as mock_set:
                EasyOCRBackend(config)
                mock_set.assert_called_once()
                assert mock_set.call_args[0][0] == 0.5