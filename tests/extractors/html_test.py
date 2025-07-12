from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import patch

import pytest

from kreuzberg._extractors._html import STREAMING_THRESHOLD_BYTES, HTMLExtractor
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


def test_extract_html_path_sync(html_document: Path, extractor: HTMLExtractor) -> None:
    """Test sync path extraction for HTML files."""
    result = extractor.extract_path_sync(html_document)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"


def test_extract_html_bytes_sync(extractor: HTMLExtractor) -> None:
    """Test sync bytes extraction for HTML content."""
    html_content = b"<html><body><h2>Sync Test</h2><p>Testing sync extraction.</p></body></html>"
    result = extractor.extract_bytes_sync(html_content)
    assert isinstance(result.content, str)
    assert result.content.strip()
    assert result.mime_type == "text/markdown"
    assert "Sync Test" in result.content
    assert "Testing sync extraction." in result.content


def test_streaming_threshold(extractor: HTMLExtractor) -> None:
    """Test that streaming is used for large documents."""
    # Create content just below threshold - should not use streaming
    small_content = b"<html><body>" + b"<p>Small content</p>" * 100 + b"</body></html>"
    assert len(small_content) < STREAMING_THRESHOLD_BYTES

    with patch.object(extractor, "_extract_with_streaming") as mock_streaming:
        result = extractor.extract_bytes_sync(small_content)
        mock_streaming.assert_not_called()
        assert "Small content" in result.content

    # Create content above threshold - should use streaming
    large_content = (
        b"<html><body>" + b"<p>Large content paragraph that is repeated many times.</p>" * 200000 + b"</body></html>"
    )
    assert len(large_content) > STREAMING_THRESHOLD_BYTES

    with patch.object(extractor, "_extract_with_streaming", return_value="# Streamed Content") as mock_streaming:
        result = extractor.extract_bytes_sync(large_content)
        mock_streaming.assert_called_once()
        assert result.content == "# Streamed Content"  # After metadata removal


def test_extract_with_streaming(extractor: HTMLExtractor) -> None:
    """Test the streaming extraction method directly."""
    html_content = """
    <html>
    <head>
        <title>Streaming Test</title>
        <meta name="description" content="Testing streaming mode">
    </head>
    <body>
        <h1>Test Document</h1>
        <p>This is a test of streaming mode.</p>
    </body>
    </html>
    """

    # Mock the convert_to_markdown function to verify streaming parameters
    with patch("kreuzberg._extractors._html.html_to_markdown.convert_to_markdown") as mock_convert:
        mock_convert.return_value = """<!--
title: Streaming Test
meta-description: Testing streaming mode
-->

# Test Document

This is a test of streaming mode."""

        result = extractor._extract_with_streaming(html_content)

        # Verify streaming was called with correct parameters
        mock_convert.assert_called_once()
        call_args = mock_convert.call_args
        assert call_args.kwargs["stream_processing"] is True
        assert call_args.kwargs["chunk_size"] == 10240
        assert "chunk_callback" in call_args.kwargs
        assert callable(call_args.kwargs["chunk_callback"])

        assert "Test Document" in result
        assert "streaming mode" in result


def test_metadata_extraction_with_streaming(extractor: HTMLExtractor) -> None:
    """Test that metadata is correctly extracted when using streaming."""
    # Create large HTML with metadata
    html_content = (
        b"""<html>
    <head>
        <title>Large Document</title>
        <meta name="author" content="Test Author">
        <meta name="keywords" content="test, streaming, large">
    </head>
    <body>"""
        + b"<p>Large paragraph content.</p>" * 200000
        + b"""
    </body>
    </html>"""
    )

    assert len(html_content) > STREAMING_THRESHOLD_BYTES

    result = extractor.extract_bytes_sync(html_content)

    # Check metadata was extracted
    assert result.metadata is not None
    assert result.metadata.get("title") == "Large Document"
    assert result.metadata.get("authors") == ["Test Author"]
    assert result.metadata.get("keywords") == ["test", "streaming", "large"]

    # Content should not contain metadata comment block
    assert "<!--" not in result.content
    assert "-->" not in result.content
