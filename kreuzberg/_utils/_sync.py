from __future__ import annotations

from functools import partial
from inspect import isawaitable, iscoroutinefunction
from typing import TYPE_CHECKING, Any, TypeVar, cast

import anyio
from anyio import create_task_group
from anyio.to_thread import run_sync as any_io_run_sync

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Awaitable, Callable

from typing import ParamSpec

T = TypeVar("T")
P = ParamSpec("P")


async def run_sync(sync_fn: Callable[P, T], *args: P.args, **kwargs: P.kwargs) -> T:
    """Run a synchronous function in an asynchronous context.

    Args:
        sync_fn: The synchronous function to run.
        *args: The positional arguments to pass to the function.
        **kwargs: The keyword arguments to pass to the function.

    Returns:
        The result of the synchronous function.
    """
    handler = partial(sync_fn, **kwargs)
    return cast("T", await any_io_run_sync(handler, *args, abandon_on_cancel=True))  # pyright: ignore [reportCallIssue]


async def run_taskgroup(*async_tasks: Awaitable[Any]) -> list[Any]:
    """Run a list of coroutines concurrently.

    Args:
        *async_tasks: The list of coroutines to run.

    Returns:
        The results of the coroutines.
    """
    results: list[Any] = [None] * len(async_tasks)

    async def run_task(index: int, task: Awaitable[T]) -> None:
        results[index] = await task

    async with create_task_group() as tg:
        for i, t in enumerate(async_tasks):
            tg.start_soon(run_task, i, t)

    return results


async def run_taskgroup_batched(*async_tasks: Awaitable[Any], batch_size: int) -> list[Any]:
    """Run a list of coroutines concurrently in batches.

    Args:
        *async_tasks: The list of coroutines to run.
        batch_size: The size of each batch.

    Returns:
        The results of the coroutines.
    """
    results: list[Any] = []

    for i in range(0, len(async_tasks), batch_size):
        batch = async_tasks[i : i + batch_size]
        results.extend(await run_taskgroup(*batch))

    return results


async def run_maybe_sync(fn: Callable[P, T | Awaitable[T]], *args: P.args, **kwargs: P.kwargs) -> T:
    """Executes a callable function and handles both synchronous and asynchronous
    results.

    This function invokes the provided callable `sync_fn` with the given
    arguments and keyword arguments. If the result of `sync_fn` is awaitable,
    it awaits the result before returning it. Otherwise, the result is returned
    directly.

    Args:
        fn: The callable to be executed. It can produce either a
            synchronous or asynchronous result.
        *args: Positional arguments to pass to `sync_fn`.
        **kwargs: Keyword arguments to pass to `sync_fn`.

    Returns:
        The result of `sync_fn` invocation. If the result is awaitable, the
        awaited value is returned. Otherwise, the synchronous result is
        returned.
    """
    result = fn(*args, **kwargs)
    if isawaitable(result):
        return cast("T", await result)
    return result


def run_maybe_async(fn: Callable[P, T | Awaitable[T]], *args: P.args, **kwargs: P.kwargs) -> T:
    """Runs a synchronous or asynchronous function, resolving the output.

    Determines if the provided function is synchronous or asynchronous. If synchronous,
    executes it directly. If asynchronous, it runs the function within the event loop
    using anyio. The return value is resolved regardless of the function type.

    Args:
        fn: The function to be executed, which can
            either be synchronous or asynchronous.
        *args: Positional arguments to be passed to the function.
        **kwargs: Keyword arguments to be passed to the function.

    Returns:
        T: The return value of the executed function, resolved if asynchronous.
    """
    return cast("T", fn(*args, **kwargs) if not iscoroutinefunction(fn) else anyio.run(partial(fn, **kwargs), *args))
