from __future__ import annotations

from kreuzberg._constants import (
    DEFAULT_MAX_CHARACTERS,
    DEFAULT_MAX_OVERLAP,
    MINIMAL_SUPPORTED_PANDOC_VERSION,
    PDF_POINTS_PER_INCH,
)


def test_constants_values() -> None:
    assert MINIMAL_SUPPORTED_PANDOC_VERSION == 2
    assert DEFAULT_MAX_CHARACTERS == 2000
    assert DEFAULT_MAX_OVERLAP == 100
    assert PDF_POINTS_PER_INCH == 72.0


def test_constants_are_final() -> None:
    assert isinstance(MINIMAL_SUPPORTED_PANDOC_VERSION, int)
    assert isinstance(DEFAULT_MAX_CHARACTERS, int)
    assert isinstance(DEFAULT_MAX_OVERLAP, int)
    assert isinstance(PDF_POINTS_PER_INCH, float)
