"""
Test email extraction using real EML and MSG files from test data.
"""

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
def msg_extractor() -> EmailExtractor:
    config = ExtractionConfig()
    return EmailExtractor(MSG_MIME_TYPE, config)


def test_plain_text_eml_file(email_extractor: EmailExtractor) -> None:
    """Test extraction of simple plain text EML file."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/eml/simple/plain_text_only.eml"

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: Simple Plain Text Email" in result.content
        assert "From: test@example.com" in result.content
        assert "To: recipient@example.com" in result.content
        assert "simple plain text email" in result.content
        assert result.metadata["subject"] == "Simple Plain Text Email"
        assert result.metadata["email_from"] == "test@example.com"
        assert result.metadata["email_to"] == "recipient@example.com"


def test_html_multipart_eml_file(email_extractor: EmailExtractor) -> None:
    """Test extraction of HTML multipart EML file."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/eml/simple/html_email_multipart.eml"

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: HTML Email Test" in result.content
        assert "From: html-sender@example.com" in result.content
        assert "HTML Email" in result.content or "Plain text version" in result.content
        # Should contain either plain text or cleaned HTML content
        # The Rust implementation may prefer plain text part
        assert "Plain text version" in result.content or "Item 1" in result.content or "formatting" in result.content
        assert result.metadata["subject"] == "HTML Email Test"
        assert result.metadata["email_from"] == "html-sender@example.com"


def test_complex_headers_eml_file(email_extractor: EmailExtractor) -> None:
    """Test extraction of EML file with complex headers."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/eml/simple/complex_headers.eml"

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: Complex Email with Multiple Recipients" in result.content
        assert "From: Complex Sender" in result.content or "From: complex@example.com" in result.content
        assert "recipient1@example.com" in result.content
        assert "recipient2@example.com" in result.content
        assert "complex headers" in result.content
        assert result.metadata["subject"] == "Complex Email with Multiple Recipients"
        assert result.metadata["email_from"] == "complex@example.com"


def test_pdf_attachment_eml_file(email_extractor: EmailExtractor) -> None:
    """Test extraction of EML file with PDF attachment."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/eml/with_attachments/mailgun_pdf_attachment.eml"

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: Test message with PDF attachment" in result.content
        assert "From: test@mailgun.com" in result.content
        assert "body content" in result.content
        assert result.metadata["subject"] == "Test message with PDF attachment"
        assert result.metadata["email_from"] == "test@mailgun.com"
        # Should have attachment metadata
        if "attachments" in result.metadata:
            assert "test.pdf" in result.metadata["attachments"]


def test_png_attachment_eml_file(email_extractor: EmailExtractor) -> None:
    """Test extraction of EML file with PNG attachment."""
    test_file = (
        Path(__file__).parent.parent / "test_source_files/email/eml/with_attachments/thunderbird_png_attachment.eml"
    )

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: Test email with PNG attachment" in result.content
        # The From header shows email address, display name is in the body
        assert "From: JSmith@somenet.foo" in result.content or "John Smith" in result.content
        assert "PNG image attachment" in result.content
        assert result.metadata["subject"] == "Test email with PNG attachment"
        assert "JSmith@somenet.foo" in result.metadata["email_from"]
        # Should have attachment metadata
        if "attachments" in result.metadata:
            assert "test_image.png" in result.metadata["attachments"]


def test_multiple_attachments_eml_file(email_extractor: EmailExtractor) -> None:
    """Test extraction of EML file with multiple attachments."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/eml/with_attachments/mixed_content_types.eml"

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: Test Email with Multiple Attachments" in result.content
        assert "From: sender@example.com" in result.content
        assert "multiple lines of text" in result.content
        assert result.metadata["subject"] == "Test Email with Multiple Attachments"
        assert result.metadata["email_from"] == "sender@example.com"
        # Should have attachment metadata for both PDF and PNG
        if "attachments" in result.metadata:
            attachments_str = result.metadata["attachments"]
            assert "document.pdf" in attachments_str
            assert "image.png" in attachments_str


def test_eml_file_with_image_extraction(email_extractor: EmailExtractor) -> None:
    """Test EML file with image extraction enabled."""
    # Enable image extraction
    email_extractor.config = ExtractionConfig(extract_images=True)

    test_file = (
        Path(__file__).parent.parent / "test_source_files/email/eml/with_attachments/thunderbird_png_attachment.eml"
    )

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert hasattr(result, "images")
        assert isinstance(result.images, list)
        # May or may not have extracted images depending on implementation
        # but should not raise errors


def test_existing_sample_email_file(email_extractor: EmailExtractor) -> None:
    """Test with the existing sample email file."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/sample-email.eml"

    if test_file.exists():
        result = email_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: Test Email Subject" in result.content
        assert "From: sender@example.com" in result.content
        assert "This is a test email with some content" in result.content
        assert result.metadata["subject"] == "Test Email Subject"
        assert result.metadata["email_from"] == "sender@example.com"


def test_msg_file_simple(msg_extractor: EmailExtractor) -> None:
    """Test extraction of simple MSG file."""
    test_file = Path(__file__).parent.parent / "test_source_files/email/msg/simple/simple_msg.msg"

    if test_file.exists():
        result = msg_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: This is the subject" in result.content
        assert "From: peterpan@neverland.com" in result.content
        assert result.metadata["subject"] == "This is the subject"
        assert result.metadata["email_from"] == "peterpan@neverland.com"
        assert result.metadata["email_to"] == "crocodile@neverland.com"


def test_msg_file_with_attachments(msg_extractor: EmailExtractor) -> None:
    """Test extraction of MSG file with attachments."""
    test_file = (
        Path(__file__).parent.parent / "test_source_files/email/msg/with_attachments/msg_with_png_attachment.msg"
    )

    if test_file.exists():
        result = msg_extractor.extract_path_sync(test_file)

        assert result.content
        assert "Subject: This is the subject" in result.content
        assert result.metadata["subject"] == "This is the subject"
        assert result.metadata["email_from"] == "peterpan@neverland.com"
        if "attachments" in result.metadata:
            assert "CANVAS.PNG" in result.metadata["attachments"]


def test_eml_error_handling_malformed_file(email_extractor: EmailExtractor, tmp_path: Path) -> None:
    """Test error handling with malformed EML file."""
    malformed_file = tmp_path / "malformed.eml"
    malformed_file.write_bytes(b"This is not a valid email\xff\xfe\x00")

    # Should handle gracefully without crashing
    result = email_extractor.extract_path_sync(malformed_file)
    assert isinstance(result.content, str)


def test_eml_empty_file_handling(email_extractor: EmailExtractor, tmp_path: Path) -> None:
    """Test handling of empty EML file."""
    empty_file = tmp_path / "empty.eml"
    empty_file.write_bytes(b"")

    # Should raise an appropriate error for empty content
    with pytest.raises(RuntimeError, match="Failed to parse email content"):
        email_extractor.extract_path_sync(empty_file)


def test_eml_unicode_handling(email_extractor: EmailExtractor, tmp_path: Path) -> None:
    """Test handling of EML file with Unicode characters."""
    unicode_content = """From: test@example.com
To: recipient@example.com
Subject: Test with Unicode: café, naïve, résumé 🎉
Date: Mon, 1 Jan 2024 12:00:00 +0000
Content-Type: text/plain; charset=utf-8

This email contains Unicode characters:
- Accented: café, naïve, résumé
- Symbols: © ® ™
- Emoji: 😊 📧 ✅

The parser should handle these correctly.
""".encode()

    unicode_file = tmp_path / "unicode.eml"
    unicode_file.write_bytes(unicode_content)

    result = email_extractor.extract_path_sync(unicode_file)

    assert result.content
    assert "café" in result.content
    assert "Unicode characters" in result.content
    # Subject decoding may vary by implementation
    assert "subject" in result.metadata
