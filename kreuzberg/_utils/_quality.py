"""Quality utilities for text processing."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Mapping

from kreuzberg._internal_bindings import (
    calculate_quality_score as calculate_quality_score_rust,
)
from kreuzberg._internal_bindings import (
    clean_extracted_text as clean_extracted_text_rust,
)

__all__ = ["calculate_quality_score", "clean_extracted_text"]


def calculate_quality_score(text: str, metadata: Mapping[str, Any] | None = None) -> float:
    """Calculate quality score for extracted text.

    Args:
        text: Text to analyze
        metadata: Optional metadata dictionary

    Returns:
        Quality score between 0.0 and 1.0
    """
    return calculate_quality_score_rust(text, metadata)


def clean_extracted_text(text: str) -> str:
    """Clean extracted text by removing artifacts and unwanted content.

    Args:
        text: Text to clean

    Returns:
        Cleaned text
    """
    return clean_extracted_text_rust(text)
