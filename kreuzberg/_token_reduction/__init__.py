from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._token_reduction._reducer import (
    batch_reduce_tokens as _batch_reduce_tokens,
)
from kreuzberg._token_reduction._reducer import (
    get_reduction_statistics,
)
from kreuzberg._token_reduction._reducer import (
    reduce_tokens as _reduce_tokens,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from kreuzberg._types import TokenReductionConfig


def reduce_tokens(
    text: str,
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> str:
    """Reduce tokens using the Rust implementation.

    Args:
        text: The text to reduce.
        config: Configuration for token reduction.
        language: Optional language code for language-specific optimizations.

    Returns:
        The reduced text.
    """
    if text is None:
        raise ValidationError("Text cannot be None")
    if not isinstance(text, str):
        raise ValidationError(f"Text must be a string, got {type(text).__name__}")
    if config is None:
        raise ValidationError("Config cannot be None")
    if language is not None and not isinstance(language, str):
        raise ValidationError(f"Language must be a string or None, got {type(language).__name__}")
    if language is not None and language.strip() == "":
        raise ValidationError("Language cannot be empty or whitespace-only")
    if language is not None and not language.replace("-", "").replace("_", "").isalnum():
        raise ValidationError("Invalid language code format")

    return _reduce_tokens(text, config=config, language=language)


def batch_reduce_tokens(
    texts: list[str],
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> list[str]:
    """Reduce tokens in multiple texts using parallel processing.

    Args:
        texts: List of texts to reduce.
        config: Configuration for token reduction.
        language: Optional language code for language-specific optimizations.

    Returns:
        List of reduced texts.
    """
    return _batch_reduce_tokens(texts, config=config, language=language)


def get_reduction_stats(original: str, reduced: str) -> dict[str, float | int]:
    """Get detailed statistics about the reduction.

    Args:
        original: The original text.
        reduced: The reduced text.

    Returns:
        Dictionary containing reduction statistics.
    """
    if original is None:
        raise ValidationError("Original text cannot be None")
    if not isinstance(original, str):
        raise ValidationError(f"Original text must be a string, got {type(original).__name__}")
    if reduced is None:
        raise ValidationError("Reduced text cannot be None")
    if not isinstance(reduced, str):
        raise ValidationError(f"Reduced text must be a string, got {type(reduced).__name__}")

    return get_reduction_statistics(original, reduced)


__all__ = [
    "batch_reduce_tokens",
    "get_reduction_stats",
    "reduce_tokens",
]
