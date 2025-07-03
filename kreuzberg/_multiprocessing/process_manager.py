"""Process pool manager for resource-aware multiprocessing."""

from __future__ import annotations

import multiprocessing as mp
from concurrent.futures import ProcessPoolExecutor
from typing import TYPE_CHECKING, Any, TypeVar

import anyio
import psutil
from typing_extensions import Self

if TYPE_CHECKING:
    import types
    from collections.abc import Callable

T = TypeVar("T")


class ProcessPoolManager:
    """Resource-aware process pool manager for CPU-intensive tasks."""

    def __init__(
        self,
        max_processes: int | None = None,
        memory_limit_gb: float | None = None,
    ) -> None:
        """Initialize the process pool manager.

        Args:
            max_processes: Maximum number of processes. Defaults to CPU count.
            memory_limit_gb: Memory limit in GB. Defaults to 75% of available memory.
        """
        self.max_processes = max_processes or mp.cpu_count()

        if memory_limit_gb is None:
            available_memory = psutil.virtual_memory().available
            self.memory_limit_bytes = int(available_memory * 0.75)  # Use 75% of available  # ~keep
        else:
            self.memory_limit_bytes = int(memory_limit_gb * 1024**3)

        self._executor: ProcessPoolExecutor | None = None
        self._active_tasks = 0

    def get_optimal_workers(self, task_memory_mb: float = 100) -> int:
        """Calculate optimal number of workers based on memory constraints.

        Args:
            task_memory_mb: Estimated memory usage per task in MB.

        Returns:
            Optimal number of workers.
        """
        task_memory_bytes = task_memory_mb * 1024**2
        memory_based_limit = max(1, int(self.memory_limit_bytes / task_memory_bytes))

        return min(self.max_processes, memory_based_limit)

    def _ensure_executor(self, max_workers: int | None = None) -> ProcessPoolExecutor:
        """Ensure process pool executor is initialized."""
        if self._executor is None or getattr(self._executor, "_max_workers", None) != max_workers:
            if self._executor is not None:
                self._executor.shutdown(wait=False)

            workers = max_workers or self.max_processes
            self._executor = ProcessPoolExecutor(max_workers=workers)

        return self._executor

    async def submit_task(
        self,
        func: Callable[..., T],
        *args: Any,
        task_memory_mb: float = 100,
    ) -> T:
        """Submit a task to the process pool.

        Args:
            func: Function to execute.
            *args: Positional arguments for the function.
            task_memory_mb: Estimated memory usage in MB.

        Returns:
            Result of the function execution.
        """
        workers = self.get_optimal_workers(task_memory_mb)
        self._ensure_executor(workers)

        self._active_tasks += 1

        try:
            return await anyio.to_thread.run_sync(func, *args)
        finally:
            self._active_tasks -= 1

    async def submit_batch(
        self,
        func: Callable[..., T],
        arg_batches: list[tuple[Any, ...]],
        task_memory_mb: float = 100,
        max_concurrent: int | None = None,
    ) -> list[T]:
        """Submit a batch of tasks to the process pool.

        Args:
            func: Function to execute.
            arg_batches: List of argument tuples for each task.
            task_memory_mb: Estimated memory usage per task in MB.
            max_concurrent: Maximum concurrent tasks. Defaults to optimal workers.

        Returns:
            List of results in the same order as input.
        """
        if not arg_batches:
            return []

        workers = self.get_optimal_workers(task_memory_mb)
        max_concurrent = max_concurrent or workers

        self._ensure_executor(workers)

        semaphore = anyio.CapacityLimiter(max_concurrent)

        async def submit_single(args: tuple[Any, ...]) -> T:
            async with semaphore:
                self._active_tasks += 1
                try:
                    return await anyio.to_thread.run_sync(func, *args)
                finally:
                    self._active_tasks -= 1

        async with anyio.create_task_group() as tg:
            results: list[T] = [None] * len(arg_batches)  # type: ignore[list-item]

            async def run_task(idx: int, args: tuple[Any, ...]) -> None:
                results[idx] = await submit_single(args)

            for idx, args in enumerate(arg_batches):
                tg.start_soon(run_task, idx, args)

        return results

    def get_system_info(self) -> dict[str, Any]:
        """Get current system resource information."""
        memory = psutil.virtual_memory()
        cpu_percent = psutil.cpu_percent(interval=1)

        return {
            "cpu_count": mp.cpu_count(),
            "cpu_percent": cpu_percent,
            "memory_total": memory.total,
            "memory_available": memory.available,
            "memory_percent": memory.percent,
            "active_tasks": self._active_tasks,
            "max_processes": self.max_processes,
            "memory_limit": self.memory_limit_bytes,
        }

    def shutdown(self, wait: bool = True) -> None:
        """Shutdown the process pool."""
        if self._executor is not None:
            self._executor.shutdown(wait=wait)
            self._executor = None

    def __enter__(self) -> Self:
        """Context manager entry."""
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: types.TracebackType | None,
    ) -> None:
        """Context manager exit."""
        self.shutdown()

    async def __aenter__(self) -> Self:
        """Async context manager entry."""
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: types.TracebackType | None,
    ) -> None:
        """Async context manager exit."""
        self.shutdown()
