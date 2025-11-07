from __future__ import annotations

from datetime import datetime, timezone
from typing import Any
from unittest.mock import Mock, patch

import pytest
from kreuzberg._playa import (
    _collect_document_permissions,
    _extract_author_metadata,
    _extract_basic_metadata,
    _extract_category_metadata,
    _extract_creator_metadata,
    _extract_date_metadata,
    _extract_document_dimensions,
    _extract_keyword_metadata,
    _extract_structure_information,
    _format_outline,
    _generate_document_summary,
    _generate_outline_description,
    _parse_date_string,
    extract_pdf_metadata,
    extract_pdf_metadata_sync,
)


def test_extract_basic_metadata_all_fields() -> None:
    pdf_info = {
        "title": b"Test Title",
        "subject": b"Test Subject",
        "publisher": b"Test Publisher",
        "copyright": b"Test Copyright",
        "comments": b"Test Comments",
        "identifier": b"Test ID",
        "license": b"Test License",
        "modifiedby": b"Test Modified By",
        "version": b"Test Version",
    }
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_basic_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["title"] == "Test Title"
    assert result["subject"] == "Test Subject"
    assert result["publisher"] == "Test Publisher"
    assert result["copyright"] == "Test Copyright"
    assert result["comments"] == "Test Comments"
    assert result["identifier"] == "Test ID"
    assert result["license"] == "Test License"
    assert result["modified_by"] == "Test Modified By"
    assert result["version"] == "Test Version"


def test_extract_basic_metadata_alternative_fields() -> None:
    pdf_info = {
        "Publisher": b"Alt Publisher",
        "rights": b"Alt Rights",
        "id": b"Alt ID",
        "last_modified_by": b"Alt Modified",
    }
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_basic_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["publisher"] == "Alt Publisher"
    assert result["copyright"] == "Alt Rights"
    assert result["identifier"] == "Alt ID"
    assert result["modified_by"] == "Alt Modified"


def test_extract_basic_metadata_skip_existing() -> None:
    pdf_info = {"title": b"New Title"}
    result: dict[str, Any] = {"title": "Existing Title"}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_basic_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["title"] == "Existing Title"


def test_extract_author_metadata_string() -> None:
    pdf_info = {"author": b"Author One; Author Two, Author Three and Author Four"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_author_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["authors"] == ["Author One", "Author Two", "Author Three", "Author Four"]


def test_extract_author_metadata_list() -> None:
    pdf_info = {"author": [b"Author One", b"Author Two"]}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_author_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["authors"] == ["Author One", "Author Two"]


def test_extract_keyword_metadata_string() -> None:
    pdf_info = {"keywords": b"keyword1, keyword2; keyword3,  , keyword4"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_keyword_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["keywords"] == ["keyword1", "keyword2", "keyword3", "keyword4"]


def test_extract_keyword_metadata_list() -> None:
    pdf_info = {"keywords": [b"keyword1", b"keyword2"]}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_keyword_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["keywords"] == ["keyword1", "keyword2"]


def test_extract_category_metadata_string() -> None:
    pdf_info = {"categories": b"cat1, cat2,  , cat3"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_category_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["categories"] == ["cat1", "cat2", "cat3"]


def test_extract_category_metadata_list() -> None:
    pdf_info = {"category": [b"cat1", b"cat2"]}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_category_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["categories"] == ["cat1", "cat2"]


def test_parse_date_string_full() -> None:
    date_str = "D:20240115142530"
    result = _parse_date_string(date_str)

    expected = datetime(2024, 1, 15, 14, 25, 30, tzinfo=timezone.utc).isoformat()
    assert result == expected


def test_parse_date_string_date_only() -> None:
    date_str = "D:20240115"
    result = _parse_date_string(date_str)

    expected = datetime(2024, 1, 15, tzinfo=timezone.utc).isoformat()
    assert result == expected


def test_parse_date_string_invalid() -> None:
    date_str = "invalid"
    result = _parse_date_string(date_str)

    assert result == "invalid"


def test_extract_date_metadata_creation_date() -> None:
    pdf_info = {"creationdate": b"D:20240115142530"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_date_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "created_at" in result
    assert "2024-01-15" in result["created_at"]


def test_extract_date_metadata_modification_date() -> None:
    pdf_info = {"moddate": b"D:20240115142530"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_date_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "modified_at" in result
    assert "2024-01-15" in result["modified_at"]


def test_extract_date_metadata_invalid_date() -> None:
    pdf_info = {
        "createdate": b"Invalid Date",
        "modificationdate": b"Also Invalid",
    }
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_date_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["created_at"] == "Invalid Date"
    assert result["modified_at"] == "Also Invalid"


def test_extract_creator_metadata_creator_only() -> None:
    pdf_info = {"creator": b"Creator App"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_creator_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["created_by"] == "Creator App"


def test_extract_creator_metadata_producer_only() -> None:
    pdf_info = {"producer": b"Producer App"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_creator_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["created_by"] == "Producer App"


def test_extract_creator_metadata_both_same() -> None:
    pdf_info = {"creator": b"Same App", "producer": b"Same App"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_creator_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["created_by"] == "Same App"


def test_extract_creator_metadata_both_different() -> None:
    pdf_info = {"creator": b"Creator App", "producer": b"Producer App"}
    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_creator_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert result["created_by"] == "Creator App (Producer: Producer App)"


def test_extract_document_dimensions_no_attributes() -> None:
    mock_page = Mock(spec=[])
    mock_document = Mock()
    mock_document.pages = [mock_page]

    result: dict[str, Any] = {}
    _extract_document_dimensions(mock_document, result)  # type: ignore[arg-type]

    assert "width" not in result
    assert "height" not in result


def test_format_outline_with_children() -> None:
    mock_child = Mock()
    mock_child.title = "Child Chapter"
    mock_child.children = None

    mock_entry = Mock()
    mock_entry.title = "Parent Chapter"
    mock_entry.children = [mock_child]

    result = _format_outline([mock_entry], level=0)

    assert result == ["- Parent Chapter"]


def test_format_outline_without_title() -> None:
    mock_entry = Mock()
    mock_entry.title = None
    mock_entry.children = None

    result = _format_outline([mock_entry], level=0)

    assert result == []


def test_generate_outline_description_empty() -> None:
    mock_document = Mock()
    mock_document.outline = []

    result = _generate_outline_description(mock_document)

    assert result == ""


def test_generate_document_summary_with_pdf_version() -> None:
    mock_document = Mock()
    mock_document.pages = [Mock()]
    mock_document.pdf_version = "1.7"
    mock_document.is_encrypted = False
    mock_document.status = None
    mock_document.is_pdf_a = False
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False

    result = _generate_document_summary(mock_document)

    assert "PDF version 1.7" in result


def test_generate_document_summary_encrypted_no_method() -> None:
    mock_document = Mock()
    mock_document.pages = [Mock()]
    mock_document.is_encrypted = True
    mock_document.encryption_method = None
    mock_document.status = None
    mock_document.is_pdf_a = False
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False

    result = _generate_document_summary(mock_document)

    assert "Document is encrypted" in result
    assert "Encryption:" not in result


def test_generate_document_summary_with_status() -> None:
    mock_document = Mock()
    mock_document.pages = [Mock()]
    mock_document.is_encrypted = False
    mock_document.status = b"Draft"
    mock_document.is_pdf_a = False
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        result = _generate_document_summary(mock_document)

    assert "Status: Draft" in result


def test_generate_document_summary_pdf_a_no_level() -> None:
    mock_document = Mock()
    mock_document.pages = [Mock()]
    mock_document.is_encrypted = False
    mock_document.status = None
    mock_document.is_pdf_a = True
    mock_document.pdf_a_level = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False

    result = _generate_document_summary(mock_document)

    assert "PDF/A compliant" in result
    assert "PDF/A-" not in result


def test_generate_document_summary_multiple_pages() -> None:
    mock_document = Mock()
    mock_document.pages = [Mock(), Mock(), Mock()]
    mock_document.is_encrypted = False
    mock_document.status = None
    mock_document.is_pdf_a = False
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False

    result = _generate_document_summary(mock_document)

    assert "3 pages" in result


def test_collect_document_permissions_all_true() -> None:
    mock_document = Mock()
    mock_document.is_printable = True
    mock_document.is_modifiable = True
    mock_document.is_extractable = True

    result = _collect_document_permissions(mock_document)

    assert result == ["printable", "modifiable", "extractable"]


def test_extract_structure_information_with_subtitle() -> None:
    mock_element = Mock()
    mock_element.language = "EN"
    mock_element.role = "H1"
    mock_element.text = b"Document Subtitle"
    mock_element.children = None

    mock_document = Mock()
    mock_document.structure = [mock_element]

    result: dict[str, Any] = {"title": "Main Title"}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_structure_information(mock_document, result)  # type: ignore[arg-type]

    assert result.get("languages") == ["en"]
    assert result["subtitle"] == "Document Subtitle"


def test_extract_structure_information_subtitle_same_as_title() -> None:
    mock_element = Mock()
    mock_element.language = None
    mock_element.role = "H1"
    mock_element.text = b"Same Title"
    mock_element.children = None

    mock_document = Mock()
    mock_document.structure = [mock_element]

    result: dict[str, Any] = {"title": "Same Title"}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_structure_information(mock_document, result)  # type: ignore[arg-type]

    assert "subtitle" not in result


def test_extract_structure_information_no_h1() -> None:
    mock_element = Mock()
    mock_element.language = "en"
    mock_element.role = "P"
    mock_element.text = b"Paragraph Text"
    mock_element.children = None

    mock_document = Mock()
    mock_document.structure = [mock_element]

    result: dict[str, Any] = {}

    with patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x):
        _extract_structure_information(mock_document, result)  # type: ignore[arg-type]

    assert result.get("languages") == ["en"]
    assert "subtitle" not in result


@pytest.mark.anyio
async def test_extract_pdf_metadata_no_summary_existing() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa._generate_document_summary") as mock_summary,
    ):
        await extract_pdf_metadata(b"pdf content")
        mock_summary.assert_called_once()

    mock_summary.reset_mock()

    mock_document.info = [{"summary": b"Existing Summary"}]

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={"summary": b"Existing Summary"}),
        patch("kreuzberg._playa._generate_document_summary") as mock_summary,
    ):
        pass


def test_extract_pdf_metadata_sync_no_outline() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa._generate_outline_description") as mock_outline,
    ):
        extract_pdf_metadata_sync(b"pdf content")
        mock_outline.assert_not_called()


def test_extract_pdf_metadata_sync_with_outline_and_existing_description() -> None:
    mock_outline_entry = Mock()
    mock_outline_entry.title = "Chapter 1"
    mock_outline_entry.children = None

    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = [mock_outline_entry]
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
        patch("kreuzberg._playa._extract_basic_metadata") as mock_basic,
        patch("kreuzberg._playa._generate_outline_description") as mock_outline,
    ):

        def set_description(pdf_info: Any, metadata: Any) -> None:
            metadata["description"] = "Existing Description"

        mock_basic.side_effect = set_description

        result = extract_pdf_metadata_sync(b"pdf content")
        mock_outline.assert_not_called()
        assert result["description"] == "Existing Description"


@pytest.mark.anyio
async def test_extract_pdf_metadata_with_summary_no_generation() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa._generate_document_summary") as mock_summary,
        patch("kreuzberg._playa._extract_basic_metadata") as mock_basic,
    ):

        def set_summary(pdf_info: Any, metadata: Any) -> None:
            metadata["summary"] = "Existing Summary"

        mock_basic.side_effect = set_summary

        result = await extract_pdf_metadata(b"pdf content")
        mock_summary.assert_not_called()
        assert result["summary"] == "Existing Summary"


def test_extract_pdf_metadata_sync_with_summary_no_generation() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa._generate_document_summary") as mock_summary,
        patch("kreuzberg._playa._extract_basic_metadata") as mock_basic,
    ):

        def set_summary(pdf_info: Any, metadata: Any) -> None:
            metadata["summary"] = "Existing Summary"

        mock_basic.side_effect = set_summary

        result = extract_pdf_metadata_sync(b"pdf content")
        mock_summary.assert_not_called()
        assert result["summary"] == "Existing Summary"


def test_extract_author_metadata_no_author() -> None:
    pdf_info: dict[str, Any] = {}
    result: dict[str, Any] = {}

    _extract_author_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "authors" not in result


def test_extract_keyword_metadata_no_keywords() -> None:
    pdf_info: dict[str, Any] = {}
    result: dict[str, Any] = {}

    _extract_keyword_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "keywords" not in result


def test_extract_category_metadata_no_categories() -> None:
    pdf_info: dict[str, Any] = {}
    result: dict[str, Any] = {}

    _extract_category_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "categories" not in result


@pytest.mark.anyio
async def test_extract_pdf_metadata_with_pages_no_dimensions() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa._extract_document_dimensions") as mock_dimensions,
    ):
        await extract_pdf_metadata(b"pdf content")
        mock_dimensions.assert_not_called()


def test_extract_pdf_metadata_sync_with_pages_no_dimensions() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa._extract_document_dimensions") as mock_dimensions,
    ):
        extract_pdf_metadata_sync(b"pdf content")
        mock_dimensions.assert_not_called()


def test_extract_author_metadata_invalid_type() -> None:
    pdf_info = {"author": 123}
    result: dict[str, Any] = {}

    _extract_author_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "authors" not in result


def test_extract_keyword_metadata_invalid_type() -> None:
    pdf_info = {"keywords": {"key": "value"}}
    result: dict[str, Any] = {}

    _extract_keyword_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "keywords" not in result


def test_extract_category_metadata_invalid_type() -> None:
    pdf_info = {"categories": 42}
    result: dict[str, Any] = {}

    _extract_category_metadata(pdf_info, result)  # type: ignore[arg-type]

    assert "categories" not in result


def test_extract_pdf_metadata_sync_with_pages() -> None:
    mock_page = Mock()
    mock_page.width = 612.0
    mock_page.height = 792.0

    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = [mock_page]
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
    ):
        result = extract_pdf_metadata_sync(b"pdf content")
        assert "width" in result
        assert "height" in result
        assert result["width"] == 612
        assert result["height"] == 792


def test_extract_pdf_metadata_sync_with_outline() -> None:
    mock_outline_entry = Mock()
    mock_outline_entry.title = "Chapter 1"
    mock_outline_entry.children = None

    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = [mock_outline_entry]
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False
    mock_document.is_encrypted = False
    mock_document.is_pdf_a = False

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
    ):
        result = extract_pdf_metadata_sync(b"pdf content")
        assert "description" in result
        assert "Table of Contents:" in result["description"]
        assert "Chapter 1" in result["description"]


def test_generate_document_summary_with_pdf_version_attr() -> None:
    mock_document = Mock()
    mock_document.pages = []
    mock_document.is_encrypted = False
    mock_document.status = None
    mock_document.is_pdf_a = False
    mock_document.is_printable = False
    mock_document.is_modifiable = False
    mock_document.is_extractable = False

    mock_document.pdf_version = "1.7"
    result = _generate_document_summary(mock_document)
    assert "PDF version 1.7" in result

    del mock_document.pdf_version
    result = _generate_document_summary(mock_document)
    assert "PDF version" not in result
