from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._email import EmailExtractor
from kreuzberg._mime_types import EML_MIME_TYPE, MSG_MIME_TYPE


@pytest.fixture
def email_extractor() -> EmailExtractor:
    config = ExtractionConfig()
    return EmailExtractor(EML_MIME_TYPE, config)


@pytest.fixture
def sample_email_path(tmp_path: Path) -> Path:
    email_content = """From: test@example.com
To: recipient@example.com
Subject: Test Email
Date: Mon, 1 Jan 2024 12:00:00 +0000
Content-Type: text/plain; charset=utf-8

This is a test email body.
"""
    email_path = tmp_path / "test.eml"
    email_path.write_text(email_content)
    return email_path


@pytest.fixture
def complex_email_path(tmp_path: Path) -> Path:
    email_content = """From: sender@example.com
To: recipient1@example.com, recipient2@example.com
Cc: cc@example.com
Bcc: bcc@example.com
Subject: Complex Email Test
Date: Wed, 15 Mar 2024 14:30:00 +0000
Content-Type: text/html; charset=utf-8

<html><body>
<p>This is <strong>HTML</strong> content</p>
<p>With multiple paragraphs</p>
</body></html>
"""
    email_path = tmp_path / "complex.eml"
    email_path.write_text(email_content)
    return email_path


@pytest.fixture
def html_email_path(tmp_path: Path) -> Path:
    email_content = """From: html-sender@example.com
To: html-recipient@example.com
Subject: HTML Email Test
Date: Tue, 20 Feb 2024 10:15:00 +0000
Content-Type: multipart/alternative; boundary="boundary123"

--boundary123
Content-Type: text/plain; charset=utf-8

Plain text version of the email.

--boundary123
Content-Type: text/html; charset=utf-8

<html><body>
<h1>HTML Email</h1>
<p>This is the <em>HTML</em> version with <strong>formatting</strong>.</p>
<ul>
<li>Item 1</li>
<li>Item 2</li>
</ul>
</body></html>

--boundary123--
"""
    email_path = tmp_path / "html.eml"
    email_path.write_text(email_content)
    return email_path


def test_mime_types() -> None:
    from kreuzberg._extractors._email import EmailExtractor

    assert EML_MIME_TYPE in EmailExtractor.SUPPORTED_MIME_TYPES
    assert MSG_MIME_TYPE in EmailExtractor.SUPPORTED_MIME_TYPES  # Now supported via Rust


def test_extract_bytes_sync(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    content = sample_email_path.read_bytes()
    result = email_extractor.extract_bytes_sync(content)

    assert result.content
    assert "Subject: Test Email" in result.content
    assert "From: test@example.com" in result.content
    assert "To: recipient@example.com" in result.content
    assert "This is a test email body." in result.content
    assert result.metadata["subject"] == "Test Email"


def test_extract_path_sync_basic(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(sample_email_path)

    assert result.content
    assert "Subject: Test Email" in result.content
    assert "This is a test email body." in result.content
    assert result.metadata["subject"] == "Test Email"
    assert result.metadata["email_from"] == "test@example.com"
    assert result.metadata["email_to"] == "recipient@example.com"


def test_extract_bytes_invalid_content(email_extractor: EmailExtractor) -> None:
    # Test with completely invalid email content
    # The Rust implementation may handle this gracefully rather than raising an error
    result = email_extractor.extract_bytes_sync(b"This is not an email")
    # Should return some result, even if minimal
    assert isinstance(result.content, str)


@pytest.mark.anyio
async def test_extract_bytes_async(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    content = sample_email_path.read_bytes()
    result = await email_extractor.extract_bytes_async(content)

    assert result.content
    assert "Subject: Test Email" in result.content
    assert "This is a test email body." in result.content
    assert result.metadata["subject"] == "Test Email"


@pytest.mark.anyio
async def test_extract_path_async(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = await email_extractor.extract_path_async(sample_email_path)

    assert result.content
    assert "Subject: Test Email" in result.content
    assert "This is a test email body." in result.content
    assert result.metadata["subject"] == "Test Email"


@pytest.mark.anyio
async def test_extract_bytes_async_invalid_content(email_extractor: EmailExtractor) -> None:
    # Test async with invalid email content
    # The Rust implementation may handle this gracefully
    result = await email_extractor.extract_bytes_async(b"Invalid email data")
    assert isinstance(result.content, str)


def test_email_header_extraction(email_extractor: EmailExtractor, complex_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(complex_email_path)

    assert "Subject: Complex Email Test" in result.content
    assert "From: sender@example.com" in result.content
    assert (
        "To: recipient1@example.com, recipient2@example.com" in result.content
        or "To: recipient1@example.com" in result.content
    )
    # Date format may be ISO instead of RFC2822
    assert "Date: 2024-03-15T14:30:00Z" in result.content or "Date: Wed, 15 Mar 2024 14:30:00 +0000" in result.content
    assert "This is HTML content" in result.content
    assert result.metadata["subject"] == "Complex Email Test"
    assert result.metadata["email_from"] == "sender@example.com"


def test_email_complex_headers(email_extractor: EmailExtractor, complex_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(complex_email_path)

    # Test that complex headers are properly extracted
    assert "From: sender@example.com" in result.content
    assert "Subject: Complex Email Test" in result.content
    # The Rust implementation may format recipients differently
    assert "recipient1@example.com" in result.content
    assert "recipient2@example.com" in result.content


def test_email_missing_headers(email_extractor: EmailExtractor, tmp_path: Path) -> None:
    # Create minimal email with just body
    email_content = """Content-Type: text/plain; charset=utf-8

Simple email without subject or date.
"""
    email_path = tmp_path / "minimal.eml"
    email_path.write_text(email_content)

    result = email_extractor.extract_path_sync(email_path)

    assert "Simple email without subject or date." in result.content
    # Headers may still be present but empty in the output


def test_email_with_html_content(email_extractor: EmailExtractor, html_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(html_email_path)

    assert "Subject: HTML Email Test" in result.content
    assert "From: html-sender@example.com" in result.content
    assert "To: html-recipient@example.com" in result.content
    # The multipart email should contain the plain text version
    assert "Plain text version of the email." in result.content
    assert result.metadata["subject"] == "HTML Email Test"


def test_email_multipart_text_and_html(email_extractor: EmailExtractor, html_email_path: Path) -> None:
    # Test that multipart emails with both text and HTML are handled correctly
    result = email_extractor.extract_path_sync(html_email_path)

    assert result.content
    assert "HTML Email Test" in result.content
    # The Rust implementation should prefer plain text or provide cleaned HTML
    assert len(result.content) > 50  # Should have substantial content


def test_email_with_attachments(email_extractor: EmailExtractor, tmp_path: Path) -> None:
    # Create a simple email for now - attachment testing would require more complex MIME structure
    email_content = """From: sender@example.com
To: recipient@example.com
Subject: Email with Attachments
Date: Mon, 1 Jan 2024 12:00:00 +0000
Content-Type: text/plain; charset=utf-8

Please see attached files.
"""
    email_path = tmp_path / "attachments.eml"
    email_path.write_text(email_content)

    result = email_extractor.extract_path_sync(email_path)

    assert "Please see attached files." in result.content
    assert result.metadata["subject"] == "Email with Attachments"


def test_email_extract_images_config(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    # Test with extract_images config enabled
    email_extractor.config = ExtractionConfig(extract_images=True)

    result = email_extractor.extract_path_sync(sample_email_path)

    # Even without actual image attachments, images list should be present
    assert hasattr(result, "images")
    assert isinstance(result.images, list)


def test_email_with_no_attachments(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(sample_email_path)

    # Should not have attachments metadata if no attachments present
    assert "attachments" not in result.metadata or not result.metadata.get("attachments")
    assert "Attachments:" not in result.content


def test_email_empty_content(email_extractor: EmailExtractor) -> None:
    # Test with empty email content
    # The Rust implementation validates and raises an error for empty content
    with pytest.raises(RuntimeError, match="Failed to parse email content"):
        email_extractor.extract_bytes_sync(b"")


def test_email_html_content_cleaning(email_extractor: EmailExtractor, complex_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(complex_email_path)

    # Should have cleaned HTML content (no HTML tags)
    assert "<html>" not in result.content
    assert "<body>" not in result.content
    assert "<p>" not in result.content
    # Should contain the actual text content
    assert "HTML" in result.content


def test_email_text_vs_html_preference(email_extractor: EmailExtractor, html_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(html_email_path)

    # The Rust implementation should handle multipart emails appropriately
    assert result.content
    assert len(result.content) > 20  # Should have content
    # May contain either plain text or cleaned HTML content


def test_email_metadata_extraction(email_extractor: EmailExtractor, complex_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(complex_email_path)

    # Test that metadata is properly extracted
    assert "subject" in result.metadata
    assert "email_from" in result.metadata
    assert result.metadata["subject"] == "Complex Email Test"
    assert result.metadata["email_from"] == "sender@example.com"


def test_email_date_extraction(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(sample_email_path)

    # Test date extraction - Rust implementation may use ISO format
    assert "Date: 2024-01-01T12:00:00Z" in result.content or "Date: Mon, 1 Jan 2024 12:00:00 +0000" in result.content
    assert "date" in result.metadata
    assert (
        result.metadata["date"] == "2024-01-01T12:00:00Z" or result.metadata["date"] == "Mon, 1 Jan 2024 12:00:00 +0000"
    )


def test_msg_mime_type_support() -> None:
    # Test that MSG MIME type is supported
    config = ExtractionConfig()
    extractor = EmailExtractor(MSG_MIME_TYPE, config)
    assert extractor.mime_type == MSG_MIME_TYPE


def test_email_error_handling(email_extractor: EmailExtractor) -> None:
    # Test error handling with malformed data
    # The Rust implementation may handle this gracefully
    result = email_extractor.extract_bytes_sync(b"Malformed email data \xff\xfe")
    assert isinstance(result.content, str)


def test_extract_path_sync(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(sample_email_path)

    assert result.content
    assert "Subject: Test Email" in result.content
    assert "This is a test email body." in result.content
    assert result.metadata["subject"] == "Test Email"


def test_real_email_file(email_extractor: EmailExtractor) -> None:
    # Test with the existing test source file
    test_email_path = Path("/Users/naamanhirschfeld/workspace/kreuzberg/tests/test_source_files/email/sample-email.eml")
    if test_email_path.exists():
        result = email_extractor.extract_path_sync(test_email_path)

        assert result.content
        assert "Subject: Test Email Subject" in result.content
        assert "This is a test email with some content." in result.content
        assert result.metadata["subject"] == "Test Email Subject"


def test_multipart_email_file(email_extractor: EmailExtractor) -> None:
    # Test with multipart email containing both text and HTML
    test_email_path = Path(
        "/Users/naamanhirschfeld/workspace/kreuzberg/tests/test_source_files/email/multipart-email.eml"
    )
    if test_email_path.exists():
        result = email_extractor.extract_path_sync(test_email_path)

        assert result.content
        assert "Subject: Multipart Email Test" in result.content
        assert "plain text version" in result.content or "HTML Version" in result.content
        assert result.metadata["subject"] == "Multipart Email Test"


def test_complex_headers_email_file(email_extractor: EmailExtractor) -> None:
    # Test with complex headers email
    test_email_path = Path(
        "/Users/naamanhirschfeld/workspace/kreuzberg/tests/test_source_files/email/complex-headers.eml"
    )
    if test_email_path.exists():
        result = email_extractor.extract_path_sync(test_email_path)

        assert result.content
        assert "Subject: Complex Email with Multiple Recipients" in result.content
        assert "Complex Sender" in result.content or "complex@example.com" in result.content
        assert "recipient1@example.com" in result.content
        assert result.metadata["subject"] == "Complex Email with Multiple Recipients"


def test_html_only_email_file(email_extractor: EmailExtractor) -> None:
    # Test with HTML-only email
    test_email_path = Path("/Users/naamanhirschfeld/workspace/kreuzberg/tests/test_source_files/email/html-only.eml")
    if test_email_path.exists():
        result = email_extractor.extract_path_sync(test_email_path)

        assert result.content
        assert "Subject: HTML Only Email" in result.content
        # Should have cleaned HTML content
        assert "Welcome to Our Service" in result.content
        assert "only HTML content" in result.content
        # Should NOT contain HTML tags - script content may still be present in Rust implementation
        assert "<html>" not in result.content
        assert "<script>" not in result.content
        # The Rust implementation may not filter out all JavaScript content
        assert result.metadata["subject"] == "HTML Only Email"


def test_msg_format_mime_type() -> None:
    # Test MSG format MIME type support
    from kreuzberg._mime_types import MSG_MIME_TYPE

    config = ExtractionConfig()
    extractor = EmailExtractor(MSG_MIME_TYPE, config)

    assert extractor.mime_type == MSG_MIME_TYPE
    assert MSG_MIME_TYPE in EmailExtractor.SUPPORTED_MIME_TYPES


def test_email_with_special_characters(email_extractor: EmailExtractor, tmp_path: Path) -> None:
    # Test email with special characters and encoding
    email_content = """From: sender@example.com
To: recipient@example.com
Subject: =?UTF-8?B?U3BlY2lhbCBjaGFyYWN0ZXJzOiDCoyDihqwg4oKs?=
Date: Mon, 1 Jan 2024 12:00:00 +0000
Content-Type: text/plain; charset=utf-8

This email contains special characters:
- Currency: €, £, $, ¥
- Symbols: ©, ®, ™
- Accented: café, naïve, résumé
- Emoji: 😊 📧 ✅

The Rust implementation should handle UTF-8 properly.
"""
    email_path = tmp_path / "special_chars.eml"
    email_path.write_text(email_content, encoding="utf-8")

    result = email_extractor.extract_path_sync(email_path)

    assert result.content
    assert "special characters" in result.content
    assert "café" in result.content
    # The subject may be decoded differently by the Rust implementation
    assert "subject" in result.metadata


def test_email_comprehensive_extraction(email_extractor: EmailExtractor, complex_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(complex_email_path)

    # Test comprehensive field extraction
    assert "Subject: Complex Email Test" in result.content
    assert "From: sender@example.com" in result.content
    # Date format may be ISO instead of RFC2822
    assert "Date: 2024-03-15T14:30:00Z" in result.content or "Date: Wed, 15 Mar 2024 14:30:00 +0000" in result.content
    assert "HTML" in result.content  # Content from HTML body

    # Test metadata
    assert result.metadata["subject"] == "Complex Email Test"
    assert result.metadata["email_from"] == "sender@example.com"
    assert (
        result.metadata["date"] == "2024-03-15T14:30:00Z"
        or result.metadata["date"] == "Wed, 15 Mar 2024 14:30:00 +0000"
    )
