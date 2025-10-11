from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig, JSONExtractionConfig
from kreuzberg._extractors._structured import StructuredDataExtractor
from kreuzberg._mime_types import JSON_MIME_TYPE

REAL_WORLD_JSON_DIR = Path(__file__).parent.parent.parent / "test_documents" / "json" / "real_world"


def test_json_config_default_values() -> None:
    config = JSONExtractionConfig()

    assert config.extract_schema is False
    assert config.custom_text_field_patterns is None
    assert config.max_depth == 10
    assert config.array_item_limit == 1000
    assert config.include_type_info is False
    assert config.flatten_nested_objects is True


def test_json_config_custom_values() -> None:
    config = JSONExtractionConfig(
        extract_schema=True,
        custom_text_field_patterns=frozenset({"summary", "note"}),
        max_depth=5,
        array_item_limit=500,
        include_type_info=True,
        flatten_nested_objects=False,
    )

    assert config.extract_schema is True
    assert config.custom_text_field_patterns == frozenset({"summary", "note"})
    assert config.max_depth == 5
    assert config.array_item_limit == 500
    assert config.include_type_info is True
    assert config.flatten_nested_objects is False


def test_extraction_config_with_json_config() -> None:
    json_config = JSONExtractionConfig(extract_schema=True)
    config = ExtractionConfig(json_extraction=json_config)

    assert config.json_extraction is not None
    assert config.json_extraction.extract_schema is True


def test_structured_extractor_json_config_property() -> None:
    json_config = JSONExtractionConfig(extract_schema=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    assert extractor._json_config is not None
    assert extractor._json_config.extract_schema is True


def test_structured_extractor_no_json_config() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    assert extractor._json_config is None


def test_include_type_info_feature() -> None:
    json_config = JSONExtractionConfig(include_type_info=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"string_val": "text", "int_val": 42, "float_val": 3.14, "bool_val": true}'
    result = extractor.extract_bytes_sync(json_content)

    assert "string_val (string)" in result.content
    assert "int_val (int)" in result.content
    assert "float_val (float)" in result.content
    assert "bool_val (bool)" in result.content


def test_include_type_info_disabled() -> None:
    json_config = JSONExtractionConfig(include_type_info=False)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"string_val": "text", "int_val": 42}'
    result = extractor.extract_bytes_sync(json_content)

    assert "(string)" not in result.content
    assert "(int)" not in result.content
    assert "string_val: text" in result.content
    assert "int_val: 42" in result.content


def test_flatten_nested_objects_enabled() -> None:
    json_config = JSONExtractionConfig(flatten_nested_objects=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"outer": {"inner": {"deep": "value"}}}'
    result = extractor.extract_bytes_sync(json_content)

    assert "outer.inner.deep: value" in result.content


def test_flatten_nested_objects_disabled() -> None:
    json_config = JSONExtractionConfig(flatten_nested_objects=False)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"outer": {"inner": {"deep": "value"}}}'
    result = extractor.extract_bytes_sync(json_content)

    assert "[nested object with" in result.content
    assert "outer.inner.deep" not in result.content


def test_custom_text_field_patterns() -> None:
    json_config = JSONExtractionConfig(custom_text_field_patterns=frozenset({"summary", "abstract"}))
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"summary": "Test Summary", "abstract": "Test Abstract", "other": "Not extracted"}'
    result = extractor.extract_bytes_sync(json_content)

    assert result.metadata.get("summary") == "Test Summary"
    assert result.metadata.get("abstract") == "Test Abstract"
    assert result.metadata.get("other") is None


def test_custom_text_field_patterns_with_nested() -> None:
    json_config = JSONExtractionConfig(custom_text_field_patterns=frozenset({"summary"}), flatten_nested_objects=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"data": {"summary": "Nested Summary", "info": {"summary": "Deep Summary"}}}'
    result = extractor.extract_bytes_sync(json_content)

    assert "attributes" in result.metadata
    assert result.metadata["attributes"].get("data.summary") == "Nested Summary"
    assert result.metadata["attributes"].get("data.info.summary") == "Deep Summary"


def test_schema_extraction_enabled() -> None:
    json_config = JSONExtractionConfig(extract_schema=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"name": "test", "value": 42, "nested": {"key": "val"}}'
    result = extractor.extract_bytes_sync(json_content)

    assert "json_schema" in result.metadata
    schema = result.metadata["json_schema"]
    assert schema["type"] == "dict"
    assert "properties" in schema
    assert "name" in schema["properties"]
    assert "value" in schema["properties"]
    assert "nested" in schema["properties"]


def test_schema_extraction_disabled() -> None:
    json_config = JSONExtractionConfig(extract_schema=False)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"name": "test", "value": 42}'
    result = extractor.extract_bytes_sync(json_content)

    assert "json_schema" not in result.metadata


def test_schema_extraction_max_depth() -> None:
    json_config = JSONExtractionConfig(extract_schema=True, max_depth=2)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"level1": {"level2": {"level3": {"level4": "deep"}}}}'
    result = extractor.extract_bytes_sync(json_content)

    schema = result.metadata["json_schema"]
    level2 = schema["properties"]["level1"]["properties"]["level2"]
    assert level2.get("max_depth_reached") is True


def test_array_item_limit() -> None:
    json_config = JSONExtractionConfig(extract_schema=True, array_item_limit=3)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"items": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]}'
    result = extractor.extract_bytes_sync(json_content)

    schema = result.metadata["json_schema"]
    assert schema["properties"]["items"]["truncated"] is True
    assert schema["properties"]["items"]["length"] == 10


def test_combined_features() -> None:
    json_config = JSONExtractionConfig(
        extract_schema=True,
        include_type_info=True,
        custom_text_field_patterns=frozenset({"abstract"}),
        flatten_nested_objects=True,
    )
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b"""{
        "title": "Test Document",
        "abstract": "Document Abstract",
        "count": 42,
        "nested": {
            "description": "Nested Description"
        }
    }"""
    result = extractor.extract_bytes_sync(json_content)

    assert "(string)" in result.content
    assert "(int)" in result.content

    assert result.metadata.get("title") == "Test Document"

    assert result.metadata.get("abstract") == "Document Abstract"

    assert "attributes" in result.metadata
    assert result.metadata["attributes"].get("nested.description") == "Nested Description"

    assert "json_schema" in result.metadata


def test_default_text_field_patterns() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    json_content = b'{"title": "Test Title", "content": "Test Content", "body": "Test Body"}'
    result = extractor.extract_bytes_sync(json_content)

    assert result.metadata.get("title") == "Test Title"
    assert result.metadata.get("content") == "Test Content"
    assert result.metadata.get("body") == "Test Body"


def test_iss_location_json() -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    test_file = REAL_WORLD_JSON_DIR / "iss_location.json"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert "iss_position" in result.content
    assert "longitude" in result.content
    assert "latitude" in result.content
    assert result.metadata.get("message") == "success"


def test_github_emojis_json() -> None:
    json_config = JSONExtractionConfig(extract_schema=True, max_depth=2)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    test_file = REAL_WORLD_JSON_DIR / "github_emojis.json"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    if "parse_error" not in result.metadata:
        assert "https://github.githubassets.com" in result.content
        assert "json_schema" in result.metadata
        assert result.metadata["json_schema"]["type"] == "dict"
    else:
        assert "https://github.githubassets.com" in result.content


def test_package_json() -> None:
    json_config = JSONExtractionConfig(
        extract_schema=True,
        flatten_nested_objects=False,
    )
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    test_file = REAL_WORLD_JSON_DIR / "package.json"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert "example-package" in result.content
    assert result.metadata.get("name") == "example-package"
    assert result.metadata.get("description") == "A sample package.json for testing"
    assert "[nested object" in result.content


def test_openapi_spec_json() -> None:
    json_config = JSONExtractionConfig(
        extract_schema=True, max_depth=3, custom_text_field_patterns=frozenset({"summary", "operationId"})
    )
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    test_file = REAL_WORLD_JSON_DIR / "openapi_spec.json"
    if not test_file.exists():
        pytest.skip(f"Test file {test_file} not found")

    result = extractor.extract_path_sync(test_file)

    assert result.content
    assert "Sample API" in result.content
    assert "3.0.0" in result.content

    assert "attributes" in result.metadata
    attrs = result.metadata["attributes"]

    assert attrs.get("info.title") == "Sample API"
    assert attrs.get("info.description") == "This is a sample API specification"

    assert "json_schema" in result.metadata
    schema = result.metadata["json_schema"]
    assert schema["type"] == "dict"
    assert "openapi" in schema["properties"]
    assert "info" in schema["properties"]


def test_large_json_array_handling() -> None:
    json_config = JSONExtractionConfig(
        extract_schema=True,
        array_item_limit=5,
    )
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    large_array_json = (
        b'{"data": [' + b",".join(f'{{"id": {i}, "value": "item_{i}"}}'.encode() for i in range(100)) + b"]}"
    )

    result = extractor.extract_bytes_sync(large_array_json)

    assert result.content
    if "json_schema" in result.metadata:
        schema = result.metadata["json_schema"]
        assert schema["properties"]["data"]["truncated"] is True
        assert schema["properties"]["data"]["length"] == 100


def test_deeply_nested_json() -> None:
    json_config = JSONExtractionConfig(extract_schema=True, max_depth=3, flatten_nested_objects=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    nested_json = b"""{
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "level5": {
                            "deep_value": "very deep content"
                        }
                    }
                }
            }
        }
    }"""

    result = extractor.extract_bytes_sync(nested_json)

    assert result.content
    assert "level1.level2.level3" in result.content
    if "json_schema" in result.metadata:
        schema = result.metadata["json_schema"]
        level3 = schema["properties"]["level1"]["properties"]["level2"]["properties"]["level3"]
        assert level3.get("max_depth_reached") is True


def test_mixed_type_json() -> None:
    json_config = JSONExtractionConfig(include_type_info=True, extract_schema=True)
    config = ExtractionConfig(json_extraction=json_config)
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    mixed_json = b"""{
        "string_val": "text",
        "int_val": 42,
        "float_val": 3.14,
        "bool_val": true,
        "null_val": null,
        "array_val": [1, "two", 3.0, false, null],
        "object_val": {"nested": "value"}
    }"""

    result = extractor.extract_bytes_sync(mixed_json)

    assert result.content
    assert "string_val (string)" in result.content
    assert "int_val (int)" in result.content
    assert "float_val (float)" in result.content
    assert "bool_val (bool)" in result.content
    assert "json_schema" in result.metadata
    schema = result.metadata["json_schema"]
    assert schema["properties"]["string_val"]["type"] == "str"
    assert schema["properties"]["int_val"]["type"] == "int"
    assert schema["properties"]["float_val"]["type"] == "float"
    assert schema["properties"]["bool_val"]["type"] == "bool"


@pytest.mark.parametrize(
    "filename,expected_content",
    [
        ("iss_location.json", "iss_position"),
        ("package.json", "dependencies"),
        ("aws_policy.json", "Statement"),
        ("openapi_spec.json", "openapi"),
    ],
)
def test_real_world_files_exist_and_parse(filename: str, expected_content: str) -> None:
    config = ExtractionConfig()
    extractor = StructuredDataExtractor(JSON_MIME_TYPE, config)

    test_file = REAL_WORLD_JSON_DIR / filename
    if test_file.exists():
        result = extractor.extract_path_sync(test_file)
        assert result.content
        assert expected_content in result.content
        assert "parse_error" not in result.metadata
