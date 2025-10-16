from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any

from kreuzberg import ExtractionResult

if TYPE_CHECKING:
    from kreuzberg import Metadata


def validate_extraction_result(result: ExtractionResult, min_content_length: int = 0) -> None:
    assert isinstance(result, ExtractionResult)
    assert isinstance(result.content, str)
    assert isinstance(result.metadata, dict)
    assert isinstance(result.mime_type, str)

    assert len(result.content) >= min_content_length, f"Content too short: {len(result.content)} < {min_content_length}"

    assert result.mime_type, "MIME type should not be empty"
    assert "/" in result.mime_type, f"Invalid MIME type format: {result.mime_type}"


def validate_metadata_completeness(
    metadata: Metadata, required_fields: list[str] | None = None, optional_fields: list[str] | None = None
) -> None:
    if required_fields:
        for field in required_fields:
            assert field in metadata, f"Required metadata field missing: {field}"
            value = metadata.get(field)
            assert value is not None, f"Required field {field} should not be None"

    known_fields = set(required_fields or []) | set(optional_fields or [])
    if known_fields:
        unexpected = set(metadata.keys()) - known_fields
        allowed_extras = {"pandoc-api-version", "blocks", "meta"}
        unexpected -= allowed_extras
        assert not unexpected, f"Unexpected metadata fields: {unexpected}"


def validate_content_preservation(
    original_text: str,
    extracted_content: str,
    required_phrases: list[str] | None = None,
    forbidden_phrases: list[str] | None = None,
) -> None:
    if required_phrases:
        for phrase in required_phrases:
            assert phrase in extracted_content, f"Required phrase not found in content: '{phrase}'"

    if forbidden_phrases:
        for phrase in forbidden_phrases:
            assert phrase not in extracted_content, f"Forbidden phrase found in content: '{phrase}'"

    assert extracted_content.strip(), "Extracted content should not be empty or whitespace only"

    assert "\\begin{" not in extracted_content, "LaTeX commands should be converted"
    assert ".. code-block::" not in extracted_content, "RST directives should be converted"
    assert "<!--" not in extracted_content, "HTML comments should be removed"


def validate_citation_extraction(metadata: Metadata, expected_citations: list[str] | None = None) -> None:
    if expected_citations:
        assert "citations" in metadata, "Citations field missing from metadata"
        citations = metadata["citations"]
        assert isinstance(citations, list), f"Citations should be a list, got {type(citations)}"

        for expected in expected_citations:
            assert expected in citations, f"Expected citation not found: {expected}"


def validate_author_extraction(metadata: Metadata, expected_authors: list[str] | None = None) -> None:
    if expected_authors:
        assert "authors" in metadata, "Authors field missing from metadata"
        authors = metadata["authors"]
        assert isinstance(authors, list), f"Authors should be a list, got {type(authors)}"

        author_strings = [str(a).lower() for a in authors]
        for expected in expected_authors:
            expected_lower = expected.lower()
            assert any(expected_lower in a for a in author_strings), f"Expected author not found: {expected}"


def validate_cross_format_consistency(
    results: dict[str, ExtractionResult],
    common_content: list[str] | None = None,
    common_metadata_fields: list[str] | None = None,
) -> None:
    if not results or len(results) < 2:
        return

    if common_content:
        for format_name, result in results.items():
            for content_piece in common_content:
                assert content_piece in result.content, f"Common content missing in {format_name}: '{content_piece}'"

    if common_metadata_fields:
        for field in common_metadata_fields:
            values = {}
            for format_name, result in results.items():
                if field in result.metadata:
                    values[format_name] = result.metadata.get(field)

            if len(values) > 1:
                value_list = [v.lower().strip() for v in values.values() if isinstance(v, str)]
                if len(value_list) > 1:
                    assert len(set(value_list)) == 1, f"Inconsistent {field} across formats: {values}"


def validate_table_extraction(content: str, expected_rows: int | None = None, expected_cols: int | None = None) -> None:
    if expected_rows:
        table_lines = [line for line in content.split("\n") if "|" in line]
        actual_rows = len([line for line in table_lines if not all(c in "|-: " for c in line)])
        assert actual_rows >= expected_rows, f"Expected at least {expected_rows} table rows, found {actual_rows}"

    if expected_cols:
        table_lines = [line for line in content.split("\n") if "|" in line]
        if table_lines:
            first_row = table_lines[0]
            col_count = first_row.count("|") - 1
            assert col_count >= expected_cols, f"Expected at least {expected_cols} columns, found {col_count}"


def validate_math_extraction(
    content: str, inline_math: list[str] | None = None, display_math: list[str] | None = None
) -> None:
    if inline_math:
        for formula in inline_math:
            assert f"${formula}$" in content or f"\\({formula}\\)" in content, f"Inline math not found: {formula}"

    if display_math:
        for formula in display_math:
            assert f"$${formula}$$" in content or f"\\[{formula}\\]" in content, f"Display math not found: {formula}"


def compare_extraction_methods(sync_result: ExtractionResult, async_result: ExtractionResult) -> None:
    assert sync_result.content == async_result.content, "Content differs between sync and async extraction"
    assert sync_result.metadata == async_result.metadata, "Metadata differs between sync and async extraction"
    assert sync_result.mime_type == async_result.mime_type, "MIME type differs between sync and async extraction"


def validate_error_handling(
    extractor_func: Any, invalid_input: Path | bytes, expected_exception_type: type[Exception]
) -> None:
    import pytest

    if isinstance(invalid_input, Path):
        with pytest.raises(expected_exception_type):
            extractor_func(invalid_input)
    else:
        with pytest.raises(expected_exception_type):
            extractor_func(invalid_input)
