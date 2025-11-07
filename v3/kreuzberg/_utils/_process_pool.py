from __future__ import annotations

import io
import multiprocessing as mp
from concurrent.futures import ProcessPoolExecutor
from contextlib import contextmanager
from typing import TYPE_CHECKING, Any, TypeVar, cast

import anyio
import psutil
import pypdfium2
from typing_extensions import Self

from kreuzberg._utils._ref import Ref

if TYPE_CHECKING:
    import types
    from collections.abc import Callable, Generator

T = TypeVar("T")

_POOL_SIZE = mp.cpu_count()

_process_pool_ref = Ref("process_pool", lambda: ProcessPoolExecutor(max_workers=_POOL_SIZE))


def _get_process_pool() -> ProcessPoolExecutor:
    return _process_pool_ref.get()


@contextmanager
def process_pool() -> Generator[ProcessPoolExecutor, None, None]:
    pool = _get_process_pool()
    try:
        yield pool
    except Exception:  # noqa: BLE001
        shutdown_process_pool()
        pool = _get_process_pool()
        yield pool


def submit_to_process_pool(func: Callable[..., T], *args: Any, **kwargs: Any) -> T:
    with process_pool() as pool:
        future = pool.submit(func, *args, **kwargs)
        return future.result()


def get_optimal_worker_count(num_tasks: int, cpu_intensive: bool = True) -> int:
    """Calculate optimal worker count based on workload.

    Optimized based on benchmarking results:
    - For 1 task: Use 1 worker (avoid overhead)
    - For 2-3 tasks: Use num_tasks workers
    - For 4+ tasks: Use all CPU cores for CPU-intensive work
    """
    cpu_count = mp.cpu_count()

    if num_tasks == 1:
        return 1
    if num_tasks <= 3:
        return min(num_tasks, cpu_count)
    if cpu_intensive:
        return cpu_count
    return min(cpu_count * 2, max(cpu_count, num_tasks))


def warmup_process_pool() -> None:
    """Warm up the process pool to reduce initialization overhead."""
    with process_pool() as pool:
        futures = [pool.submit(lambda: None) for _ in range(_POOL_SIZE)]
        for future in futures:
            future.result()


def shutdown_process_pool() -> None:
    if _process_pool_ref.is_initialized():
        pool = _process_pool_ref.get()
        pool.shutdown(wait=True)
        _process_pool_ref.clear()


def _extract_pdf_text_worker(pdf_path: str) -> tuple[str, str]:
    pdf = None
    try:
        pdf = pypdfium2.PdfDocument(pdf_path)
        text_parts = []
        for page in pdf:
            text_page = page.get_textpage()
            text = text_page.get_text_bounded()
            text_parts.append(text)
            text_page.close()
            page.close()
        return (pdf_path, "".join(text_parts))
    except Exception as e:  # noqa: BLE001
        return (pdf_path, f"ERROR: {e}")
    finally:
        if pdf:
            pdf.close()


def _extract_pdf_images_worker(pdf_path: str, scale: float = 4.25) -> tuple[str, list[bytes]]:
    pdf = None
    try:
        pdf = pypdfium2.PdfDocument(pdf_path)
        image_bytes = []
        for page in pdf:
            bitmap = page.render(scale=scale)
            pil_image = bitmap.to_pil()
            img_bytes = io.BytesIO()
            pil_image.save(img_bytes, format="PNG")
            image_bytes.append(img_bytes.getvalue())
            bitmap.close()
            page.close()
        return (pdf_path, image_bytes)
    except Exception:  # noqa: BLE001
        return (pdf_path, [])
    finally:
        if pdf:
            pdf.close()


class ProcessPoolManager:
    def __init__(
        self,
        max_processes: int | None = None,
        memory_limit_gb: float | None = None,
    ) -> None:
        self.max_processes = max_processes or mp.cpu_count()

        if memory_limit_gb is None:
            available_memory = psutil.virtual_memory().available
            self.memory_limit_bytes = int(available_memory * 0.75)  # Use 75% of available  # ~keep
        else:
            self.memory_limit_bytes = int(memory_limit_gb * 1024**3)

        self._executor: ProcessPoolExecutor | None = None
        self._active_tasks = 0

    def get_optimal_workers(self, task_memory_mb: float = 100) -> int:
        task_memory_bytes = task_memory_mb * 1024**2
        memory_based_limit = max(1, int(self.memory_limit_bytes / task_memory_bytes))

        return min(self.max_processes, memory_based_limit)

    def _ensure_executor(self, max_workers: int | None = None) -> ProcessPoolExecutor:
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
            results: list[T | None] = [None] * len(arg_batches)

            async def run_task(idx: int, args: tuple[Any, ...]) -> None:
                results[idx] = await submit_single(args)

            for idx, args in enumerate(arg_batches):
                tg.start_soon(run_task, idx, args)

        return cast("list[T]", results)

    def get_system_info(self) -> dict[str, Any]:
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
        if self._executor is not None:
            self._executor.shutdown(wait=wait)
            self._executor = None

    def __enter__(self) -> Self:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: types.TracebackType | None,
    ) -> None:
        self.shutdown()

    async def __aenter__(self) -> Self:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: types.TracebackType | None,
    ) -> None:
        self.shutdown()
