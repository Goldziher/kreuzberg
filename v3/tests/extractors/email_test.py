from pathlib import Path
from typing import Any
from unittest.mock import MagicMock, patch

import pytest
from kreuzberg import ExtractionConfig
from kreuzberg._extractors._email import EmailExtractor
from kreuzberg._mime_types import EML_MIME_TYPE, MSG_MIME_TYPE
from kreuzberg.exceptions import MissingDependencyError


@pytest.fixture
def email_extractor() -> EmailExtractor:
    config = ExtractionConfig()
    return EmailExtractor(EML_MIME_TYPE, config)


@pytest.fixture
def sample_email_path(tmp_path: Path) -> Path:
    email_content = """Subject: Test Email
From: test@example.com
To: recipient@example.com

This is a test email body.
"""
    email_path = tmp_path / "test.eml"
    email_path.write_text(email_content)
    return email_path


def test_mime_types() -> None:
    from kreuzberg._extractors._email import EmailExtractor

    assert EML_MIME_TYPE in EmailExtractor.SUPPORTED_MIME_TYPES
    assert MSG_MIME_TYPE not in EmailExtractor.SUPPORTED_MIME_TYPES


def test_extract_bytes_sync(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "Test Subject",
            "text": "Email body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy email content")

        assert result.content
        assert "Test Subject" in result.content
        assert "Email body content" in result.content
        assert result.metadata["subject"] == "Test Subject"


def test_extract_path_sync_basic(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(sample_email_path)

    assert result.content
    assert "Test Email" in result.content
    assert result.metadata["subject"] == "Test Email"


def test_missing_mailparse_dependency_basic() -> None:
    config = ExtractionConfig()
    extractor = EmailExtractor(EML_MIME_TYPE, config)

    with patch("kreuzberg._extractors._email.mailparse", None):
        with pytest.raises(MissingDependencyError, match="mailparse is required"):
            extractor.extract_bytes_sync(b"dummy email content")


@pytest.mark.anyio
async def test_extract_bytes_async(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "Async Test",
            "text": "Async email body",
        }

        result = await email_extractor.extract_bytes_async(b"dummy email content")

        assert result.content
        assert "Async Test" in result.content
        assert "Async email body" in result.content
        assert result.metadata["subject"] == "Async Test"


@pytest.mark.anyio
async def test_extract_path_async(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = await email_extractor.extract_path_async(sample_email_path)

    assert result.content
    assert "Test Email" in result.content
    assert result.metadata["subject"] == "Test Email"


@pytest.mark.anyio
async def test_missing_mailparse_dependency_async() -> None:
    config = ExtractionConfig()
    extractor = EmailExtractor(EML_MIME_TYPE, config)

    with patch("kreuzberg._extractors._email.mailparse", None):
        with pytest.raises(MissingDependencyError, match="mailparse is required"):
            await extractor.extract_bytes_async(b"dummy email content")


def test_email_header_extraction(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "cc": "cc@example.com",
            "bcc": "bcc@example.com",
            "subject": "Header Test",
            "date": "Mon, 1 Jan 2024 12:00:00 +0000",
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "Subject: Header Test" in result.content
        assert "From: sender@example.com" in result.content
        assert "To: recipient@example.com" in result.content
        assert "CC: cc@example.com" in result.content
        assert "BCC: bcc@example.com" in result.content
        assert "Date: Mon, 1 Jan 2024 12:00:00 +0000" in result.content
        assert "Body content" in result.content


def test_email_complex_headers(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": {"email": "sender@example.com", "name": "Sender Name"},
            "to": [
                {"email": "recipient1@example.com", "name": "Recipient 1"},
                {"email": "recipient2@example.com", "name": "Recipient 2"},
            ],
            "cc": [
                {"email": "cc1@example.com"},
                {"email": "cc2@example.com", "name": "CC Person"},
            ],
            "subject": "Complex Headers",
            "text": "Email with complex headers",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "From: sender@example.com" in result.content
        assert "To: recipient1@example.com, recipient2@example.com" in result.content
        assert "CC: cc1@example.com, cc2@example.com" in result.content


def test_email_missing_headers(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": "Simple email without subject or date.",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == "Simple email without subject or date."
        assert "Subject:" not in result.content
        assert "From:" not in result.content


def test_email_with_html_content_with_html2text(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "HTML Test",
            "html": "<p>This is <strong>HTML</strong> content</p>",
        }

        with patch("kreuzberg._extractors._email.html2text") as mock_html2text:
            mock_converter = MagicMock()
            mock_converter.handle.return_value = "This is **HTML** content"
            mock_html2text.HTML2Text.return_value = mock_converter

            result = email_extractor.extract_bytes_sync(b"dummy")

            assert "This is **HTML** content" in result.content
            mock_converter.handle.assert_called_once_with("<p>This is <strong>HTML</strong> content</p>")


def test_email_with_html_content_without_html2text(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "HTML Test",
            "html": "<p>This is &lt;HTML&gt; content</p>",
        }

        with patch("kreuzberg._extractors._email.html2text", None):
            result = email_extractor.extract_bytes_sync(b"dummy")

            assert "This is <HTML> content" in result.content


def test_email_with_attachments(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "Email with Attachments",
            "text": "Please see attached files.",
            "attachments": [
                {"name": "document.pdf"},
                {"name": "image.jpg"},
                {},
            ],
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

    assert result.metadata["attachments"] == ["document.pdf", "image.jpg", "unknown"]


def test_email_image_attachments_to_images(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "Email with Image",
            "text": "See image.",
            "attachments": [
                {"name": "inline.png", "mime": "image/png", "data": b"\x89PNG\r\n"},
                {"name": "note.txt", "mime": "text/plain", "data": b"hello"},
            ],
        }

        email_extractor.config = email_extractor.config.__class__(
            **{
                **email_extractor.config.to_dict(),
                "extract_images": True,
            }
        )

        result = email_extractor.extract_bytes_sync(b"dummy")
        assert isinstance(result.images, list)
        assert any(img.format in {"png"} and img.filename == "inline.png" for img in result.images)


def test_email_with_empty_attachments(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "No Attachments",
            "text": "No files attached.",
            "attachments": [],
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "attachments" not in result.metadata
        assert "Attachments:" not in result.content


def test_missing_mailparse_dependency() -> None:
    config = ExtractionConfig()
    extractor = EmailExtractor(EML_MIME_TYPE, config)

    with patch("kreuzberg._extractors._email.mailparse", None):
        with pytest.raises(MissingDependencyError, match="mailparse is required"):
            extractor.extract_bytes_sync(b"dummy email content")


def test_email_with_html_body_without_html2text(email_extractor: EmailExtractor) -> None:
    with patch("kreuzberg._extractors._email.html2text", None), patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "HTML Email",
            "html": "<html><body><p>Hello <b>World</b> &amp; Friends</p></body></html>",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "Hello World & Friends" in result.content
        assert "<p>" not in result.content
        assert "&amp;" not in result.content


def test_email_text_preferred_over_html(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "Multipart Email",
            "text": "Plain text version",
            "html": "<html><body>HTML version</body></html>",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "Plain text version" in result.content
        assert "HTML version" not in result.content


def test_email_with_attachments_detailed(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "Email with attachments",
            "text": "See attached files",
            "attachments": [
                {"name": "document.pdf", "content": b"fake pdf"},
                {"name": "image.jpg", "content": b"fake image"},
                {},
            ],
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "Attachments: document.pdf, image.jpg, unknown" in result.content
        assert result.metadata["attachments"] == ["document.pdf", "image.jpg", "unknown"]


def test_email_without_attachments(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "to": "recipient@example.com",
            "subject": "No attachments",
            "text": "Email body",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "Attachments:" not in result.content
        assert "attachments" not in result.metadata


def test_missing_mailparse_dependency_with_fixture(email_extractor: EmailExtractor) -> None:
    with patch("kreuzberg._extractors._email.mailparse", None):
        with pytest.raises(MissingDependencyError, match="mailparse is required"):
            email_extractor.extract_bytes_sync(b"dummy")


def test_mailparse_exception(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load", side_effect=Exception("Parse error")):
        with pytest.raises(RuntimeError, match="Failed to parse email content"):
            email_extractor.extract_bytes_sync(b"invalid email content")


def test_extract_path_sync(email_extractor: EmailExtractor, sample_email_path: Path) -> None:
    result = email_extractor.extract_path_sync(sample_email_path)

    assert result.content
    assert "Test Email" in result.content
    assert result.metadata["subject"] == "Test Email"


def test_empty_email(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {}

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == ""
        assert result.metadata == {}


def test_email_with_all_fields(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": {"email": "sender@example.com", "name": "Sender Name"},
            "to": [{"email": "recipient@example.com", "name": "Recipient"}],
            "cc": "cc@example.com",
            "bcc": "bcc@example.com",
            "subject": "Complete Email",
            "date": "Mon, 1 Jan 2024 12:00:00 +0000",
            "text": "Email body text",
            "attachments": [{"name": "file.txt"}],
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "Subject: Complete Email" in result.content
        assert "From: sender@example.com" in result.content
        assert "To: recipient@example.com" in result.content
        assert "CC: cc@example.com" in result.content
        assert "BCC: bcc@example.com" in result.content
        assert "Date: Mon, 1 Jan 2024 12:00:00 +0000" in result.content
        assert "Email body text" in result.content
        assert "Attachments: file.txt" in result.content

        assert result.metadata["subject"] == "Complete Email"
        assert result.metadata["email_from"] == "sender@example.com"
        assert result.metadata["email_to"] == "recipient@example.com"
        assert result.metadata["email_cc"] == "cc@example.com"
        assert result.metadata["email_bcc"] == "bcc@example.com"
        assert result.metadata["date"] == "Mon, 1 Jan 2024 12:00:00 +0000"
        assert result.metadata["attachments"] == ["file.txt"]


def test_email_format_field_list_with_dicts(email_extractor: EmailExtractor) -> None:
    field = [
        {"email": "test1@example.com", "name": "Test User 1"},
        {"email": "test2@example.com", "name": "Test User 2"},
        {"email": "test3@example.com"},
    ]
    result = email_extractor._format_email_field(field)
    assert result == "test1@example.com, test2@example.com, test3@example.com"


def test_email_format_field_list_with_dicts_empty_email(email_extractor: EmailExtractor) -> None:
    field = [
        {"email": "", "name": "Empty Email"},
        {"email": "valid@example.com", "name": "Valid User"},
        {"name": "No Email Key"},
    ]
    result = email_extractor._format_email_field(field)
    assert result == "valid@example.com"


def test_email_format_field_list_with_strings(email_extractor: EmailExtractor) -> None:
    field = ["email1@example.com", "email2@example.com", "email3@example.com"]
    result = email_extractor._format_email_field(field)
    assert result == "email1@example.com, email2@example.com, email3@example.com"


def test_email_format_field_list_mixed_types(email_extractor: EmailExtractor) -> None:
    field = [
        {"email": "dict@example.com", "name": "Dict User"},
        "string@example.com",
        123,
        {"email": "another@example.com"},
    ]
    result = email_extractor._format_email_field(field)
    assert result == "dict@example.com, string@example.com, 123, another@example.com"


def test_email_format_field_single_dict(email_extractor: EmailExtractor) -> None:
    field = {"email": "single@example.com", "name": "Single User"}
    result = email_extractor._format_email_field(field)
    assert result == "single@example.com"


def test_email_format_field_single_dict_no_email(email_extractor: EmailExtractor) -> None:
    field = {"name": "No Email Key"}
    result = email_extractor._format_email_field(field)
    assert result == ""


def test_email_format_field_single_string(email_extractor: EmailExtractor) -> None:
    field = "single@example.com"
    result = email_extractor._format_email_field(field)
    assert result == "single@example.com"


def test_email_format_field_none_value(email_extractor: EmailExtractor) -> None:
    field = None
    result = email_extractor._format_email_field(field)
    assert result == "None"


def test_email_format_field_empty_list(email_extractor: EmailExtractor) -> None:
    field: list[Any] = []
    result = email_extractor._format_email_field(field)
    assert result == ""


def test_email_header_extraction_comprehensive_dict_with_name(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": {"email": "sender@example.com", "name": "Sender Name"},
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_from"] == "sender@example.com"
        assert "From: sender@example.com" in result.content


def test_email_header_extraction_comprehensive_dict_no_email(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": {"name": "Sender Name"},
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_from"] == ""
        assert "From:" in result.content


def test_email_header_extraction_comprehensive_from_field_string(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "sender@example.com",
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_from"] == "sender@example.com"
        assert "From: sender@example.com" in result.content


def test_email_header_extraction_comprehensive_to_field_list_dict(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "to": [
                {"email": "recipient1@example.com", "name": "Recipient 1"},
                {"email": "recipient2@example.com"},
            ],
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_to"] == "recipient1@example.com"
        assert "To: recipient1@example.com, recipient2@example.com" in result.content


def test_email_header_extraction_comprehensive_to_field_list_empty(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "to": [],
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "email_to" not in result.metadata
        assert "To:" not in result.content


def test_email_header_extraction_comprehensive_to_field_list_first_no_email(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "to": [
                {"name": "No Email"},
                {"email": "recipient2@example.com"},
            ],
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_to"] == ""
        assert "To: recipient2@example.com" in result.content


def test_email_header_extraction_comprehensive_to_field_list_strings(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "to": ["recipient1@example.com", "recipient2@example.com"],
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_to"] == "recipient1@example.com"
        assert "To: recipient1@example.com, recipient2@example.com" in result.content


def test_email_header_extraction_comprehensive_to_field_single_dict(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "to": {"email": "single@example.com", "name": "Single Recipient"},
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_to"] == "single@example.com"
        assert "To: single@example.com" in result.content


def test_email_header_extraction_comprehensive_to_field_single_string(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "to": "single@example.com",
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_to"] == "single@example.com"
        assert "To: single@example.com" in result.content


def test_email_header_extraction_comprehensive_cc_bcc_various_types(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "cc": {"email": "cc@example.com", "name": "CC Person"},
            "bcc": ["bcc1@example.com", "bcc2@example.com"],
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["email_cc"] == {"email": "cc@example.com", "name": "CC Person"}  # type: ignore[comparison-overlap]
        assert result.metadata["email_bcc"] == ["bcc1@example.com", "bcc2@example.com"]  # type: ignore[comparison-overlap]
        assert "CC: cc@example.com" in result.content
        assert "BCC: bcc1@example.com, bcc2@example.com" in result.content


def test_email_header_extraction_comprehensive_none_values(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": None,
            "to": None,
            "subject": None,
            "date": None,
            "cc": None,
            "bcc": None,
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "email_from" not in result.metadata
        assert "email_to" not in result.metadata
        assert "subject" not in result.metadata
        assert "date" not in result.metadata
        assert "email_cc" not in result.metadata
        assert "email_bcc" not in result.metadata
        assert result.content == "Body content"


def test_email_header_extraction_comprehensive_empty_string_values(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "",
            "to": "",
            "subject": "",
            "date": "",
            "cc": "",
            "bcc": "",
            "text": "Body content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "email_from" not in result.metadata
        assert "email_to" not in result.metadata
        assert "subject" not in result.metadata
        assert "date" not in result.metadata
        assert "email_cc" not in result.metadata
        assert "email_bcc" not in result.metadata
        assert result.content == "Body content"


def test_email_body_extraction_comprehensive_text_only(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": "Plain text content",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == "Plain text content"


def test_email_body_extraction_comprehensive_html_only_with_html2text(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "html": "<p>HTML <strong>content</strong></p>",
        }

        mock_converter = MagicMock()
        mock_converter.handle.return_value = "HTML **content**"

        with patch("kreuzberg._extractors._email.html2text") as mock_html2text:
            mock_html2text.HTML2Text.return_value = mock_converter

            result = email_extractor.extract_bytes_sync(b"dummy")

            assert result.content == "HTML **content**"
            assert mock_converter.ignore_links is True
            assert mock_converter.ignore_images is True


def test_email_body_extraction_comprehensive_html_only_without_html2text(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "html": "<p>HTML &amp; <strong>content</strong> &lt;test&gt;</p>",
        }

        with patch("kreuzberg._extractors._email.html2text", None):
            result = email_extractor.extract_bytes_sync(b"dummy")

            assert result.content == "HTML & content <test>"


def test_email_body_extraction_comprehensive_both_text_and_html(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": "Plain text version",
            "html": "<p>HTML version</p>",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == "Plain text version"


def test_email_body_extraction_comprehensive_no_content(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "subject": "Subject Only",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == "Subject: Subject Only"


def test_email_body_extraction_comprehensive_empty_text_and_html(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": "",
            "html": "",
            "subject": "Empty Body",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == "Subject: Empty Body"


def test_email_body_extraction_comprehensive_none_text_and_html(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": None,
            "html": None,
            "subject": "No Body",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.content == "Subject: No Body"


def test_email_attachment_extraction_comprehensive_with_names(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "attachments": [
                {"name": "document.pdf", "size": 1024},
                {"name": "image.jpg"},
                {"name": "", "type": "image/png"},
                {},
                {"name": None},
            ],
            "text": "Body with attachments",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        expected_names = ["document.pdf", "image.jpg", "unknown", "unknown", "unknown"]
        assert result.metadata["attachments"] == expected_names
        assert f"Attachments: {', '.join(expected_names)}" in result.content


def test_email_attachment_extraction_comprehensive_none_list(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "attachments": None,
            "text": "Body without attachments",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "attachments" not in result.metadata
        assert "Attachments:" not in result.content


def test_email_attachment_extraction_comprehensive_falsy_but_exists(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "attachments": 0,
            "text": "Body",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert "attachments" not in result.metadata
        assert "Attachments:" not in result.content


def test_email_attachment_extraction_comprehensive_empty_names_only(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "attachments": [
                {"size": 1024},
                {"name": ""},
                {},
            ],
            "text": "Body",
        }

        result = email_extractor.extract_bytes_sync(b"dummy")

        assert result.metadata["attachments"] == ["unknown", "unknown", "unknown"]
        assert "Attachments: unknown, unknown, unknown" in result.content


def test_email_error_handling_comprehensive_generic_exception(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load", side_effect=ValueError("Invalid email format")):
        with pytest.raises(RuntimeError, match="Failed to parse email content: Invalid email format"):
            email_extractor.extract_bytes_sync(b"invalid email")


def test_email_error_handling_comprehensive_attribute_error(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load", side_effect=AttributeError("Missing attribute")):
        with pytest.raises(RuntimeError, match="Failed to parse email content: Missing attribute"):
            email_extractor.extract_bytes_sync(b"malformed email")


def test_email_error_handling_comprehensive_key_error(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load", side_effect=KeyError("missing_key")):
        with pytest.raises(RuntimeError, match="Failed to parse email content: 'missing_key'"):
            email_extractor.extract_bytes_sync(b"incomplete email")


def test_email_error_handling_comprehensive_unicode_decode_error(email_extractor: EmailExtractor) -> None:
    unicode_error = UnicodeDecodeError("utf-8", b"\xff\xfe", 0, 2, "invalid start byte")
    with patch("mailparse.EmailDecode.load", side_effect=unicode_error):
        with pytest.raises(RuntimeError, match="Failed to parse email content"):
            email_extractor.extract_bytes_sync(b"binary email data")


def test_email_integration_comprehensive_complex_email_all_features(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": {"email": "complex@example.com", "name": "Complex Sender"},
            "to": [
                {"email": "recipient1@example.com", "name": "Recipient 1"},
                "recipient2@example.com",
                {"email": "recipient3@example.com"},
            ],
            "cc": [
                {"email": "cc1@example.com"},
                {"email": "cc2@example.com", "name": "CC Person"},
            ],
            "bcc": "bcc@example.com",
            "subject": "Complex Email with All Features",
            "date": "Wed, 15 Mar 2024 14:30:00 +0000",
            "text": "This is the body of a complex email with multiple recipients and attachments.",
            "html": "<p>This HTML should be ignored since text exists</p>",
            "attachments": [
                {"name": "document.pdf", "size": 2048, "type": "application/pdf"},
                {"name": "presentation.pptx", "size": 5120},
                {"name": "data.xlsx"},
            ],
        }

        result = email_extractor.extract_bytes_sync(b"complex email content")

        assert "Subject: Complex Email with All Features" in result.content
        assert "From: complex@example.com" in result.content
        assert "To: recipient1@example.com, recipient2@example.com, recipient3@example.com" in result.content
        assert "CC: cc1@example.com, cc2@example.com" in result.content
        assert "BCC: bcc@example.com" in result.content
        assert "Date: Wed, 15 Mar 2024 14:30:00 +0000" in result.content

        assert "This is the body of a complex email" in result.content
        assert "This HTML should be ignored" not in result.content

        assert "Attachments: document.pdf, presentation.pptx, data.xlsx" in result.content

        assert result.metadata["subject"] == "Complex Email with All Features"
        assert result.metadata["email_from"] == "complex@example.com"
        assert result.metadata["email_to"] == "recipient1@example.com"
        assert result.metadata["date"] == "Wed, 15 Mar 2024 14:30:00 +0000"
        assert result.metadata["attachments"] == ["document.pdf", "presentation.pptx", "data.xlsx"]


def test_email_integration_comprehensive_malformed_email_structure_recovery(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": {"not_email_key": "malformed@example.com"},
            "to": [123, {"email": "valid@example.com"}],
            "subject": ["should", "be", "string"],
            "text": 12345,
            "attachments": "not_a_list",
        }

        result = email_extractor.extract_bytes_sync(b"malformed email")

        assert "From:" in result.content
        assert "To: 123, valid@example.com" in result.content
        assert "Subject: ['should', 'be', 'string']" in result.content
        assert "\n12345" in result.content
        assert "Attachments:" not in result.content


@pytest.mark.anyio
async def test_email_integration_comprehensive_async_complex_email(email_extractor: EmailExtractor) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "from": "async@example.com",
            "to": "recipient@example.com",
            "subject": "Async Complex Email",
            "text": "Processed asynchronously",
            "attachments": [{"name": "async_file.txt"}],
        }

        result = await email_extractor.extract_bytes_async(b"async email content")

        assert "Subject: Async Complex Email" in result.content
        assert "From: async@example.com" in result.content
        assert "Processed asynchronously" in result.content
        assert "Attachments: async_file.txt" in result.content
        assert result.metadata["subject"] == "Async Complex Email"


def test_email_integration_comprehensive_html_with_complex_entities_without_html2text(
    email_extractor: EmailExtractor,
) -> None:
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "html": """
            <html>
                <body>
                    <h1>Title &amp; Subtitle</h1>
                    <p>Price: &euro;100 &lt;discount&gt;</p>
                    <div>Quote: &ldquo;Hello&rdquo;</div>
                    <script>alert('should be removed');</script>
                    <style>body { color: red; }</style>
                </body>
            </html>
            """,
        }

        with patch("kreuzberg._extractors._email.html2text", None):
            result = email_extractor.extract_bytes_sync(b"html email")

            assert "<html>" not in result.content
            assert "<body>" not in result.content
            assert "<script>" not in result.content
            assert "<style>" not in result.content

            assert "Title & Subtitle" in result.content
            assert "Price: €100 <discount>" in result.content
            assert 'Quote: "Hello"' in result.content
