"""Text chunking - Rust-backed via text-splitter crate."""

from kreuzberg._internal_bindings import MarkdownSplitter, TextSplitter
from kreuzberg._types import ChunkingConfig

__all__ = ["ChunkingConfig", "MarkdownSplitter", "TextSplitter", "chunk_text"]


def chunk_text(text: str, config: ChunkingConfig | None = None) -> list[str]:
    """Chunk text using Rust-backed splitter (zero overhead).

    Args:
        text: The text to chunk
        config: Chunking configuration. If None, uses defaults.

    Returns:
        List of text chunks
    """
    config = config or ChunkingConfig()

    splitter = TextSplitter(
        max_characters=config.max_chars,
        overlap=config.max_overlap,
    )

    return splitter.chunks(text)


def chunk_markdown(text: str, config: ChunkingConfig | None = None) -> list[str]:
    """Chunk markdown text using Rust-backed splitter (zero overhead).

    Args:
        text: The markdown text to chunk
        config: Chunking configuration. If None, uses defaults.

    Returns:
        List of markdown chunks
    """
    config = config or ChunkingConfig()

    splitter = MarkdownSplitter(
        max_characters=config.max_chars,
        overlap=config.max_overlap,
    )

    return splitter.chunks(text)
