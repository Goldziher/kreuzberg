from kreuzberg import ExtractionConfig
from kreuzberg._extractors._email import EmailExtractor
from kreuzberg._mime_types import EML_MIME_TYPE


class TestEmailExtractor:
    def test_supports_eml_mime_type(self) -> None:
        assert EmailExtractor.supports_mimetype(EML_MIME_TYPE)

    def test_extract_simple_email(self) -> None:
        config = ExtractionConfig()
        extractor = EmailExtractor(EML_MIME_TYPE, config)

        email_content = b"""From: sender@example.com
To: recipient@example.com
Subject: Test Subject
Date: Mon, 1 Jan 2024 12:00:00 +0000
Content-Type: text/plain; charset=utf-8

This is a test email body.
"""

        result = extractor.extract_bytes_sync(email_content)

        assert result.content
        assert "Test Subject" in result.content
        assert "sender@example.com" in result.content
        assert "This is a test email body" in result.content
        assert result.metadata["subject"] == "Test Subject"
        assert result.metadata["email_from"] == "sender@example.com"
        assert result.metadata["email_to"] == ["recipient@example.com"]
