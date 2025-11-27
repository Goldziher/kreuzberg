from abc import ABC, abstractmethod
from pathlib import Path
from typing import Generic, TypeVar

from PIL.Image import Image

from kreuzberg._types import ExtractionResult
from kreuzberg._utils._sync import run_taskgroup

try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


T = TypeVar("T")


class OCRBackend(ABC, Generic[T]):
    @abstractmethod
    async def process_image(self, image: Image, **kwargs: Unpack[T]) -> ExtractionResult: ...

    @abstractmethod
    async def process_file(self, path: Path, **kwargs: Unpack[T]) -> ExtractionResult: ...

    @abstractmethod
    def process_image_sync(self, image: Image, **kwargs: Unpack[T]) -> ExtractionResult: ...

    @abstractmethod
    def process_file_sync(self, path: Path, **kwargs: Unpack[T]) -> ExtractionResult: ...

    def process_batch_sync(self, paths: list[Path], **kwargs: Unpack[T]) -> list[ExtractionResult]:
        return [self.process_file_sync(path, **kwargs) for path in paths]  # pragma: no cover

    async def process_batch(self, paths: list[Path], **kwargs: Unpack[T]) -> list[ExtractionResult]:
        tasks = [self.process_file(path, **kwargs) for path in paths]
        return await run_taskgroup(*tasks)  # pragma: no cover

    def __hash__(self) -> int:
        return hash(type(self).__name__)  # pragma: no cover
