from __future__ import annotations

import json
from typing import Any

import pytest


@pytest.mark.anyio
async def test_header_config_with_nested_lists_dicts_does_not_error(test_client: Any) -> None:
    payload = b"hello world"
    files = [("data", ("test.txt", payload, "text/plain"))]

    header_config = {
        "html_to_markdown_config": {"wrap": True, "wrap_width": 72},
        "language_detection_config": {"multilingual": True, "top_k": 2},
    }

    response = await test_client.post(
        "/extract?chunking.max_chars=5&chunking.max_overlap=1",
        files=files,
        headers={"X-Extraction-Config": json.dumps(header_config)},
    )

    assert response.status_code == 201
    data = response.json()
    assert isinstance(data, list)
    assert len(data) == 1
    assert "content" in data[0]
