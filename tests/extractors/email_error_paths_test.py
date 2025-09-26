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
    # Test with completely invalid email data
    # The Rust implementation may handle this gracefully
    result = extractor.extract_bytes_sync(b"This is not an email at all")
    assert isinstance(result.content, str)


def test_email_empty_content() -> None:
    extractor = _make_extractor()
    # Test with empty content
    # The Rust implementation validates and raises an error for empty content
    with pytest.raises(RuntimeError, match="Failed to parse email content"):
        extractor.extract_bytes_sync(b"")


def test_email_binary_garbage() -> None:
    extractor = _make_extractor()
    # Test with binary garbage data
    # The Rust implementation may handle this gracefully
    result = extractor.extract_bytes_sync(b"\xff\xfe\x00\x00\x01\x02\x03")
    assert isinstance(result.content, str)


def test_email_malformed_headers(tmp_path: Path) -> None:
    extractor = _make_extractor()
    # Create email with malformed headers
    email_content = """From sender@example.com
To: recipient@example.com
Subject Test Email Without Colon
Date: Not a valid date

Body content.
"""
    email_path = tmp_path / "malformed.eml"
    email_path.write_text(email_content)

    # The Rust implementation should handle this gracefully or raise an error
    try:
        result = extractor.extract_path_sync(email_path)
        # If it succeeds, should still have content
        assert "Body content." in result.content
    except RuntimeError:
        # It's also acceptable for malformed emails to raise an error
        pass


@pytest.mark.anyio
async def test_email_async_error_handling() -> None:
    extractor = _make_extractor()
    # The Rust implementation may handle this gracefully
    result = await extractor.extract_bytes_async(b"Invalid email data")
    assert isinstance(result.content, str)


def test_email_with_null_bytes() -> None:
    extractor = _make_extractor()
    # Test with null bytes mixed in
    # The Rust implementation may handle this gracefully
    result = extractor.extract_bytes_sync(b"From: test@example.com\x00\x00\nTo: recipient@example.com\x00")
    assert isinstance(result.content, str)


def test_email_extremely_long_headers(tmp_path: Path) -> None:
    extractor = _make_extractor()
    # Create email with extremely long headers
    long_subject = "A" * 10000  # Very long subject
    email_content = f"""From: sender@example.com
To: recipient@example.com
Subject: {long_subject}
Date: Mon, 1 Jan 2024 12:00:00 +0000

Body content.
"""
    email_path = tmp_path / "long_headers.eml"
    email_path.write_text(email_content)

    # Should either work or fail gracefully
    try:
        result = extractor.extract_path_sync(email_path)
        assert "Body content." in result.content
        assert long_subject in result.content
    except RuntimeError:
        # Acceptable to fail on extremely long headers
        pass
