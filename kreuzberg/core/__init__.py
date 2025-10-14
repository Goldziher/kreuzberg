"""Core module - thin wrappers around Rust bindings (zero overhead)."""

from kreuzberg.core.bindings import (
    EmailAttachment,
    EmailExtractionResult,
    ExcelSheet,
    ExcelWorkbook,
    ExtractedInlineImage,
    GenericCache,
    HtmlExtractionResult,
    MarkdownSplitter,
    TextExtractionResult,
    TextSplitter,
    XmlExtractionResult,
    batch_generate_cache_keys,
    batch_process_texts,
    build_email_text_output,
    calculate_quality_score,
    clean_extracted_text,
    convert_html_to_markdown,
    extract_email_content,
    fix_mojibake,
    generate_cache_key,
    normalize_spaces,
    parse_eml_content,
    parse_msg_content,
    parse_text,
    parse_xml,
    process_html,
    read_excel_bytes,
    read_excel_file,
    safe_decode,
    table_from_arrow_to_markdown,
)
from kreuzberg.core.chunking import chunk_markdown, chunk_text
from kreuzberg.core.extraction import (
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)
from kreuzberg.core.types import ChunkingConfig

__all__ = [
    # Chunking
    "ChunkingConfig",
    "EmailAttachment",
    # Email
    "EmailExtractionResult",
    "ExcelSheet",
    # Excel
    "ExcelWorkbook",
    "ExtractedInlineImage",
    # Cache (Rust-backed)
    "GenericCache",
    # HTML
    "HtmlExtractionResult",
    "MarkdownSplitter",
    # Text
    "TextExtractionResult",
    # Chunking (Rust-backed)
    "TextSplitter",
    # XML
    "XmlExtractionResult",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_file",
    "batch_extract_file_sync",
    "batch_generate_cache_keys",
    "batch_process_texts",
    "build_email_text_output",
    # Quality (Rust-backed)
    "calculate_quality_score",
    "chunk_markdown",
    "chunk_text",
    "clean_extracted_text",
    "convert_html_to_markdown",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_email_content",
    # Extraction functions
    "extract_file",
    "extract_file_sync",
    "fix_mojibake",
    "generate_cache_key",
    "normalize_spaces",
    "parse_eml_content",
    "parse_msg_content",
    "parse_text",
    "parse_xml",
    "process_html",
    "read_excel_bytes",
    "read_excel_file",
    # Text utilities (Rust-backed)
    "safe_decode",
    # Table utilities (Rust-backed)
    "table_from_arrow_to_markdown",
]
