from __future__ import annotations

from json import dumps
from typing import Any


class KreuzbergError(Exception):
    """Base exception for all Kreuzberg errors."""

    __slots__ = ("context",)

    context: Any
    """The context of the error."""

    def __init__(self, message: str, *, context: Any = None) -> None:
        self.context = context
        super().__init__(message)

    def _serialize_context(self, obj: Any) -> Any:
        if isinstance(obj, bytes):
            return obj.decode("utf-8", errors="replace")
        if isinstance(obj, dict):
            return {k: self._serialize_context(v) for k, v in obj.items()}
        if isinstance(obj, (list, tuple)):
            return [self._serialize_context(x) for x in obj]
        if isinstance(obj, Exception):
            return {
                "type": obj.__class__.__name__,
                "message": str(obj),
            }
        return obj

    def __str__(self) -> str:
        """Return a string representation of the exception."""
        if self.context:
            serialized_context = self._serialize_context(self.context)
            ctx = f"\n\nContext: {dumps(serialized_context)}"
        else:
            ctx = ""

        return f"{self.__class__.__name__}: {super().__str__()}{ctx}"


class ParsingError(KreuzbergError):
    """Raised when a parsing error occurs."""

    __slots__ = ()


class ValidationError(KreuzbergError):
    """Raised when a validation error occurs."""

    __slots__ = ()


class MissingDependencyError(KreuzbergError):
    """Raised when a dependency is missing."""

    __slots__ = ()

    @classmethod
    def create_for_package(
        cls, *, dependency_group: str, functionality: str, package_name: str
    ) -> MissingDependencyError:
        """Creates a MissingDependencyError for a specified package and functionality.

        This class method generates an error message to notify users about a
        missing package dependency required for specific functionality. The error
        message includes details about the missing package and the optional
        dependency group required for installation.

        Args:
            dependency_group: The name of the optional dependency group that includes
                the required package.
            functionality: The functionality that requires the missing package.
            package_name: The name of the missing package.

        Returns:
            MissingDependencyError: A customized error indicating the missing
            dependency and how to resolve it.

        """
        return MissingDependencyError(
            f"The package '{package_name}' is required to use {functionality}. You can install using the provided optional dependency group by installing `kreuzberg['{dependency_group}']`."
        )


class OCRError(KreuzbergError):
    """Raised when an OCR error occurs."""

    __slots__ = ()
