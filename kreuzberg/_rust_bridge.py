"""Bridge module for Rust acceleration functions."""

from __future__ import annotations

from typing import Any

from kreuzberg.kreuzberg_rust import (  # type: ignore[import-not-found]
    batch_process_texts_rust,
    calculate_quality_score_rust,
    clean_extracted_text_rust,
    normalize_spaces_rust,
    safe_decode_rust,
)

__all__ = [
    "batch_process_texts",
    "calculate_quality_score",
    "clean_extracted_text",
    "normalize_spaces",
    "safe_decode",
]


def calculate_quality_score(text: str, metadata: dict[str, Any] | None = None) -> float:
    """Calculate quality score for extracted text."""
    return calculate_quality_score_rust(text, metadata)  # type: ignore[no-any-return]


def clean_extracted_text(text: str) -> str:
    """Clean extracted text by removing artifacts and unwanted content."""
    return clean_extracted_text_rust(text)  # type: ignore[no-any-return]


def normalize_spaces(text: str) -> str:
    """Normalize spaces in text."""
    return normalize_spaces_rust(text)  # type: ignore[no-any-return]


def safe_decode(byte_data: bytes, encoding: str | None = None) -> str:
    """Safe decode bytes to string with encoding detection."""
    return safe_decode_rust(byte_data, encoding)  # type: ignore[no-any-return]


def batch_process_texts(texts: list[str]) -> list[str]:
    """Process multiple texts in parallel."""
    return batch_process_texts_rust(texts)  # type: ignore[no-any-return]
