from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from semantic_text_splitter import MarkdownSplitter, TextSplitter

_chunkers: dict[tuple[int, int, str], MarkdownSplitter | TextSplitter] = {}


def get_chunker(
    mime_type: str,
    max_characters: int = DEFAULT_MAX_CHARACTERS,
    overlap_characters: int = DEFAULT_MAX_OVERLAP,
) -> MarkdownSplitter | TextSplitter:
    key = (max_characters, overlap_characters, mime_type)
    if key not in _chunkers:
        try:
            match mime_type:
                case x if x == MARKDOWN_MIME_TYPE:
                    from semantic_text_splitter import MarkdownSplitter  # noqa: PLC0415

                    _chunkers[key] = MarkdownSplitter(max_characters, overlap_characters)
                case _:
                    from semantic_text_splitter import TextSplitter  # noqa: PLC0415

                    _chunkers[key] = TextSplitter(max_characters, overlap_characters)
        except ImportError as e:  # pragma: no cover
            raise MissingDependencyError.create_for_package(
                dependency_group="chunking", functionality="chunking", package_name="semantic-text-splitter"
            ) from e

    return _chunkers[key]
