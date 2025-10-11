from __future__ import annotations

from kreuzberg._chunker import get_chunker
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE


def test_get_chunker_markdown_default_params() -> None:
    chunker = get_chunker(MARKDOWN_MIME_TYPE)
    assert chunker is not None
    assert type(chunker).__name__ == "MarkdownSplitter"


def test_get_chunker_text_default_params() -> None:
    chunker = get_chunker("text/plain")
    assert chunker is not None
    assert type(chunker).__name__ == "TextSplitter"


def test_get_chunker_custom_params() -> None:
    chunker = get_chunker("text/plain", max_characters=500, overlap_characters=50)
    assert chunker is not None
    assert type(chunker).__name__ == "TextSplitter"


def test_get_chunker_caching() -> None:
    chunker1 = get_chunker("text/plain", max_characters=1000, overlap_characters=100)

    chunker2 = get_chunker("text/plain", max_characters=1000, overlap_characters=100)

    assert chunker1 is chunker2

    chunker3 = get_chunker("text/plain", max_characters=2000, overlap_characters=100)

    assert chunker1 is not chunker3


def test_get_chunker_markdown_vs_text() -> None:
    markdown_chunker = get_chunker(MARKDOWN_MIME_TYPE)
    text_chunker = get_chunker("text/plain")

    assert type(markdown_chunker).__name__ == "MarkdownSplitter"
    assert type(text_chunker).__name__ == "TextSplitter"
    assert markdown_chunker is not text_chunker


def test_get_chunker_different_mime_types() -> None:
    mime_types = ["text/html", "application/json", "text/csv", "application/xml"]

    for mime_type in mime_types:
        chunker = get_chunker(mime_type)
        assert chunker is not None
        assert type(chunker).__name__ == "TextSplitter"


def test_get_chunker_cache_key_uniqueness() -> None:
    chunkers = []
    params = [
        (MARKDOWN_MIME_TYPE, 1000, 100),
        (MARKDOWN_MIME_TYPE, 1000, 200),
        (MARKDOWN_MIME_TYPE, 2000, 100),
        ("text/plain", 1000, 100),
    ]

    for mime_type, max_chars, overlap in params:
        chunker = get_chunker(mime_type, max_chars, overlap)
        chunkers.append(chunker)

    for i in range(len(chunkers)):
        for j in range(i + 1, len(chunkers)):
            assert chunkers[i] is not chunkers[j], f"Chunkers {i} and {j} should be different instances"
