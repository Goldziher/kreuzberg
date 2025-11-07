from __future__ import annotations

from unittest.mock import patch

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._email import EmailExtractor


def _make_extractor() -> EmailExtractor:
    return EmailExtractor(mime_type="message/rfc822", config=ExtractionConfig())


def test_email_attachments_not_list_is_ignored() -> None:
    extractor = _make_extractor()
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": "Hello",
            "attachments": "not_a_list",
        }

        result = extractor.extract_bytes_sync(b"raw")
        assert result.content == "Hello"
        assert "attachments" not in result.metadata or result.metadata.get("attachments") == []


def test_email_attachments_names_none_replaced_with_unknown() -> None:
    extractor = _make_extractor()
    with patch("mailparse.EmailDecode.load") as mock_load:
        mock_load.return_value = {
            "text": "Body",
            "attachments": [
                {"name": None},
                {},
                {"name": "file.jpg"},
            ],
        }

        result = extractor.extract_bytes_sync(b"raw")
        assert result.metadata.get("attachments") == ["unknown", "unknown", "file.jpg"]
