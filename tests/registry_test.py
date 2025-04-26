from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from pathlib import Path

from kreuzberg._extractors._base import Extractor
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._mime_types import PDF_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._registry import ExtractorRegistry
from kreuzberg._types import ExtractionConfig, ExtractionResult


class MockExtractor(Extractor):
    def _create_extraction_result(self, content: str, mime_type: str) -> ExtractionResult:
        return ExtractionResult(content=content, mime_type=mime_type, metadata={}, chunks=[])

    @classmethod
    def supports_mimetype(cls, mime_type: str) -> bool:
        return mime_type == "application/mock"

    async def extract_bytes_async(self, _: bytes) -> ExtractionResult:
        return self._create_extraction_result("Mock content", PLAIN_TEXT_MIME_TYPE)

    async def extract_path_async(self, _: Path) -> ExtractionResult:
        return self._create_extraction_result("Mock content from file", PLAIN_TEXT_MIME_TYPE)

    def extract_bytes_sync(self, _: bytes) -> ExtractionResult:
        return self._create_extraction_result("Mock content sync", PLAIN_TEXT_MIME_TYPE)

    def extract_path_sync(self, _: Path) -> ExtractionResult:
        return self._create_extraction_result("Mock content from file sync", PLAIN_TEXT_MIME_TYPE)


@pytest.fixture
def default_config() -> ExtractionConfig:
    return ExtractionConfig()


def test_get_extractor_for_supported_mime_type(default_config: ExtractionConfig) -> None:
    extractor = ExtractorRegistry.get_extractor(PDF_MIME_TYPE, default_config)
    assert extractor is not None
    assert isinstance(extractor, PDFExtractor)


def test_get_extractor_for_unsupported_mime_type(default_config: ExtractionConfig) -> None:
    extractor = ExtractorRegistry.get_extractor("application/unsupported", default_config)
    assert extractor is None


def test_get_extractor_for_none_mime_type(default_config: ExtractionConfig) -> None:
    extractor = ExtractorRegistry.get_extractor(None, default_config)
    assert extractor is None


def test_add_and_get_custom_extractor(default_config: ExtractionConfig) -> None:
    try:
        ExtractorRegistry.add_extractor(MockExtractor)

        extractor = ExtractorRegistry.get_extractor("application/mock", default_config)

        assert extractor is not None
        assert isinstance(extractor, MockExtractor)
        assert extractor.mime_type == "application/mock"
    finally:
        ExtractorRegistry.remove_extractor(MockExtractor)


def test_remove_extractor(default_config: ExtractionConfig) -> None:
    ExtractorRegistry.add_extractor(MockExtractor)

    extractor = ExtractorRegistry.get_extractor("application/mock", default_config)
    assert extractor is not None
    assert isinstance(extractor, MockExtractor)

    ExtractorRegistry.remove_extractor(MockExtractor)

    extractor = ExtractorRegistry.get_extractor("application/mock", default_config)
    assert extractor is None


def test_remove_nonexistent_extractor() -> None:
    ExtractorRegistry.remove_extractor(MockExtractor)


def test_extractor_order_precedence(default_config: ExtractionConfig) -> None:
    class CustomPDFExtractor(Extractor):
        def _create_extraction_result(self, content: str, mime_type: str) -> ExtractionResult:
            return ExtractionResult(content=content, mime_type=mime_type, metadata={}, chunks=[])

        @classmethod
        def supports_mimetype(cls, mime_type: str) -> bool:
            return mime_type == PDF_MIME_TYPE

        async def extract_bytes_async(self, _: bytes) -> ExtractionResult:
            return self._create_extraction_result("Custom PDF content", PLAIN_TEXT_MIME_TYPE)

        async def extract_path_async(self, _: Path) -> ExtractionResult:
            return self._create_extraction_result("Custom PDF content from file", PLAIN_TEXT_MIME_TYPE)

        def extract_bytes_sync(self, _: bytes) -> ExtractionResult:
            return self._create_extraction_result("Custom PDF content sync", PLAIN_TEXT_MIME_TYPE)

        def extract_path_sync(self, _: Path) -> ExtractionResult:
            return self._create_extraction_result("Custom PDF content from file sync", PLAIN_TEXT_MIME_TYPE)

    try:
        ExtractorRegistry.add_extractor(CustomPDFExtractor)

        extractor = ExtractorRegistry.get_extractor(PDF_MIME_TYPE, default_config)

        assert extractor is not None
        assert isinstance(extractor, CustomPDFExtractor)
        assert not isinstance(extractor, PDFExtractor)
    finally:
        ExtractorRegistry.remove_extractor(CustomPDFExtractor)


def test_cache_clearing_on_registry_changes(default_config: ExtractionConfig) -> None:
    pdf_extractor1 = ExtractorRegistry.get_extractor(PDF_MIME_TYPE, default_config)
    assert pdf_extractor1 is not None

    ExtractorRegistry.add_extractor(MockExtractor)

    try:
        pdf_extractor2 = ExtractorRegistry.get_extractor(PDF_MIME_TYPE, default_config)
        assert pdf_extractor2 is not None

        assert pdf_extractor1 is not pdf_extractor2
    finally:
        ExtractorRegistry.remove_extractor(MockExtractor)


def test_multiple_extractors_same_mime_type(default_config: ExtractionConfig) -> None:
    class FirstExtractor(Extractor):
        def _create_extraction_result(self, content: str, mime_type: str) -> ExtractionResult:
            return ExtractionResult(content=content, mime_type=mime_type, metadata={}, chunks=[])

        @classmethod
        def supports_mimetype(cls, mime_type: str) -> bool:
            return mime_type == "application/custom"

        async def extract_bytes_async(self, _: bytes) -> ExtractionResult:
            return self._create_extraction_result("First extractor content", PLAIN_TEXT_MIME_TYPE)

        async def extract_path_async(self, _: Path) -> ExtractionResult:
            return self._create_extraction_result("First extractor content from file", PLAIN_TEXT_MIME_TYPE)

        def extract_bytes_sync(self, _: bytes) -> ExtractionResult:
            return self._create_extraction_result("First extractor content sync", PLAIN_TEXT_MIME_TYPE)

        def extract_path_sync(self, _: Path) -> ExtractionResult:
            return self._create_extraction_result("First extractor content from file sync", PLAIN_TEXT_MIME_TYPE)

    class SecondExtractor(Extractor):
        def _create_extraction_result(self, content: str, mime_type: str) -> ExtractionResult:
            return ExtractionResult(content=content, mime_type=mime_type, metadata={}, chunks=[])

        @classmethod
        def supports_mimetype(cls, mime_type: str) -> bool:
            return mime_type == "application/custom"

        async def extract_bytes_async(self, _: bytes) -> ExtractionResult:
            return self._create_extraction_result("Second extractor content", PLAIN_TEXT_MIME_TYPE)

        async def extract_path_async(self, _: Path) -> ExtractionResult:
            return self._create_extraction_result("Second extractor content from file", PLAIN_TEXT_MIME_TYPE)

        def extract_bytes_sync(self, _: bytes) -> ExtractionResult:
            return self._create_extraction_result("Second extractor content sync", PLAIN_TEXT_MIME_TYPE)

        def extract_path_sync(self, _: Path) -> ExtractionResult:
            return self._create_extraction_result("Second extractor content from file sync", PLAIN_TEXT_MIME_TYPE)

    try:
        ExtractorRegistry.add_extractor(SecondExtractor)

        ExtractorRegistry.add_extractor(FirstExtractor)

        extractor = ExtractorRegistry.get_extractor("application/custom", default_config)

        assert extractor is not None
        assert isinstance(extractor, SecondExtractor)
        assert not isinstance(extractor, FirstExtractor)
    finally:
        ExtractorRegistry.remove_extractor(FirstExtractor)
        ExtractorRegistry.remove_extractor(SecondExtractor)
