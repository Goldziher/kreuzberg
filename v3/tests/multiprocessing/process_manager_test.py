from __future__ import annotations

import multiprocessing as mp
from typing import Any
from unittest.mock import patch

import pytest
from kreuzberg._utils._process_pool import ProcessPoolManager


def simple_function(x: int) -> int:
    return x * 2


def add_function(x: int, y: int) -> int:
    return x + y


def error_function() -> None:
    raise ValueError("Test error")


def test_process_pool_manager_init_default() -> None:
    manager = ProcessPoolManager()
    expected_processes = mp.cpu_count()
    assert manager.max_processes == expected_processes
    assert manager._executor is None
    assert manager._active_tasks == 0


def test_process_pool_manager_init_custom_processes() -> None:
    manager = ProcessPoolManager(max_processes=4)
    assert manager.max_processes == 4


def test_process_pool_manager_init_custom_memory_limit() -> None:
    manager = ProcessPoolManager(memory_limit_gb=2.0)
    expected_bytes = int(2.0 * 1024**3)
    assert manager.memory_limit_bytes == expected_bytes


def test_process_pool_manager_init_default_memory_limit() -> None:
    with patch("psutil.virtual_memory") as mock_memory:
        mock_memory.return_value.available = 8 * 1024**3
        manager = ProcessPoolManager()
        expected_bytes = int(8 * 1024**3 * 0.75)
        assert manager.memory_limit_bytes == expected_bytes


def test_process_pool_manager_get_optimal_workers_memory_constrained() -> None:
    manager = ProcessPoolManager(max_processes=8, memory_limit_gb=1.0)

    workers = manager.get_optimal_workers(task_memory_mb=500)
    assert workers == 2


def test_process_pool_manager_get_optimal_workers_cpu_constrained() -> None:
    manager = ProcessPoolManager(max_processes=2, memory_limit_gb=10.0)

    workers = manager.get_optimal_workers(task_memory_mb=100)
    assert workers == 2


def test_process_pool_manager_get_optimal_workers_minimum_one() -> None:
    manager = ProcessPoolManager(max_processes=1, memory_limit_gb=0.001)
    workers = manager.get_optimal_workers(task_memory_mb=1000)
    assert workers == 1


def test_process_pool_manager_ensure_executor_creation() -> None:
    manager = ProcessPoolManager(max_processes=2)
    executor = manager._ensure_executor()

    assert manager._executor is not None
    assert executor is manager._executor


def test_process_pool_manager_ensure_executor_reuse() -> None:
    manager = ProcessPoolManager(max_processes=2)
    executor1 = manager._ensure_executor(max_workers=2)
    executor2 = manager._ensure_executor(max_workers=2)

    assert executor1 is executor2


def test_process_pool_manager_ensure_executor_recreation() -> None:
    manager = ProcessPoolManager(max_processes=4)

    class _MockExecutor:
        def __init__(self) -> None:
            self._max_workers = 2
            self.shutdown_called = False

        def shutdown(self, wait: bool = True) -> None:
            self.shutdown_called = True

    mock_executor = _MockExecutor()
    manager._executor = mock_executor  # type: ignore[assignment]

    new_executor = manager._ensure_executor(max_workers=4)

    assert mock_executor.shutdown_called
    assert new_executor is not mock_executor  # type: ignore[comparison-overlap]


@pytest.mark.anyio
async def test_process_pool_manager_submit_task_success() -> None:
    manager = ProcessPoolManager(max_processes=2)

    result = await manager.submit_task(simple_function, 5)

    assert result == 10
    assert manager._active_tasks == 0


@pytest.mark.anyio
async def test_process_pool_manager_submit_task_with_memory_constraint() -> None:
    manager = ProcessPoolManager(max_processes=8, memory_limit_gb=1.0)

    result = await manager.submit_task(simple_function, 3, task_memory_mb=500)

    assert result == 6
    assert manager._active_tasks == 0


@pytest.mark.anyio
async def test_process_pool_manager_submit_task_tracks_active_tasks() -> None:
    manager = ProcessPoolManager(max_processes=2)

    result = await manager.submit_task(simple_function, 21)

    assert result == 42
    assert manager._active_tasks == 0


@pytest.mark.anyio
async def test_process_pool_manager_submit_batch_success() -> None:
    manager = ProcessPoolManager(max_processes=2)

    arg_batches = [(1,), (2,), (3,), (4,)]
    results = await manager.submit_batch(simple_function, arg_batches)

    assert results == [2, 4, 6, 8]
    assert manager._active_tasks == 0


@pytest.mark.anyio
async def test_process_pool_manager_submit_batch_empty() -> None:
    manager = ProcessPoolManager(max_processes=2)

    results = await manager.submit_batch(simple_function, [])

    assert results == []


@pytest.mark.anyio
async def test_process_pool_manager_submit_batch_with_concurrency_limit() -> None:
    manager = ProcessPoolManager(max_processes=4)

    arg_batches = [(1,), (2,), (3,), (4,), (5,)]
    results = await manager.submit_batch(simple_function, arg_batches, max_concurrent=2)

    assert results == [2, 4, 6, 8, 10]
    assert manager._active_tasks == 0


@pytest.mark.anyio
async def test_process_pool_manager_submit_batch_with_memory_constraint() -> None:
    manager = ProcessPoolManager(max_processes=8, memory_limit_gb=1.0)

    arg_batches = [(1,), (2,)]
    results = await manager.submit_batch(
        simple_function,
        arg_batches,
        task_memory_mb=500,
    )

    assert results == [2, 4]


def test_process_pool_manager_get_system_info() -> None:
    manager = ProcessPoolManager(max_processes=4, memory_limit_gb=2.0)

    with patch("psutil.virtual_memory") as mock_memory, patch("psutil.cpu_percent") as mock_cpu:
        mock_memory.return_value.total = 16 * 1024**3
        mock_memory.return_value.available = 8 * 1024**3
        mock_memory.return_value.percent = 50.0
        mock_cpu.return_value = 25.5

        info = manager.get_system_info()

        assert info["cpu_count"] == mp.cpu_count()
        assert info["cpu_percent"] == 25.5
        assert info["memory_total"] == 16 * 1024**3
        assert info["memory_available"] == 8 * 1024**3
        assert info["memory_percent"] == 50.0
        assert info["active_tasks"] == 0
        assert info["max_processes"] == 4
        assert info["memory_limit"] == int(2.0 * 1024**3)


def test_process_pool_manager_shutdown_with_executor() -> None:
    manager = ProcessPoolManager(max_processes=2)
    manager._ensure_executor()

    class _MockExecutor:
        def __init__(self) -> None:
            self.shutdown_called_with: bool | None = None

        def shutdown(self, wait: bool = True) -> None:
            self.shutdown_called_with = wait

    mock_executor = _MockExecutor()
    manager._executor = mock_executor  # type: ignore[assignment]

    manager.shutdown(wait=True)

    assert mock_executor.shutdown_called_with is True
    assert manager._executor is None


def test_process_pool_manager_shutdown_without_executor() -> None:
    manager = ProcessPoolManager(max_processes=2)

    manager.shutdown()
    assert manager._executor is None


def test_process_pool_manager_context_manager_sync() -> None:
    manager = ProcessPoolManager(max_processes=2)

    shutdown_called = False
    original_shutdown = manager.shutdown

    def mock_shutdown(*args: Any, **kwargs: Any) -> None:
        nonlocal shutdown_called
        shutdown_called = True
        return original_shutdown(*args, **kwargs)

    manager.shutdown = mock_shutdown  # type: ignore[method-assign]

    with manager:
        pass

    assert shutdown_called


@pytest.mark.anyio
async def test_process_pool_manager_context_manager_async() -> None:
    manager = ProcessPoolManager(max_processes=2)

    shutdown_called = False
    original_shutdown = manager.shutdown

    def mock_shutdown(*args: Any, **kwargs: Any) -> None:
        nonlocal shutdown_called
        shutdown_called = True
        return original_shutdown(*args, **kwargs)

    manager.shutdown = mock_shutdown  # type: ignore[method-assign]

    async with manager:
        pass

    assert shutdown_called


@pytest.mark.anyio
async def test_process_pool_manager_context_manager_with_task() -> None:
    async with ProcessPoolManager(max_processes=2) as manager:
        result = await manager.submit_task(simple_function, 7)
        assert result == 14
