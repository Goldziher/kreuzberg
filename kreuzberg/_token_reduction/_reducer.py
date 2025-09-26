"""Token reduction implementation.

This module provides a high-performance token reduction system
with semantic awareness, SIMD optimization, and parallel processing capabilities.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._internal_bindings import (
    ReductionLevelDTO,
    TokenReductionConfigDTO,
)
from kreuzberg._internal_bindings import (
    batch_reduce_tokens as _batch_reduce_tokens,
)
from kreuzberg._internal_bindings import (
    get_reduction_statistics as _get_reduction_statistics,
)
from kreuzberg._internal_bindings import (
    reduce_tokens as _reduce_tokens,
)

if TYPE_CHECKING:
    from kreuzberg._types import TokenReductionConfig


def reduce_tokens(
    text: str,
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> str:
    """Reduce tokens using the optimized implementation.

    Args:
        text: The text to reduce.
        config: Configuration for token reduction.
        language: Optional language code for language-specific optimizations.

    Returns:
        The reduced text.
    """
    rust_config = _convert_config_to_rust(config)
    return _reduce_tokens(text, rust_config, language)


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
    rust_config = _convert_config_to_rust(config)
    return _batch_reduce_tokens(texts, rust_config, language)


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
    ) = _get_reduction_statistics(original, reduced)

    return {
        "character_reduction_ratio": char_reduction,
        "token_reduction_ratio": token_reduction,
        "original_characters": original_chars,
        "reduced_characters": reduced_chars,
        "original_tokens": original_tokens,
        "reduced_tokens": reduced_tokens,
    }


def _convert_config_to_rust(config: TokenReductionConfig) -> TokenReductionConfigDTO:
    """Convert Python config to Rust config DTO."""
    level_mapping = {
        "off": ReductionLevelDTO.Off,
        "light": ReductionLevelDTO.Light,
        "moderate": ReductionLevelDTO.Moderate,
        "aggressive": ReductionLevelDTO.Aggressive,
        "maximum": ReductionLevelDTO.Maximum,
    }

    dto = TokenReductionConfigDTO()
    dto.level = level_mapping.get(config.mode, ReductionLevelDTO.Moderate)
    dto.language_hint = config.language_hint
    dto.preserve_markdown = config.preserve_markdown
    dto.preserve_code = True
    dto.semantic_threshold = 0.3
    dto.enable_parallel = True
    dto.use_simd = True
    dto.custom_stopwords = list(config.custom_stopwords) if config.custom_stopwords else []
    dto.preserve_patterns = []
    dto.target_reduction = None
    dto.enable_semantic_clustering = config.mode in ("aggressive", "maximum")
    return dto
