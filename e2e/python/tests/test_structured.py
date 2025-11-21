# Auto-generated tests for structured fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_structured_json_basic() -> None:
    """Structured JSON extraction should stream and preserve content."""

    document_path = helpers.resolve_document("json/sample_document.json")
    if not document_path.exists():
        pytest.skip(f"Skipping structured_json_basic: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/json"])
    helpers.assert_min_content_length(result, 20)
    helpers.assert_content_contains_any(result, ["Sample Document", "Test Author"])


def test_structured_json_simple() -> None:
    """Simple JSON document to verify structured extraction."""

    document_path = helpers.resolve_document("data_formats/simple.json")
    if not document_path.exists():
        pytest.skip(f"Skipping structured_json_simple: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/json"])
    helpers.assert_min_content_length(result, 10)
    helpers.assert_content_contains_any(result, ["{", "name"])


def test_structured_yaml_simple() -> None:
    """Simple YAML document to validate structured extraction."""

    document_path = helpers.resolve_document("data_formats/simple.yaml")
    if not document_path.exists():
        pytest.skip(f"Skipping structured_yaml_simple: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/x-yaml"])
    helpers.assert_min_content_length(result, 10)
