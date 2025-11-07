from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any
from unittest.mock import AsyncMock, patch

import pytest
from kreuzberg import ExtractionConfig, ExtractionResult
from kreuzberg._api.main import merge_configs

if TYPE_CHECKING:
    from pathlib import Path

    from litestar.testing import AsyncTestClient


@pytest.mark.anyio
async def test_extract_with_query_params_chunking(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?chunk_content=true&max_chars=500&max_overlap=50",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "chunks" in data[0]
    assert len(data[0]["chunks"]) > 0
    for chunk in data[0]["chunks"]:
        assert len(chunk) <= 550


@pytest.mark.anyio
async def test_extract_with_query_params_keywords(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?extract_keywords=true&keyword_count=5",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    if data[0]["keywords"]:
        assert len(data[0]["keywords"]) <= 5


@pytest.mark.anyio
async def test_extract_with_query_params_entities(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?extract_entities=true",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "entities" in data[0]


@pytest.mark.anyio
async def test_extract_with_query_params_language_detection(
    test_client: AsyncTestClient[Any], searchable_pdf: Path
) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?auto_detect_language=true",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "detected_languages" in data[0]


@pytest.mark.anyio
async def test_extract_with_query_params_ocr_backend(test_client: AsyncTestClient[Any], ocr_image: Path) -> None:
    with ocr_image.open("rb") as f:
        response = await test_client.post(
            "/extract?ocr_backend=tesseract&force_ocr=true",
            files=[("data", (ocr_image.name, f.read(), "image/jpeg"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert len(data[0]["content"]) > 0


@pytest.mark.anyio
async def test_extract_with_markdown_response_format(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    mock_result = ExtractionResult(
        content="Markdown body",
        mime_type="text/markdown",
        metadata={"title": "Sample"},
        tables=[],
    )

    with (
        patch("kreuzberg._api.main.batch_extract_bytes", new_callable=AsyncMock) as mock_extract,
        searchable_pdf.open("rb") as f,
    ):
        mock_extract.return_value = [mock_result]
        response = await test_client.post(
            "/extract?response_format=markdown",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    assert response.headers["content-type"].startswith("text/markdown")
    assert "# " in response.text
    assert "Markdown body" in response.text


@pytest.mark.anyio
async def test_extract_with_header_config(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    config = {
        "chunk_content": True,
        "max_chars": 300,
        "extract_keywords": True,
        "keyword_count": 3,
    }

    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
            headers={"X-Extraction-Config": json.dumps(config)},
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert len(data[0]["chunks"]) > 0
    for chunk in data[0]["chunks"]:
        assert len(chunk) <= 350


@pytest.mark.anyio
async def test_extract_with_nested_config_in_header(test_client: AsyncTestClient[Any], ocr_image: Path) -> None:
    config = {
        "force_ocr": True,
        "ocr_backend": "tesseract",
        "ocr_config": {
            "language": "eng",
            "psm": 3,
            "output_format": "text",
        },
    }

    with ocr_image.open("rb") as f:
        response = await test_client.post(
            "/extract",
            files=[("data", (ocr_image.name, f.read(), "image/jpeg"))],
            headers={"X-Extraction-Config": json.dumps(config)},
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert len(data[0]["content"]) > 0


@pytest.mark.anyio
async def test_extract_config_precedence(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    header_config = {
        "chunk_content": True,
        "max_chars": 200,
    }

    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?chunk_content=true&max_chars=500",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
            headers={"X-Extraction-Config": json.dumps(header_config)},
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert len(data[0]["chunks"]) > 0
    for chunk in data[0]["chunks"]:
        assert len(chunk) <= 250


@pytest.mark.anyio
async def test_extract_with_invalid_header_json(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
            headers={"X-Extraction-Config": "invalid-json{"},
        )

    assert response.status_code == 400
    error = response.json()
    assert "Invalid JSON" in error["message"]


@pytest.mark.anyio
async def test_extract_with_pdf_password_query_param(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?pdf_password=test123",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1


@pytest.mark.anyio
async def test_extract_multiple_files_with_query_params(
    test_client: AsyncTestClient[Any], searchable_pdf: Path, ocr_image: Path
) -> None:
    with searchable_pdf.open("rb") as pdf_f, ocr_image.open("rb") as img_f:
        response = await test_client.post(
            "/extract?extract_entities=true&extract_keywords=true",
            files=[
                ("data", (searchable_pdf.name, pdf_f.read(), "application/pdf")),
                ("data", (ocr_image.name, img_f.read(), "image/jpeg")),
            ],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 2
    for item in data:
        assert "entities" in item
        assert "keywords" in item


@pytest.mark.anyio
async def test_extract_with_boolean_query_params_variations(
    test_client: AsyncTestClient[Any], searchable_pdf: Path
) -> None:
    test_cases = [
        ("true", True),
        ("false", False),
        ("1", True),
        ("0", False),
        ("yes", True),
        ("no", False),
    ]

    for value_str, expected in test_cases:
        with searchable_pdf.open("rb") as f:
            response = await test_client.post(
                f"/extract?chunk_content={value_str}",
                files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
            )

        assert response.status_code == 201
        data = response.json()
        if expected:
            assert len(data[0]["chunks"]) > 0
        else:
            assert len(data[0]["chunks"]) == 0


@pytest.mark.anyio
async def test_extract_query_params_with_static_config(
    test_client: AsyncTestClient[Any], searchable_pdf: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    static_config = ExtractionConfig(chunk_content=False, max_chars=1000)

    with patch("kreuzberg._api.main.discover_config", return_value=static_config), searchable_pdf.open("rb") as f:
        response = await test_client.post(
            "/extract?chunk_content=true&max_chars=200",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data[0]["chunks"]) > 0
    for chunk in data[0]["chunks"]:
        assert len(chunk) <= 250


def test_merge_configs_function() -> None:
    static = ExtractionConfig(chunk_content=False, max_chars=1000)
    query = {"chunk_content": True, "max_chars": 500, "extract_keywords": None}
    header = {"max_chars": 200, "extract_entities": True}

    result = merge_configs(static, query, header)

    assert result.chunk_content is True
    assert result.max_chars == 200
    assert result.extract_entities is True
    assert result.extract_keywords is False


def test_merge_configs_type_conversion() -> None:
    static = ExtractionConfig()
    query = {
        "chunk_content": "true",
        "max_chars": "500",
        "keyword_count": "10",
        "ocr_backend": "tesseract",
    }
    header = None

    result = merge_configs(static, query, header)

    assert result.chunk_content is True
    assert result.max_chars == 500
    assert result.keyword_count == 10
    assert result.ocr_backend == "tesseract"


@pytest.mark.anyio
async def test_extract_with_all_query_params(test_client: AsyncTestClient[Any], searchable_pdf: Path) -> None:
    params = {
        "chunk_content": "true",
        "max_chars": "400",
        "max_overlap": "40",
        "extract_tables": "false",
        "extract_entities": "false",
        "extract_keywords": "false",
        "keyword_count": "7",
        "force_ocr": "false",
        "ocr_backend": "tesseract",
        "auto_detect_language": "false",
        "pdf_password": "",
    }

    query_string = "&".join(f"{k}={v}" for k, v in params.items())

    with searchable_pdf.open("rb") as f:
        response = await test_client.post(
            f"/extract?{query_string}",
            files=[("data", (searchable_pdf.name, f.read(), "application/pdf"))],
        )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert len(data[0]["chunks"]) > 0
    assert "entities" in data[0]
    assert "keywords" in data[0]


@pytest.mark.anyio
async def test_extract_images_from_html_base64(test_client: AsyncTestClient[Any]) -> None:
    html = (
        b"<html><body>"
        b'<img src="data:image/png;base64,'
        b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
        b'" alt="Red dot">'
        b"</body></html>"
    )

    response = await test_client.post(
        "/extract?extract_images=true",
        files=[("data", ("inline.html", html, "text/html"))],
    )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "images" in data[0]
    assert len(data[0]["images"]) == 1
    img0 = data[0]["images"][0]
    assert img0["format"] == "png"
    assert (
        img0["data"]
        == "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
    )
    assert img0["description"] == "Red dot"


@pytest.mark.anyio
async def test_extract_images_with_ocr_skip_small(test_client: AsyncTestClient[Any]) -> None:
    html = (
        b"<html><body>"
        b'<img src="data:image/png;base64,'
        b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
        b'" alt="Tiny">'
        b"</body></html>"
    )

    response = await test_client.post(
        "/extract?extract_images=true&ocr_extracted_images=true",
        files=[("data", ("inline.html", html, "text/html"))],
    )

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1
    assert "image_ocr_results" in data[0]
    assert len(data[0]["image_ocr_results"]) == 1
    assert data[0]["image_ocr_results"][0]["skipped_reason"] is not None
