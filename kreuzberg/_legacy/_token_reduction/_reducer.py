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
from kreuzberg._types import _normalize_stopwords_config

if TYPE_CHECKING:
    from kreuzberg._types import TokenReductionConfig


def reduce_tokens(
    text: str,
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> str:
    rust_config = _convert_config_to_rust(config)
    return _reduce_tokens(text, rust_config, language)


def batch_reduce_tokens(
    texts: list[str],
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> list[str]:
    rust_config = _convert_config_to_rust(config)
    return _batch_reduce_tokens(texts, rust_config, language)


def get_reduction_statistics(original: str, reduced: str) -> dict[str, float | int]:
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
    normalized_stopwords = _normalize_stopwords_config(config.custom_stopwords)
    if normalized_stopwords:
        dto.custom_stopwords = {lang: list(words) for lang, words in normalized_stopwords}
    else:
        dto.custom_stopwords = None
    dto.preserve_patterns = []
    dto.target_reduction = None
    dto.enable_semantic_clustering = config.mode in ("aggressive", "maximum")
    return dto
