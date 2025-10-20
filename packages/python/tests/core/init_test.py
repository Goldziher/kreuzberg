# mypy: ignore-errors
"""Tests for kreuzberg package __init__.py module.

These tests verify the package structure, version, and public API consistency.
Redundant API export checks are covered by basic_smoke_test.py.
"""

from __future__ import annotations

import kreuzberg


def test_version() -> None:
    """Test that __version__ is available and well-formed."""
    assert hasattr(kreuzberg, "__version__")
    assert isinstance(kreuzberg.__version__, str)
    # Version should be in format X.Y.Z or similar
    assert "." in kreuzberg.__version__


def test_all_attribute() -> None:
    """Test that __all__ is defined and all exports are importable."""
    assert hasattr(kreuzberg, "__all__")
    assert isinstance(kreuzberg.__all__, list)
    assert "__version__" in kreuzberg.__all__

    # Verify all items in __all__ are actually importable
    for name in kreuzberg.__all__:
        assert hasattr(kreuzberg, name), f"Item in __all__ not importable: {name}"


def test_exception_hierarchy() -> None:
    """Test that exception classes properly inherit from KreuzbergError."""
    assert issubclass(kreuzberg.MissingDependencyError, kreuzberg.KreuzbergError)
    assert issubclass(kreuzberg.OCRError, kreuzberg.KreuzbergError)
    assert issubclass(kreuzberg.ParsingError, kreuzberg.KreuzbergError)
    assert issubclass(kreuzberg.ValidationError, kreuzberg.KreuzbergError)


def test_extraction_functions_exist() -> None:
    """Test that all extraction functions are callable."""
    assert callable(kreuzberg.extract_file)
    assert callable(kreuzberg.extract_file_sync)
    assert callable(kreuzberg.extract_bytes)
    assert callable(kreuzberg.extract_bytes_sync)
    assert callable(kreuzberg.batch_extract_files)
    assert callable(kreuzberg.batch_extract_files_sync)
    assert callable(kreuzberg.batch_extract_bytes)
    assert callable(kreuzberg.batch_extract_bytes_sync)
