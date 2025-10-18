"""Shared pytest fixtures for kreuzberg tests."""

from __future__ import annotations

import pytest

from kreuzberg import ExtractionConfig


@pytest.fixture
def sample_text() -> str:
    """Sample text content for testing."""
    return "This is a sample document for testing extraction."


@pytest.fixture
def sample_config() -> ExtractionConfig:
    """Sample extraction configuration."""
    return ExtractionConfig()
