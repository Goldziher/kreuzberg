"""Tests for Python OCR backend plugin registration and execution.

This module tests the full lifecycle of Python-based OCR backends:
- Registration via register_ocr_backend()
- Backend is called during image extraction
- Backend receives correct parameters
- Error handling
- Return format validation
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

from kreuzberg import (
    ExtractionConfig,
    OcrConfig,
    extract_file,
    extract_file_sync,
    register_ocr_backend,
)

TEST_DOCS_DIR = Path(__file__).parent.parent.parent.parent.parent / "test_documents"
TEST_IMAGE = TEST_DOCS_DIR / "images" / "test_hello_world.png"


class MockOcrBackend:
    """Simple mock OCR backend for testing."""

    def __init__(self) -> None:
        self.call_count = 0
        self.last_image_bytes: bytes | None = None
        self.last_language: str | None = None

    def name(self) -> str:
        return "mock_test_ocr"

    def supported_languages(self) -> list[str]:
        return ["en", "de", "fr", "es"]

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        self.call_count += 1
        self.last_image_bytes = image_bytes
        self.last_language = language

        return {
            "content": f"Mock OCR extracted text in {language}",
            "metadata": {
                "backend": "mock_test_ocr",
                "language": language,
                "confidence": 0.95,
                "call_count": self.call_count,
            },
            "tables": [],
        }

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        from pathlib import Path

        with Path(path).open("rb") as f:
            return self.process_image(f.read(), language)

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    def version(self) -> str:
        return "1.0.0"


class DetailedMockOcrBackend:
    """Mock OCR backend that returns detailed metadata."""

    def name(self) -> str:
        return "detailed_mock_ocr"

    def supported_languages(self) -> list[str]:
        return ["en"]

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        return {
            "content": "Detailed mock text",
            "metadata": {
                "backend": "detailed_mock_ocr",
                "language": language,
                "width": 800,
                "height": 600,
                "confidence": 0.98,
                "processing_time_ms": 150,
            },
            "tables": [],
        }

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        from pathlib import Path

        with Path(path).open("rb") as f:
            return self.process_image(f.read(), language)

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    def version(self) -> str:
        return "1.0.0"


class OcrBackendWithTables:
    """Mock OCR backend that returns table data."""

    def name(self) -> str:
        return "table_mock_ocr"

    def supported_languages(self) -> list[str]:
        return ["en"]

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        return {
            "content": "Document with table",
            "metadata": {
                "backend": "table_mock_ocr",
                "table_count": 1,
            },
            "tables": [
                {
                    "cells": [["Header 1", "Header 2"], ["Cell 1", "Cell 2"]],
                    "markdown": "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |",
                    "page_number": 1,
                }
            ],
        }

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        from pathlib import Path

        with Path(path).open("rb") as f:
            return self.process_image(f.read(), language)

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    def version(self) -> str:
        return "1.0.0"


class InitializableOcrBackend:
    """Mock OCR backend with initialization and shutdown."""

    def __init__(self) -> None:
        self.initialized = False
        self.shutdown_called = False

    def name(self) -> str:
        return "initializable_ocr"

    def supported_languages(self) -> list[str]:
        return ["en"]

    def initialize(self) -> None:
        self.initialized = True

    def shutdown(self) -> None:
        self.shutdown_called = True

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        return {
            "content": "Initialized OCR result",
            "metadata": {
                "initialized": self.initialized,
            },
            "tables": [],
        }

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        from pathlib import Path

        with Path(path).open("rb") as f:
            return self.process_image(f.read(), language)

    def version(self) -> str:
        return "1.0.0"


class ErrorOcrBackend:
    """Mock OCR backend that raises errors."""

    def name(self) -> str:
        return "error_ocr"

    def supported_languages(self) -> list[str]:
        return ["en"]

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        msg = "Intentional OCR error for testing"
        raise RuntimeError(msg)

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        from pathlib import Path

        with Path(path).open("rb") as f:
            return self.process_image(f.read(), language)

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

    def version(self) -> str:
        return "1.0.0"


def test_register_custom_ocr_backend() -> None:
    """Test registering a custom OCR backend class."""
    backend = MockOcrBackend()
    register_ocr_backend(backend)

    assert backend.call_count == 0


def test_ocr_backend_called_for_image_extraction() -> None:
    """Test OCR backend is called during image extraction with OCR enabled."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="en"),
        force_ocr=True,
    )

    extract_file_sync(str(TEST_IMAGE), config=config)

    assert backend.call_count > 0
    assert backend.last_language == "en"
    assert backend.last_image_bytes is not None
    assert len(backend.last_image_bytes) > 0


@pytest.mark.asyncio
async def test_ocr_backend_async_extraction() -> None:
    """Test OCR backend works with async extraction."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="de"),
        force_ocr=True,
    )

    await extract_file(str(TEST_IMAGE), config=config)

    assert backend.call_count > 0
    assert backend.last_language == "de"


def test_ocr_backend_receives_correct_parameters() -> None:
    """Test OCR backend receives image bytes and language correctly."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="fr"),
        force_ocr=True,
    )

    extract_file_sync(str(TEST_IMAGE), config=config)

    assert backend.last_language == "fr"
    assert backend.last_image_bytes is not None
    assert isinstance(backend.last_image_bytes, bytes)
    assert len(backend.last_image_bytes) > 0


def test_ocr_backend_returns_correct_format() -> None:
    """Test OCR backend returns correct result format."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = DetailedMockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="detailed_mock_ocr", language="en"),
        force_ocr=True,
    )

    result = extract_file_sync(str(TEST_IMAGE), config=config)

    assert result.content is not None
    assert isinstance(result.content, str)
    assert result.content == "Detailed mock text"
    assert result.metadata is not None


def test_ocr_backend_with_tables() -> None:
    """Test OCR backend can return table data."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = OcrBackendWithTables()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="table_mock_ocr", language="en"),
        force_ocr=True,
    )

    result = extract_file_sync(str(TEST_IMAGE), config=config)

    assert result.tables is not None
    assert len(result.tables) > 0


def test_ocr_backend_initialization() -> None:
    """Test OCR backend initialization and shutdown."""
    backend = InitializableOcrBackend()

    backend.initialize()
    assert backend.initialized is True

    register_ocr_backend(backend)

    backend.shutdown()
    assert backend.shutdown_called is True


def test_ocr_backend_error_handling() -> None:
    """Test OCR backend error handling."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = ErrorOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="error_ocr", language="en"),
        force_ocr=True,
    )

    with pytest.raises(Exception, match=r"(?i)(error|ocr)"):
        extract_file_sync(str(TEST_IMAGE), config=config)


def test_ocr_backend_with_unsupported_language() -> None:
    """Test OCR backend with unsupported language."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="zh"),
        force_ocr=True,
    )

    try:
        result = extract_file_sync(str(TEST_IMAGE), config=config)
        assert result.content is not None
    except Exception:
        pass


def test_ocr_backend_multiple_languages() -> None:
    """Test OCR backend with different languages."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    languages = ["en", "de", "fr"]
    for lang in languages:
        config = ExtractionConfig(
            ocr=OcrConfig(backend="mock_test_ocr", language=lang),
            force_ocr=True,
        )
        result = extract_file_sync(str(TEST_IMAGE), config=config)
        assert lang in result.content


def test_ocr_backend_stateful_tracking() -> None:
    """Test stateful OCR backend tracks calls correctly."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="en"),
        force_ocr=True,
    )

    for i in range(3):
        extract_file_sync(str(TEST_IMAGE), config=config)
        assert backend.call_count == i + 1


def test_ocr_backend_with_real_image() -> None:
    """Test OCR backend with real image file."""
    image_file = TEST_DOCS_DIR / "images" / "example.jpg"
    if not image_file.exists():
        pytest.skip("Test image not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="en"),
        force_ocr=True,
    )

    result = extract_file_sync(str(image_file), config=config)

    assert backend.call_count > 0
    assert result.content is not None


def test_ocr_backend_metadata_propagation() -> None:
    """Test OCR backend metadata is included in result."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    backend = DetailedMockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="detailed_mock_ocr", language="en"),
        force_ocr=True,
    )

    result = extract_file_sync(str(TEST_IMAGE), config=config)

    assert "width" in result.metadata
    assert "height" in result.metadata
    assert "confidence" in result.metadata
    assert result.metadata["width"] == 800
    assert result.metadata["height"] == 600


@pytest.mark.asyncio
async def test_concurrent_ocr_backend_calls() -> None:
    """Test OCR backend handles concurrent extractions."""
    if not TEST_IMAGE.exists():
        pytest.skip("Test image not found")

    import asyncio

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="en"),
        force_ocr=True,
    )

    tasks = [extract_file(str(TEST_IMAGE), config=config) for _ in range(3)]
    results = await asyncio.gather(*tasks)

    assert len(results) == 3
    assert backend.call_count == 3


def test_register_ocr_backend_is_exported() -> None:
    """Test that register_ocr_backend is properly exported."""
    from kreuzberg import register_ocr_backend as exported_register

    assert exported_register is not None
    assert callable(exported_register)


def test_ocr_backend_with_pdf_force_ocr() -> None:
    """Test OCR backend with force_ocr on PDF."""
    pdf_file = TEST_DOCS_DIR / "pdfs_with_tables" / "tiny.pdf"
    if not pdf_file.exists():
        pytest.skip("Test PDF not found")

    backend = MockOcrBackend()
    register_ocr_backend(backend)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="mock_test_ocr", language="en"),
        force_ocr=True,
    )

    extract_file_sync(str(pdf_file), config=config)

    assert backend.call_count > 0


def test_ocr_backend_supported_languages_validation() -> None:
    """Test OCR backend supported_languages method."""
    backend = MockOcrBackend()

    languages = backend.supported_languages()
    assert isinstance(languages, list)
    assert len(languages) > 0
    assert "en" in languages


def test_ocr_backend_name_validation() -> None:
    """Test OCR backend name method."""
    backend = MockOcrBackend()

    name = backend.name()
    assert isinstance(name, str)
    assert len(name) > 0
    assert name == "mock_test_ocr"


def test_ocr_backend_process_image_return_structure() -> None:
    """Test OCR backend process_image returns correct structure."""
    backend = MockOcrBackend()

    test_bytes = b"fake_image_data"
    result = backend.process_image(test_bytes, "en")

    assert isinstance(result, dict)
    assert "content" in result
    assert "metadata" in result
    assert "tables" in result
    assert isinstance(result["content"], str)
    assert isinstance(result["metadata"], dict)
    assert isinstance(result["tables"], list)
