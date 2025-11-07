from __future__ import annotations

from unittest.mock import Mock, patch

import pytest
from kreuzberg._playa import (
    extract_pdf_metadata,
    extract_pdf_metadata_sync,
)
from kreuzberg.exceptions import ParsingError


@pytest.mark.anyio
async def test_extract_pdf_metadata_success() -> None:
    mock_page = Mock()
    mock_page.width = 612.0
    mock_page.height = 792.0

    mock_document = Mock()
    mock_document.info = [{"title": b"Test Document", "author": b"Test Author"}]
    mock_document.pages = [mock_page]
    mock_document.outline = None
    mock_document.structure = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={"title": b"Test Document", "author": b"Test Author"}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
    ):
        result = await extract_pdf_metadata(b"fake pdf content", "password123")

    assert isinstance(result, dict)
    assert "summary" in result


@pytest.mark.anyio
async def test_extract_pdf_metadata_with_outline() -> None:
    mock_outline_entry = Mock()
    mock_outline_entry.title = "Chapter 1"
    mock_outline_entry.children = None

    mock_page = Mock()
    mock_page.width = 612.0
    mock_page.height = 792.0

    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = [mock_page]
    mock_document.outline = [mock_outline_entry]
    mock_document.structure = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
    ):
        result = await extract_pdf_metadata(b"fake pdf content")

    assert "description" in result
    assert "Table of Contents:" in result["description"]


@pytest.mark.anyio
async def test_extract_pdf_metadata_parsing_error() -> None:
    with patch("kreuzberg._playa.parse", side_effect=Exception("Parsing failed")):
        with pytest.raises(ParsingError) as exc_info:
            await extract_pdf_metadata(b"invalid pdf content")

    assert "Failed to extract PDF metadata" in str(exc_info.value)
    assert "Parsing failed" in str(exc_info.value)


@pytest.mark.anyio
async def test_extract_pdf_metadata_with_password() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document) as mock_parse,
        patch("kreuzberg._playa.asobj", return_value={}),
    ):
        await extract_pdf_metadata(b"encrypted pdf content", password="secret123")

    mock_parse.assert_called_once_with(b"encrypted pdf content", max_workers=1, password="secret123")


@pytest.mark.anyio
async def test_extract_pdf_metadata_with_pages_for_dimensions() -> None:
    mock_page = Mock()
    mock_page.width = 612.0
    mock_page.height = 792.0

    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = [mock_page]
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with patch("kreuzberg._playa.parse", return_value=mock_document), patch("kreuzberg._playa.asobj", return_value={}):
        result = await extract_pdf_metadata(b"pdf with pages")

    assert result["width"] == 612
    assert result["height"] == 792


@pytest.mark.anyio
async def test_extract_pdf_metadata_with_structure() -> None:
    mock_element = Mock()
    mock_element.language = "en"
    mock_element.role = "H1"
    mock_element.text = b"Subtitle Text"
    mock_element.children = None

    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = [mock_element]
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
    ):
        result = await extract_pdf_metadata(b"pdf with structure")

    assert "languages" in result
    assert "en" in result["languages"]


def test_extract_pdf_metadata_sync_success() -> None:
    mock_document = Mock()
    mock_document.info = [{"title": b"Sync Test Document"}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={"title": b"Sync Test Document"}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
    ):
        result = extract_pdf_metadata_sync(b"fake pdf content")

    assert isinstance(result, dict)
    assert "summary" in result


def test_extract_pdf_metadata_sync_with_password() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document) as mock_parse,
        patch("kreuzberg._playa.asobj", return_value={}),
    ):
        extract_pdf_metadata_sync(b"encrypted pdf content", password="sync_secret")

    mock_parse.assert_called_once_with(b"encrypted pdf content", max_workers=1, password="sync_secret")


def test_extract_pdf_metadata_sync_parsing_error() -> None:
    with patch("kreuzberg._playa.parse", side_effect=Exception("Sync parsing failed")):
        with pytest.raises(ParsingError) as exc_info:
            extract_pdf_metadata_sync(b"invalid pdf content")

    assert "Failed to extract PDF metadata" in str(exc_info.value)
    assert "Sync parsing failed" in str(exc_info.value)


@pytest.mark.anyio
async def test_extract_pdf_metadata_complex_structure() -> None:
    mock_child = Mock()
    mock_child.language = "fr"
    mock_child.children = None

    mock_element = Mock()
    mock_element.language = "en"
    mock_element.role = "P"
    mock_element.children = [mock_child]

    mock_document = Mock()
    mock_document.info = [{"title": b"Complex Document"}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = [mock_element]

    with (
        patch("kreuzberg._playa.parse", return_value=mock_document),
        patch("kreuzberg._playa.asobj", return_value={"title": b"Complex Document"}),
        patch("kreuzberg._playa.decode_text", side_effect=lambda x: x.decode("utf-8") if isinstance(x, bytes) else x),
    ):
        result = await extract_pdf_metadata(b"complex pdf")

    assert "languages" in result
    assert "en" in result["languages"]
    assert "fr" in result["languages"]


@pytest.mark.anyio
async def test_extract_pdf_metadata_document_permissions() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = True
    mock_document.is_modifiable = False
    mock_document.is_extractable = True
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with patch("kreuzberg._playa.parse", return_value=mock_document), patch("kreuzberg._playa.asobj", return_value={}):
        result = await extract_pdf_metadata(b"pdf with permissions")

    assert "summary" in result
    assert "printable" in result["summary"]
    assert "extractable" in result["summary"]


@pytest.mark.anyio
async def test_extract_pdf_metadata_encrypted_document() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = True
    mock_document.encryption_method = "AES-256"
    mock_document.is_pdf_a = None
    mock_document.pdf_a_level = None

    with patch("kreuzberg._playa.parse", return_value=mock_document), patch("kreuzberg._playa.asobj", return_value={}):
        result = await extract_pdf_metadata(b"encrypted pdf")

    assert "summary" in result
    assert "encrypted" in result["summary"]
    assert "AES-256" in result["summary"]


@pytest.mark.anyio
async def test_extract_pdf_metadata_pdf_a_compliant() -> None:
    mock_document = Mock()
    mock_document.info = [{}]
    mock_document.pages = []
    mock_document.outline = None
    mock_document.structure = None
    mock_document.status = None
    mock_document.is_printable = None
    mock_document.is_modifiable = None
    mock_document.is_extractable = None
    mock_document.is_encrypted = None
    mock_document.encryption_method = None
    mock_document.is_pdf_a = True
    mock_document.pdf_a_level = "2b"

    with patch("kreuzberg._playa.parse", return_value=mock_document), patch("kreuzberg._playa.asobj", return_value={}):
        result = await extract_pdf_metadata(b"pdf/a document")

    assert "summary" in result
    assert "PDF/A-2b compliant" in result["summary"]
