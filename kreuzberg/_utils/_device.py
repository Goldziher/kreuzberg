# ~keep

from __future__ import annotations

import warnings
from dataclasses import dataclass
from itertools import chain
from typing import Literal

from kreuzberg._utils._torch import (
    clear_gpu_cache,
    get_cuda_device_count,
    get_cuda_device_properties,
    get_cuda_memory_info,
    is_cuda_available,
    is_mps_available,
)
from kreuzberg.exceptions import ValidationError

DeviceType = Literal["cpu", "cuda", "mps", "auto"]


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class DeviceInfo:
    device_type: Literal["cpu", "cuda", "mps"]
    """The type of device."""
    device_id: int | None = None
    """Device ID for multi-GPU systems. None for CPU or single GPU."""
    memory_total: float | None = None
    """Total memory in GB. None if unknown."""
    memory_available: float | None = None
    """Available memory in GB. None if unknown."""
    name: str | None = None
    """Human-readable device name."""


def detect_available_devices() -> list[DeviceInfo]:
    cpu_device = DeviceInfo(device_type="cpu", name="CPU")

    cuda_devices = _get_cuda_devices() if is_cuda_available() else []
    mps_device = _get_mps_device() if is_mps_available() else None
    mps_devices = [mps_device] if mps_device else []

    return list(chain(cuda_devices, mps_devices, [cpu_device]))


def get_optimal_device() -> DeviceInfo:
    devices = detect_available_devices()
    return devices[0] if devices else DeviceInfo(device_type="cpu", name="CPU")


def validate_device_request(
    requested: DeviceType,
    backend: str,
    *,
    memory_limit: float | None = None,
    fallback_to_cpu: bool = True,
) -> DeviceInfo:
    available_devices = detect_available_devices()

    if requested == "auto":
        device = get_optimal_device()
        if memory_limit is not None:
            _validate_memory_limit(device, memory_limit)
        return device

    matching_devices = [d for d in available_devices if d.device_type == requested]

    if not matching_devices:
        if fallback_to_cpu and requested != "cpu":
            warnings.warn(
                f"Requested device '{requested}' not available for {backend}. Falling back to CPU.",
                UserWarning,
                stacklevel=2,
            )
            cpu_device = next((d for d in available_devices if d.device_type == "cpu"), None)
            if cpu_device:
                return cpu_device

        raise ValidationError(
            f"Requested device '{requested}' is not available for {backend}",
            context={
                "requested_device": requested,
                "backend": backend,
                "available_devices": [d.device_type for d in available_devices],
            },
        )

    device = matching_devices[0]

    if memory_limit is not None:
        _validate_memory_limit(device, memory_limit)

    return device


def get_device_memory_info(device: DeviceInfo) -> tuple[float | None, float | None]:
    if device.device_type == "cpu":
        return None, None

    if device.device_type == "cuda":
        return _get_cuda_memory_info(device.device_id or 0)

    if device.device_type == "mps":
        return _get_mps_memory_info()

    return None, None


def _get_cuda_devices() -> list[DeviceInfo]:
    devices: list[DeviceInfo] = []

    if not is_cuda_available():
        return devices

    device_count = get_cuda_device_count()
    for i in range(device_count):
        props = get_cuda_device_properties(i)
        memory_info = get_cuda_memory_info(i)

        if props and memory_info:
            total_memory, available_memory = memory_info
            devices.append(
                DeviceInfo(
                    device_type="cuda",
                    device_id=i,
                    memory_total=total_memory,
                    memory_available=available_memory,
                    name=props["name"],
                )
            )

    return devices


def _get_mps_device() -> DeviceInfo | None:
    if not is_mps_available():
        return None

    return DeviceInfo(
        device_type="mps",
        name="Apple Silicon GPU (MPS)",
    )


def _get_cuda_memory_info(device_id: int) -> tuple[float | None, float | None]:
    return get_cuda_memory_info(device_id) or (None, None)


def _get_mps_memory_info() -> tuple[float | None, float | None]:
    return None, None


def _validate_memory_limit(device: DeviceInfo, memory_limit: float) -> None:
    if device.device_type == "cpu":
        # CPU memory validation is complex and OS-dependent, skip for now  # ~keep
        return

    total_memory, available_memory = get_device_memory_info(device)

    if total_memory is not None and memory_limit > total_memory:
        raise ValidationError(
            f"Requested memory limit ({memory_limit:.1f}GB) exceeds device capacity ({total_memory:.1f}GB)",
            context={
                "device": device.device_type,
                "device_name": device.name,
                "requested_memory": memory_limit,
                "total_memory": total_memory,
                "available_memory": available_memory,
            },
        )

    if available_memory is not None and memory_limit > available_memory:
        warnings.warn(
            f"Requested memory limit ({memory_limit:.1f}GB) exceeds available memory "
            f"({available_memory:.1f}GB) on {device.name or device.device_type}",
            UserWarning,
            stacklevel=3,
        )


def is_backend_gpu_compatible(backend: str) -> bool:
    # EasyOCR and PaddleOCR support GPU, Tesseract does not  # ~keep
    return backend.lower() in ("easyocr", "paddleocr")


def get_recommended_batch_size(device: DeviceInfo, input_size_mb: float = 10.0) -> int:
    if device.device_type == "cpu":
        # Conservative batch size for CPU  # ~keep
        return 1

    _, available_memory = get_device_memory_info(device)

    if available_memory is None:
        return 4

    # Use approximately 50% of available memory for batching  # ~keep
    usable_memory_gb = available_memory * 0.5
    usable_memory_mb = usable_memory_gb * 1024

    # Estimate batch size (conservative)  # ~keep
    estimated_batch_size = max(1, int(usable_memory_mb / (input_size_mb * 4)))

    # Cap at reasonable limits  # ~keep
    return min(estimated_batch_size, 32)


def cleanup_device_memory(_device: DeviceInfo) -> None:
    clear_gpu_cache()
