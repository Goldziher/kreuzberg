"""Type definitions for Kreuzberg extraction results.

These TypedDicts mirror the strongly-typed Rust metadata structures,
providing type hints for Python users while the actual data comes from
the Rust core via PyO3 bindings.
"""

# ruff: noqa: A005
from __future__ import annotations

from typing import Any, TypedDict


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

    Common fields:
        language: Document language (ISO 639-1 code)
        date: Document date (ISO 8601 format)
        subject: Document subject
        format: File format name

    Format-specific metadata:
        pdf: PDF metadata (requires pdf feature)
        excel: Excel/spreadsheet metadata
        email: Email metadata
        pptx: PowerPoint metadata
        archive: Archive (ZIP/TAR/7Z) metadata
        image: Image metadata
        xml: XML metadata
        text: Text/Markdown metadata

    Processing metadata:
        ocr: OCR processing metadata
        image_preprocessing: Image preprocessing metadata

    Structured data:
        json_schema: JSON schema for structured extraction

    Error handling:
        error: Error metadata for batch operations

    Custom fields:
        Any additional fields added by Python postprocessors (entity extraction,
        keyword extraction, etc.) will appear as top-level keys in the dict.

    Example:
        >>> result = extract_file("document.pdf")
        >>> metadata: Metadata = result["metadata"]
        >>> if "pdf" in metadata:
        ...     pdf_meta = metadata["pdf"]
        ...     print(f"Pages: {pdf_meta['page_count']}")
        >>> if "entities" in metadata:  # Custom field from postprocessor
        ...     entities = metadata["entities"]
    """

    # Common fields
    language: str | None
    date: str | None
    subject: str | None
    format: str | None

    # Format-specific metadata
    pdf: PdfMetadata | None
    excel: ExcelMetadata | None
    email: EmailMetadata | None
    pptx: PptxMetadata | None
    archive: ArchiveMetadata | None
    image: ImageMetadata | None
    xml: XmlMetadata | None
    text: TextMetadata | None

    # Processing metadata
    ocr: OcrMetadata | None
    image_preprocessing: ImagePreprocessingMetadata | None

    # Structured data
    json_schema: Any | None

    # Error metadata
    error: ErrorMetadata | None


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
