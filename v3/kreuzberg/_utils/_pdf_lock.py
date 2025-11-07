from __future__ import annotations

import hashlib
import threading
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING, Any
from weakref import WeakValueDictionary

if TYPE_CHECKING:
    from collections.abc import Generator


_PYPDFIUM_LOCK = threading.RLock()


_FILE_LOCKS_CACHE = WeakValueDictionary[str, threading.RLock]()
_FILE_LOCKS_LOCK = threading.Lock()


def _get_file_key(file_path: Path | str) -> str:
    path_str = str(Path(file_path).resolve())
    return hashlib.md5(path_str.encode()).hexdigest()  # noqa: S324


def _get_file_lock(file_path: Path | str) -> threading.RLock:
    file_key = _get_file_key(file_path)

    with _FILE_LOCKS_LOCK:
        if file_key in _FILE_LOCKS_CACHE:
            return _FILE_LOCKS_CACHE[file_key]

        lock = threading.RLock()
        _FILE_LOCKS_CACHE[file_key] = lock
        return lock


@contextmanager
def pypdfium_lock() -> Generator[None, None, None]:
    with _PYPDFIUM_LOCK:
        yield


@contextmanager
def pypdfium_file_lock(file_path: Path | str) -> Generator[None, None, None]:
    lock = _get_file_lock(file_path)
    with lock:
        yield


def with_pypdfium_lock(func: Any) -> Any:
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with pypdfium_lock():
            return func(*args, **kwargs)

    return wrapper
