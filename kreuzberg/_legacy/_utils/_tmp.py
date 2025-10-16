from __future__ import annotations

import contextlib
import os
import tempfile
from contextlib import suppress
from pathlib import Path
from tempfile import NamedTemporaryFile
from typing import TYPE_CHECKING

from anyio import Path as AsyncPath

from kreuzberg._utils._sync import run_sync

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import AsyncGenerator, Callable, Coroutine, Generator


async def create_temp_file(
    extension: str, content: bytes | None = None
) -> tuple[Path, Callable[[], Coroutine[None, None, None]]]:
    file = await run_sync(NamedTemporaryFile, suffix=extension, delete=False)
    if content:
        await AsyncPath(file.name).write_bytes(content)
    await run_sync(file.close)

    async def unlink() -> None:
        with suppress(OSError, PermissionError):
            await AsyncPath(file.name).unlink(missing_ok=True)

    return Path(file.name), unlink


@contextlib.asynccontextmanager
async def temporary_file(extension: str, content: bytes | None = None) -> AsyncGenerator[Path, None]:
    file_path, unlink = await create_temp_file(extension, content)
    try:
        yield file_path
    finally:
        await unlink()


@contextlib.contextmanager
def temporary_file_sync(extension: str, content: bytes | None = None) -> Generator[Path, None, None]:
    fd, temp_path = tempfile.mkstemp(suffix=extension)
    try:
        if content:
            with os.fdopen(fd, "wb") as f:
                f.write(content)
        else:
            os.close(fd)
        yield Path(temp_path)
    finally:
        with suppress(OSError, PermissionError):
            Path(temp_path).unlink()


@contextlib.contextmanager
def temporary_directory() -> Generator[Path, None, None]:
    with tempfile.TemporaryDirectory() as temp_dir:
        yield Path(temp_dir)
