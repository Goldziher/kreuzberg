from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any
from unittest.mock import patch

import pytest

if TYPE_CHECKING:
    from litestar.testing import AsyncTestClient


def test_get_max_upload_size_default() -> None:
    from kreuzberg._api.main import _get_max_upload_size

    with patch.dict(os.environ, {}, clear=True):
        assert _get_max_upload_size() == 1024 * 1024 * 1024


def test_get_max_upload_size_custom() -> None:
    from kreuzberg._api.main import _get_max_upload_size

    custom_size = 2 * 1024 * 1024 * 1024
    with patch.dict(os.environ, {"KREUZBERG_MAX_UPLOAD_SIZE": str(custom_size)}):
        assert _get_max_upload_size() == custom_size


def test_get_max_upload_size_invalid_value() -> None:
    from kreuzberg._api.main import _get_max_upload_size

    with patch.dict(os.environ, {"KREUZBERG_MAX_UPLOAD_SIZE": "invalid"}):
        assert _get_max_upload_size() == 1024 * 1024 * 1024


def test_is_opentelemetry_enabled_default() -> None:
    from kreuzberg._api.main import _is_opentelemetry_enabled

    with patch.dict(os.environ, {}, clear=True):
        assert _is_opentelemetry_enabled() is True


def test_is_opentelemetry_enabled_false() -> None:
    from kreuzberg._api.main import _is_opentelemetry_enabled

    test_cases = ["false", "False", "FALSE", "0", "no", "No", "off", "Off"]
    for value in test_cases:
        with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": value}):
            assert _is_opentelemetry_enabled() is False, f"Failed for value: {value}"


def test_is_opentelemetry_enabled_true() -> None:
    from kreuzberg._api.main import _is_opentelemetry_enabled

    test_cases = ["true", "True", "TRUE", "1", "yes", "Yes", "on", "On"]
    for value in test_cases:
        with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": value}):
            assert _is_opentelemetry_enabled() is True, f"Failed for value: {value}"


def test_get_plugins_with_opentelemetry_enabled() -> None:
    from kreuzberg._api.main import _get_plugins

    with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": "true"}):
        plugins = _get_plugins()
        assert len(plugins) == 1
        assert type(plugins[0]).__name__ == "OpenTelemetryPlugin"


def test_get_plugins_with_opentelemetry_disabled() -> None:
    from kreuzberg._api.main import _get_plugins

    with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": "false"}):
        plugins = _get_plugins()
        assert len(plugins) == 0


@pytest.mark.anyio
async def test_app_configuration_with_custom_upload_size() -> None:
    from kreuzberg._api.main import _get_max_upload_size

    custom_size = 512 * 1024 * 1024

    with patch.dict(os.environ, {"KREUZBERG_MAX_UPLOAD_SIZE": str(custom_size)}):
        assert _get_max_upload_size() == custom_size


@pytest.mark.anyio
async def test_large_file_upload_respected(test_client: AsyncTestClient[Any], tmp_path: Any) -> None:
    test_file = tmp_path / "large_test.txt"
    large_content = "x" * (2 * 1024 * 1024)
    test_file.write_text(large_content)

    with test_file.open("rb") as f:
        response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 201


def test_environment_variable_combinations() -> None:
    from kreuzberg._api.main import _get_max_upload_size, _is_opentelemetry_enabled

    test_env = {
        "KREUZBERG_MAX_UPLOAD_SIZE": "5368709120",
        "KREUZBERG_ENABLE_OPENTELEMETRY": "false",
    }

    with patch.dict(os.environ, test_env):
        assert _get_max_upload_size() == 5368709120
        assert _is_opentelemetry_enabled() is False


def test_edge_cases_for_upload_size() -> None:
    from kreuzberg._api.main import _get_max_upload_size

    with patch.dict(os.environ, {"KREUZBERG_MAX_UPLOAD_SIZE": "0"}):
        assert _get_max_upload_size() == 0

    large_size = str(10 * 1024 * 1024 * 1024)
    with patch.dict(os.environ, {"KREUZBERG_MAX_UPLOAD_SIZE": large_size}):
        assert _get_max_upload_size() == int(large_size)

    with patch.dict(os.environ, {"KREUZBERG_MAX_UPLOAD_SIZE": "-1"}):
        assert _get_max_upload_size() == 1024 * 1024 * 1024


def test_edge_cases_for_opentelemetry() -> None:
    from kreuzberg._api.main import _is_opentelemetry_enabled

    with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": ""}):
        assert _is_opentelemetry_enabled() is False

    with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": "random"}):
        assert _is_opentelemetry_enabled() is False

    with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": "2"}):
        assert _is_opentelemetry_enabled() is False

    with patch.dict(os.environ, {"KREUZBERG_ENABLE_OPENTELEMETRY": "1"}):
        assert _is_opentelemetry_enabled() is True
