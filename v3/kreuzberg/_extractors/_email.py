from __future__ import annotations

import base64
import re
from html import unescape
from typing import TYPE_CHECKING, Any, ClassVar

from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import EML_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._types import ExtractedImage, ExtractionResult, ImageOCRResult, normalize_metadata
from kreuzberg._utils._sync import run_maybe_async, run_sync
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path

try:
    import mailparse
except ImportError:  # pragma: no cover
    mailparse = None

try:
    import html2text  # type: ignore[import-not-found]
except ImportError:  # pragma: no cover
    html2text = None

_HTML_TAG_PATTERN = re.compile(r"<[^>]+>")
_UNICODE_QUOTES_PATTERN = re.compile(r"[\u201c\u201d]")
_UNICODE_SINGLE_QUOTES_PATTERN = re.compile(r"[\u2018\u2019]")


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
                metadata["email_to"] = to_email
            elif isinstance(to_info, dict):
                metadata["email_to"] = to_info.get("email", "")
            else:
                metadata["email_to"] = str(to_info)

            to_formatted = self._format_email_field(to_info)
            text_parts.append(f"To: {to_formatted}")

        date = parsed_email.get("date")
        if date:
            metadata["date"] = date
            text_parts.append(f"Date: {date}")

        cc = parsed_email.get("cc")
        if cc:
            metadata["email_cc"] = cc
            cc_formatted = self._format_email_field(cc)
            text_parts.append(f"CC: {cc_formatted}")

        bcc = parsed_email.get("bcc")
        if bcc:
            metadata["email_bcc"] = bcc
            bcc_formatted = self._format_email_field(bcc)
            text_parts.append(f"BCC: {bcc_formatted}")

    def _format_email_field(self, field: Any) -> str:
        match field:
            case list():
                emails = []
                for item in field:
                    if isinstance(item, dict):
                        if email := item.get("email", ""):
                            emails.append(str(email))
                    else:
                        emails.append(str(item))
                return ", ".join(emails)
            case dict():
                return str(field.get("email", ""))
            case _:
                return str(field)

    def _extract_email_body(self, parsed_email: dict[str, Any], text_parts: list[str]) -> None:
        text_content = parsed_email.get("text")
        if text_content:
            text_parts.append(str(text_content))
            return

        html_content = parsed_email.get("html")
        if html_content:
            if html2text is not None:
                h = html2text.HTML2Text()
                h.ignore_links = True
                h.ignore_images = True
                converted_text = h.handle(html_content)
                text_parts.append(converted_text)
            else:
                cleaned = re.sub(r"<script[^>]*>.*?</script>", "", html_content, flags=re.IGNORECASE | re.DOTALL)
                cleaned = re.sub(r"<style[^>]*>.*?</style>", "", cleaned, flags=re.IGNORECASE | re.DOTALL)
                clean_html = _HTML_TAG_PATTERN.sub("", cleaned)
                clean_html = unescape(clean_html)
                clean_html = _UNICODE_QUOTES_PATTERN.sub('"', clean_html)
                clean_html = _UNICODE_SINGLE_QUOTES_PATTERN.sub("'", clean_html)
                text_parts.append(clean_html)

    def _extract_email_attachments(
        self, parsed_email: dict[str, Any], text_parts: list[str], metadata: dict[str, Any]
    ) -> None:
        attachments = parsed_email.get("attachments")
        if not isinstance(attachments, list):
            return
        names: list[str] = []
        for att in attachments:
            name_val: str = "unknown"
            if isinstance(att, dict):
                n = att.get("name") or att.get("filename")
                if isinstance(n, str) and n:
                    name_val = n
            names.append(name_val)
        if names:
            metadata["attachments"] = names
            text_parts.append("Attachments: " + ", ".join(names))

    def _extract_images_from_attachments(self, parsed_email: dict[str, Any]) -> list[ExtractedImage]:
        images: list[ExtractedImage] = []
        attachments = parsed_email.get("attachments") or []
        if not isinstance(attachments, list):
            return []

        for idx, att in enumerate(attachments, start=1):
            if not isinstance(att, dict):
                continue

            mime = att.get("mime") or att.get("content_type") or att.get("type")
            if not isinstance(mime, str) or not mime.startswith("image/"):
                continue

            name = att.get("name") or att.get("filename")
            name = name if isinstance(name, str) else None
            data = att.get("data") or att.get("content") or att.get("payload")
            raw: bytes | None = None
            if isinstance(data, (bytes, bytearray)):
                raw = bytes(data)
            elif isinstance(data, str):
                try:
                    raw = base64.b64decode(data)
                except Exception:  # noqa: BLE001
                    raw = data.encode()

            if raw is None:
                continue

            fmt = mime.split("/", 1)[1].lower()
            if name and "." in name:
                ext = name.rsplit(".", 1)[-1].lower()
                if ext:
                    fmt = ext

            filename = name or f"attachment_image_{idx}.{fmt}"
            images.append(
                ExtractedImage(
                    data=raw,
                    format=fmt,
                    filename=filename,
                    page_number=None,
                )
            )

        return images

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        if mailparse is None:
            msg = "mailparse is required for email extraction. Install with: pip install 'kreuzberg[additional-extensions]'"
            raise MissingDependencyError(msg)

        try:
            parsed_email = mailparse.EmailDecode.load(content)
            text_parts: list[str] = []
            metadata: dict[str, Any] = {}

            self._extract_email_headers(parsed_email, text_parts, metadata)
            self._extract_email_body(parsed_email, text_parts)
            self._extract_email_attachments(parsed_email, text_parts, metadata)

            combined_text = "\n".join(text_parts)

            result = ExtractionResult(
                content=combined_text,
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=normalize_metadata(metadata),
                chunks=[],
            )

            if self.config.extract_images:
                images = self._extract_images_from_attachments(parsed_email)
                result.images = images
                if self.config.ocr_extracted_images and result.images:
                    image_ocr_results: list[ImageOCRResult] = run_maybe_async(
                        self._process_images_with_ocr, result.images
                    )
                    result.image_ocr_results = image_ocr_results

            return result

        except Exception as e:
            msg = f"Failed to parse email content: {e}"
            raise RuntimeError(msg) from e

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        content = path.read_bytes()
        return self.extract_bytes_sync(content)
