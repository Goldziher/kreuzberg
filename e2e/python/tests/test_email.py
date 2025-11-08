from __future__ import annotations

import pytest

from kreuzberg import extract_file_sync

from . import helpers


def test_email_sample_eml() -> None:
    """Sample EML email file to verify email parsing."""

    document_path = helpers.resolve_document("email/sample.eml")
    if not document_path.exists():
        pytest.skip(f"Skipping email_sample_eml: missing document at {document_path}")

    config = helpers.build_config(None)

    result = extract_file_sync(document_path, None, config)

    helpers.assert_expected_mime(result, ["message/rfc822"])
    helpers.assert_min_content_length(result, 20)
