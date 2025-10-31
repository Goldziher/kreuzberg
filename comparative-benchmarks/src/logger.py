from __future__ import annotations

import logging
from enum import Enum
from functools import cache
from typing import Any

from rich.console import Console
from rich.logging import RichHandler


class LogLevel(str, Enum):
    DEBUG = "debug"
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"


@cache
def _get_console() -> Console:
    return Console()


class BenchmarkLogger:
    def __init__(
        self, name: str = "benchmark", level: LogLevel = LogLevel.INFO
    ) -> None:
        self.console = _get_console()
        self.logger = logging.getLogger(name)
        self.logger.setLevel(getattr(logging, level.upper()))

        self.logger.handlers.clear()

        rich_handler = RichHandler(console=self.console, show_path=False)
        rich_handler.setFormatter(logging.Formatter(fmt="%(message)s", datefmt="[%X]"))
        self.logger.addHandler(rich_handler)

    def debug(self, message: str, **kwargs: Any) -> None:
        self._log(LogLevel.DEBUG, message, **kwargs)

    def info(self, message: str, **kwargs: Any) -> None:
        self._log(LogLevel.INFO, message, **kwargs)

    def warning(self, message: str, **kwargs: Any) -> None:
        self._log(LogLevel.WARNING, message, **kwargs)

    def error(self, message: str, **kwargs: Any) -> None:
        self._log(LogLevel.ERROR, message, **kwargs)

    def _log(self, level: LogLevel, message: str, **kwargs: Any) -> None:
        if kwargs:
            extra_info = " | ".join(f"{k}={v}" for k, v in kwargs.items())
            message = f"{message} [{extra_info}]"

        getattr(self.logger, level.value)(message)


def get_logger(
    name: str | None = None, level: LogLevel = LogLevel.INFO
) -> BenchmarkLogger:
    if name is None:
        import inspect

        frame = inspect.currentframe()
        if frame and frame.f_back:
            name = frame.f_back.f_globals.get("__name__", "benchmark")

    return BenchmarkLogger(name or "benchmark", level)
