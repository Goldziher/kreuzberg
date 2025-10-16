"""Shared pytest fixtures for kreuzberg tests."""

import pytest


@pytest.fixture
def sample_text():
    """Sample text content for testing."""
    return "This is a sample document for testing extraction."


@pytest.fixture
def sample_config():
    """Sample extraction configuration."""
    from kreuzberg import ExtractionConfig

    return ExtractionConfig()
