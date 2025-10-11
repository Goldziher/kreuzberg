from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any
from unittest.mock import patch

import pytest

from kreuzberg import ChunkingConfig, ExtractionConfig

if TYPE_CHECKING:
    from pathlib import Path

    from litestar.testing import AsyncTestClient


@pytest.mark.anyio
class TestExtractionEndpoint:
    async def test_extract_single_file(self, test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
        with searchable_pdf.open("rb") as f:
            response = await test_client.post(
                "/extract",
                files=[("files", (searchable_pdf.name, f.read(), "application/pdf"))],
            )

        assert response.status_code == 201
        data = response.json()
        assert len(data) == 1
        assert "content" in data[0]
        assert "mime_type" in data[0]
        assert len(data[0]["content"]) > 0

    async def test_extract_multiple_files(
        self, test_client: AsyncTestClient[Any], searchable_pdf: Path, ocr_image: Path
    ) -> None:
        with searchable_pdf.open("rb") as pdf_f, ocr_image.open("rb") as img_f:
            response = await test_client.post(
                "/extract",
                files=[
                    ("files", (searchable_pdf.name, pdf_f.read(), "application/pdf")),
                    ("files", (ocr_image.name, img_f.read(), "image/jpeg")),
                ],
            )

        assert response.status_code == 201
        data = response.json()
        assert len(data) == 2
        for item in data:
            assert "content" in item
            assert "mime_type" in item

    async def test_extract_with_config(self, test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
        config = {"chunking": {"max_chars": 300, "max_overlap": 50}}

        with searchable_pdf.open("rb") as f:
            response = await test_client.post(
                "/extract",
                files=[("files", (searchable_pdf.name, f.read(), "application/pdf"))],
                data={"config": json.dumps(config)},
            )

        assert response.status_code == 201
        data = response.json()
        assert len(data) == 1
        assert len(data[0]["chunks"]) > 0
        for chunk in data[0]["chunks"]:
            assert len(chunk) <= 350

    async def test_extract_no_files_error(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.post("/extract", files=[])

        assert response.status_code == 400
        error = response.json()
        assert error["error_type"] == "ValidationError"
        assert "No files provided" in error["message"]
        assert "context" in error
        assert error["context"]["file_count"] == 0

    async def test_extract_with_static_config(
        self, test_client: AsyncTestClient[Any], searchable_pdf: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        static_config = ExtractionConfig(chunking=ChunkingConfig(max_chars=500))

        with patch("kreuzberg._api.main.discover_config_cached", return_value=static_config):
            with searchable_pdf.open("rb") as f:
                response = await test_client.post(
                    "/extract",
                    files=[("files", (searchable_pdf.name, f.read(), "application/pdf"))],
                )

        assert response.status_code == 201
        data = response.json()
        assert len(data[0]["chunks"]) > 0

    async def test_extract_result_serialization(self, test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
        config = {
            "chunking": {"max_chars": 300, "max_overlap": 50},
            "language_detection": {},
        }

        with searchable_pdf.open("rb") as f:
            response = await test_client.post(
                "/extract",
                files=[("files", (searchable_pdf.name, f.read(), "application/pdf"))],
                data={"config": json.dumps(config)},
            )

        assert response.status_code == 201
        data = response.json()
        result = data[0]

        assert "content" in result
        assert "mime_type" in result
        assert "metadata" in result
        assert "chunks" in result
        assert len(result["chunks"]) > 0
        assert "detected_languages" in result or result["detected_languages"] is None

    async def test_extract_rejects_legacy_fields(self, test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
        legacy_config = {"chunk_content": True}

        with searchable_pdf.open("rb") as f:
            response = await test_client.post(
                "/extract",
                files=[("files", (searchable_pdf.name, f.read(), "application/pdf"))],
                data={"config": json.dumps(legacy_config)},
            )

        assert response.status_code == 400
        error = response.json()
        assert error["error_type"] == "ValidationError"
        assert "Legacy configuration fields" in error["message"]


@pytest.mark.anyio
class TestCacheEndpoints:
    async def test_get_all_cache_stats(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/stats")

        assert response.status_code == 200
        data = response.json()

        assert "ocr" in data
        assert "documents" in data
        assert "tables" in data
        assert "mime" in data
        assert "total_size_mb" in data
        assert "total_files" in data

        assert isinstance(data["total_size_mb"], (int, float))
        assert isinstance(data["total_files"], int)

    async def test_get_specific_cache_stats_ocr(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/ocr/stats")

        assert response.status_code == 200
        data = response.json()

        assert "cache_type" in data
        assert data["cache_type"] == "ocr"
        assert "cached_results" in data
        assert "total_cache_size_mb" in data

    async def test_get_specific_cache_stats_documents(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/documents/stats")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "documents"

    async def test_get_specific_cache_stats_tables(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/tables/stats")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "tables"

    async def test_get_specific_cache_stats_mime(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/mime/stats")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "mime"

    async def test_get_cache_stats_all(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/all/stats")

        assert response.status_code == 200
        data = response.json()

        assert "ocr" in data
        assert "documents" in data
        assert "tables" in data
        assert "mime" in data

    async def test_get_cache_stats_invalid_type(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/invalid/stats")

        assert response.status_code == 400
        error = response.json()
        assert error["error_type"] == "ValidationError"
        assert "Invalid cache type" in error["message"]
        assert "context" in error
        assert error["context"]["cache_type"] == "invalid"

    async def test_clear_cache_all(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.delete("/cache/all")

        assert response.status_code == 200
        data = response.json()

        assert data["message"] == "Cache 'all' cleared successfully"
        assert data["cache_type"] == "all"
        assert "cleared_at" in data
        assert isinstance(data["cleared_at"], (int, float))

    async def test_clear_cache_ocr(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.delete("/cache/ocr")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "ocr"

    async def test_clear_cache_documents(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.delete("/cache/documents")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "documents"

    async def test_clear_cache_tables(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.delete("/cache/tables")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "tables"

    async def test_clear_cache_mime(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.delete("/cache/mime")

        assert response.status_code == 200
        data = response.json()
        assert data["cache_type"] == "mime"

    async def test_clear_cache_invalid_type(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.delete("/cache/invalid")

        assert response.status_code == 400
        error = response.json()
        assert error["error_type"] == "ValidationError"
        assert "Invalid cache type" in error["message"]


@pytest.mark.anyio
class TestInfoEndpoint:
    async def test_get_info(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/info")

        assert response.status_code == 200
        data = response.json()

        assert "version" in data
        assert "config" in data
        assert "cache_enabled" in data
        assert "available_backends" in data

        assert isinstance(data["version"], str)
        assert isinstance(data["cache_enabled"], bool)
        assert isinstance(data["available_backends"], dict)

        backends = data["available_backends"]
        assert "tesseract" in backends
        assert "easyocr" in backends
        assert "paddleocr" in backends
        assert "vision_tables" in backends
        assert "spacy" in backends

    async def test_get_info_with_static_config(self, test_client: AsyncTestClient[Any]) -> None:
        static_config = ExtractionConfig(chunking=ChunkingConfig(max_chars=1000))

        with patch("kreuzberg._api.main.discover_config_cached", return_value=static_config):
            response = await test_client.get("/info")

        assert response.status_code == 200
        data = response.json()

        assert data["config"] is not None
        assert data["config"]["chunk_content"] is True
        assert data["config"]["max_chars"] == 1000


@pytest.mark.anyio
class TestHealthEndpoint:
    async def test_health_check(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/health")

        assert response.status_code == 200
        data = response.json()

        assert data["status"] == "ok"


@pytest.mark.anyio
class TestExceptionHandling:
    async def test_validation_error_400(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.post("/extract", files=[])

        assert response.status_code == 400
        error = response.json()

        assert error["error_type"] == "ValidationError"
        assert "message" in error
        assert "context" in error
        assert error["status_code"] == 400

    async def test_invalid_cache_type_400(self, test_client: AsyncTestClient[Any]) -> None:
        response = await test_client.get("/cache/invalid/stats")

        assert response.status_code == 400
        error = response.json()

        assert error["error_type"] == "ValidationError"
        assert "context" in error
        assert "valid_types" in error["context"]


@pytest.mark.anyio
class TestResponseSerialization:
    async def test_serialization_with_images(self, test_client: AsyncTestClient[Any]) -> None:
        html = (
            b"<html><body>"
            b'<img src="data:image/png;base64,'
            b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
            b'" alt="Red dot">'
            b"</body></html>"
        )

        config = {"extract_images": True}

        response = await test_client.post(
            "/extract",
            files=[("files", ("test.html", html, "text/html"))],
            data={"config": json.dumps(config)},
        )

        assert response.status_code == 201
        data = response.json()
        assert len(data) == 1

        if "images" in data[0] and data[0]["images"]:
            img = data[0]["images"][0]
            assert "data" in img
            assert "format" in img

    async def test_serialization_bytes_as_base64(self, test_client: AsyncTestClient[Any]) -> None:
        html = b"<html><body><p>Test content</p></body></html>"

        config = {"extract_images": True}

        response = await test_client.post(
            "/extract",
            files=[("files", ("test.html", html, "text/html"))],
            data={"config": json.dumps(config)},
        )

        assert response.status_code == 201
        data = response.json()
        assert isinstance(data, list)
        assert len(data) == 1
