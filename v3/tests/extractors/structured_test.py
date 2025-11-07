import pytest
from kreuzberg import ExtractionConfig
from kreuzberg._extractors._structured import StructuredDataExtractor
from kreuzberg._mime_types import JSON_MIME_TYPE, TOML_MIME_TYPE, YAML_MIME_TYPE


def test_structured_supports_json_mime_type() -> None:
    assert StructuredDataExtractor.supports_mimetype(JSON_MIME_TYPE)
    assert StructuredDataExtractor.supports_mimetype("text/json")


def test_structured_supports_yaml_mime_type() -> None:
    assert StructuredDataExtractor.supports_mimetype(YAML_MIME_TYPE)
    assert StructuredDataExtractor.supports_mimetype("text/yaml")


def test_structured_supports_toml_mime_type() -> None:
    assert StructuredDataExtractor.supports_mimetype(TOML_MIME_TYPE)
    assert StructuredDataExtractor.supports_mimetype("text/toml")


def test_structured_extract_json_content() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"title": "Test Document", "content": "This is test content", "count": 42}'

    result = extractor.extract_bytes_sync(json_content)

    assert result.content
    assert "Test Document" in result.content
    assert "This is test content" in result.content
    assert "42" in result.content
    assert result.metadata["title"] == "Test Document"
    assert result.metadata["content"] == "This is test content"


def test_structured_extract_yaml_content() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(YAML_MIME_TYPE, config)

    yaml_content = b"""title: Test Config
description: This is a test configuration
items:
  - name: first item
  - name: second item
"""

    result = extractor.extract_bytes_sync(yaml_content)

    assert result.content
    assert "Test Config" in result.content
    assert "test configuration" in result.content
    assert "first item" in result.content
    assert result.metadata["title"] == "Test Config"
    assert result.metadata["description"] == "This is a test configuration"


def test_structured_extract_toml_content() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(TOML_MIME_TYPE, config)

    toml_content = b"""title = "Test Project"
description = "This is a test TOML configuration"

[database]
host = "localhost"
port = 5432

[[features]]
name = "authentication"
enabled = true
"""

    result = extractor.extract_bytes_sync(toml_content)

    assert result.content
    assert "Test Project" in result.content
    assert "test TOML configuration" in result.content
    assert "localhost" in result.content
    assert "authentication" in result.content
    assert result.metadata["title"] == "Test Project"
    assert result.metadata["description"] == "This is a test TOML configuration"


def test_structured_extract_invalid_json_fallback() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    invalid_json = b'{"invalid": json content'

    result = extractor.extract_bytes_sync(invalid_json)

    assert result.content
    assert "invalid" in result.content
    assert "parse_error" in result.metadata


@pytest.mark.anyio
async def test_structured_extract_bytes_async() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"title": "Async Test", "content": "Async content"}'
    result = await extractor.extract_bytes_async(json_content)

    assert result.content
    assert "Async Test" in result.content
    assert result.metadata["title"] == "Async Test"


@pytest.mark.anyio
async def test_structured_extract_path_async() -> None:
    import tempfile
    from pathlib import Path

    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        f.write('{"title": "Path Test", "description": "Path content"}')
        temp_path = Path(f.name)

    try:
        result = await extractor.extract_path_async(temp_path)
        assert "Path Test" in result.content
        assert result.metadata["title"] == "Path Test"
    finally:
        temp_path.unlink()


def test_structured_extract_path_sync() -> None:
    import tempfile
    from pathlib import Path

    config = ExtractionConfig()
    extractor = StructuredDataExtractor(YAML_MIME_TYPE, config)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
        f.write("title: Sync Path Test\ndescription: Sync path content")
        temp_path = Path(f.name)

    try:
        result = extractor.extract_path_sync(temp_path)
        assert "Sync Path Test" in result.content
        assert result.metadata["title"] == "Sync Path Test"
    finally:
        temp_path.unlink()


def test_structured_extract_toml_without_tomllib() -> None:
    import sys
    from unittest.mock import patch

    config = ExtractionConfig()
    extractor = StructuredDataExtractor(TOML_MIME_TYPE, config)

    toml_content = b'title = "No Tomllib Test"'

    with patch.object(sys.modules[extractor.__module__], "tomllib", None):
        result = extractor.extract_bytes_sync(toml_content)

    assert result.content
    assert "No Tomllib Test" in result.content
    assert "warning" in result.metadata
    assert "tomllib/tomli not available" in result.metadata["warning"]


def test_structured_extract_yaml_without_pyyaml() -> None:
    import sys
    from unittest.mock import patch

    config = ExtractionConfig()
    extractor = StructuredDataExtractor(YAML_MIME_TYPE, config)

    yaml_content = b"title: No PyYAML Test"

    with patch.object(sys.modules[extractor.__module__], "yaml", None):
        result = extractor.extract_bytes_sync(yaml_content)

    assert result.content
    assert "No PyYAML Test" in result.content
    assert "warning" in result.metadata
    assert "PyYAML not available" in result.metadata["warning"]


def test_structured_extract_list_data() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    list_content = b'[{"name": "Item 1", "value": 100}, {"name": "Item 2", "value": 200}]'
    result = extractor.extract_bytes_sync(list_content)

    assert result.content
    assert "Item 1" in result.content
    assert "Item 2" in result.content
    assert "100" in result.content
    assert "200" in result.content


def test_structured_extract_simple_string_data() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    simple_content = b'"This is a simple string"'
    result = extractor.extract_bytes_sync(simple_content)

    assert result.content
    assert "This is a simple string" in result.content


def test_structured_extract_simple_number_data() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    number_content = b"42"
    result = extractor.extract_bytes_sync(number_content)

    assert result.content
    assert "42" in result.content


def test_structured_extract_complex_nested_structure() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    complex_content = b"""{
        "title": "Complex Document",
        "sections": [
            {
                "name": "Introduction",
                "content": "This is the introduction text",
                "subsections": [
                    {"title": "Overview", "body": "Overview content"}
                ]
            },
            {
                "name": "Details",
                "description": "Detailed information"
            }
        ],
        "metadata": {
            "author": "Test Author",
            "version": 1.2
        }
    }"""

    result = extractor.extract_bytes_sync(complex_content)

    assert result.content
    assert "Complex Document" in result.content
    assert "Introduction" in result.content
    assert "introduction text" in result.content
    assert "Overview content" in result.content
    assert "Test Author" in result.content

    assert result.metadata["title"] == "Complex Document"


def test_structured_extract_nested_lists() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    nested_content = b"""[
        [
            {"name": "Nested Item 1"},
            {"name": "Nested Item 2"}
        ],
        [
            "Simple String",
            123,
            null
        ]
    ]"""

    result = extractor.extract_bytes_sync(nested_content)

    assert result.content
    assert "Nested Item 1" in result.content
    assert "Nested Item 2" in result.content
    assert "Simple String" in result.content
    assert "123" in result.content


def test_structured_extract_with_none_values() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    content_with_nulls = b"""{
        "title": "Test Document",
        "content": null,
        "items": [
            {"name": "Item 1", "value": null},
            {"name": "Item 2", "value": "Valid Value"}
        ]
    }"""

    result = extractor.extract_bytes_sync(content_with_nulls)

    assert result.content
    assert "Test Document" in result.content
    assert "Item 1" in result.content
    assert "Valid Value" in result.content
