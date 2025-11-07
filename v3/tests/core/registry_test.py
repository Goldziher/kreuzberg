from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._extractors._base import Extractor
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._registry import ExtractorRegistry
from kreuzberg._types import ExtractionConfig, ExtractionResult

if TYPE_CHECKING:
    from pathlib import Path


class MockExtractor(Extractor):
    SUPPORTED_MIME_TYPES = {"test/mock", "test/"}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="mock", mime_type="text/plain", metadata={})

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="mock", mime_type="text/plain", metadata={})

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="mock", mime_type="text/plain", metadata={})

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="mock", mime_type="text/plain", metadata={})


class AnotherMockExtractor(Extractor):
    SUPPORTED_MIME_TYPES = {"another/mock"}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="another", mime_type="text/plain", metadata={})

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="another", mime_type="text/plain", metadata={})

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return ExtractionResult(content="another", mime_type="text/plain", metadata={})

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return ExtractionResult(content="another", mime_type="text/plain", metadata={})


def test_get_extractor_with_supported_mime_type() -> None:
    config = ExtractionConfig()

    ExtractorRegistry.get_extractor.cache_clear()

    extractor = ExtractorRegistry.get_extractor("application/pdf", config)
    assert extractor is not None
    assert isinstance(extractor, PDFExtractor)


def test_get_extractor_with_unsupported_mime_type() -> None:
    config = ExtractionConfig()

    ExtractorRegistry.get_extractor.cache_clear()

    extractor = ExtractorRegistry.get_extractor("application/unknown", config)
    assert extractor is None


def test_get_extractor_with_none_mime_type() -> None:
    config = ExtractionConfig()

    ExtractorRegistry.get_extractor.cache_clear()

    extractor = ExtractorRegistry.get_extractor(None, config)
    assert extractor is None


def test_add_extractor() -> None:
    config = ExtractionConfig()

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    extractor = ExtractorRegistry.get_extractor("test/mock", config)
    assert extractor is None

    ExtractorRegistry.add_extractor(MockExtractor)

    extractor = ExtractorRegistry.get_extractor("test/mock", config)
    assert extractor is not None
    assert isinstance(extractor, MockExtractor)

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()


def test_add_extractor_clears_cache() -> None:
    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    ExtractorRegistry.add_extractor(MockExtractor)

    ExtractorRegistry._registered_extractors.clear()


def test_remove_extractor() -> None:
    config = ExtractionConfig()

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    ExtractorRegistry.add_extractor(MockExtractor)

    extractor = ExtractorRegistry.get_extractor("test/mock", config)
    assert isinstance(extractor, MockExtractor)

    ExtractorRegistry.remove_extractor(MockExtractor)

    extractor = ExtractorRegistry.get_extractor("test/mock", config)
    assert extractor is None

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()


def test_remove_extractor_not_in_registry() -> None:
    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    ExtractorRegistry.remove_extractor(MockExtractor)

    ExtractorRegistry.get_extractor.cache_clear()


def test_remove_extractor_clears_cache() -> None:
    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    ExtractorRegistry.add_extractor(MockExtractor)

    ExtractorRegistry.remove_extractor(MockExtractor)

    ExtractorRegistry._registered_extractors.clear()


def test_registered_extractors_priority() -> None:
    config = ExtractionConfig()

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    class CustomPDFExtractor(Extractor):
        SUPPORTED_MIME_TYPES = {"application/pdf"}

        async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
            return ExtractionResult(content="custom pdf", mime_type="text/plain", metadata={})

        async def extract_path_async(self, path: Path) -> ExtractionResult:
            return ExtractionResult(content="custom pdf", mime_type="text/plain", metadata={})

        def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
            return ExtractionResult(content="custom pdf", mime_type="text/plain", metadata={})

        def extract_path_sync(self, path: Path) -> ExtractionResult:
            return ExtractionResult(content="custom pdf", mime_type="text/plain", metadata={})

    ExtractorRegistry.add_extractor(CustomPDFExtractor)

    extractor = ExtractorRegistry.get_extractor("application/pdf", config)
    assert extractor is not None
    assert isinstance(extractor, CustomPDFExtractor)
    assert not isinstance(extractor, PDFExtractor)

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()


def test_get_extractor_caching() -> None:
    config = ExtractionConfig()

    ExtractorRegistry.get_extractor.cache_clear()

    extractor1 = ExtractorRegistry.get_extractor("application/pdf", config)

    extractor2 = ExtractorRegistry.get_extractor("application/pdf", config)

    assert extractor1 is extractor2


def test_multiple_extractors() -> None:
    config = ExtractionConfig()

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    ExtractorRegistry.add_extractor(MockExtractor)
    ExtractorRegistry.add_extractor(AnotherMockExtractor)

    extractor1 = ExtractorRegistry.get_extractor("test/mock", config)
    assert isinstance(extractor1, MockExtractor)

    extractor2 = ExtractorRegistry.get_extractor("another/mock", config)
    assert isinstance(extractor2, AnotherMockExtractor)

    ExtractorRegistry.remove_extractor(MockExtractor)

    extractor1 = ExtractorRegistry.get_extractor("test/mock", config)
    assert extractor1 is None

    extractor2 = ExtractorRegistry.get_extractor("another/mock", config)
    assert isinstance(extractor2, AnotherMockExtractor)

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()


def test_extractor_with_prefix_mime_type() -> None:
    config = ExtractionConfig()

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()

    ExtractorRegistry.add_extractor(MockExtractor)

    extractor = ExtractorRegistry.get_extractor("test/anything", config)
    assert isinstance(extractor, MockExtractor)

    ExtractorRegistry._registered_extractors.clear()
    ExtractorRegistry.get_extractor.cache_clear()
