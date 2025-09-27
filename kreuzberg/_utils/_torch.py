"""Centralized PyTorch utilities and optional dependency management.

This module provides a unified way to handle PyTorch dependencies across Kreuzberg,
supporting both CPU-only and GPU-accelerated workflows.

Features that REQUIRE torch (will raise MissingDependencyError):
- GMFT table extraction (Table Transformer models)

Features that OPTIONALLY use torch (graceful fallback):
- EasyOCR GPU acceleration
- Advanced device detection and memory management
"""

from __future__ import annotations

import contextlib
import logging
from contextlib import nullcontext
from typing import TYPE_CHECKING, Any

from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    import torch
else:
    # Import torch conditionally to support CPU-only installations
    try:
        import torch
    except ImportError:
        torch = None

logger = logging.getLogger(__name__)


def is_torch_available() -> bool:
    """Check if PyTorch is available.

    Returns:
        True if torch is installed and importable, False otherwise
    """
    return torch is not None


def require_torch(functionality: str, dependency_group: str = "gmft") -> None:
    """Require PyTorch for a specific functionality.

    Args:
        functionality: Description of what needs torch (e.g., "table extraction")
        dependency_group: Which dependency group provides torch (e.g., "gmft")

    Raises:
        MissingDependencyError: If torch is not available
    """
    if not is_torch_available():
        raise MissingDependencyError.create_for_package(
            dependency_group=dependency_group,
            functionality=functionality,
            package_name="torch",
        )


def is_cuda_available() -> bool:
    """Check if CUDA is available for GPU acceleration.

    Returns:
        True if torch is available and CUDA is supported, False otherwise
    """
    if not is_torch_available():
        return False
    return torch.cuda.is_available()


def is_mps_available() -> bool:
    """Check if MPS (Apple Silicon GPU) is available.

    Returns:
        True if torch is available and MPS is supported, False otherwise
    """
    if not is_torch_available():
        return False
    try:
        return torch.backends.mps.is_available()
    except AttributeError:
        # Older PyTorch versions don't have MPS
        return False


def resolve_device(device: str = "auto") -> str:
    """Resolve device string to actual device.

    Args:
        device: Device specification ("auto", "cpu", "cuda", "mps", "cuda:0", etc.)

    Returns:
        Resolved device string ("cpu", "cuda", "mps", "cuda:0", etc.)
    """
    if device == "auto":
        if is_cuda_available():
            return "cuda"
        if is_mps_available():
            return "mps"
        return "cpu"
    return device


def get_cuda_device_count() -> int:
    """Get the number of available CUDA devices.

    Returns:
        Number of CUDA devices, 0 if CUDA not available
    """
    if not is_cuda_available():
        return 0
    return torch.cuda.device_count()


def get_cuda_device_properties(device_id: int) -> dict[str, Any] | None:
    """Get CUDA device properties.

    Args:
        device_id: CUDA device ID

    Returns:
        Dictionary with device properties, None if not available
    """
    if not is_cuda_available():
        return None

    try:
        props = torch.cuda.get_device_properties(device_id)
        return {
            "name": props.name,
            "total_memory": props.total_memory / (1024**3),  # GB
            "major": props.major,
            "minor": props.minor,
            "multi_processor_count": props.multi_processor_count,
        }
    except (AttributeError, RuntimeError):
        return None


def get_cuda_memory_info(device_id: int) -> tuple[float, float] | None:
    """Get CUDA memory usage information.

    Args:
        device_id: CUDA device ID

    Returns:
        Tuple of (total_memory_gb, available_memory_gb), None if not available
    """
    if not is_cuda_available():
        return None

    try:
        props = torch.cuda.get_device_properties(device_id)
        total_memory = props.total_memory / (1024**3)  # GB

        # Get allocated memory
        allocated = torch.cuda.memory_allocated(device_id) / (1024**3)
        available_memory = total_memory - allocated

        return total_memory, available_memory
    except (AttributeError, RuntimeError):
        return None


def clear_gpu_cache() -> None:
    """Clear GPU memory cache if available.

    This helps free up GPU memory for other processes.
    """
    if is_cuda_available():
        with contextlib.suppress(Exception):
            torch.cuda.empty_cache()

    if is_mps_available():
        with contextlib.suppress(AttributeError, Exception):
            torch.mps.empty_cache()


def with_no_grad() -> Any:
    """Context manager for disabling gradient computation.

    Returns:
        torch.no_grad() if available, otherwise a no-op context manager

    Raises:
        MissingDependencyError: If torch is required but not available
    """
    if not is_torch_available():
        # Return a no-op context manager for CPU-only fallbacks
        return nullcontext()

    return torch.no_grad()


def tensor(data: Any, **kwargs: Any) -> Any:
    """Create a tensor if torch is available.

    Args:
        data: Data to convert to tensor
        **kwargs: Additional arguments for torch.tensor()

    Returns:
        torch.Tensor if torch is available

    Raises:
        MissingDependencyError: If torch is required but not available
    """
    require_torch("tensor operations")
    return torch.tensor(data, **kwargs)


def get_torch_version() -> str | None:
    """Get PyTorch version string.

    Returns:
        PyTorch version string, None if not available
    """
    if not is_torch_available():
        return None
    return torch.__version__


def log_torch_info() -> None:
    """Log PyTorch configuration information."""
    if not is_torch_available():
        logger.info("PyTorch: Not available (CPU-only mode)")
        return

    version = get_torch_version()
    cuda_available = is_cuda_available()
    mps_available = is_mps_available()

    logger.info(
        "PyTorch: v%s (CUDA: %s, MPS: %s)",
        version,
        "available" if cuda_available else "unavailable",
        "available" if mps_available else "unavailable",
    )

    if cuda_available:
        device_count = get_cuda_device_count()
        logger.info("CUDA devices: %d", device_count)
