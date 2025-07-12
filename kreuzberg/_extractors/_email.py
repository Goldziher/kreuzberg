from __future__ import annotations

import re
from html import unescape
from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import EML_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path

# Import optional dependencies at module level with proper error handling
try:
    import mailparse
except ImportError:
    mailparse = None

try:
    import html2text  # type: ignore[import-not-found]
except ImportError:
    html2text = None

# Compile regex pattern once at module level
_HTML_TAG_PATTERN = re.compile(r"<[^>]+>")


class EmailExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {EML_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def _extract_email_headers(
        self, parsed_email: dict[str, Any], text_parts: list[str], metadata: dict[str, Any]
    ) -> None:
        """Extract and process email headers."""
        # Use single dict access where possible to avoid repeated lookups
        subject = parsed_email.get("subject")
        if subject:
            metadata["subject"] = subject
            text_parts.append(f"Subject: {subject}")

        from_info = parsed_email.get("from")
        if from_info:
            from_email = from_info.get("email", "") if isinstance(from_info, dict) else str(from_info)
            metadata["email_from"] = from_email
            text_parts.append(f"From: {from_email}")

        to_info = parsed_email.get("to")
        if to_info:
            if isinstance(to_info, list) and to_info:
                to_email = to_info[0].get("email", "") if isinstance(to_info[0], dict) else str(to_info[0])
            elif isinstance(to_info, dict):
                to_email = to_info.get("email", "")
            else:
                to_email = str(to_info)
            metadata["email_to"] = to_email
            text_parts.append(f"To: {to_email}")

        date = parsed_email.get("date")
        if date:
            metadata["date"] = date
            text_parts.append(f"Date: {date}")

        cc = parsed_email.get("cc")
        if cc:
            metadata["email_cc"] = cc
            text_parts.append(f"CC: {cc}")

        bcc = parsed_email.get("bcc")
        if bcc:
            metadata["email_bcc"] = bcc
            text_parts.append(f"BCC: {bcc}")

    def _extract_email_body(self, parsed_email: dict[str, Any], text_parts: list[str]) -> None:
        """Extract and process email body content."""
        text_content = parsed_email.get("text")
        if text_content:
            text_parts.append(f"\n{text_content}")
            return  # If we have text, prefer it over HTML

        html_content = parsed_email.get("html")
        if html_content:
            if html2text is not None:
                # Use html2text if available (faster path)
                h = html2text.HTML2Text()
                h.ignore_links = True
                h.ignore_images = True
                converted_text = h.handle(html_content)
                text_parts.append(f"\n{converted_text}")
            else:
                # Fallback: strip HTML tags and unescape entities
                clean_html = _HTML_TAG_PATTERN.sub("", html_content)
                clean_html = unescape(clean_html)
                text_parts.append(f"\n{clean_html}")

    def _extract_email_attachments(
        self, parsed_email: dict[str, Any], text_parts: list[str], metadata: dict[str, Any]
    ) -> None:
        """Extract and process email attachments info."""
        if parsed_email.get("attachments"):
            attachment_names = [att.get("name", "unknown") for att in parsed_email["attachments"]]
            metadata["attachments"] = attachment_names
            if attachment_names:
                text_parts.append(f"\nAttachments: {', '.join(attachment_names)}")

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        if mailparse is None:
            msg = "mailparse is required for email extraction. Install with: pip install 'kreuzberg[additional-extensions]'"
            raise MissingDependencyError(msg)

        try:
            parsed_email = mailparse.EmailDecode.load(content)
            text_parts: list[str] = []
            metadata: dict[str, Any] = {}

            # Extract headers, body, and attachments
            self._extract_email_headers(parsed_email, text_parts, metadata)
            self._extract_email_body(parsed_email, text_parts)
            self._extract_email_attachments(parsed_email, text_parts, metadata)

            # Join efficiently
            combined_text = "\n".join(text_parts)

            return ExtractionResult(
                content=normalize_spaces(combined_text),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

        except Exception as e:
            msg = f"Failed to parse email content: {e}"
            raise RuntimeError(msg) from e

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
