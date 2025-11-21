# Auto-generated tests for image fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_image_metadata_only() -> None:
    """JPEG image to validate metadata extraction without OCR."""

    document_path = helpers.resolve_document("images/example.jpg")
    if not document_path.exists():
        pytest.skip(f"Skipping image_metadata_only: missing document at {document_path}")

    config = helpers.build_config({"ocr": None})

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["image/jpeg"])
    helpers.assert_max_content_length(result, 100)

