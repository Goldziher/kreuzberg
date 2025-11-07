from __future__ import annotations

import platform
import traceback
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import psutil

from kreuzberg.exceptions import ValidationError

_SYSTEM_ERROR_KEYWORDS = frozenset({"memory", "resource", "process", "thread"})
_TRANSIENT_ERROR_PATTERNS = frozenset(
    {
        "temporary",
        "locked",
        "in use",
        "access denied",
        "permission",
        "timeout",
        "connection",
        "network",
        "too many open files",
        "cannot allocate memory",
        "resource temporarily unavailable",
        "broken pipe",
        "subprocess",
        "signal",
    }
)
_RESOURCE_ERROR_PATTERNS = frozenset(
    {
        "memory",
        "out of memory",
        "cannot allocate",
        "too many open files",
        "file descriptor",
        "resource",
        "exhausted",
        "limit",
        "cpu",
        "thread",
        "process",
    }
)


def create_error_context(
    *,
    operation: str,
    file_path: Path | str | None = None,
    error: Exception | None = None,
    **extra: Any,
) -> dict[str, Any]:
    context: dict[str, Any] = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "operation": operation,
    }

    if file_path:
        path = Path(file_path) if isinstance(file_path, str) else file_path
        context["file"] = {
            "path": str(path),
            "name": path.name,
            "exists": path.exists(),
            "size": path.stat().st_size if path.exists() else None,
        }

    if error:
        context["error"] = {
            "type": type(error).__name__,
            "message": str(error),
            "traceback": traceback.format_exception_only(type(error), error),
        }

    if error and any(keyword in str(error).lower() for keyword in _SYSTEM_ERROR_KEYWORDS):
        try:
            mem = psutil.virtual_memory()
            context["system"] = {
                "memory_available_mb": mem.available / 1024 / 1024,
                "memory_percent": mem.percent,
                "cpu_percent": psutil.cpu_percent(interval=0.1),
                "process_count": len(psutil.pids()),
                "platform": platform.platform(),
            }
        except Exception:  # noqa: BLE001
            pass

    context.update(extra)

    return context


def is_transient_error(error: Exception) -> bool:
    transient_types = (
        OSError,
        PermissionError,
        TimeoutError,
        ConnectionError,
        BrokenPipeError,
    )

    if isinstance(error, transient_types):
        return True

    error_str = str(error).lower()
    return any(pattern in error_str for pattern in _TRANSIENT_ERROR_PATTERNS)


def is_resource_error(error: Exception) -> bool:
    error_str = str(error).lower()
    return any(pattern in error_str for pattern in _RESOURCE_ERROR_PATTERNS)


def should_retry(error: Exception, attempt: int, max_attempts: int = 3) -> bool:
    if attempt >= max_attempts:
        return False

    if isinstance(error, ValidationError):
        return False

    return is_transient_error(error)


class BatchExtractionResult:
    __slots__ = ("failed", "successful", "total_count")

    def __init__(self) -> None:
        self.successful: list[tuple[int, Any]] = []
        self.failed: list[tuple[int, dict[str, Any]]] = []
        self.total_count: int = 0

    def add_success(self, index: int, result: Any) -> None:
        self.successful.append((index, result))

    def add_failure(self, index: int, error: Exception, context: dict[str, Any]) -> None:
        error_info = {
            "error": {
                "type": type(error).__name__,
                "message": str(error),
            },
            "context": context,
        }
        self.failed.append((index, error_info))

    @property
    def success_count(self) -> int:
        return len(self.successful)

    @property
    def failure_count(self) -> int:
        return len(self.failed)

    @property
    def success_rate(self) -> float:
        if self.total_count == 0:
            return 0.0
        return (self.success_count / self.total_count) * 100

    def get_ordered_results(self) -> list[Any | None]:
        results = [None] * self.total_count
        for index, result in self.successful:
            results[index] = result
        return results

    def get_summary(self) -> dict[str, Any]:
        return {
            "total": self.total_count,
            "successful": self.success_count,
            "failed": self.failure_count,
            "success_rate": f"{self.success_rate:.1f}%",
            "failures": [
                {
                    "index": idx,
                    "error": info["error"]["type"],
                    "message": info["error"]["message"],
                }
                for idx, info in self.failed
            ],
        }
