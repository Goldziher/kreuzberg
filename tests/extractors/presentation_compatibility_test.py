from __future__ import annotations

from typing import TYPE_CHECKING, Any

import pytest

from kreuzberg import ExtractionConfig, ImageExtractionConfig
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from pathlib import Path


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


def test_slide_numbering_format(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert "<!-- Slide number: 1 -->" in result.content
    assert "<!-- Slide number: 2 -->" in result.content
    assert "<!-- Slide number: 3 -->" in result.content


def test_title_formatting(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert "# " in result.content, "Expected markdown title formatting (# prefix) not found"

    title_count = result.content.count("\n# ")
    result.content.count("<!-- Slide number:")
    assert title_count > 0, f"Expected at least some titles, found {title_count}"


def test_table_formatting(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    if "<table>" in result.content:
        assert "<th>" in result.content or "<td>" in result.content, "Table found but no cells"
        assert "</table>" in result.content, "Table opening tag found but no closing tag"


def test_html_escaping_in_tables(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    if "<table>" in result.content:
        assert result.content.count("<table>") == result.content.count("</table>")


def test_image_references(extractor_with_images: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor_with_images.extract_path_sync(test_pptx_file)

    result.content.count("![")


def test_notes_formatting(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    result.content.count("### Notes:")

    if "### Notes:" in result.content:
        notes_index = result.content.find("### Notes:")
        remaining_content = result.content[notes_index + len("### Notes:") :]
        assert len(remaining_content.strip()) > 0, "Notes section found but no content"


def test_metadata_fields(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    expected_fields = [
        "authors",
        "title",
        "subject",
        "languages",
        "categories",
        "fonts",
        "description",
        "summary",
    ]

    missing_fields = [field for field in expected_fields if field not in result.metadata]

    if missing_fields:
        pass


def test_metadata_structure_info(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    if "description" in result.metadata:
        desc = result.metadata["description"]
        assert "slide" in str(desc).lower(), f"Description should mention slides: {desc}"

    if "summary" in result.metadata:
        summary = result.metadata["summary"]
        assert "presentation" in str(summary).lower(), f"Summary should mention presentation: {summary}"


def test_fonts_extraction(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    if "fonts" in result.metadata:
        fonts = result.metadata["fonts"]
        assert isinstance(fonts, list), f"Fonts should be a list, got {type(fonts)}"
    else:
        pass


def test_mime_type(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert result.mime_type == "text/markdown"


def test_content_not_empty(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    result = extractor.extract_path_sync(test_pptx_file)

    assert len(result.content) > 0, "Content should not be empty"
    assert len(result.content.strip()) > 0, "Content should not be just whitespace"


def test_async_sync_compatibility(extractor: PresentationExtractor, test_pptx_file: Path) -> None:
    import asyncio

    sync_result = extractor.extract_path_sync(test_pptx_file)

    async def get_async_result() -> Any:
        return await extractor.extract_path_async(test_pptx_file)

    async_result = asyncio.run(get_async_result())

    assert sync_result.content == async_result.content
    assert sync_result.mime_type == async_result.mime_type
    assert sync_result.metadata == async_result.metadata
