from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

import pytest

if TYPE_CHECKING:
    from pytest_mock import MockerFixture

from kreuzberg._utils._device import (
    DeviceInfo,
    cleanup_device_memory,
    detect_available_devices,
    get_device_memory_info,
    get_optimal_device,
    get_recommended_batch_size,
    is_backend_gpu_compatible,
    validate_device_request,
)
from kreuzberg._utils._torch import is_cuda_available, is_mps_available
from kreuzberg.exceptions import ValidationError


def test_device_info_creation() -> None:
    device = DeviceInfo(device_type="cpu", name="CPU")
    assert device.device_type == "cpu"
    assert device.name == "CPU"
    assert device.device_id is None
    assert device.memory_total is None


def test_device_info_with_gpu_details() -> None:
    device = DeviceInfo(
        device_type="cuda",
        device_id=0,
        memory_total=8.0,
        memory_available=6.0,
        name="NVIDIA GeForce RTX 3080",
    )
    assert device.device_type == "cuda"
    assert device.device_id == 0
    assert device.memory_total == 8.0
    assert device.memory_available == 6.0
    assert device.name == "NVIDIA GeForce RTX 3080"


def test_detect_devices_cpu_only() -> None:
    with (
        patch("kreuzberg._utils._device.is_cuda_available", return_value=False),
        patch("kreuzberg._utils._device.is_mps_available", return_value=False),
        patch("kreuzberg._utils._device._get_cuda_devices", return_value=[]),
        patch("kreuzberg._utils._device._get_mps_device", return_value=None),
    ):
        devices = detect_available_devices()
        assert len(devices) == 1
        assert devices[0].device_type == "cpu"
        assert devices[0].name == "CPU"


def test_detect_devices_with_cuda() -> None:
    with (
        patch("kreuzberg._utils._device.is_cuda_available", return_value=True),
        patch("kreuzberg._utils._device.is_mps_available", return_value=False),
        patch("kreuzberg._utils._device._get_cuda_devices") as mock_get_cuda,
        patch("kreuzberg._utils._device._get_mps_device", return_value=None),
    ):
        mock_get_cuda.return_value = [
            DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080", memory_total=10.0)
        ]

        devices = detect_available_devices()
        assert len(devices) == 2
        assert devices[0].device_type == "cuda"
        assert devices[1].device_type == "cpu"


def test_detect_devices_with_mps() -> None:
    with (
        patch("kreuzberg._utils._device.is_cuda_available", return_value=False),
        patch("kreuzberg._utils._device.is_mps_available", return_value=True),
        patch("kreuzberg._utils._device._get_cuda_devices", return_value=[]),
        patch("kreuzberg._utils._device._get_mps_device") as mock_get_mps,
    ):
        mock_get_mps.return_value = DeviceInfo(device_type="mps", name="Apple Silicon GPU (MPS)")

        devices = detect_available_devices()
        assert len(devices) == 2
        assert devices[0].device_type == "mps"
        assert devices[1].device_type == "cpu"


@patch("kreuzberg._utils._device.detect_available_devices")
def test_get_optimal_device_gpu_preferred(mock_detect: Mock) -> None:
    mock_detect.return_value = [
        DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080"),
        DeviceInfo(device_type="cpu", name="CPU"),
    ]

    device = get_optimal_device()
    assert device.device_type == "cuda"


@patch("kreuzberg._utils._device.detect_available_devices")
def test_get_optimal_device_cpu_fallback(mock_detect: Mock) -> None:
    mock_detect.return_value = [DeviceInfo(device_type="cpu", name="CPU")]

    device = get_optimal_device()
    assert device.device_type == "cpu"


@patch("kreuzberg._utils._device.detect_available_devices")
def test_validate_auto_device(mock_detect: Mock) -> None:
    mock_detect.return_value = [
        DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080"),
        DeviceInfo(device_type="cpu", name="CPU"),
    ]

    device = validate_device_request("auto", "EasyOCR")
    assert device.device_type == "cuda"


@patch("kreuzberg._utils._device.detect_available_devices")
def test_validate_specific_device_available(mock_detect: Mock) -> None:
    mock_detect.return_value = [
        DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080"),
        DeviceInfo(device_type="cpu", name="CPU"),
    ]

    device = validate_device_request("cuda", "EasyOCR")
    assert device.device_type == "cuda"


@patch("kreuzberg._utils._device.detect_available_devices")
def test_validate_device_unavailable_with_fallback(mock_detect: Mock) -> None:
    mock_detect.return_value = [DeviceInfo(device_type="cpu", name="CPU")]

    with pytest.warns(UserWarning, match="Requested device 'cuda' not available"):
        device = validate_device_request("cuda", "EasyOCR", fallback_to_cpu=True)
    assert device.device_type == "cpu"


@patch("kreuzberg._utils._device.detect_available_devices")
def test_validate_device_unavailable_no_fallback(mock_detect: Mock) -> None:
    mock_detect.return_value = [DeviceInfo(device_type="cpu", name="CPU")]

    with pytest.raises(ValidationError, match="Requested device 'cuda' is not available"):
        validate_device_request("cuda", "EasyOCR", fallback_to_cpu=False)


@patch("kreuzberg._utils._device.detect_available_devices")
@patch("kreuzberg._utils._device.get_device_memory_info")
def test_validate_device_memory_limit_exceeded(mock_memory: Mock, mock_detect: Mock) -> None:
    mock_detect.return_value = [DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080", memory_total=8.0)]
    mock_memory.return_value = (8.0, 6.0)

    with pytest.raises(ValidationError, match=r"Requested memory limit.*exceeds device capacity"):
        validate_device_request("cuda", "EasyOCR", memory_limit=10.0)


@patch("kreuzberg._utils._device.detect_available_devices")
@patch("kreuzberg._utils._device.get_device_memory_info")
def test_validate_device_memory_limit_warns_low_available(mock_memory: Mock, mock_detect: Mock) -> None:
    mock_detect.return_value = [DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080", memory_total=8.0)]
    mock_memory.return_value = (8.0, 2.0)

    with pytest.warns(UserWarning, match="Requested memory limit.*exceeds available memory"):
        device = validate_device_request("cuda", "EasyOCR", memory_limit=4.0)
    assert device.device_type == "cuda"


def test_is_cuda_available_true() -> None:
    mock_torch = Mock()
    mock_torch.cuda.is_available.return_value = True
    with patch("kreuzberg._utils._torch.torch", mock_torch):
        assert is_cuda_available() is True


def test_is_cuda_available_false() -> None:
    mock_torch = Mock()
    mock_torch.cuda.is_available.return_value = False
    with patch("kreuzberg._utils._torch.torch", mock_torch):
        assert is_cuda_available() is False


def test_is_cuda_available_no_torch() -> None:
    with patch("kreuzberg._utils._torch.torch", None):
        assert is_cuda_available() is False


def test_get_cuda_devices() -> None:
    mock_devices = [
        DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080", memory_total=10.0),
        DeviceInfo(device_type="cuda", device_id=1, name="NVIDIA RTX 3090", memory_total=24.0),
    ]

    with patch("kreuzberg._utils._device._get_cuda_devices", return_value=mock_devices):
        from kreuzberg._utils._device import _get_cuda_devices

        devices = _get_cuda_devices()
        assert len(devices) == 2
        assert devices[0].device_type == "cuda"
        assert devices[0].device_id == 0
        assert devices[0].name == "NVIDIA RTX 3080"
        assert devices[0].memory_total == 10.0
        assert devices[1].device_id == 1
        assert devices[1].name == "NVIDIA RTX 3090"


def test_is_mps_available_true() -> None:
    mock_torch = Mock()
    mock_torch.backends.mps.is_available.return_value = True
    with patch("kreuzberg._utils._torch.torch", mock_torch):
        assert is_mps_available() is True


def test_is_mps_available_false() -> None:
    mock_torch = Mock()
    mock_torch.backends.mps.is_available.return_value = False
    with patch("kreuzberg._utils._torch.torch", mock_torch):
        assert is_mps_available() is False


def test_is_mps_available_no_torch(mocker: MockerFixture) -> None:
    with patch("kreuzberg._utils._torch.torch", None):
        assert is_mps_available() is False


def test_get_mps_device() -> None:
    mock_device = DeviceInfo(device_type="mps", name="Apple Silicon GPU (MPS)")

    with patch("kreuzberg._utils._device._get_mps_device", return_value=mock_device):
        from kreuzberg._utils._device import _get_mps_device

        device = _get_mps_device()
        assert device is not None
        assert device.device_type == "mps"
        assert device.name == "Apple Silicon GPU (MPS)"


def test_get_mps_device_unavailable() -> None:
    with patch("kreuzberg._utils._device._get_mps_device", return_value=None):
        from kreuzberg._utils._device import _get_mps_device

        device = _get_mps_device()
        assert device is None


def test_get_device_memory_info_cpu() -> None:
    device = DeviceInfo(device_type="cpu", name="CPU")
    total, available = get_device_memory_info(device)
    assert total is None
    assert available is None


@patch("kreuzberg._utils._device._get_cuda_memory_info")
def test_get_device_memory_info_cuda(mock_cuda_memory: Mock) -> None:
    mock_cuda_memory.return_value = (8.0, 6.0)
    device = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080")

    total, available = get_device_memory_info(device)
    assert total == 8.0
    assert available == 6.0
    mock_cuda_memory.assert_called_once_with(0)


@patch("kreuzberg._utils._device._get_mps_memory_info")
def test_get_device_memory_info_mps(mock_mps_memory: Mock) -> None:
    mock_mps_memory.return_value = (None, None)
    device = DeviceInfo(device_type="mps", name="Apple Silicon GPU")

    total, available = get_device_memory_info(device)
    assert total is None
    assert available is None


def test_is_backend_gpu_compatible() -> None:
    assert is_backend_gpu_compatible("easyocr") is True
    assert is_backend_gpu_compatible("paddleocr") is True
    assert is_backend_gpu_compatible("tesseract") is False
    assert is_backend_gpu_compatible("unknown") is False


def test_get_recommended_batch_size_cpu() -> None:
    device = DeviceInfo(device_type="cpu", name="CPU")
    batch_size = get_recommended_batch_size(device)
    assert batch_size == 1


@patch("kreuzberg._utils._device.get_device_memory_info")
def test_get_recommended_batch_size_gpu_unknown_memory(mock_memory: Mock) -> None:
    mock_memory.return_value = (None, None)
    device = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080")

    batch_size = get_recommended_batch_size(device)
    assert batch_size == 4


@patch("kreuzberg._utils._device.get_device_memory_info")
def test_get_recommended_batch_size_gpu_with_memory(mock_memory: Mock) -> None:
    mock_memory.return_value = (8.0, 6.0)
    device = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080")

    batch_size = get_recommended_batch_size(device, input_size_mb=100.0)

    assert batch_size >= 1
    assert batch_size <= 32


def test_cleanup_device_memory_cuda() -> None:
    device = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080")

    cleanup_device_memory(device)


def test_cleanup_device_memory_mps() -> None:
    device = DeviceInfo(device_type="mps", name="Apple Silicon GPU")

    cleanup_device_memory(device)


def test_cleanup_device_memory_cpu() -> None:
    device = DeviceInfo(device_type="cpu", name="CPU")

    cleanup_device_memory(device)


def test_cleanup_device_memory_no_torch() -> None:
    device = DeviceInfo(device_type="cuda", device_id=0, name="NVIDIA RTX 3080")

    cleanup_device_memory(device)
