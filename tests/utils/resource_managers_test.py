from __future__ import annotations

import contextlib
from pathlib import Path  # noqa: TC003  # Used in fixture at runtime
from unittest.mock import MagicMock, Mock, patch

import pytest

from kreuzberg._utils._resource_managers import (
    image_resources,
    pdf_document,
    pdf_document_sync,
    pdf_resources_sync,
)


@pytest.fixture
def mock_pdf_path(tmp_path: Path) -> Path:
    pdf_file = tmp_path / "test.pdf"
    pdf_file.write_bytes(b"%PDF-1.4\n%%EOF\n")
    return pdf_file


def test_pdf_document_sync_normal_flow(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    with patch("kreuzberg._utils._resource_managers.pypdfium2.PdfDocument") as mock_doc_class:
        mock_document = MagicMock()
        mock_doc_class.return_value = mock_document

        with pdf_document_sync(mock_pdf_path) as doc:
            assert doc is mock_document

        mock_doc_class.assert_called_once_with(str(mock_pdf_path))
        mock_document.close.assert_called_once()


def test_pdf_document_sync_with_exception(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    with patch("kreuzberg._utils._resource_managers.pypdfium2.PdfDocument") as mock_doc_class:
        mock_document = MagicMock()
        mock_doc_class.return_value = mock_document

        with contextlib.suppress(ValueError):
            with pdf_document_sync(mock_pdf_path):
                raise ValueError("Test error")

        mock_document.close.assert_called_once()


def test_pdf_document_sync_close_fails(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    with patch("kreuzberg._utils._resource_managers.pypdfium2.PdfDocument") as mock_doc_class:
        mock_document = MagicMock()
        mock_document.close.side_effect = RuntimeError("Close failed")
        mock_doc_class.return_value = mock_document

        with pdf_document_sync(mock_pdf_path):
            pass


def test_pdf_document_sync_none_document(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    with patch("kreuzberg._utils._resource_managers.pypdfium2.PdfDocument") as mock_doc_class:
        mock_doc_class.side_effect = RuntimeError("Failed to open")

        with contextlib.suppress(RuntimeError):
            with pdf_document_sync(mock_pdf_path):
                pass


@pytest.mark.anyio
async def test_pdf_document_async_normal_flow(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    mock_document = MagicMock()

    async def mock_run_sync(func: object, *args: object, **kwargs: object) -> MagicMock:
        return mock_document

    with patch("kreuzberg._utils._resource_managers.run_sync", side_effect=mock_run_sync):
        async with pdf_document(mock_pdf_path) as doc:
            assert doc is mock_document


@pytest.mark.anyio
async def test_pdf_document_async_with_exception(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    mock_document = MagicMock()
    call_count = 0

    async def mock_run_sync(func: object, *args: object, **kwargs: object) -> MagicMock | None:
        nonlocal call_count
        call_count += 1
        if call_count == 1:
            return mock_document
        return None

    with patch("kreuzberg._utils._resource_managers.run_sync", side_effect=mock_run_sync):
        with contextlib.suppress(ValueError):
            async with pdf_document(mock_pdf_path):
                raise ValueError("Test error")


@pytest.mark.anyio
async def test_pdf_document_async_close_fails(mock_pdf_path: Path) -> None:
    # Mock pypdfium2.PdfDocument for testing ~keep
    mock_document = MagicMock()

    async def mock_run_sync(func: object, *args: object, **kwargs: object) -> MagicMock:
        if func == mock_document.close:
            raise RuntimeError("Close failed")
        return mock_document

    with patch("kreuzberg._utils._resource_managers.run_sync", side_effect=mock_run_sync):
        async with pdf_document(mock_pdf_path):
            pass


def test_pdf_resources_sync_normal_flow() -> None:
    mock_page = MagicMock()
    mock_textpage = MagicMock()
    mock_bitmap = MagicMock()

    with pdf_resources_sync(mock_page, mock_textpage, mock_bitmap):
        pass

    mock_page.close.assert_called_once()
    mock_textpage.close.assert_called_once()
    mock_bitmap.close.assert_called_once()


def test_pdf_resources_sync_with_exception() -> None:
    mock_resource = MagicMock()

    with contextlib.suppress(ValueError):
        with pdf_resources_sync(mock_resource):
            raise ValueError("Test error")

    mock_resource.close.assert_called_once()


def test_pdf_resources_sync_close_fails() -> None:
    mock_resource = MagicMock()
    mock_resource.close.side_effect = RuntimeError("Close failed")

    with pdf_resources_sync(mock_resource):
        pass


def test_pdf_resources_sync_no_close_method() -> None:
    mock_resource = Mock(spec=[])

    with pdf_resources_sync(mock_resource):
        pass


def test_pdf_resources_sync_empty() -> None:
    with pdf_resources_sync():
        pass


def test_image_resources_normal_flow() -> None:
    mock_image1 = MagicMock()
    mock_image2 = MagicMock()

    with image_resources(mock_image1, mock_image2):
        pass

    mock_image1.close.assert_called_once()
    mock_image2.close.assert_called_once()


def test_image_resources_with_exception() -> None:
    mock_image = MagicMock()

    with contextlib.suppress(ValueError):
        with image_resources(mock_image):
            raise ValueError("Test error")

    mock_image.close.assert_called_once()


def test_image_resources_close_fails() -> None:
    mock_image = MagicMock()
    mock_image.close.side_effect = RuntimeError("Close failed")

    with image_resources(mock_image):
        pass


def test_image_resources_no_close_method() -> None:
    mock_image = Mock(spec=[])

    with image_resources(mock_image):
        pass


def test_image_resources_empty() -> None:
    with image_resources():
        pass


def test_pdf_resources_sync_multiple_close_errors() -> None:
    mock_resource1 = MagicMock()
    mock_resource2 = MagicMock()
    mock_resource1.close.side_effect = RuntimeError("First close failed")
    mock_resource2.close.side_effect = RuntimeError("Second close failed")

    with pdf_resources_sync(mock_resource1, mock_resource2):
        pass

    mock_resource1.close.assert_called_once()
    mock_resource2.close.assert_called_once()


def test_image_resources_multiple_close_errors() -> None:
    mock_image1 = MagicMock()
    mock_image2 = MagicMock()
    mock_image1.close.side_effect = RuntimeError("First close failed")
    mock_image2.close.side_effect = RuntimeError("Second close failed")

    with image_resources(mock_image1, mock_image2):
        pass

    mock_image1.close.assert_called_once()
    mock_image2.close.assert_called_once()
