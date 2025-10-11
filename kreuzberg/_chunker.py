from __future__ import annotations

from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg._internal_bindings import MarkdownSplitter, TextSplitter
from kreuzberg._mime_types import MARKDOWN_MIME_TYPE

_chunkers: dict[tuple[int, int, str], MarkdownSplitter | TextSplitter] = {}


def get_chunker(
    mime_type: str,
    max_characters: int = DEFAULT_MAX_CHARACTERS,
    overlap_characters: int = DEFAULT_MAX_OVERLAP,
) -> MarkdownSplitter | TextSplitter:
    key = (max_characters, overlap_characters, mime_type)
    if key not in _chunkers:
        match mime_type:
            case x if x == MARKDOWN_MIME_TYPE:
                _chunkers[key] = MarkdownSplitter(max_characters, overlap_characters)
            case _:
                _chunkers[key] = TextSplitter(max_characters, overlap_characters)

    return _chunkers[key]
