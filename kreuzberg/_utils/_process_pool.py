"""Process pool utilities for CPU-intensive operations."""

from __future__ import annotations

import multiprocessing as mp
from concurrent.futures import ProcessPoolExecutor
from contextlib import contextmanager
from typing import TYPE_CHECKING, Any, TypeVar

if TYPE_CHECKING:
    from collections.abc import Callable, Generator

T = TypeVar("T")


_PROCESS_POOL: ProcessPoolExecutor | None = None
_POOL_SIZE = max(1, mp.cpu_count() - 1)


def _init_process_pool() -> ProcessPoolExecutor:
    """Initialize the global process pool."""
    global _PROCESS_POOL
    if _PROCESS_POOL is None:
        _PROCESS_POOL = ProcessPoolExecutor(max_workers=_POOL_SIZE)
    return _PROCESS_POOL


@contextmanager
def process_pool() -> Generator[ProcessPoolExecutor, None, None]:
    """Get the global process pool."""
    pool = _init_process_pool()
    try:
        yield pool
    except Exception:  # noqa: BLE001
        shutdown_process_pool()
        pool = _init_process_pool()
        yield pool


def submit_to_process_pool(func: Callable[..., T], *args: Any, **kwargs: Any) -> T:
    """Submit a function to the process pool and wait for result."""
    with process_pool() as pool:
        future = pool.submit(func, *args, **kwargs)
        return future.result()


def shutdown_process_pool() -> None:
    """Shutdown the global process pool."""
    global _PROCESS_POOL
    if _PROCESS_POOL is not None:
        _PROCESS_POOL.shutdown(wait=True)
        _PROCESS_POOL = None


def _extract_pdf_text_worker(pdf_path: str) -> tuple[str, str]:
    """Worker function for extracting PDF text in a separate process."""
    import pypdfium2

    pdf = None
    try:
        pdf = pypdfium2.PdfDocument(pdf_path)
        text_parts = []
        for page in pdf:
            text_page = page.get_textpage()
            text = text_page.get_text_range()
            text_parts.append(text)
            text_page.close()
            page.close()
        return (pdf_path, "".join(text_parts))
    except Exception as e:  # noqa: BLE001
        return (pdf_path, f"ERROR: {e}")
    finally:
        if pdf:
            pdf.close()


def _extract_pdf_images_worker(pdf_path: str, scale: float = 4.25) -> tuple[str, list[bytes]]:
    """Worker function for converting PDF to images in a separate process."""
    import io

    import pypdfium2

    pdf = None
    try:
        pdf = pypdfium2.PdfDocument(pdf_path)
        image_bytes = []
        for page in pdf:
            bitmap = page.render(scale=scale)
            pil_image = bitmap.to_pil()
            img_bytes = io.BytesIO()
            pil_image.save(img_bytes, format="PNG")
            image_bytes.append(img_bytes.getvalue())
            bitmap.close()
            page.close()
        return (pdf_path, image_bytes)
    except Exception:  # noqa: BLE001
        return (pdf_path, [])
    finally:
        if pdf:
            pdf.close()
