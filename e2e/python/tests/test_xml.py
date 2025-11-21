# Auto-generated tests for xml fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_xml_plant_catalog() -> None:
    """XML plant catalog to validate streaming XML extraction."""

    document_path = helpers.resolve_document("xml/plant_catalog.xml")
    if not document_path.exists():
        pytest.skip(f"Skipping xml_plant_catalog: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["application/xml"])
    helpers.assert_min_content_length(result, 100)
    helpers.assert_metadata_expectation(result, "element_count", {"gte": 1})
