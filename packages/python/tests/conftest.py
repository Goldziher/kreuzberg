"""Shared pytest fixtures for binding-specific tests."""

from __future__ import annotations

from pathlib import Path

import pytest


@pytest.fixture
def docx_document() -> Path:
    """Path to DOCX test file used across binding-specific suites."""
    path = Path(__file__).parent.parent.parent.parent / "test_documents" / "documents" / "lorem_ipsum.docx"
    if not path.exists():
        pytest.skip(f"Test file not found: {path}")
    return path
