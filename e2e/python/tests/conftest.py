"""
Pytest configuration for E2E tests.

Provides fixtures and markers for managing test execution,
particularly for slow tests that may timeout in CI environments.
"""

from __future__ import annotations

import contextlib

import pytest


def pytest_configure(config: pytest.Config) -> None:
    """Register custom markers."""
    config.addinivalue_line(
        "markers",
        "slow: marks tests as slow (deselect with '-m \"not slow\"')",
    )


@pytest.fixture(autouse=True)
def _slow_test_timeout(request: pytest.FixtureRequest) -> None:
    """
    Auto-apply timeout to slow tests.

    Slow tests get 300 seconds (5 minutes) timeout.
    Regular tests get default timeout.
    """

    # Try to set pytest-timeout if available
    with contextlib.suppress(pytest.FixtureLookupError):
        request.getfixturevalue("pytest_timeout_set_sleep_func")
