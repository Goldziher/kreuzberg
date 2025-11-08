"""Tests to improve coverage for kreuzberg/__init__.py."""

from __future__ import annotations

import contextlib
from pathlib import Path
from unittest.mock import patch

import pytest

from kreuzberg import (
    ExtractionConfig,
    MissingDependencyError,
    OcrConfig,
    _hash_kwargs,
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_files,
    batch_extract_files_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)


def test_hash_kwargs_with_serializable_dict() -> None:
    """Test _hash_kwargs with normal serializable dictionary."""
    kwargs = {"key": "value", "number": 42}
    hash1 = _hash_kwargs(kwargs)
    hash2 = _hash_kwargs(kwargs)

    assert hash1 == hash2
    assert isinstance(hash1, str)
    assert len(hash1) == 32


def test_hash_kwargs_with_unserializable_dict() -> None:
    """Test _hash_kwargs fallback for unserializable dictionary."""

    class UnserializableClass:
        def __init__(self) -> None:
            self.ref = self

    unserializable_obj = UnserializableClass()
    kwargs = {"key": unserializable_obj}
    hash1 = _hash_kwargs(kwargs)

    assert isinstance(hash1, str)
    assert len(hash1) == 32


def test_hash_kwargs_different_dicts_produce_different_hashes() -> None:
    """Test that different dictionaries produce different hashes."""
    hash1 = _hash_kwargs({"key": "value1"})
    hash2 = _hash_kwargs({"key": "value2"})

    assert hash1 != hash2


def test_extract_file_sync_with_none_config(docx_document: Path) -> None:
    """Test extract_file_sync uses default config when None."""
    result = extract_file_sync(docx_document, config=None)

    assert result is not None
    assert hasattr(result, "content")
    assert isinstance(result.content, str)


def test_extract_bytes_sync_with_none_config(docx_document: Path) -> None:
    """Test extract_bytes_sync uses default config when None."""
    with Path(docx_document).open("rb") as f:
        data = f.read()

    result = extract_bytes_sync(
        data,
        mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        config=None,
    )

    assert result is not None
    assert hasattr(result, "content")


def test_batch_extract_files_sync_with_none_config(docx_document: Path) -> None:
    """Test batch_extract_files_sync uses default config when None."""
    results = batch_extract_files_sync([docx_document], config=None)

    assert len(results) == 1
    assert results[0] is not None


def test_batch_extract_bytes_sync_with_none_config(docx_document: Path) -> None:
    """Test batch_extract_bytes_sync uses default config when None."""
    with Path(docx_document).open("rb") as f:
        data = f.read()

    results = batch_extract_bytes_sync(
        [data],
        mime_types=["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
        config=None,
    )

    assert len(results) == 1
    assert results[0] is not None


@pytest.mark.asyncio
async def test_extract_file_with_none_config(docx_document: Path) -> None:
    """Test async extract_file uses default config when None."""
    result = await extract_file(docx_document, config=None)

    assert result is not None
    assert hasattr(result, "content")


@pytest.mark.asyncio
async def test_extract_bytes_with_none_config(docx_document: Path) -> None:
    """Test async extract_bytes uses default config when None."""
    data = Path(docx_document).read_bytes()

    result = await extract_bytes(
        data,
        mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        config=None,
    )

    assert result is not None


@pytest.mark.asyncio
async def test_batch_extract_files_with_none_config(docx_document: Path) -> None:
    """Test async batch_extract_files uses default config when None."""
    results = await batch_extract_files([docx_document], config=None)

    assert len(results) == 1
    assert results[0] is not None


@pytest.mark.asyncio
async def test_batch_extract_bytes_with_none_config(docx_document: Path) -> None:
    """Test async batch_extract_bytes uses default config when None."""
    data = Path(docx_document).read_bytes()

    results = await batch_extract_bytes(
        [data],
        mime_types=["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
        config=None,
    )

    assert len(results) == 1
    assert results[0] is not None


def test_ensure_ocr_backend_registered_with_tesseract(docx_document: Path) -> None:
    """Test OCR backend registration skips for tesseract."""
    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))

    result = extract_file_sync(docx_document, config=config)
    assert result is not None


def test_ensure_ocr_backend_registered_with_easyocr_missing_dependency(docx_document: Path) -> None:
    """Test OCR backend registration raises for missing easyocr."""
    config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="en"))

    with patch.dict("sys.modules", {"kreuzberg.ocr.easyocr": None}):
        with pytest.raises(MissingDependencyError) as exc_info:
            extract_file_sync(docx_document, config=config)

        assert "easyocr" in str(exc_info.value).lower()


def test_ensure_ocr_backend_registered_with_paddleocr_missing_dependency(docx_document: Path) -> None:
    """Test OCR backend registration raises for missing paddleocr."""
    config = ExtractionConfig(ocr=OcrConfig(backend="paddleocr", language="en"))

    with patch.dict("sys.modules", {"kreuzberg.ocr.paddleocr": None}):
        with pytest.raises(MissingDependencyError) as exc_info:
            extract_file_sync(docx_document, config=config)

        assert "paddleocr" in str(exc_info.value).lower()


def test_ocr_backend_cache_eviction(docx_document: Path) -> None:
    """Test that OCR backend cache evicts oldest entry when full."""
    import kreuzberg

    kreuzberg._REGISTERED_OCR_BACKENDS.clear()

    configs = []
    for _i in range(12):
        config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))
        configs.append(config)

    for i, config in enumerate(configs):
        with contextlib.suppress(Exception):
            extract_file_sync(docx_document, config=config, easyocr_kwargs={"use_gpu": i % 2 == 0})

    assert len(kreuzberg._REGISTERED_OCR_BACKENDS) <= kreuzberg._MAX_CACHE_SIZE


def test_easyocr_kwargs_passed_to_backend(docx_document: Path) -> None:
    """Test that easyocr_kwargs are properly passed to EasyOCR backend."""
    pytest.importorskip("easyocr", reason="EasyOCR not installed")

    config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="en"))

    result = extract_file_sync(docx_document, config=config, easyocr_kwargs={"use_gpu": False})

    assert result is not None


def test_paddleocr_kwargs_passed_to_backend(docx_document: Path) -> None:
    """Test that paddleocr_kwargs are properly passed to PaddleOCR backend."""
    pytest.importorskip("paddleocr", reason="PaddleOCR not installed")

    config = ExtractionConfig(ocr=OcrConfig(backend="paddleocr", language="en"))

    result = extract_file_sync(docx_document, config=config, paddleocr_kwargs={"use_gpu": False})

    assert result is not None


def test_batch_functions_with_pathlib_paths(docx_document: Path) -> None:
    """Test batch extraction functions work with Path objects."""
    path_obj = Path(docx_document)

    results = batch_extract_files_sync([path_obj])
    assert len(results) == 1
    assert results[0] is not None


@pytest.mark.asyncio
async def test_batch_functions_async_with_pathlib_paths(docx_document: Path) -> None:
    """Test async batch extraction functions work with Path objects."""
    path_obj = Path(docx_document)

    results = await batch_extract_files([path_obj])
    assert len(results) == 1
    assert results[0] is not None


def test_batch_extract_bytes_with_bytearray(docx_document: Path) -> None:
    """Test batch_extract_bytes_sync works with bytearray."""
    with Path(docx_document).open("rb") as f:
        data = bytearray(f.read())

    results = batch_extract_bytes_sync(
        [data],
        mime_types=["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    )

    assert len(results) == 1
    assert results[0] is not None


@pytest.mark.asyncio
async def test_batch_extract_bytes_async_with_bytearray(docx_document: Path) -> None:
    """Test async batch_extract_bytes works with bytearray."""
    data = bytearray(Path(docx_document).read_bytes())

    results = await batch_extract_bytes(
        [data],
        mime_types=["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
    )

    assert len(results) == 1
    assert results[0] is not None
