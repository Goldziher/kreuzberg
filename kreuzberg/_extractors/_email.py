from __future__ import annotations

from typing import TYPE_CHECKING, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import EML_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._types import ExtractionResult, normalize_metadata
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path


class EmailExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {EML_MIME_TYPE}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return await run_sync(self.extract_bytes_sync, content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content = await AsyncPath(path).read_bytes()
        return await self.extract_bytes_async(content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        try:
            import mailparse
        except ImportError as e:
            msg = "mailparse is required for email extraction. Install with: pip install 'kreuzberg[additional-extensions]'"
            raise MissingDependencyError(msg) from e

        try:
            parsed_email = mailparse.EmailDecode.load(content)

            text_parts = []
            metadata = {}

            if "subject" in parsed_email:
                metadata["subject"] = parsed_email["subject"]
                text_parts.append(f"Subject: {parsed_email['subject']}")

            if parsed_email.get("from"):
                from_info = parsed_email["from"]
                from_email = from_info.get("email", "") if isinstance(from_info, dict) else str(from_info)
                metadata["email_from"] = from_email
                text_parts.append(f"From: {from_email}")

            if parsed_email.get("to"):
                to_info = parsed_email["to"]
                if isinstance(to_info, list) and to_info:
                    to_email = to_info[0].get("email", "") if isinstance(to_info[0], dict) else str(to_info[0])
                elif isinstance(to_info, dict):
                    to_email = to_info.get("email", "")
                else:
                    to_email = str(to_info)
                metadata["email_to"] = to_email
                text_parts.append(f"To: {to_email}")

            if "date" in parsed_email:
                metadata["date"] = parsed_email["date"]
                text_parts.append(f"Date: {parsed_email['date']}")

            if "cc" in parsed_email:
                metadata["email_cc"] = parsed_email["cc"]
                text_parts.append(f"CC: {parsed_email['cc']}")

            if "bcc" in parsed_email:
                metadata["email_bcc"] = parsed_email["bcc"]
                text_parts.append(f"BCC: {parsed_email['bcc']}")

            if parsed_email.get("text"):
                text_parts.append(f"\n{parsed_email['text']}")

            if "html" in parsed_email and parsed_email["html"] and not parsed_email.get("text"):
                html_content = parsed_email["html"]
                try:
                    import html2text  # type: ignore[import-not-found]

                    h = html2text.HTML2Text()
                    h.ignore_links = True
                    h.ignore_images = True
                    text_content = h.handle(html_content)
                    text_parts.append(f"\n{text_content}")
                except ImportError:
                    import re
                    from html import unescape

                    html_content = re.sub(r"<[^>]+>", "", html_content)
                    html_content = unescape(html_content)
                    text_parts.append(f"\n{html_content}")

            if parsed_email.get("attachments"):
                attachment_names = [att.get("name", "unknown") for att in parsed_email["attachments"]]
                metadata["attachments"] = attachment_names
                if attachment_names:
                    text_parts.append(f"\nAttachments: {', '.join(attachment_names)}")

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
