from __future__ import annotations

import contextlib
from typing import TYPE_CHECKING

import pypdfium2

from kreuzberg._utils._pdf_lock import pypdfium_file_lock
from kreuzberg._utils._sync import run_sync

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import AsyncGenerator, Generator
    from pathlib import Path


@contextlib.asynccontextmanager
async def pdf_document(file_path: Path) -> AsyncGenerator[pypdfium2.PdfDocument, None]:
    document = None
    try:
        with pypdfium_file_lock(file_path):
            document = await run_sync(pypdfium2.PdfDocument, str(file_path))
            yield document
    finally:
        if document:
            with pypdfium_file_lock(file_path), contextlib.suppress(Exception):
                await run_sync(document.close)


@contextlib.contextmanager
def pdf_document_sync(file_path: Path) -> Generator[pypdfium2.PdfDocument, None, None]:
    document = None
    try:
        with pypdfium_file_lock(file_path):
            document = pypdfium2.PdfDocument(str(file_path))
            yield document
    finally:
        if document:
            with pypdfium_file_lock(file_path), contextlib.suppress(Exception):
                document.close()


@contextlib.contextmanager
def pdf_resources_sync(*resources: object) -> Generator[None, None, None]:
    try:
        yield
    finally:
        for resource in resources:
            with contextlib.suppress(Exception):
                if hasattr(resource, "close"):
                    resource.close()


@contextlib.contextmanager
def image_resources(*images: object) -> Generator[None, None, None]:
    try:
        yield
    finally:
        for image in images:
            with contextlib.suppress(Exception):
                if hasattr(image, "close"):
                    image.close()
