from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg.extraction import DEFAULT_CONFIG


@pytest.fixture(scope="session")
def extractor() -> PresentationExtractor:
    return PresentationExtractor(
        mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=DEFAULT_CONFIG
    )


@pytest.fixture(scope="session")
def extractor_with_images() -> PresentationExtractor:
    config = ExtractionConfig(images=ImageExtractionConfig())
    return PresentationExtractor(
        mime_type="application/vnd.openxmlformats-officedocument.presentationml.presentation", config=config
    )


@pytest.fixture(scope="session")
def test_pptx_file(pptx_document: Path) -> Path:
    return pptx_document


def test_extract_bytes_sync_basic(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    content = test_pptx_file.read_bytes()
    result = extractor.extract_bytes_sync(content)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert "slide" in result.content.lower()


def test_extract_path_sync_basic(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert "slide" in result.content.lower()


@pytest.mark.anyio
async def test_extract_bytes_async_basic(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    content = test_pptx_file.read_bytes()
    result = await extractor.extract_bytes_async(content)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert "slide" in result.content.lower()


@pytest.mark.anyio
async def test_extract_path_async_basic(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = await extractor.extract_path_async(test_pptx_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0
    assert "slide" in result.content.lower()


def test_extract_slide_numbering(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert "<!-- Slide number: 1 -->" in result.content
    assert "<!-- Slide number: 2 -->" in result.content
    assert "<!-- Slide number: 3 -->" in result.content


def test_extract_content_structure(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    lines = result.content.split("\n")

    slide_headers = [line for line in lines if line.startswith("<!-- Slide number:")]
    assert len(slide_headers) > 0

    content_lines = [line for line in lines if line.strip() and not line.startswith("<!--")]
    assert len(content_lines) > 0


def test_extract_metadata_basic(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert result.metadata is not None
    assert isinstance(result.metadata, dict)


def test_extract_with_images_enabled(extractor_with_images: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor_with_images.extract_path_sync(test_pptx_file)

    assert result.mime_type == "text/markdown"
    assert len(result.content) > 0


def test_extract_error_handling_invalid_bytes(extractor: PresentationExtractor) -> None:
    with pytest.raises((OSError, ValueError, RuntimeError)):
        extractor.extract_bytes_sync(b"invalid pptx data")


def test_extract_error_handling_missing_file(extractor: PresentationExtractor) -> None:
    with pytest.raises((FileNotFoundError, OSError, RuntimeError)):
        extractor.extract_path_sync(Path("nonexistent.pptx"))


def test_rust_vs_sync_consistency(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    path_result = extractor.extract_path_sync(test_pptx_file)

    content = test_pptx_file.read_bytes()
    bytes_result = extractor.extract_bytes_sync(content)

    assert path_result.mime_type == bytes_result.mime_type
    assert len(path_result.content) > 0
    assert len(bytes_result.content) > 0

    assert "<!-- Slide number:" in path_result.content
    assert "<!-- Slide number:" in bytes_result.content


@pytest.mark.anyio
async def test_async_vs_sync_consistency(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    sync_result = extractor.extract_path_sync(test_pptx_file)

    async_result = await extractor.extract_path_async(test_pptx_file)

    assert sync_result.mime_type == async_result.mime_type
    assert sync_result.content == async_result.content
