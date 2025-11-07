from __future__ import annotations

from typing import Any

import pytest


@pytest.mark.anyio
async def test_api_image_extraction_from_html(test_client: Any) -> None:
    html_doc = (
        b"<html><body>"
        b'<img src="data:image/png;base64,'
        b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
        b'" alt="Red dot">'
        b"</body></html>"
    )

    files = [("data", ("inline.html", html_doc, "text/html"))]
    response = await test_client.post("/extract?extract_images=true", files=files)

    assert response.status_code == 201
    data = response.json()
    assert len(data) == 1

    images = data[0].get("images")
    assert isinstance(images, list)
    if images:
        img0 = images[0]
        assert img0["format"] == "png"
        assert (
            img0["data"]
            == "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
        )
        assert img0["description"] == "Red dot"


@pytest.mark.anyio
async def test_api_image_ocr_skip_small(test_client: Any) -> None:
    html_doc = (
        b"<html><body>"
        b'<img src="data:image/png;base64,'
        b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
        b'" alt="Tiny">'
        b"</body></html>"
    )

    files = [("data", ("inline.html", html_doc, "text/html"))]
    response = await test_client.post(
        "/extract?extract_images=true&ocr_extracted_images=true",
        files=files,
    )

    assert response.status_code == 201
    payload = response.json()
    assert len(payload) == 1
    results = payload[0].get("image_ocr_results", [])
    assert isinstance(results, list)
    if results:
        assert results[0]["skipped_reason"] is not None
