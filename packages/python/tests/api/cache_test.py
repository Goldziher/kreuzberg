from typing import Any

import pytest
from litestar.testing import AsyncTestClient


@pytest.mark.anyio
async def test_get_cache_stats_all(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/cache/stats")
    assert response.status_code == 200
    data = response.json()
    assert "ocr" in data
    assert "documents" in data
    assert "tables" in data
    assert "mime" in data
    assert "total_size_mb" in data
    assert "total_files" in data


@pytest.mark.anyio
async def test_get_cache_stats_ocr(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/cache/ocr/stats")
    assert response.status_code == 200
    data = response.json()
    assert isinstance(data, dict)


@pytest.mark.anyio
async def test_get_cache_stats_documents(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/cache/documents/stats")
    assert response.status_code == 200
    data = response.json()
    assert isinstance(data, dict)


@pytest.mark.anyio
async def test_get_cache_stats_tables(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/cache/tables/stats")
    assert response.status_code == 200
    data = response.json()
    assert isinstance(data, dict)


@pytest.mark.anyio
async def test_get_cache_stats_mime(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/cache/mime/stats")
    assert response.status_code == 200
    data = response.json()
    assert isinstance(data, dict)


@pytest.mark.anyio
async def test_get_cache_stats_invalid_type(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/cache/invalid/stats")
    assert response.status_code == 400
    data = response.json()
    assert "Invalid cache type" in data["message"]
    assert data["context"]["cache_type"] == "invalid"


@pytest.mark.anyio
async def test_clear_cache_all(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.delete("/cache/all")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Cache 'all' cleared successfully"
    assert data["cache_type"] == "all"
    assert "cleared_at" in data


@pytest.mark.anyio
async def test_clear_cache_ocr(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.delete("/cache/ocr")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Cache 'ocr' cleared successfully"
    assert data["cache_type"] == "ocr"


@pytest.mark.anyio
async def test_clear_cache_documents(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.delete("/cache/documents")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Cache 'documents' cleared successfully"
    assert data["cache_type"] == "documents"


@pytest.mark.anyio
async def test_clear_cache_tables(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.delete("/cache/tables")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Cache 'tables' cleared successfully"
    assert data["cache_type"] == "tables"


@pytest.mark.anyio
async def test_clear_cache_mime(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.delete("/cache/mime")
    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Cache 'mime' cleared successfully"
    assert data["cache_type"] == "mime"


@pytest.mark.anyio
async def test_clear_cache_invalid_type(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.delete("/cache/invalid")
    assert response.status_code == 400
    data = response.json()
    assert "Invalid cache type" in data["message"]
    assert data["context"]["cache_type"] == "invalid"
