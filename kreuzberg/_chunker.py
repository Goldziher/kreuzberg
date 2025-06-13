from __future__ import annotations

from typing import Protocol

from kreuzberg import MissingDependencyError
from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE


class Chunker(Protocol):
    def chunks(self, text: str) -> list[str]: ...


_chunkers: dict[tuple[int, int, str], Chunker] = {}


def get_chunker(
    mime_type: str,
    max_characters: int = DEFAULT_MAX_CHARACTERS,
    overlap_characters: int = DEFAULT_MAX_OVERLAP,
) -> Chunker:
    """Creates and returns a Chunker object configured with the given maximum
    characters per chunk and overlap between chunks.

    Args:
        mime_type: The mime type of the content.
        max_characters: Maximum number of characters allowed in each chunk.
        overlap_characters: Number of characters overlapping between two consecutive chunks.

    Raises:
        MissingDependencyError: if semantic-text-splitter is not installed.

    Returns:
        Chunker: A Chunker object configured with the specified maximum
            characters and overlap.
    """
    key = (max_characters, overlap_characters, mime_type)
    if key not in _chunkers:
        try:
            if mime_type == MARKDOWN_MIME_TYPE:
                from semantic_text_splitter import MarkdownSplitter  # type: ignore[import-not-found]

                _chunkers[key] = MarkdownSplitter(max_characters, overlap_characters)
            else:
                from semantic_text_splitter import TextSplitter

                _chunkers[key] = TextSplitter(max_characters, overlap_characters)
        except ImportError as e:
            raise MissingDependencyError.create_for_package(
                dependency_group="chunking", functionality="chunking", package_name="semantic-text-splitter"
            ) from e

    return _chunkers[key]
