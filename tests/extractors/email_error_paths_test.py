from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._email import EmailExtractor

if TYPE_CHECKING:
    from pathlib import Path


def _make_extractor() -> EmailExtractor:
    return EmailExtractor(mime_type="message/rfc822", config=ExtractionConfig())


def test_email_invalid_email_data() -> None:
    extractor = _make_extractor()
    result = extractor.extract_bytes_sync(b"This is not an email at all")
    assert isinstance(result.content, str)


def test_email_empty_content() -> None:
    extractor = _make_extractor()
    with pytest.raises(RuntimeError, match="Failed to parse email content"):
        extractor.extract_bytes_sync(b"")


def test_email_binary_garbage() -> None:
    extractor = _make_extractor()
    result = extractor.extract_bytes_sync(b"\xff\xfe\x00\x00\x01\x02\x03")
    assert isinstance(result.content, str)


def test_email_malformed_headers(tmp_path: Path) -> None:
    extractor = _make_extractor()
    email_content = """From sender@example.com
To: recipient@example.com
Subject Test Email Without Colon
Date: Not a valid date

Body content.
"""
    email_path = tmp_path / "malformed.eml"
    email_path.write_text(email_content)

    try:
        result = extractor.extract_path_sync(email_path)
        assert "Body content." in result.content
    except RuntimeError:
        pass


@pytest.mark.anyio
async def test_email_async_error_handling() -> None:
    extractor = _make_extractor()
    result = await extractor.extract_bytes_async(b"Invalid email data")
    assert isinstance(result.content, str)


def test_email_with_null_bytes() -> None:
    extractor = _make_extractor()
    result = extractor.extract_bytes_sync(b"From: test@example.com\x00\x00\nTo: recipient@example.com\x00")
    assert isinstance(result.content, str)


def test_email_extremely_long_headers(tmp_path: Path) -> None:
    extractor = _make_extractor()
    long_subject = "A" * 10000
    email_content = f"""From: sender@example.com
To: recipient@example.com
Subject: {long_subject}
Date: Mon, 1 Jan 2024 12:00:00 +0000

Body content.
"""
    email_path = tmp_path / "long_headers.eml"
    email_path.write_text(email_content)

    try:
        result = extractor.extract_path_sync(email_path)
        assert "Body content." in result.content
        assert long_subject in result.content
    except RuntimeError:
        pass
