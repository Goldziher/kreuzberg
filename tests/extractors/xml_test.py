from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._extractors._xml import XMLExtractor
from kreuzberg._mime_types import SVG_MIME_TYPE, XML_MIME_TYPE, XML_TEXT_MIME_TYPE

TEST_DOCUMENTS_DIR = Path(__file__).parent.parent.parent / "test_documents" / "xml"


def test_xml_extractor_supports_mime_types() -> None:
    assert XMLExtractor.supports_mimetype(XML_MIME_TYPE)
    assert XMLExtractor.supports_mimetype(XML_TEXT_MIME_TYPE)
    assert XMLExtractor.supports_mimetype(SVG_MIME_TYPE)
    assert not XMLExtractor.supports_mimetype("application/json")


def test_simple_xml_extraction_sync() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root><item>Hello</item><item>World</item></root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content == "Hello World"
    assert result.mime_type == XML_MIME_TYPE
    assert result.metadata["element_count"] == 3
    assert result.metadata["unique_elements"] == 2


@pytest.mark.anyio
async def test_simple_xml_extraction_async() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root><item>Hello</item><item>World</item></root>"
    result = await extractor.extract_bytes_async(xml_content)

    assert result.content == "Hello World"
    assert result.mime_type == XML_MIME_TYPE
    assert result.metadata["element_count"] == 3
    assert result.metadata["unique_elements"] == 2


def test_xml_with_nested_elements() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"""
    <document>
        <section>
            <title>Introduction</title>
            <paragraph>This is the first paragraph.</paragraph>
        </section>
        <section>
            <title>Conclusion</title>
            <paragraph>This is the conclusion.</paragraph>
        </section>
    </document>
    """
    result = extractor.extract_bytes_sync(xml_content)

    assert "Introduction" in result.content
    assert "first paragraph" in result.content
    assert "Conclusion" in result.content
    assert "conclusion" in result.content


def test_xml_with_attributes_ignored() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b'<root><item id="1" name="test">Content</item></root>'
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content == "Content"
    assert "id" not in result.content
    assert "test" not in result.content
    assert result.metadata["element_count"] == 2


def test_xml_with_cdata() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root><![CDATA[Special <characters> & data]]></root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert "Special <characters> & data" in result.content


def test_xml_with_special_characters() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root><item>Text with &lt;brackets&gt; and &amp; symbols</item></root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert "<brackets>" in result.content
    assert "& symbols" in result.content


def test_empty_xml() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root></root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content == ""
    assert result.metadata["element_count"] == 1


def test_xml_whitespace_handling() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root>  <item>  Text with spaces  </item>  </root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content == "Text with spaces"


def test_malformed_xml_lenient_parsing() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root><item>Unclosed<item2>Content</root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content
    assert "Content" in result.content


def test_xml_with_namespaces() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"""
    <root xmlns:custom="http://example.com/ns">
        <custom:item>Namespaced Content</custom:item>
        <item>Regular Content</item>
    </root>
    """
    result = extractor.extract_bytes_sync(xml_content)

    assert "Namespaced Content" in result.content
    assert "Regular Content" in result.content


def test_large_xml_content() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    items = "".join(f"<item>Content {i}</item>" for i in range(1000))
    xml_content = f"<root>{items}</root>".encode()

    result = extractor.extract_bytes_sync(xml_content)

    assert result.content
    assert result.metadata["element_count"] == 1001
    assert "Content 0" in result.content
    assert "Content 999" in result.content


def test_xml_with_mixed_content() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"""
    <article>
        <title>Article Title</title>
        <author>
            <name>John Doe</name>
            <email>john@example.com</email>
        </author>
        <content>
            <paragraph>First paragraph with <emphasis>emphasis</emphasis>.</paragraph>
            <paragraph>Second paragraph.</paragraph>
        </content>
    </article>
    """
    result = extractor.extract_bytes_sync(xml_content)

    assert "Article Title" in result.content
    assert "John Doe" in result.content
    assert "john@example.com" in result.content
    assert "First paragraph" in result.content
    assert "emphasis" in result.content


def test_xml_comments_ignored() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"""
    <root>
        <!-- This is a comment -->
        <item>Visible content</item>
        <!-- Another comment -->
    </root>
    """
    result = extractor.extract_bytes_sync(xml_content)

    assert "Visible content" in result.content
    assert "This is a comment" not in result.content
    assert "Another comment" not in result.content


def test_xml_processing_instructions_ignored() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"""<?xml version="1.0" encoding="UTF-8"?>
    <?xml-stylesheet type="text/xsl" href="style.xsl"?>
    <root>
        <item>Content</item>
    </root>
    """
    result = extractor.extract_bytes_sync(xml_content)

    assert "Content" in result.content
    assert "xml-stylesheet" not in result.content


def test_completely_invalid_xml_lenient_handling() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"This is not XML at all, just plain text"
    result = extractor.extract_bytes_sync(xml_content)

    assert "This is not XML" in result.content
    assert result.metadata.get("element_count", 0) >= 0


def test_xml_path_extraction_sync(tmp_path: Path) -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_file = tmp_path / "test.xml"
    xml_file.write_bytes(b"<root><item>Test Content</item></root>")

    result = extractor.extract_path_sync(xml_file)

    assert result.content == "Test Content"
    assert result.metadata["element_count"] == 2


@pytest.mark.anyio
async def test_xml_path_extraction_async(tmp_path: Path) -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_file = tmp_path / "test.xml"
    xml_file.write_bytes(b"<root><item>Test Content</item></root>")

    result = await extractor.extract_path_async(xml_file)

    assert result.content == "Test Content"
    assert result.metadata["element_count"] == 2


def test_xml_unicode_content() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = """
    <root>
        <text>Hello 世界</text>
        <text>Привет мир</text>
        <text>مرحبا العالم</text>
    </root>
    """.encode()

    result = extractor.extract_bytes_sync(xml_content)

    assert "世界" in result.content
    assert "Привет мир" in result.content
    assert "مرحبا العالم" in result.content


def test_xml_empty_elements() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root><empty/><item>Content</item><empty/></root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content == "Content"
    assert result.metadata["element_count"] == 4


def test_text_xml_mime_type() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_TEXT_MIME_TYPE, config)

    xml_content = b"<root><item>Content</item></root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert result.content == "Content"
    assert result.mime_type == XML_TEXT_MIME_TYPE


@pytest.mark.parametrize(
    "xml_content,expected_text",
    [
        (b"<root>Simple</root>", "Simple"),
        (b"<a><b><c>Nested</c></b></a>", "Nested"),
        (b"<root><item>A</item><item>B</item></root>", "A B"),
        (b"<root>Text &amp; more</root>", "Text & more"),
        (b"<root><![CDATA[CDATA content]]></root>", "CDATA content"),
    ],
)
def test_xml_extraction_patterns(xml_content: bytes, expected_text: str) -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    result = extractor.extract_bytes_sync(xml_content)

    assert expected_text in result.content


def test_xml_extractor_quality_processing() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    xml_content = b"<root>   Multiple    spaces   between   words   </root>"
    result = extractor.extract_bytes_sync(xml_content)

    assert "Multiple spaces between words" in result.content
    assert "    " not in result.content


def test_real_world_simple_note() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    test_file = TEST_DOCUMENTS_DIR / "simple_note.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert "Tove" in result.content
    assert "Jani" in result.content
    assert "Reminder" in result.content
    assert "Don't forget me this weekend!" in result.content
    assert result.metadata["element_count"] == 5


def test_real_world_plant_catalog() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    test_file = TEST_DOCUMENTS_DIR / "plant_catalog.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert "Bloodroot" in result.content or "plant" in result.content.lower()
    assert result.metadata["element_count"] > 10


def test_real_world_cd_catalog() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    test_file = TEST_DOCUMENTS_DIR / "cd_catalog.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert "Empire Burlesque" in result.content or "CD" in result.content or "TITLE" in result.content
    assert result.metadata["element_count"] > 20


def test_real_world_rss_feed() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(XML_MIME_TYPE, config)

    test_file = TEST_DOCUMENTS_DIR / "rss_feed.xml"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert len(result.content) > 0
    assert result.metadata["element_count"] > 5


def test_real_world_svg() -> None:
    config = ExtractionConfig()
    extractor = XMLExtractor(SVG_MIME_TYPE, config)

    test_file = TEST_DOCUMENTS_DIR / "simple_svg.svg"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert "Simple SVG Example" in result.content
    assert "Hello SVG" in result.content
    assert "basic shapes" in result.content
    assert result.metadata["element_count"] >= 5


@pytest.mark.parametrize(
    "filename,expected_keywords",
    [
        ("simple_note.xml", ["Tove", "Reminder"]),
        ("cd_catalog.xml", ["Empire Burlesque"]),
        ("plant_catalog.xml", ["Bloodroot"]),
        ("simple_svg.svg", ["Simple SVG", "Hello SVG"]),
    ],
)
def test_real_world_files_extraction(filename: str, expected_keywords: list[str]) -> None:
    config = ExtractionConfig()
    mime_type = SVG_MIME_TYPE if filename.endswith(".svg") else XML_MIME_TYPE
    extractor = XMLExtractor(mime_type, config)

    test_file = TEST_DOCUMENTS_DIR / filename
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert any(keyword in result.content for keyword in expected_keywords)
    assert result.metadata["element_count"] > 0
    assert result.metadata["unique_elements"] > 0
