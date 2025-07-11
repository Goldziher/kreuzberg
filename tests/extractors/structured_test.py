from kreuzberg import ExtractionConfig
from kreuzberg._extractors._structured import StructuredDataExtractor
from kreuzberg._mime_types import JSON_MIME_TYPE, TOML_MIME_TYPE, YAML_MIME_TYPE


class TestStructuredDataExtractor:
    def test_supports_json_mime_type(self) -> None:
        assert StructuredDataExtractor.supports_mimetype(JSON_MIME_TYPE)
        assert StructuredDataExtractor.supports_mimetype("text/json")

    def test_supports_yaml_mime_type(self) -> None:
        assert StructuredDataExtractor.supports_mimetype(YAML_MIME_TYPE)
        assert StructuredDataExtractor.supports_mimetype("text/yaml")

    def test_supports_toml_mime_type(self) -> None:
        assert StructuredDataExtractor.supports_mimetype(TOML_MIME_TYPE)
        assert StructuredDataExtractor.supports_mimetype("text/toml")

    def test_extract_json_content(self) -> None:
        config = ExtractionConfig()
        extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

        json_content = b'{"title": "Test Document", "content": "This is test content", "count": 42}'

        result = extractor.extract_bytes_sync(json_content)

        assert result.content
        assert "Test Document" in result.content
        assert "This is test content" in result.content
        assert "42" in result.content
        assert result.metadata["title"] == "Test Document"
        # Content is now extracted as text, not metadata

    def test_extract_yaml_content(self) -> None:
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

    def test_extract_toml_content(self) -> None:
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

    def test_extract_invalid_json_fallback(self) -> None:
        config = ExtractionConfig()
        extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

        invalid_json = b'{"invalid": json content'

        # Should gracefully handle invalid JSON and return raw text
        result = extractor.extract_bytes_sync(invalid_json)

        assert result.content
        assert "invalid" in result.content
        # Should fallback to plain text content without metadata
