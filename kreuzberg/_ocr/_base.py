from abc import ABC, abstractmethod
from pathlib import Path
from typing import Generic, TypeVar

from PIL.Image import Image

from kreuzberg._types import ExtractionResult

try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


T = TypeVar("T")


class OCRBackend(ABC, Generic[T]):
    """Abstract base class for Optical Character Recognition (OCR) backend implementations.

    This class provides the blueprint for OCR backend implementations,
    offering both synchronous and asynchronous methods to process images
    and files for text extraction.
    """

    __slots__ = ()

    @abstractmethod
    async def process_image(self, image: Image, **kwargs: Unpack[T]) -> ExtractionResult:
        """Asynchronously process an image and extract its text and metadata.

        Args:
            image: An instance of PIL.Image representing the input image.
            **kwargs: Any kwargs related to the given backend

        Returns:
            The extraction result object
        """
        ...

    @abstractmethod
    async def process_file(self, path: Path, **kwargs: Unpack[T]) -> ExtractionResult:
        """Asynchronously process a file and extract its text and metadata.

        Args:
            path: A Path object representing the file to be processed.
            **kwargs: Any kwargs related to the given backend

        Returns:
            The extraction result object
        """
        ...

    def __hash__(self) -> int:
        """Hash function for allowing caching."""
        return hash(type(self).__name__)
