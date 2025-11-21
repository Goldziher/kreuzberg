# Auto-generated tests for html fixtures.
from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_html_complex_layout() -> None:
    """Large Wikipedia HTML page to validate complex conversion."""

    document_path = helpers.resolve_document("web/taylor_swift.html")
    if not document_path.exists():
        pytest.skip(f"Skipping html_complex_layout: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["text/html"])
    helpers.assert_min_content_length(result, 1000)

def test_html_simple_table() -> None:
    """HTML table converted to markdown should retain structure."""

    document_path = helpers.resolve_document("web/simple_table.html")
    if not document_path.exists():
        pytest.skip(f"Skipping html_simple_table: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["text/html"])
    helpers.assert_min_content_length(result, 20)
    helpers.assert_content_contains_all(result, ["|"])

