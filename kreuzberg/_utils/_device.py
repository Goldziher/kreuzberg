"""Device detection and validation utilities for GPU acceleration."""

from __future__ import annotations

import logging
from typing import Literal

import torch

from kreuzberg.exceptions import DeviceNotAvailableError

logger = logging.getLogger(__name__)

DeviceType = Literal["cpu", "cuda", "mps", "auto", "invalid"]


def detect_available_devices() -> list[str]:
    """Detect available compute devices.

    Returns:
        list[str]: List of available device types (e.g. ["cpu", "cuda", "mps"])
    """
    devices = ["cpu"]

    if torch.cuda.is_available():
        devices.append("cuda")
        logger.info("CUDA GPU detected")

    if torch.backends.mps.is_available():
        devices.append("mps")
        logger.info("Apple Silicon GPU detected")

    return devices


def validate_device(requested: DeviceType, backend: str) -> str:
    """Validate and return usable device.

    Args:
        requested: Requested device type
        backend: OCR backend name

    Returns:
        str: Validated device type to use

    Raises:
        DeviceNotAvailableError: If requested device is not available
    """
    available_devices = detect_available_devices()

    if requested == "auto":
        # Prefer CUDA over MPS over CPU
        if "cuda" in available_devices:
            return "cuda"
        if "mps" in available_devices:
            return "mps"
        return "cpu"

    if requested not in available_devices:
        raise DeviceNotAvailableError(
            f"Requested device '{requested}' not available for {backend}. Available devices: {available_devices}"
        )

    return requested


def get_device_memory_info(device: str) -> dict[str, int] | None:
    """Get memory information for the specified device.

    Args:
        device: Device type ("cuda", "mps", or "cpu")

    Returns:
        Optional[dict[str, int]]: Memory info dict with keys 'total', 'free', 'allocated' and their values in bytes,
        or None if not available
    """
    if device == "cuda" and torch.cuda.is_available():
        return {
            "total": torch.cuda.get_device_properties(0).total_memory,
            "free": torch.cuda.memory_reserved(0) - torch.cuda.memory_allocated(0),
            "allocated": torch.cuda.memory_allocated(0),
        }
    return None


def set_memory_limit(device: str, limit_gb: float) -> None:
    """Set memory limit for GPU operations.

    Args:
        device: Device type ("cuda", "mps", or "cpu")
        limit_gb: Memory limit in gigabytes
    """
    if device == "cuda" and torch.cuda.is_available():
        limit_bytes = int(limit_gb * 1024 * 1024 * 1024)
        torch.cuda.set_per_process_memory_fraction(limit_bytes / torch.cuda.get_device_properties(0).total_memory)
        logger.info("Set CUDA memory limit to %.1fGB", limit_gb)
