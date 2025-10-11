from pathlib import Path
from typing import Any
from unittest.mock import AsyncMock, patch

import pytest
from litestar.testing import AsyncTestClient


@pytest.mark.anyio
async def test_missing_dependency_error_handler(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import MissingDependencyError

    test_file = tmp_path / "test.txt"
    test_file.write_text("test content")

    with patch("kreuzberg._api.main.extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = MissingDependencyError(
            "tesseract-ocr is required", context={"dependency": "tesseract-ocr"}
        )

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("files", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 503
    data = response.json()
    assert "tesseract-ocr is required" in data["message"]
    assert data["error_type"] == "MissingDependencyError"
    assert data["context"]["dependency"] == "tesseract-ocr"


@pytest.mark.anyio
async def test_memory_limit_error_handler(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import MemoryLimitError

    test_file = tmp_path / "test.txt"
    test_file.write_text("test content")

    with patch("kreuzberg._api.main.extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = MemoryLimitError("Memory limit exceeded", context={"limit_mb": 1024})

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("files", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 507
    data = response.json()
    assert "Memory limit exceeded" in data["message"]
    assert data["error_type"] == "MemoryLimitError"
    assert data["context"]["limit_mb"] == 1024


@pytest.mark.anyio
async def test_generic_kreuzberg_error_handler(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import KreuzbergError

    test_file = tmp_path / "test.txt"
    test_file.write_text("test content")

    with patch("kreuzberg._api.main.extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = KreuzbergError("Generic error", context={"detail": "test"})

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("files", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 500
    data = response.json()
    assert "Generic error" in data["message"]
    assert data["error_type"] == "KreuzbergError"
    assert data["context"]["detail"] == "test"


@pytest.mark.anyio
async def test_max_upload_size_with_invalid_env_var(
    test_client: AsyncTestClient[Any], monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("KREUZBERG_MAX_UPLOAD_SIZE", "invalid_number")

    response = await test_client.get("/info")
    assert response.status_code == 200


@pytest.mark.anyio
async def test_max_upload_size_with_negative_value(
    test_client: AsyncTestClient[Any], monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("KREUZBERG_MAX_UPLOAD_SIZE", "-1000")

    response = await test_client.get("/info")
    assert response.status_code == 200


@pytest.mark.anyio
async def test_info_endpoint_cache_disabled(test_client: AsyncTestClient[Any], monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("KREUZBERG_CACHE_ENABLED", "false")

    response = await test_client.get("/info")
    assert response.status_code == 200
    data = response.json()
    assert data["cache_enabled"] is False


@pytest.mark.anyio
async def test_info_endpoint_cache_enabled_variations(
    test_client: AsyncTestClient[Any], monkeypatch: pytest.MonkeyPatch
) -> None:
    for value in ["true", "1", "yes", "on"]:
        monkeypatch.setenv("KREUZBERG_CACHE_ENABLED", value)
        response = await test_client.get("/info")
        assert response.status_code == 200
        data = response.json()
        assert data["cache_enabled"] is True


@pytest.mark.anyio
async def test_info_endpoint_shows_backend_availability(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/info")
    assert response.status_code == 200
    data = response.json()

    assert "available_backends" in data
    assert "ocr" in data["available_backends"]
    assert "features" in data["available_backends"]

    assert "tesseract" in data["available_backends"]["ocr"]
    assert "easyocr" in data["available_backends"]["ocr"]
    assert "paddleocr" in data["available_backends"]["ocr"]

    assert data["available_backends"]["ocr"]["tesseract"] is True

    assert "vision_tables" in data["available_backends"]["features"]
    assert "spacy" in data["available_backends"]["features"]
