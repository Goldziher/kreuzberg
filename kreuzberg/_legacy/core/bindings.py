"""Direct re-exports from Rust bindings - zero overhead."""

# Email
# Excel
# HTML
# XML
# Text
# Chunking (Rust-backed)
# Quality (Rust-backed)
# Cache (Rust-backed)
# Text utilities (Rust-backed)
# Table utilities (Rust-backed)
from kreuzberg._internal_bindings import (
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

__all__ = [
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
    "batch_generate_cache_keys",
    "batch_process_texts",
    "build_email_text_output",
    # Quality (Rust-backed)
    "calculate_quality_score",
    "clean_extracted_text",
    "convert_html_to_markdown",
    "extract_email_content",
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
