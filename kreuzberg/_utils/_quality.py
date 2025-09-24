"""Quality utilities for text processing."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Mapping

from kreuzberg._internal_bindings import (
    calculate_quality_score as _calculate_quality_score,
)
from kreuzberg._internal_bindings import (
    clean_extracted_text as _clean_extracted_text,
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
    return _calculate_quality_score(text, metadata)


def clean_extracted_text(text: str) -> str:
    """Clean extracted text by removing artifacts and unwanted content.

    Args:
        text: Text to clean

    Returns:
        Cleaned text
    """
    return _clean_extracted_text(text)
