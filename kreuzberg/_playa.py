from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Any, cast

from playa import asobj, parse
from playa.utils import decode_text

from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:
    from playa.document import Document

    from kreuzberg._types import Metadata


GRAY_COMPONENTS = 1
RGB_COMPONENTS = 3
CMYK_COMPONENTS = 4
UTF16BE_BOM = b"\xfe\xff"
UTF16BE_ENCODING = "utf-16be"
MIN_DATE_LENGTH = 8
FULL_DATE_LENGTH = 14
BOM_CHAR = "\ufeff"


async def extract_pdf_metadata(pdf_content: bytes) -> Metadata:
    """Extract metadata from a PDF document.

    Args:
        pdf_content: The bytes of the PDF document.

    Raises:
        ParsingError: If the PDF metadata could not be extracted.

    Returns:
        A dictionary of metadata extracted from the PDF.
    """
    try:
        document = parse(pdf_content, max_workers=1)
        metadata: Metadata = {}

        for raw_info in document.info:
            pdf_info = {k.lower(): v for k, v in asobj(raw_info).items()}
            _extract_basic_metadata(pdf_info, metadata)
            _extract_author_metadata(pdf_info, metadata)
            _extract_keyword_metadata(pdf_info, metadata)
            _extract_category_metadata(pdf_info, metadata)
            _extract_date_metadata(pdf_info, metadata)
            _extract_creator_metadata(pdf_info, metadata)

        if document.pages:
            _extract_document_dimensions(document, metadata)

        if document.outline and "description" not in metadata:
            metadata["description"] = _generate_outline_description(document)

        if "summary" not in metadata:
            metadata["summary"] = _generate_document_summary(document)

        _extract_structure_information(document, metadata)

        return metadata
    except Exception as e:
        raise ParsingError(f"Failed to extract PDF metadata: {e!s}") from e


def _extract_basic_metadata(pdf_info: dict[str, Any], result: Metadata) -> None:
    if "title" not in result and (title := pdf_info.get("title")):
        result["title"] = decode_text(title)

    if "subject" not in result and (subject := pdf_info.get("subject")):
        result["subject"] = decode_text(subject)

    if "publisher" not in result and (publisher := pdf_info.get("Publisher", pdf_info.get("publisher"))):
        result["publisher"] = decode_text(publisher)

    if "copyright" not in result and (copyright_info := pdf_info.get("copyright") or pdf_info.get("rights")):
        result["copyright"] = decode_text(copyright_info)

    if "comments" not in result and (comments := pdf_info.get("comments")):
        result["comments"] = decode_text(comments)

    if "identifier" not in result and (identifier := pdf_info.get("identifier") or pdf_info.get("id")):
        result["identifier"] = decode_text(identifier)

    if "license" not in result and (license_info := pdf_info.get("license")):
        result["license"] = decode_text(license_info)

    if "modified_by" not in result and (modified_by := pdf_info.get("modifiedby") or pdf_info.get("last_modified_by")):
        result["modified_by"] = decode_text(modified_by)

    if "version" not in result and (version := pdf_info.get("version")):
        result["version"] = decode_text(version)


def _extract_author_metadata(pdf_info: dict[str, Any], result: Metadata) -> None:
    if author := pdf_info.get("author"):
        if isinstance(author, (str, bytes)):
            author_str = decode_text(author)
            author_str = author_str.replace(" and ", ", ")

            authors = []
            for author_segment in author_str.split(";"):
                authors.extend(
                    [author_name.strip() for author_name in author_segment.split(",") if author_name.strip()]
                )
            result["authors"] = authors
        elif isinstance(author, list):
            result["authors"] = [decode_text(a) for a in author]


def _extract_keyword_metadata(pdf_info: dict[str, Any], result: Metadata) -> None:
    if keywords := pdf_info.get("keywords"):
        if isinstance(keywords, (str, bytes)):
            kw_str = decode_text(keywords)
            kw_list = [k.strip() for k in kw_str.split(",")]
            kw_list = [k.strip() for k in " ".join(kw_list).split(";")]
            result["keywords"] = [k for k in kw_list if k]
        elif isinstance(keywords, list):
            result["keywords"] = [decode_text(k) for k in keywords]


def _extract_category_metadata(pdf_info: dict[str, Any], result: Metadata) -> None:
    if categories := pdf_info.get("categories") or pdf_info.get("category"):
        if isinstance(categories, (str, bytes)):
            cat_str = decode_text(categories)
            cat_list = [c.strip() for c in cat_str.split(",")]
            result["categories"] = [c for c in cat_list if c]
        elif isinstance(categories, list):
            result["categories"] = [decode_text(c) for c in categories]


def _parse_date_string(date_str: str) -> str:
    date_str = date_str.removeprefix("D:")
    if len(date_str) >= MIN_DATE_LENGTH:
        year = date_str[0:4]
        month = date_str[4:6]
        day = date_str[6:8]
        time_part = ""
        if len(date_str) >= FULL_DATE_LENGTH:
            hour = date_str[8:10]
            minute = date_str[10:12]
            second = date_str[12:14]
            time_part = f"T{hour}:{minute}:{second}"
        return datetime.strptime(f"{year}-{month}-{day}{time_part}", "%Y%m%d%H%M%S").isoformat()  # noqa: DTZ007
    return date_str


def _extract_date_metadata(pdf_info: dict[str, Any], result: Metadata) -> None:
    if created := pdf_info.get("creationdate") or pdf_info.get("createdate"):
        try:
            date_str = decode_text(created)
            result["created_at"] = _parse_date_string(date_str)
        except (ValueError, IndexError):
            result["created_at"] = decode_text(created)

    if modified := pdf_info.get("moddate") or pdf_info.get("modificationdate"):
        try:
            date_str = decode_text(modified)
            result["modified_at"] = _parse_date_string(date_str)
        except (ValueError, IndexError):
            result["modified_at"] = decode_text(modified)


def _extract_creator_metadata(pdf_info: dict[str, Any], result: Metadata) -> None:
    if creator := pdf_info.get("creator"):
        result["created_by"] = decode_text(creator)

    if producer := pdf_info.get("producer"):
        producer_str = decode_text(producer)
        if "created_by" not in result:
            result["created_by"] = producer_str
        elif producer_str not in result["created_by"]:
            result["created_by"] = f"{result['created_by']} (Producer: {producer_str})"


def _extract_document_dimensions(document: Document, result: Metadata) -> None:
    first_page = document.pages[0]
    if hasattr(first_page, "width") and hasattr(first_page, "height"):
        result["width"] = int(first_page.width)
        result["height"] = int(first_page.height)


def _format_outline(entries: list[Any], level: int = 0) -> list[str]:
    outline_text: list[str] = []
    for entry in entries:
        if hasattr(entry, "title") and entry.title:
            indent = "  " * level
            outline_text.append(f"{indent}- {entry.title}")
        if hasattr(entry, "children") and entry.children:
            _format_outline(entry.children, level + 1)

    return outline_text


def _generate_outline_description(document: Document) -> str:
    if outline_text := _format_outline(cast("list[Any]", document.outline)):
        return "Table of Contents:\n" + "\n".join(outline_text)
    return ""


def _generate_document_summary(document: Document) -> str:
    summary_parts = []

    page_count = len(document.pages)
    summary_parts.append(f"PDF document with {page_count} page{'s' if page_count != 1 else ''}.")

    if hasattr(document, "pdf_version"):
        summary_parts.append(f"PDF version {document.pdf_version}.")

    if hasattr(document, "is_encrypted") and document.is_encrypted:
        summary_parts.append("Document is encrypted.")

        if hasattr(document, "encryption_method") and document.encryption_method:
            summary_parts.append(f"Encryption: {document.encryption_method}.")

    permissions = _collect_document_permissions(document)
    if permissions:
        summary_parts.append(f"Document is {', '.join(permissions)}.")

    if hasattr(document, "status") and document.status:
        status = decode_text(document.status)
        summary_parts.append(f"Status: {status}.")

    if hasattr(document, "is_pdf_a") and document.is_pdf_a:
        if hasattr(document, "pdf_a_level") and document.pdf_a_level:
            summary_parts.append(f"PDF/A-{document.pdf_a_level} compliant.")
        else:
            summary_parts.append("PDF/A compliant.")

    return " ".join(summary_parts)


def _collect_document_permissions(document: Document) -> list[str]:
    permissions = []
    if document.is_printable:
        permissions.append("printable")
    if document.is_modifiable:
        permissions.append("modifiable")
    if document.is_extractable:
        permissions.append("extractable")
    return permissions


def _extract_structure_information(document: Document, result: Metadata) -> None:
    """Extract language and subtitle from document structure."""
    if document.structure:
        languages = set()
        subtitle = None

        def extract_languages(elements: list[Any]) -> None:
            nonlocal subtitle
            for element in elements:
                if hasattr(element, "language") and element.language:
                    languages.add(element.language.lower())

                if (
                    subtitle is None
                    and hasattr(element, "role")
                    and element.role == "H1"
                    and hasattr(element, "text")
                    and element.text
                ):
                    subtitle = decode_text(element.text)

                if hasattr(element, "children") and element.children:
                    extract_languages(element.children)

        extract_languages(cast("list[Any]", document.structure))

        if languages:
            result["languages"] = list(languages)

        if subtitle and "title" in result and subtitle != result["title"]:
            result["subtitle"] = subtitle
