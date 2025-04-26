from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg._extractors._html import HTMLExtractor
from kreuzberg.extraction import DEFAULT_CONFIG

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def extractor() -> HTMLExtractor:
    return HTMLExtractor(mime_type="text/html", config=DEFAULT_CONFIG)


@pytest.mark.anyio
async def test_extract_html_string(html_document: Path, extractor: HTMLExtractor) -> None:
    result = await extractor.extract_path_async(html_document)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"


@pytest.mark.anyio
async def test_extract_html_string_bytes(extractor: HTMLExtractor) -> None:
    html_content = b"<html><body><h1>Test</h1><p>This is a test.</p></body></html>"
    result = await extractor.extract_bytes_async(html_content)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"
    assert "Test" in result.content
    assert "This is a test." in result.content
