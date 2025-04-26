from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import MagicMock, patch

import pytest

from kreuzberg._chunker import _chunkers, get_chunker
from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture(autouse=True)
def clear_chunkers() -> Generator[None, None, None]:
    _chunkers.clear()
    yield
    _chunkers.clear()


def test_get_chunker_markdown() -> None:
    with patch("semantic_text_splitter.MarkdownSplitter") as mock_splitter:
        mock_instance = MagicMock()
        mock_splitter.return_value = mock_instance

        chunker = get_chunker(MARKDOWN_MIME_TYPE)

        mock_splitter.assert_called_once_with(DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP)
        assert chunker == mock_instance
        assert (DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP, MARKDOWN_MIME_TYPE) in _chunkers


def test_get_chunker_text() -> None:
    with patch("semantic_text_splitter.TextSplitter") as mock_splitter:
        mock_instance = MagicMock()
        mock_splitter.return_value = mock_instance

        chunker = get_chunker(PLAIN_TEXT_MIME_TYPE)

        mock_splitter.assert_called_once_with(DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP)
        assert chunker == mock_instance
        assert (DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP, PLAIN_TEXT_MIME_TYPE) in _chunkers


def test_get_chunker_custom_parameters() -> None:
    with patch("semantic_text_splitter.TextSplitter") as mock_splitter:
        mock_instance = MagicMock()
        mock_splitter.return_value = mock_instance

        max_chars = 1000
        overlap = 50

        chunker = get_chunker(PLAIN_TEXT_MIME_TYPE, max_chars, overlap)

        mock_splitter.assert_called_once_with(max_chars, overlap)
        assert chunker == mock_instance
        assert (max_chars, overlap, PLAIN_TEXT_MIME_TYPE) in _chunkers


def test_get_chunker_caching() -> None:
    with patch("semantic_text_splitter.TextSplitter") as mock_splitter:
        mock_instance = MagicMock()
        mock_splitter.return_value = mock_instance

        chunker1 = get_chunker(PLAIN_TEXT_MIME_TYPE)
        assert mock_splitter.call_count == 1

        chunker2 = get_chunker(PLAIN_TEXT_MIME_TYPE)
        assert mock_splitter.call_count == 1
        assert chunker1 == chunker2


def test_get_chunker_missing_dependency() -> None:
    with patch(
        "semantic_text_splitter.TextSplitter", side_effect=ImportError("No module named 'semantic_text_splitter'")
    ):
        with pytest.raises(MissingDependencyError) as excinfo:
            get_chunker(PLAIN_TEXT_MIME_TYPE)

        assert "semantic-text-splitter" in str(excinfo.value)
        assert "chunking" in str(excinfo.value)


def test_get_chunker_different_mime_types() -> None:
    with (
        patch("semantic_text_splitter.MarkdownSplitter") as mock_markdown_splitter,
        patch("semantic_text_splitter.TextSplitter") as mock_text_splitter,
    ):
        markdown_instance = MagicMock()
        text_instance = MagicMock()
        mock_markdown_splitter.return_value = markdown_instance
        mock_text_splitter.return_value = text_instance

        markdown_chunker = get_chunker(MARKDOWN_MIME_TYPE)
        text_chunker = get_chunker(PLAIN_TEXT_MIME_TYPE)

        assert markdown_chunker == markdown_instance
        assert text_chunker == text_instance
        assert markdown_chunker != text_chunker
