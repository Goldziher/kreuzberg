"""Modern Rust-based token reduction implementation.

This module provides a high-performance token reduction system implemented in Rust
with semantic awareness, SIMD optimization, and parallel processing capabilities.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._internal_bindings import (  # type: ignore[attr-defined]
    ReductionLevel,
    batch_reduce_tokens_rust,
    get_reduction_statistics_rust,
    reduce_tokens_rust,
)
from kreuzberg._internal_bindings import (  # type: ignore[attr-defined]
    TokenReductionConfig as RustTokenReductionConfig,
)

if TYPE_CHECKING:
    from kreuzberg._types import TokenReductionConfig


def reduce_tokens_with_rust(
    text: str,
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> str:
    """Reduce tokens using the modern Rust implementation.

    Args:
        text: The text to reduce.
        config: Configuration for token reduction.
        language: Optional language code for language-specific optimizations.

    Returns:
        The reduced text.
    """
    rust_config = _convert_config_to_rust(config)
    return reduce_tokens_rust(text, rust_config, language)  # type: ignore[no-any-return]


def batch_reduce_tokens_with_rust(
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
    rust_config = _convert_config_to_rust(config)
    return batch_reduce_tokens_rust(texts, rust_config, language)  # type: ignore[no-any-return]


def get_reduction_statistics(original: str, reduced: str) -> dict[str, float | int]:
    """Get detailed statistics about the reduction.

    Args:
        original: The original text.
        reduced: The reduced text.

    Returns:
        Dictionary containing reduction statistics.
    """
    (
        char_reduction,
        token_reduction,
        original_chars,
        reduced_chars,
        original_tokens,
        reduced_tokens,
    ) = get_reduction_statistics_rust(original, reduced)

    return {
        "character_reduction_ratio": char_reduction,
        "token_reduction_ratio": token_reduction,
        "original_characters": original_chars,
        "reduced_characters": reduced_chars,
        "original_tokens": original_tokens,
        "reduced_tokens": reduced_tokens,
    }


def _convert_config_to_rust(config: TokenReductionConfig) -> RustTokenReductionConfig:
    """Convert Python config to Rust config."""
    # Map Python mode strings to Rust ReductionLevel enum
    level_mapping = {
        "off": ReductionLevel.Off,
        "light": ReductionLevel.Light,
        "moderate": ReductionLevel.Moderate,
        "aggressive": ReductionLevel.Aggressive,
        "maximum": ReductionLevel.Maximum,
    }

    rust_level = level_mapping.get(config.mode, ReductionLevel.Moderate)

    # Create Rust configuration with modern features
    return RustTokenReductionConfig(
        level=rust_level,
        language_hint=config.language_hint,
        preserve_markdown=config.preserve_markdown,
        preserve_code=True,  # Always preserve code for better results
        semantic_threshold=0.3,  # Default semantic importance threshold
        enable_parallel=True,  # Enable parallel processing for performance
        use_simd=True,  # Enable SIMD optimizations
        custom_stopwords=config.custom_stopwords,
        preserve_patterns=[],  # Could be extended to preserve regex patterns
        target_reduction=None,  # Could be configured for adaptive reduction
        enable_semantic_clustering=rust_level in (ReductionLevel.Aggressive, ReductionLevel.Maximum),
    )
