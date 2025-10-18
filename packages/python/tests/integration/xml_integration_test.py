from pathlib import Path

import pytest

from kreuzberg import extract_bytes_sync, extract_file_sync

TEST_DOCUMENTS_DIR = Path(__file__).parent.parent.parent / "test_documents" / "xml"


def test_extract_xml_via_bytes_sync() -> None:
    xml_content = b"""<?xml version="1.0"?>
    <library>
        <book>
            <title>Test Book</title>
            <author>Test Author</author>
        </book>
    </library>"""

    result = extract_bytes_sync(xml_content, mime_type="application/xml")

    assert "Test Book" in result.content
    assert "Test Author" in result.content
    assert result.metadata["element_count"] > 0


def test_extract_xml_via_file_sync() -> None:
    test_file = TEST_DOCUMENTS_DIR / "simple_note.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extract_file_sync(test_file)

    assert "Tove" in result.content
    assert "Jani" in result.content
    assert "Reminder" in result.content
    assert result.metadata["element_count"] == 5


@pytest.mark.asyncio
async def test_extract_xml_via_bytes_async() -> None:
    from kreuzberg import extract_bytes

    xml_content = b"""<?xml version="1.0"?>
    <data>
        <item>Content 1</item>
        <item>Content 2</item>
    </data>"""

    result = await extract_bytes(xml_content, mime_type="application/xml")

    assert "Content 1" in result.content
    assert "Content 2" in result.content


@pytest.mark.asyncio
async def test_extract_xml_via_file_async() -> None:
    from kreuzberg import extract_file

    test_file = TEST_DOCUMENTS_DIR / "cd_catalog.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = await extract_file(test_file)

    assert result.content
    assert result.metadata["element_count"] > 10


def test_extract_svg_file() -> None:
    test_file = TEST_DOCUMENTS_DIR / "simple_svg.svg"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extract_file_sync(test_file)

    assert "Simple SVG Example" in result.content
    assert "Hello SVG" in result.content


def test_extract_rss_feed() -> None:
    test_file = TEST_DOCUMENTS_DIR / "rss_feed.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extract_file_sync(test_file)

    assert result.content
    assert len(result.content) > 0
