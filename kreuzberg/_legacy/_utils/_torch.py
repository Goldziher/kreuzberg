from __future__ import annotations

import contextlib
import logging
from contextlib import nullcontext
from typing import TYPE_CHECKING, Any

from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    import torch
else:
    try:
        import torch
    except ImportError:
        torch = None

logger = logging.getLogger(__name__)


def is_torch_available() -> bool:
    return torch is not None


def require_torch(functionality: str, dependency_group: str = "vision-tables") -> None:
    if not is_torch_available():
        raise MissingDependencyError.create_for_package(
            dependency_group=dependency_group,
            functionality=functionality,
            package_name="torch",
        )


def is_cuda_available() -> bool:
    if not is_torch_available():
        return False
    return bool(torch.cuda.is_available())


def is_mps_available() -> bool:
    if not is_torch_available():
        return False
    try:
        return bool(torch.backends.mps.is_available())
    except AttributeError:
        return False


def resolve_device(device: str = "auto") -> str:
    if device == "auto":
        if is_cuda_available():
            return "cuda"
        if is_mps_available():
            return "mps"
        return "cpu"
    return device


def get_cuda_device_count() -> int:
    if not is_cuda_available():
        return 0
    return int(torch.cuda.device_count())


def get_cuda_device_properties(device_id: int) -> dict[str, Any] | None:
    if not is_cuda_available():
        return None

    try:
        props = torch.cuda.get_device_properties(device_id)
        return {
            "name": props.name,
            "total_memory": props.total_memory / (1024**3),
            "major": props.major,
            "minor": props.minor,
            "multi_processor_count": props.multi_processor_count,
        }
    except (AttributeError, RuntimeError):
        return None


def get_cuda_memory_info(device_id: int) -> tuple[float, float] | None:
    if not is_cuda_available():
        return None

    try:
        props = torch.cuda.get_device_properties(device_id)
        total_memory = props.total_memory / (1024**3)

        allocated = torch.cuda.memory_allocated(device_id) / (1024**3)
        available_memory = total_memory - allocated

        return total_memory, available_memory
    except (AttributeError, RuntimeError):
        return None


def clear_gpu_cache() -> None:
    if is_cuda_available():
        with contextlib.suppress(Exception):
            torch.cuda.empty_cache()

    if is_mps_available():
        with contextlib.suppress(AttributeError, Exception):
            torch.mps.empty_cache()


def with_no_grad() -> Any:
    if not is_torch_available():
        return nullcontext()

    return torch.no_grad()


def tensor(data: Any, **kwargs: Any) -> Any:
    require_torch("tensor operations")
    return torch.tensor(data, **kwargs)


def get_torch_version() -> str | None:
    if not is_torch_available():
        return None
    return str(torch.__version__)


def log_torch_info() -> None:
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
