"""Tests for device detection and validation utilities."""

from unittest.mock import Mock, patch

import pytest

from kreuzberg._utils._device import (
    detect_available_devices,
    get_device_memory_info,
    set_memory_limit,
    validate_device,
)
from kreuzberg.exceptions import DeviceNotAvailableError


def test_detect_available_devices() -> None:
    devices = detect_available_devices()
    assert "cpu" in devices

    with patch("torch.cuda.is_available", return_value=True):
        devices = detect_available_devices()
        assert "cuda" in devices

    with patch("torch.backends.mps.is_available", return_value=True):
        devices = detect_available_devices()
        assert "mps" in devices


def test_validate_device() -> None:
    with patch("torch.cuda.is_available", return_value=True):
        device = validate_device("auto", "test")
        assert device == "cuda"

    with (
        patch("torch.cuda.is_available", return_value=False),
        patch("torch.backends.mps.is_available", return_value=True),
    ):
        device = validate_device("auto", "test")
        assert device == "mps"

    with (
        patch("torch.cuda.is_available", return_value=False),
        patch("torch.backends.mps.is_available", return_value=False),
    ):
        device = validate_device("auto", "test")
        assert device == "cpu"

    device = validate_device("cpu", "test")
    assert device == "cpu"

    with pytest.raises(DeviceNotAvailableError):
        validate_device("invalid", "test")


def test_get_device_memory_info() -> None:
    with patch("torch.cuda.is_available", return_value=False):
        info = get_device_memory_info("cuda")
        assert info is None

    with (
        patch("torch.cuda.is_available", return_value=True),
        patch("torch.cuda.get_device_properties", return_value=Mock(total_memory=1024**3)),  # 1GB
        patch("torch.cuda.memory_reserved", return_value=512 * 1024**2),  # 512MB
        patch("torch.cuda.memory_allocated", return_value=256 * 1024**2),  # 256MB
    ):
        info = get_device_memory_info("cuda")
        assert info is not None
        assert info["total"] == 1024**3
        assert info["free"] == (512 * 1024**2 - 256 * 1024**2)
        assert info["allocated"] == 256 * 1024**2


def test_set_memory_limit() -> None:
    with patch("torch.cuda.is_available", return_value=False):
        set_memory_limit("cuda", 4.0)  # Should not raise

    with (
        patch("torch.cuda.is_available", return_value=True),
        patch("torch.cuda.get_device_properties", return_value=Mock(total_memory=8 * 1024**3)),  # 8GB
        patch("torch.cuda.set_per_process_memory_fraction") as mock_set,
    ):
        set_memory_limit("cuda", 4.0)
        mock_set.assert_called_once()
        # Check that the fraction is set to 4GB/8GB = 0.5
        assert mock_set.call_args[0][0] == 0.5
