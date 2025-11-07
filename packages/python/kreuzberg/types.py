"""Type definitions for Kreuzberg extraction results.

These TypedDicts mirror the strongly-typed Rust metadata structures,
providing type hints for Python users while the actual data comes from
the Rust core via PyO3 bindings.
"""

# ruff: noqa: A005
from __future__ import annotations

from typing import Any, Literal, TypedDict


class ExcelMetadata(TypedDict, total=False):
    """Excel/spreadsheet metadata."""

    sheet_count: int
    sheet_names: list[str]


class EmailMetadata(TypedDict, total=False):
    """Email metadata."""

    from_email: str | None
    from_name: str | None
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str | None
    attachments: list[str]


class ArchiveMetadata(TypedDict, total=False):
    """Archive (ZIP/TAR/7Z) metadata."""

    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int | None


class ImageMetadata(TypedDict, total=False):
    """Image metadata."""

    width: int
    height: int
    format: str
    exif: dict[str, str]


class XmlMetadata(TypedDict, total=False):
    """XML metadata."""

    element_count: int
    unique_elements: list[str]


class TextMetadata(TypedDict, total=False):
    """Text/Markdown metadata."""

    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None


class PdfMetadata(TypedDict, total=False):
    """PDF metadata."""

    title: str | None
    author: str | None
    subject: str | None
    keywords: str | None
    creator: str | None
    producer: str | None
    creation_date: str | None
    modification_date: str | None
    page_count: int


class HtmlMetadata(TypedDict, total=False):
    """HTML metadata."""

    title: str | None
    description: str | None
    keywords: str | None
    author: str | None
    canonical: str | None
    base_href: str | None
    og_title: str | None
    og_description: str | None
    og_image: str | None
    og_url: str | None
    og_type: str | None
    og_site_name: str | None
    twitter_card: str | None
    twitter_title: str | None
    twitter_description: str | None
    twitter_image: str | None
    twitter_site: str | None
    twitter_creator: str | None
    link_author: str | None
    link_license: str | None
    link_alternate: str | None


class PptxMetadata(TypedDict, total=False):
    """PowerPoint metadata."""

    title: str | None
    author: str | None
    description: str | None
    summary: str | None
    fonts: list[str]


class OcrMetadata(TypedDict, total=False):
    """OCR processing metadata."""

    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int | None
    table_cols: int | None


class ImagePreprocessingMetadata(TypedDict, total=False):
    """Image preprocessing metadata."""

    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    scale_factor: float
    auto_adjusted: bool
    final_dpi: int
    new_dimensions: tuple[int, int] | None
    resample_method: str
    dimension_clamped: bool
    calculated_dpi: int | None
    skipped_resize: bool
    resize_error: str | None


class ErrorMetadata(TypedDict, total=False):
    """Error metadata for batch operations."""

    error_type: str
    message: str


class Metadata(TypedDict, total=False):
    """Strongly-typed metadata for extraction results.

    This TypedDict mirrors the Rust Metadata struct, providing type hints
    for the most common metadata fields. The actual data comes from the
    Rust core and may include additional custom fields from postprocessors.

    All fields are optional (total=False) since they depend on:
    - File format being extracted
    - Feature flags (e.g., PDF support)
    - Postprocessors enabled
    - Extraction configuration

    Format-specific fields are flattened at the root level. Use the format_type
    discriminator to determine which fields are present.

    Common fields:
        language: Document language (ISO 639-1 code)
        date: Document date (ISO 8601 format)
        subject: Document subject

    Discriminator:
        format_type: Format discriminator ("pdf", "excel", "email", etc.)

    Format-specific fields (flattened at root level):
        PDF fields (when format_type == "pdf"):
            title, authors, keywords, created_at, modified_at, created_by,
            producer, page_count, pdf_version, is_encrypted, width, height, summary

        Excel fields (when format_type == "excel"):
            sheet_count, sheet_names

        Email fields (when format_type == "email"):
            from_email, from_name, to_emails, cc_emails, bcc_emails,
            message_id, attachments

        PowerPoint fields (when format_type == "pptx"):
            author, description, fonts

        Archive fields (when format_type == "archive"):
            format, file_count, file_list, total_size, compressed_size

        Image fields (when format_type == "image"):
            exif

        XML fields (when format_type == "xml"):
            element_count, unique_elements

        Text/Markdown fields (when format_type == "text"):
            line_count, word_count, character_count, headers, links, code_blocks

        HTML fields (when format_type == "html"):
            canonical, base_href, og_title, og_description, og_image, og_url,
            og_type, og_site_name, twitter_card, twitter_title,
            twitter_description, twitter_image, twitter_site, twitter_creator,
            link_author, link_license, link_alternate

        OCR fields (when format_type == "ocr"):
            psm, output_format, table_count, table_rows, table_cols

    Processing metadata:
        image_preprocessing: Image preprocessing metadata dict

    Structured data:
        json_schema: JSON schema dict for structured extraction

    Error handling:
        error: Error metadata dict for batch operations

    Custom fields:
        Any additional fields added by Python postprocessors (entity extraction,
        keyword extraction, etc.) will appear as top-level keys in the dict.

    Example:
        >>> result = extract_file("document.xml")
        >>> metadata: Metadata = result["metadata"]
        >>> if metadata.get("format_type") == "xml":
        ...     element_count = metadata["element_count"]
        ...     print(f"Elements: {element_count}")
        >>> if "entities" in metadata:  # Custom field from postprocessor
        ...     entities = metadata["entities"]
    """

    # Common fields
    language: str
    date: str
    subject: str

    # Discriminator field - indicates which format-specific fields are present
    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]

    # PDF format fields (when format_type == "pdf")
    title: str  # PDF
    authors: list[str]  # PDF
    keywords: list[str]  # PDF
    created_at: str  # PDF
    modified_at: str  # PDF
    created_by: str  # PDF
    producer: str  # PDF
    page_count: int  # PDF
    pdf_version: str  # PDF
    is_encrypted: bool  # PDF
    width: int  # PDF
    height: int  # PDF
    summary: str  # PDF

    # Excel format fields (when format_type == "excel")
    sheet_count: int  # Excel
    sheet_names: list[str]  # Excel

    # Email format fields (when format_type == "email")
    from_email: str  # Email
    from_name: str  # Email
    to_emails: list[str]  # Email
    cc_emails: list[str]  # Email
    bcc_emails: list[str]  # Email
    message_id: str  # Email
    attachments: list[str]  # Email

    # PowerPoint format fields (when format_type == "pptx")
    author: str  # PPTX
    description: str  # PPTX
    fonts: list[str]  # PPTX

    # Archive format fields (when format_type == "archive")
    format: str  # Archive
    file_count: int  # Archive
    file_list: list[str]  # Archive
    total_size: int  # Archive
    compressed_size: int  # Archive

    # Image format fields (when format_type == "image")
    exif: dict[str, str]  # Image

    # XML format fields (when format_type == "xml")
    element_count: int  # XML
    unique_elements: list[str]  # XML

    # Text/Markdown format fields (when format_type == "text")
    line_count: int  # Text
    word_count: int  # Text
    character_count: int  # Text
    headers: list[str]  # Text (Markdown only)
    links: list[tuple[str, str]]  # Text (Markdown only)
    code_blocks: list[tuple[str, str]]  # Text (Markdown only)

    # HTML format fields (when format_type == "html")
    canonical: str  # HTML
    base_href: str  # HTML
    og_title: str  # HTML Open Graph
    og_description: str  # HTML Open Graph
    og_image: str  # HTML Open Graph
    og_url: str  # HTML Open Graph
    og_type: str  # HTML Open Graph
    og_site_name: str  # HTML Open Graph
    twitter_card: str  # HTML Twitter Card
    twitter_title: str  # HTML Twitter Card
    twitter_description: str  # HTML Twitter Card
    twitter_image: str  # HTML Twitter Card
    twitter_site: str  # HTML Twitter Card
    twitter_creator: str  # HTML Twitter Card
    link_author: str  # HTML Link Relations
    link_license: str  # HTML Link Relations
    link_alternate: str  # HTML Link Relations

    # OCR format fields (when format_type == "ocr")
    psm: int  # OCR
    output_format: str  # OCR
    table_count: int  # OCR
    table_rows: int  # OCR
    table_cols: int  # OCR

    # Other metadata fields
    image_preprocessing: ImagePreprocessingMetadata
    json_schema: dict[str, Any]
    error: ErrorMetadata


class Table(TypedDict):
    """Extracted table structure."""

    cells: list[list[str]]
    markdown: str
    page_number: int


class ExtractionResult(TypedDict):
    """Extraction result returned by all extraction functions.

    Attributes:
        content: Extracted text content
        mime_type: MIME type of the processed document
        metadata: Strongly-typed metadata (see Metadata TypedDict)
        tables: List of extracted tables
        detected_languages: List of detected language codes (ISO 639-1)
    """

    content: str
    mime_type: str
    metadata: Metadata
    tables: list[Table]
    detected_languages: list[str] | None


__all__ = [
    "ArchiveMetadata",
    "EmailMetadata",
    "ErrorMetadata",
    "ExcelMetadata",
    "ExtractionResult",
    "HtmlMetadata",
    "ImageMetadata",
    "ImagePreprocessingMetadata",
    "Metadata",
    "OcrMetadata",
    "PdfMetadata",
    "PptxMetadata",
    "Table",
    "TextMetadata",
    "XmlMetadata",
]
