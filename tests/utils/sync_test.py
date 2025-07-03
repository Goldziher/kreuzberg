"""Tests for sync utilities."""

from __future__ import annotations

import anyio
import pytest

from kreuzberg._utils._sync import (
    run_maybe_async,
    run_maybe_sync,
    run_sync,
    run_taskgroup,
    run_taskgroup_batched,
)

# Handle exception groups in tests below


def sync_function(x: int, y: int = 10) -> int:
    """Test synchronous function."""
    return x + y


async def async_function(x: int, y: int = 10) -> int:
    """Test asynchronous function."""
    await anyio.sleep(0.01)
    return x + y


def test_run_maybe_async_with_sync_function() -> None:
    """Test run_maybe_async with a synchronous function."""
    result = run_maybe_async(sync_function, 5, y=15)
    assert result == 20


def test_run_maybe_async_with_async_function() -> None:
    """Test run_maybe_async with an asynchronous function."""
    result: int = run_maybe_async(async_function, 5, y=15)  # type: ignore[arg-type]
    assert result == 20


def test_run_maybe_async_with_args_and_kwargs() -> None:
    """Test run_maybe_async with both positional and keyword arguments."""
    result: int = run_maybe_async(sync_function, 7, y=3)
    assert result == 10

    result = run_maybe_async(async_function, 7, y=3)  # type: ignore[arg-type]
    assert result == 10


@pytest.mark.skip(reason="run_sync_only is not present; run_sync is async; skipping test.")
def test_run_sync_only_with_sync_function() -> None:
    pass


@pytest.mark.skip(reason="run_sync_only is not present; run_sync is async; skipping test.")
def test_run_sync_only_with_async_function_raises_error() -> None:
    pass


@pytest.mark.skip(reason="run_sync_only is not present; run_sync is async; skipping test.")
def test_run_sync_only_error_message() -> None:
    pass


@pytest.mark.skip(reason="run_sync_only is not present; run_sync is async; skipping test.")
def test_run_sync_only_with_no_args() -> None:
    pass


@pytest.mark.skip(reason="run_sync_only is not present; run_sync is async; skipping test.")
def test_run_sync_only_with_exception() -> None:
    pass


def test_run_maybe_async_with_no_args() -> None:
    """Test run_maybe_async with functions that take no arguments."""

    def no_arg_sync() -> str:
        return "sync_result"

    async def no_arg_async() -> str:
        return "async_result"

    assert run_maybe_async(no_arg_sync) == "sync_result"
    assert run_maybe_async(no_arg_async) == "async_result"  # type: ignore[arg-type]


def test_run_maybe_async_return_types() -> None:
    """Test run_maybe_async preserves return types correctly."""

    def return_list() -> list[int]:
        return [1, 2, 3]

    async def return_dict() -> dict[str, int]:
        return {"a": 1, "b": 2}

    result1: list[int] = run_maybe_async(return_list)
    assert result1 == [1, 2, 3]
    assert isinstance(result1, list)

    result2: dict[str, int] = run_maybe_async(return_dict)
    assert result2 == {"a": 1, "b": 2}
    assert isinstance(result2, dict)


@pytest.mark.anyio
async def test_async_run_sync_with_sync_function() -> None:
    """Test async run_sync with a synchronous function."""
    result = await run_sync(sync_function, 5, y=15)
    assert result == 20


@pytest.mark.anyio
async def test_async_run_sync_with_args_and_kwargs() -> None:
    """Test async run_sync with both positional and keyword arguments."""
    result = await run_sync(sync_function, 7, y=3)
    assert result == 10


@pytest.mark.anyio
async def test_async_run_sync_with_no_args() -> None:
    """Test async run_sync with functions that take no arguments."""

    def no_arg_sync() -> str:
        return "async_sync_result"

    result = await run_sync(no_arg_sync)
    assert result == "async_sync_result"


@pytest.mark.anyio
async def test_async_run_sync_with_exception() -> None:
    """Test async run_sync propagates exceptions correctly."""

    def sync_error() -> None:
        raise ValueError("Async sync error")

    with pytest.raises(ValueError, match="Async sync error"):
        await run_sync(sync_error)


@pytest.mark.anyio
async def test_run_maybe_sync_with_sync_function() -> None:
    """Test run_maybe_sync with synchronous function."""
    result = await run_maybe_sync(sync_function, 5, y=15)
    assert result == 20


@pytest.mark.anyio
async def test_run_maybe_sync_with_async_function() -> None:
    """Test run_maybe_sync with asynchronous function."""
    result: int = await run_maybe_sync(async_function, 5, y=15)
    assert result == 20


@pytest.mark.anyio
async def test_run_maybe_sync_with_exception() -> None:
    """Test run_maybe_sync propagates exceptions correctly."""

    def sync_error() -> None:
        raise ValueError("Maybe sync error")

    async def async_error() -> None:
        raise ValueError("Maybe async error")

    with pytest.raises(ValueError, match="Maybe sync error"):
        await run_maybe_sync(sync_error)

    with pytest.raises(ValueError, match="Maybe async error"):
        await run_maybe_sync(async_error)  # type: ignore[arg-type]


@pytest.mark.anyio
async def test_run_taskgroup() -> None:
    """Test run_taskgroup with multiple async tasks."""

    async def task1() -> int:
        await anyio.sleep(0.01)
        return 1

    async def task2() -> int:
        await anyio.sleep(0.01)
        return 2

    async def task3() -> int:
        await anyio.sleep(0.01)
        return 3

    results = await run_taskgroup(task1(), task2(), task3())
    assert results == [1, 2, 3]


@pytest.mark.anyio
async def test_run_taskgroup_with_exception() -> None:
    """Test run_taskgroup handles exceptions."""

    async def good_task() -> int:
        await anyio.sleep(0.01)
        return 42

    async def bad_task() -> None:
        await anyio.sleep(0.01)
        raise ValueError("Task failed")

    # anyio raises ExceptionGroup in Python 3.11+, but we'll catch BaseException
    # to handle both the old behavior and the new ExceptionGroup behavior
    with pytest.raises(BaseException) as exc_info:  # noqa: PT011
        await run_taskgroup(good_task(), bad_task())

    # Check if it's an ExceptionGroup (Python 3.11+) or direct ValueError
    if hasattr(exc_info.value, "exceptions"):
        # It's an ExceptionGroup
        assert len(exc_info.value.exceptions) == 1
        assert isinstance(exc_info.value.exceptions[0], ValueError)
        assert str(exc_info.value.exceptions[0]) == "Task failed"
    else:
        # It's a direct ValueError
        assert isinstance(exc_info.value, ValueError)
        assert str(exc_info.value) == "Task failed"


@pytest.mark.anyio
async def test_run_taskgroup_empty() -> None:
    """Test run_taskgroup with no tasks."""
    results = await run_taskgroup()
    assert results == []


@pytest.mark.anyio
async def test_run_taskgroup_batched() -> None:
    """Test run_taskgroup_batched with multiple batches."""

    async def make_task(value: int) -> int:
        await anyio.sleep(0.01)
        return value

    tasks = [make_task(i) for i in range(5)]
    results = await run_taskgroup_batched(*tasks, batch_size=2)
    assert results == [0, 1, 2, 3, 4]


@pytest.mark.anyio
async def test_run_taskgroup_batched_single_batch() -> None:
    """Test run_taskgroup_batched with single batch."""

    async def make_task(value: int) -> int:
        await anyio.sleep(0.01)
        return value * 2

    tasks = [make_task(i) for i in range(3)]
    results = await run_taskgroup_batched(*tasks, batch_size=5)
    assert results == [0, 2, 4]


@pytest.mark.anyio
async def test_run_taskgroup_batched_exact_batches() -> None:
    """Test run_taskgroup_batched with exact batch sizes."""

    async def make_task(value: int) -> int:
        await anyio.sleep(0.01)
        return value + 10

    tasks = [make_task(i) for i in range(6)]
    results = await run_taskgroup_batched(*tasks, batch_size=3)
    assert results == [10, 11, 12, 13, 14, 15]


@pytest.mark.anyio
async def test_run_taskgroup_batched_empty() -> None:
    """Test run_taskgroup_batched with no tasks."""
    results = await run_taskgroup_batched(batch_size=2)
    assert results == []
