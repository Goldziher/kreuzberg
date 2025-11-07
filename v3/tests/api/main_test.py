from __future__ import annotations

from typing import TYPE_CHECKING, Any
from unittest.mock import AsyncMock, Mock, patch

import pytest
from kreuzberg._api.main import exception_handler
from kreuzberg.exceptions import OCRError, ParsingError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path

    from litestar.testing import AsyncTestClient


@pytest.mark.anyio
async def test_health_check(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok"}


@pytest.mark.anyio
async def test_extract_from_file(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract", files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))]
        )

    assert response.status_code == 201
    data = response.json()
    assert "Sample PDF" in data[0]["content"]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_from_multiple_files(
    test_client: AsyncTestClient[Any], searchable_pdf: Path, scanned_pdf: Path
) -> None:
    with searchable_pdf.open("rb") as f1, scanned_pdf.open("rb") as f2:
        response = await test_client.post(
            "/extract",
            files=[
                ("data", (searchable_pdf.name, f1.read(), "application/pdf")),
                ("data", (scanned_pdf.name, f2.read(), "application/pdf")),
            ],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 2
    assert "Sample PDF" in data[0]["content"]
    assert data[1]["content"]


@pytest.mark.anyio
async def test_extract_from_file_extraction_error(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello world")

    with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = Exception("Test error")
        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 500
    error_response = response.json()

    assert "error_type" in error_response
    assert "message" in error_response
    assert error_response["error_type"] == "Exception"
    assert error_response["message"] == "Test error"


@pytest.mark.anyio
async def test_extract_validation_error_response(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello world")

    with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = ValidationError("Invalid configuration", context={"param": "invalid_value"})
        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 400
    error_response = response.json()
    assert "Invalid configuration" in error_response["message"]
    assert '"param": "invalid_value"' in error_response["details"]


@pytest.mark.anyio
async def test_extract_parsing_error_response(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello world")

    with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = ParsingError("Failed to parse document", context={"file_type": "unknown"})
        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 422
    error_response = response.json()
    assert "Failed to parse document" in error_response["message"]
    assert '"file_type": "unknown"' in error_response["details"]


@pytest.mark.anyio
async def test_extract_ocr_error_response(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello world")

    with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = OCRError("OCR processing failed", context={"engine": "tesseract"})
        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 500
    error_response = response.json()
    assert "OCR processing failed" in error_response["message"]
    assert '"engine": "tesseract"' in error_response["details"]


@pytest.mark.anyio
async def test_extract_from_unsupported_file(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.unsupported"
    test_file.write_text("hello world")

    with test_file.open("rb") as f:
        response = await test_client.post("/extract", files=[("data", (test_file.name, f.read()))])

    assert response.status_code in [201, 400, 422]
    if response.status_code != 201:
        error_response = response.json()
        assert "message" in error_response


@pytest.mark.anyio
async def test_extract_from_docx(test_client: AsyncTestClient[Any], docx_document: Path) -> None:
    with docx_document.open("rb") as f:
        response = await test_client.post(
            "/extract",
            files=[
                (
                    "data",
                    (
                        docx_document.name,
                        f.read(),
                        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                    ),
                )
            ],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_from_image(test_client: AsyncTestClient[Any], ocr_image: Path) -> None:
    with ocr_image.open("rb") as f:
        response = await test_client.post("/extract", files=[("data", (ocr_image.name, f.read(), "image/jpeg"))])

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_from_excel(test_client: AsyncTestClient[Any], excel_document: Path) -> None:
    with excel_document.open("rb") as f:
        response = await test_client.post(
            "/extract",
            files=[
                (
                    "data",
                    (
                        excel_document.name,
                        f.read(),
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    ),
                )
            ],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_from_html(test_client: AsyncTestClient[Any], html_document: Path) -> None:
    with html_document.open("rb") as f:
        response = await test_client.post("/extract", files=[("data", (html_document.name, f.read(), "text/html"))])

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_from_markdown(test_client: AsyncTestClient[Any], markdown_document: Path) -> None:
    with markdown_document.open("rb") as f:
        response = await test_client.post(
            "/extract", files=[("data", (markdown_document.name, f.read(), "text/markdown"))]
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_from_pptx(test_client: AsyncTestClient[Any], pptx_document: Path) -> None:
    with pptx_document.open("rb") as f:
        response = await test_client.post(
            "/extract",
            files=[
                (
                    "data",
                    (
                        pptx_document.name,
                        f.read(),
                        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                    ),
                )
            ],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_mixed_file_types(
    test_client: AsyncTestClient[Any], searchable_pdf: Path, docx_document: Path, excel_document: Path
) -> None:
    files = []
    with searchable_pdf.open("rb") as f1, docx_document.open("rb") as f2, excel_document.open("rb") as f3:
        files = [
            ("data", (searchable_pdf.name, f1.read(), "application/pdf")),
            (
                "data",
                (
                    docx_document.name,
                    f2.read(),
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                ),
            ),
            (
                "data",
                (excel_document.name, f3.read(), "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            ),
        ]
        response = await test_client.post("/extract", files=files)

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 3
    for item in data:
        assert "content" in item
        assert item["mime_type"] in ["text/plain", "text/markdown"]


@pytest.mark.anyio
async def test_extract_empty_file_list(test_client: AsyncTestClient[Any]) -> None:
    response = await test_client.post("/extract", files=[])
    assert response.status_code == 500


@pytest.mark.anyio
async def test_extract_non_ascii_pdf(test_client: AsyncTestClient[Any], non_ascii_pdf: Path) -> None:
    with non_ascii_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract", files=[("data", (non_ascii_pdf.name, f.read(), "application/pdf"))]
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]
    assert data[0]["mime_type"] in ["text/plain", "text/markdown"]


def test_exception_handler_validation_error() -> None:
    mock_request = Mock()
    mock_request.method = "POST"
    mock_request.url = "http://test.com/extract"
    mock_app = Mock()
    mock_app.logger = Mock()
    mock_request.app = mock_app

    error = ValidationError("Invalid input", context={"field": "test"})

    response = exception_handler(mock_request, error)

    assert response.status_code == 400
    assert "Invalid input" in response.content["message"]
    assert '"field": "test"' in response.content["details"]

    mock_app.logger.error.assert_called_once()
    call_args = mock_app.logger.error.call_args
    assert call_args[0][0] == "API error"
    assert call_args[1]["method"] == "POST"
    assert call_args[1]["status_code"] == 400


def test_exception_handler_parsing_error() -> None:
    mock_request = Mock()
    mock_request.method = "GET"
    mock_request.url = "http://test.com/health"
    mock_app = Mock()
    mock_app.logger = Mock()
    mock_request.app = mock_app

    error = ParsingError("Failed to parse document", context={"file": "test.pdf"})

    response = exception_handler(mock_request, error)

    assert response.status_code == 422
    assert "Failed to parse document" in response.content["message"]
    assert '"file": "test.pdf"' in response.content["details"]


def test_exception_handler_other_error() -> None:
    mock_request = Mock()
    mock_request.method = "POST"
    mock_request.url = "http://test.com/extract"
    mock_app = Mock()
    mock_app.logger = Mock()
    mock_request.app = mock_app

    error = OCRError("OCR processing failed", context={"engine": "tesseract"})

    response = exception_handler(mock_request, error)

    assert response.status_code == 500
    assert "OCR processing failed" in response.content["message"]
    assert '"engine": "tesseract"' in response.content["details"]


def test_exception_handler_no_logger() -> None:
    mock_request = Mock()
    mock_request.method = "POST"
    mock_request.url = "http://test.com/extract"
    mock_app = Mock()
    mock_app.logger = None
    mock_request.app = mock_app

    error = ValidationError("Invalid input", context={"field": "test"})

    response = exception_handler(mock_request, error)

    assert response.status_code == 400
    assert "Invalid input" in response.content["message"]


def test_import_error_handling() -> None:
    from kreuzberg.exceptions import MissingDependencyError

    import_error = ImportError("No module named 'litestar'")
    try:
        raise MissingDependencyError.create_for_package(
            dependency_group="litestar",
            functionality="Litestar API and docker container",
            package_name="litestar",
        ) from import_error
    except MissingDependencyError as e:
        assert "litestar" in str(e).lower()
        assert e.__cause__ is import_error


@pytest.mark.anyio
async def test_get_configuration_no_config(
    test_client: AsyncTestClient[Any], tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    with patch("kreuzberg._api.main.discover_config", return_value=None):
        response = await test_client.get("/config")

    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "No configuration file found"
    assert data["config"] is None


@pytest.mark.anyio
async def test_get_configuration_with_config(test_client: AsyncTestClient[Any]) -> None:
    from kreuzberg import ExtractionConfig, PSMMode, TesseractConfig

    test_config = ExtractionConfig(
        ocr_backend="tesseract",
        ocr_config=TesseractConfig(language="fra", psm=PSMMode.SINGLE_BLOCK),
        extract_tables=True,
        chunk_content=True,
        enable_quality_processing=True,
        max_chars=5000,
    )

    with patch("kreuzberg._api.main.discover_config", return_value=test_config):
        response = await test_client.get("/config")

    assert response.status_code == 200
    data = response.json()
    assert data["message"] == "Configuration loaded successfully"
    assert data["config"] is not None
    assert data["config"]["ocr_backend"] == "tesseract"
    assert data["config"]["extract_tables"] is True
    assert data["config"]["chunk_content"] is True
    assert data["config"]["enable_quality_processing"] is True
    assert data["config"]["max_chars"] == 5000


@pytest.mark.anyio
async def test_extract_with_discovered_config(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    from kreuzberg import ExtractionConfig

    test_config = ExtractionConfig(chunk_content=True, max_chars=1000, max_overlap=200)

    with patch("kreuzberg._api.main.discover_config_cached", return_value=test_config):
        with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
            mock_extract.return_value = [
                {"content": "Test content", "mime_type": "text/plain", "metadata": {}, "chunks": ["chunk1", "chunk2"]}
            ]

            with searchable_pdf.open("rb") as f:
                response = await test_client.post(
                    "/extract", files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))]
                )

            assert mock_extract.called
            call_args = mock_extract.call_args
            used_config = call_args[1]["config"]
            assert used_config.chunk_content is True
            assert used_config.max_chars == 1000
            assert used_config.max_overlap == 200

    assert response.status_code == 201


@pytest.mark.anyio
async def test_extract_without_discovered_config(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with patch("kreuzberg._api.main.discover_config", return_value=None):
        with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
            mock_extract.return_value = [
                {"content": "Test content", "mime_type": "text/plain", "metadata": {}, "chunks": []}
            ]

            with searchable_pdf.open("rb") as f:
                response = await test_client.post(
                    "/extract", files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))]
                )

            assert mock_extract.called
            call_args = mock_extract.call_args
            used_config = call_args[1]["config"]
            assert used_config.chunk_content is False
            assert used_config.max_chars == 2000

    assert response.status_code == 201


@pytest.mark.anyio
async def test_extract_large_file_list(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    files = []
    for i in range(20):
        test_file = tmp_path / f"test_{i}.txt"
        test_file.write_text(f"Content {i}")
        with test_file.open("rb") as f:
            files.append(("data", (test_file.name, f.read(), "text/plain")))

    response = await test_client.post("/extract", files=files)

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 20
    for i, item in enumerate(data):
        assert f"Content {i}" in item["content"]


@pytest.mark.anyio
async def test_extract_with_custom_mime_types(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.bin"
    test_file.write_bytes(b"binary content")

    with test_file.open("rb") as f:
        response = await test_client.post("/extract", files=[("data", (test_file.name, f.read()))])

    assert response.status_code in [201, 400, 422]


@pytest.mark.anyio
async def test_extract_file_with_none_content_type(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello world")

    with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.return_value = [
            {"content": "Hello world", "mime_type": "text/plain", "metadata": {}, "chunks": []}
        ]

        with test_file.open("rb") as f:
            response = await test_client.post(
                "/extract",
                files=[("data", (test_file.name, f.read(), None))],
            )

        assert mock_extract.called
        call_args = mock_extract.call_args[0][0]
        assert len(call_args) == 1

    assert response.status_code == 201


@pytest.mark.anyio
async def test_health_check_idempotent(test_client: AsyncTestClient[Any]) -> None:
    responses = []
    for _ in range(5):
        response = await test_client.get("/health")
        responses.append(response)

    for response in responses:
        assert response.status_code == 200
        assert response.json() == {"status": "ok"}


@pytest.mark.anyio
async def test_extract_memory_error_handling(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    test_file = tmp_path / "test.txt"
    test_file.write_text("test content")

    with patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract:
        mock_extract.side_effect = MemoryError("Out of memory")

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 500


def test_exception_handler_with_empty_context() -> None:
    mock_request = Mock()
    mock_request.method = "POST"
    mock_request.url = "http://test.com/extract"
    mock_app = Mock()
    mock_app.logger = Mock()
    mock_request.app = mock_app

    error = ValidationError("Test error", context={})

    response = exception_handler(mock_request, error)

    assert response.status_code == 400
    assert response.content["message"] == "ValidationError: Test error"
    assert response.content["details"] == "{}"


def test_exception_handler_context_serialization() -> None:
    mock_request = Mock()
    mock_request.method = "POST"
    mock_request.url = "http://test.com/extract"
    mock_app = Mock()
    mock_app.logger = Mock()
    mock_request.app = mock_app

    error = ParsingError(
        "Complex error",
        context={"numbers": [1, 2, 3], "nested": {"key": "value"}, "boolean": True, "none": None, "float": 3.14},
    )

    response = exception_handler(mock_request, error)

    assert response.status_code == 422
    assert '"numbers": [1, 2, 3]' in response.content["details"]
    assert '"nested": {"key": "value"}' in response.content["details"]
    assert '"boolean": true' in response.content["details"]
    assert '"none": null' in response.content["details"]
    assert '"float": 3.14' in response.content["details"]


@pytest.mark.anyio
async def test_api_routes_registration(test_client: AsyncTestClient[Any]) -> None:
    from kreuzberg._api.main import app

    routes = [(route.path, route.methods) for route in app.routes if hasattr(route, "path")]

    expected_routes = [("/extract", ["POST"]), ("/health", ["GET"]), ("/config", ["GET"])]

    for path, methods in expected_routes:
        found = False
        for route_path, route_methods in routes:
            if path in str(route_path):
                found = True
                for method in methods:
                    assert method in route_methods
                break
        assert found, f"Route {path} not found"


@pytest.mark.anyio
async def test_opentelemetry_plugin_loaded() -> None:
    from kreuzberg._api.main import app

    plugin_types = [type(plugin).__name__ for plugin in app.plugins]
    assert "OpenTelemetryPlugin" in plugin_types


@pytest.mark.anyio
async def test_structured_logging_configured() -> None:
    from kreuzberg._api.main import app

    assert app.logging_config is not None
    assert type(app.logging_config).__name__ == "StructLoggingConfig"


@pytest.mark.anyio
async def test_exception_handlers_registered() -> None:
    from kreuzberg._api.main import app
    from kreuzberg.exceptions import KreuzbergError

    assert KreuzbergError in app.exception_handlers


@pytest.mark.anyio
async def test_msgspec_serialization_deterministic(test_client: AsyncTestClient[Any]) -> None:
    import msgspec
    from kreuzberg import ExtractionConfig

    config = ExtractionConfig(
        ocr_backend="tesseract", extract_tables=True, chunk_content=True, max_chars=5000, max_overlap=1000
    )

    serialized_results = []
    for _ in range(5):
        serialized = msgspec.to_builtins(config, order="deterministic")
        serialized_results.append(str(serialized))

    assert all(s == serialized_results[0] for s in serialized_results)


@pytest.mark.anyio
async def test_extract_with_invalid_config_file(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import ValidationError

    with patch("kreuzberg._api.main.discover_config_cached") as mock_discover:
        mock_discover.side_effect = ValidationError(
            "Invalid TOML in configuration file: Expected '=' after key",
            context={"file": "/path/to/kreuzberg.toml", "error": "TOML parse error"},
        )

        test_file = tmp_path / "test.txt"
        test_file.write_text("test content")

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 400
    error_response = response.json()
    assert "Invalid TOML in configuration file" in error_response["message"]
    assert "TOML parse error" in error_response["details"]
    assert "/path/to/kreuzberg.toml" in error_response["details"]


@pytest.mark.anyio
async def test_get_config_with_invalid_config_file(test_client: AsyncTestClient[Any]) -> None:
    from kreuzberg.exceptions import ValidationError

    with patch("kreuzberg._api.main.discover_config") as mock_discover:
        mock_discover.side_effect = ValidationError(
            "Invalid OCR backend: unknown. Must be one of: easyocr, paddleocr, tesseract or 'none'",
            context={"provided": "unknown", "valid": ["easyocr", "paddleocr", "tesseract"]},
        )

        response = await test_client.get("/config")

    assert response.status_code == 400
    error_response = response.json()
    assert "Invalid OCR backend: unknown" in error_response["message"]
    assert '"provided": "unknown"' in error_response["details"]
    assert "easyocr" in error_response["details"]


@pytest.mark.anyio
async def test_extract_with_invalid_ocr_config(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import ValidationError

    with patch("kreuzberg._api.main.discover_config_cached") as mock_discover:
        mock_discover.side_effect = ValidationError(
            "Invalid configuration for OCR backend 'tesseract': Invalid PSM mode value: 99",
            context={"psm_value": 99, "error": "99 is not a valid PSMMode"},
        )

        test_file = tmp_path / "test.txt"
        test_file.write_text("test content")

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 400
    error_response = response.json()
    assert "Invalid PSM mode value: 99" in error_response["message"]
    assert '"psm_value": 99' in error_response["details"]


@pytest.mark.anyio
async def test_extract_with_invalid_gmft_config(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import ValidationError

    with patch("kreuzberg._api.main.discover_config_cached") as mock_discover:
        mock_discover.side_effect = ValidationError(
            "Invalid GMFT configuration: Invalid parameter 'invalid_param'",
            context={"gmft_config": {"invalid_param": "value"}, "error": "Unknown parameter"},
        )

        test_file = tmp_path / "test.pdf"
        test_file.write_bytes(b"%PDF-1.4")

        with test_file.open("rb") as f:
            response = await test_client.post(
                "/extract", files=[("data", (test_file.name, f.read(), "application/pdf"))]
            )

    assert response.status_code == 400
    error_response = response.json()
    assert "Invalid GMFT configuration" in error_response["message"]
    assert "invalid_param" in error_response["details"]


@pytest.mark.anyio
async def test_extract_with_unreadable_config_file(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import ValidationError

    with patch("kreuzberg._api.main.discover_config_cached") as mock_discover:
        mock_discover.side_effect = ValidationError(
            "Failed to read pyproject.toml: Permission denied",
            context={"file": "/path/to/pyproject.toml", "error": "Permission denied"},
        )

        test_file = tmp_path / "test.txt"
        test_file.write_text("test content")

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 400
    error_response = response.json()
    assert "Failed to read pyproject.toml" in error_response["message"]
    assert "Permission denied" in error_response["details"]
    assert "/path/to/pyproject.toml" in error_response["details"]


@pytest.mark.anyio
async def test_extract_with_invalid_extraction_config(test_client: AsyncTestClient[Any], tmp_path: Path) -> None:
    from kreuzberg.exceptions import ValidationError

    with patch("kreuzberg._api.main.discover_config_cached") as mock_discover:
        mock_discover.side_effect = ValidationError(
            "Invalid extraction configuration: max_chars must be positive",
            context={"config": {"max_chars": -100}, "error": "Negative max_chars"},
        )

        test_file = tmp_path / "test.txt"
        test_file.write_text("test content")

        with test_file.open("rb") as f:
            response = await test_client.post("/extract", files=[("data", (test_file.name, f.read(), "text/plain"))])

    assert response.status_code == 400
    error_response = response.json()
    assert "Invalid extraction configuration" in error_response["message"]
    assert "max_chars must be positive" in error_response["message"]
    assert '"max_chars": -100' in error_response["details"]


@pytest.mark.anyio
async def test_extract_pdf_without_tables_with_table_extraction_enabled(
    test_client: AsyncTestClient[Any], searchable_pdf: Path
) -> None:
    from kreuzberg import ExtractionConfig, GMFTConfig

    with pytest.warns(FutureWarning):
        gmft_config = GMFTConfig(
            detector_base_threshold=0.9,
            remove_null_rows=True,
        )

    test_config = ExtractionConfig(
        extract_tables=True,
        gmft_config=gmft_config,
    )

    with patch("kreuzberg._api.main.discover_config_cached", return_value=test_config):
        with searchable_pdf.open("rb") as f:
            response = await test_client.post(
                "/extract", files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))]
            )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "content" in data[0]

    assert "Sample PDF" in data[0]["content"]

    if "tables" in data[0]:
        assert isinstance(data[0]["tables"], list)
