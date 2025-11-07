from __future__ import annotations

import anyio
import pytest
from kreuzberg._utils._sync import (
    run_maybe_async,
    run_maybe_sync,
    run_sync,
    run_sync_only,
    run_taskgroup,
    run_taskgroup_batched,
)


def sync_function(x: int, y: int = 10) -> int:
    return x + y


async def async_function(x: int, y: int = 10) -> int:
    await anyio.sleep(0.01)
    return x + y


def test_run_maybe_async_with_sync_function() -> None:
    result = run_maybe_async(sync_function, 5, y=15)
    assert result == 20


def test_run_maybe_async_with_async_function() -> None:
    result: int = run_maybe_async(async_function, 5, y=15)  # type: ignore[arg-type]
    assert result == 20


def test_run_maybe_async_with_args_and_kwargs() -> None:
    result: int = run_maybe_async(sync_function, 7, y=3)
    assert result == 10

    result = run_maybe_async(async_function, 7, y=3)  # type: ignore[arg-type]
    assert result == 10


def test_run_sync_only_with_sync_function() -> None:
    result = run_sync_only(sync_function, 8, y=12)
    assert result == 20


def test_run_sync_only_with_async_function_raises_error() -> None:
    with pytest.raises(RuntimeError, match="Cannot run async function async_function in sync-only context"):
        run_sync_only(async_function, 5, y=15)  # type: ignore[arg-type]


def test_run_sync_only_error_message() -> None:
    async def custom_async_function() -> None:
        pass

    with pytest.raises(RuntimeError) as exc_info:
        run_sync_only(custom_async_function)  # type: ignore[arg-type]

    assert "custom_async_function" in str(exc_info.value)
    assert "sync-only context" in str(exc_info.value)


def test_run_maybe_async_with_no_args() -> None:
    def no_arg_sync() -> str:
        return "sync_result"

    async def no_arg_async() -> str:
        return "async_result"

    assert run_maybe_async(no_arg_sync) == "sync_result"
    assert run_maybe_async(no_arg_async) == "async_result"  # type: ignore[arg-type]


def test_run_sync_only_with_no_args() -> None:
    def no_arg_sync() -> str:
        return "sync_only_result"

    assert run_sync_only(no_arg_sync) == "sync_only_result"


def test_run_maybe_async_with_exception() -> None:
    def sync_error() -> None:
        raise ValueError("Sync error")

    async def async_error() -> None:
        raise ValueError("Async error")

    with pytest.raises(ValueError, match="Sync error"):
        run_maybe_async(sync_error)

    with pytest.raises(ValueError, match="Async error"):
        run_maybe_async(async_error)  # type: ignore[arg-type]


def test_run_sync_only_with_exception() -> None:
    def sync_error() -> None:
        raise ValueError("Sync only error")

    with pytest.raises(ValueError, match="Sync only error"):
        run_sync_only(sync_error)


def test_run_maybe_async_return_types() -> None:
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
    result = await run_sync(sync_function, 5, y=15)
    assert result == 20


@pytest.mark.anyio
async def test_async_run_sync_with_args_and_kwargs() -> None:
    result = await run_sync(sync_function, 7, y=3)
    assert result == 10


@pytest.mark.anyio
async def test_async_run_sync_with_no_args() -> None:
    def no_arg_sync() -> str:
        return "async_sync_result"

    result = await run_sync(no_arg_sync)
    assert result == "async_sync_result"


@pytest.mark.anyio
async def test_async_run_sync_with_exception() -> None:
    def sync_error() -> None:
        raise ValueError("Async sync error")

    with pytest.raises(ValueError, match="Async sync error"):
        await run_sync(sync_error)


@pytest.mark.anyio
async def test_run_maybe_sync_with_sync_function() -> None:
    result = await run_maybe_sync(sync_function, 5, y=15)
    assert result == 20


@pytest.mark.anyio
async def test_run_maybe_sync_with_async_function() -> None:
    result: int = await run_maybe_sync(async_function, 5, y=15)
    assert result == 20


@pytest.mark.anyio
async def test_run_maybe_sync_with_exception() -> None:
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
    async def good_task() -> int:
        await anyio.sleep(0.01)
        return 42

    async def bad_task() -> None:
        await anyio.sleep(0.01)
        raise ValueError("Task failed")

    with pytest.raises(BaseException) as exc_info:
        await run_taskgroup(good_task(), bad_task())

    if hasattr(exc_info.value, "exceptions"):
        assert len(exc_info.value.exceptions) == 1
        assert isinstance(exc_info.value.exceptions[0], ValueError)
        assert str(exc_info.value.exceptions[0]) == "Task failed"
    else:
        assert isinstance(exc_info.value, ValueError)
        assert str(exc_info.value) == "Task failed"


@pytest.mark.anyio
async def test_run_taskgroup_empty() -> None:
    results = await run_taskgroup()
    assert results == []


@pytest.mark.anyio
async def test_run_taskgroup_batched() -> None:
    async def make_task(value: int) -> int:
        await anyio.sleep(0.01)
        return value

    tasks = [make_task(i) for i in range(5)]
    results = await run_taskgroup_batched(*tasks, batch_size=2)
    assert results == [0, 1, 2, 3, 4]


@pytest.mark.anyio
async def test_run_taskgroup_batched_single_batch() -> None:
    async def make_task(value: int) -> int:
        await anyio.sleep(0.01)
        return value * 2

    tasks = [make_task(i) for i in range(3)]
    results = await run_taskgroup_batched(*tasks, batch_size=5)
    assert results == [0, 2, 4]


@pytest.mark.anyio
async def test_run_taskgroup_batched_exact_batches() -> None:
    async def make_task(value: int) -> int:
        await anyio.sleep(0.01)
        return value + 10

    tasks = [make_task(i) for i in range(6)]
    results = await run_taskgroup_batched(*tasks, batch_size=3)
    assert results == [10, 11, 12, 13, 14, 15]


@pytest.mark.anyio
async def test_run_taskgroup_batched_empty() -> None:
    results = await run_taskgroup_batched(batch_size=2)
    assert results == []
