from __future__ import annotations

import multiprocessing as mp
from concurrent.futures import ProcessPoolExecutor
from typing import TYPE_CHECKING, Any
from unittest.mock import Mock, patch

from kreuzberg._utils._process_pool import (
    _POOL_SIZE,
    _extract_pdf_images_worker,
    _extract_pdf_text_worker,
    _get_process_pool,
    _process_pool_ref,
    process_pool,
    shutdown_process_pool,
    submit_to_process_pool,
)
from PIL import Image

if TYPE_CHECKING:
    from pathlib import Path


def _simple_add(x: int, y: int) -> int:
    return x + y


def _process_data(data: dict[str, Any]) -> dict[str, Any]:
    return {"processed": True, "value": data.get("value", 0) * 2}


def _failing_function() -> None:
    raise ValueError("Test error")


def _compute_square(n: int) -> int:
    return n * n


def test_pool_size() -> None:
    expected_size = mp.cpu_count()
    assert expected_size == _POOL_SIZE


def test_init_process_pool() -> None:
    shutdown_process_pool()

    pool = _get_process_pool()
    assert isinstance(pool, ProcessPoolExecutor)
    assert pool._max_workers == _POOL_SIZE  # type: ignore[attr-defined]

    same_pool = _get_process_pool()
    assert same_pool is pool

    shutdown_process_pool()


def test_process_pool_context_manager() -> None:
    shutdown_process_pool()

    with process_pool() as pool:
        assert isinstance(pool, ProcessPoolExecutor)

    with process_pool() as pool2:
        assert isinstance(pool2, ProcessPoolExecutor)

    shutdown_process_pool()


def test_process_pool_error_recovery() -> None:
    shutdown_process_pool()

    with patch("kreuzberg._utils._process_pool.ProcessPoolExecutor") as mock_pool_class:
        mock_pool = Mock(spec=ProcessPoolExecutor)
        mock_pool_class.return_value = mock_pool

        yields = [Exception("Pool error"), mock_pool]

        def side_effect() -> Any:
            if yields:
                result = yields.pop(0)
                if isinstance(result, Exception):
                    raise result
                return result
            return None

        with patch("kreuzberg._utils._process_pool._get_process_pool", side_effect=side_effect):
            try:
                with process_pool():
                    pass
            except Exception:
                pass

    shutdown_process_pool()


def test_submit_to_process_pool() -> None:
    shutdown_process_pool()

    result = submit_to_process_pool(_simple_add, 5, 10)
    assert result == 15

    result = submit_to_process_pool(_simple_add, x=3, y=7)
    assert result == 10

    shutdown_process_pool()


def test_shutdown_process_pool() -> None:
    _get_process_pool()

    assert _process_pool_ref.is_initialized()

    shutdown_process_pool()

    assert not _process_pool_ref.is_initialized()

    shutdown_process_pool()


def test_extract_pdf_text_worker(searchable_pdf: Path) -> None:
    path_str = str(searchable_pdf)

    pdf_path_str, text = _extract_pdf_text_worker(path_str)

    assert pdf_path_str == path_str
    assert isinstance(text, str)
    assert len(text) > 0
    assert "Lorem ipsum" in text


def test_extract_pdf_text_worker_error() -> None:
    result = _extract_pdf_text_worker("/nonexistent/file.pdf")

    assert result[0] == "/nonexistent/file.pdf"
    assert result[1].startswith("ERROR:")


def test_extract_pdf_text_worker_with_mock() -> None:
    with patch("pypdfium2.PdfDocument") as mock_pdf_class:
        mock_pdf = Mock()
        mock_page = Mock()
        mock_text_page = Mock()

        mock_pdf.__iter__ = Mock(return_value=iter([mock_page]))
        mock_pdf_class.return_value = mock_pdf
        mock_page.get_textpage.return_value = mock_text_page
        mock_text_page.get_text_bounded.return_value = "Test text"

        result = _extract_pdf_text_worker("test.pdf")

        assert result == ("test.pdf", "Test text")

        mock_text_page.close.assert_called_once()
        mock_page.close.assert_called_once()
        mock_pdf.close.assert_called_once()


def test_extract_pdf_images_worker(searchable_pdf: Path) -> None:
    path_str = str(searchable_pdf)

    pdf_path_str, images = _extract_pdf_images_worker(path_str, scale=2.0)

    assert pdf_path_str == path_str
    assert isinstance(images, list)
    assert len(images) > 0

    import io

    img = Image.open(io.BytesIO(images[0]))
    assert img.format == "PNG"


def test_extract_pdf_images_worker_error() -> None:
    result = _extract_pdf_images_worker("/nonexistent/file.pdf")

    assert result[0] == "/nonexistent/file.pdf"
    assert result[1] == []


def test_extract_pdf_images_worker_with_mock() -> None:
    with patch("pypdfium2.PdfDocument") as mock_pdf_class:
        mock_pdf = Mock()
        mock_page = Mock()
        mock_bitmap = Mock()
        mock_pil_image = Mock(spec=Image.Image)

        mock_pdf.__iter__ = Mock(return_value=iter([mock_page]))
        mock_pdf_class.return_value = mock_pdf
        mock_page.render.return_value = mock_bitmap
        mock_bitmap.to_pil.return_value = mock_pil_image

        saved_bytes = b"fake png data"

        def mock_save(buffer: Any, format: Any = None, **kwargs: Any) -> None:  # noqa: A002
            buffer.write(saved_bytes)

        mock_pil_image.save = mock_save

        result = _extract_pdf_images_worker("test.pdf", scale=3.0)

        assert result[0] == "test.pdf"
        assert len(result[1]) == 1
        assert result[1][0] == saved_bytes

        mock_page.render.assert_called_once_with(scale=3.0)
        mock_bitmap.close.assert_called_once()
        mock_page.close.assert_called_once()
        mock_pdf.close.assert_called_once()


def test_process_pool_concurrent_usage() -> None:
    shutdown_process_pool()

    results = []
    for i in range(5):
        result = submit_to_process_pool(_compute_square, i)
        results.append(result)

    assert results == [0, 1, 4, 9, 16]

    shutdown_process_pool()
