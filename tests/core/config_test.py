from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig
from kreuzberg._config import (
    build_extraction_config_from_dict,
    discover_and_load_config,
    discover_config,
    find_config_file,
    find_default_config,
    load_config_from_file,
    load_config_from_path,
    load_default_config,
)
from kreuzberg._types import (
    ChunkingConfig,
    EntityExtractionConfig,
    KeywordExtractionConfig,
    LanguageDetectionConfig,
    TableExtractionConfig,
    TesseractConfig,
)
from kreuzberg.exceptions import ValidationError


def test_load_config_from_file_kreuzberg_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
force_ocr = true

[ocr]
backend = "tesseract"
language = "eng"

[tables]
detection_threshold = 0.7
""")
    config_dict = load_config_from_file(config_file)
    assert config_dict["force_ocr"] is True
    assert config_dict["ocr"]["backend"] == "tesseract"
    assert config_dict["ocr"]["language"] == "eng"
    assert config_dict["tables"]["detection_threshold"] == 0.7


def test_load_config_from_file_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[tool.kreuzberg]
force_ocr = true

[tool.kreuzberg.ocr]
backend = "tesseract"
language = "eng"
""")
    config_dict = load_config_from_file(config_file)
    assert config_dict["force_ocr"] is True
    assert config_dict["ocr"]["backend"] == "tesseract"


def test_build_extraction_config_from_dict_v4_format() -> None:
    config_dict = {
        "force_ocr": True,
        "ocr": {"backend": "tesseract", "language": "eng"},
        "tables": {"detection_threshold": 0.7},
        "chunking": {"max_chars": 1000, "max_overlap": 200},
    }
    config = build_extraction_config_from_dict(config_dict)
    assert config.force_ocr is True
    assert isinstance(config.ocr, TesseractConfig)
    assert config.ocr.language == "eng"
    assert isinstance(config.tables, TableExtractionConfig)
    assert config.tables.detection_threshold == 0.7
    assert isinstance(config.chunking, ChunkingConfig)
    assert config.chunking.max_chars == 1000


def test_build_extraction_config_from_dict_v3_ocr_backend_throws_error() -> None:
    config_dict = {"ocr_backend": "tesseract"}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_chunk_content_throws_error() -> None:
    config_dict = {"chunk_content": True}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_extract_tables_throws_error() -> None:
    config_dict = {"extract_tables": True}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_extract_keywords_throws_error() -> None:
    config_dict = {"extract_keywords": True}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_extract_entities_throws_error() -> None:
    config_dict = {"extract_entities": True}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_auto_detect_language_throws_error() -> None:
    config_dict = {"auto_detect_language": True}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_keyword_count_throws_error() -> None:
    config_dict = {"keyword_count": 5}
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        build_extraction_config_from_dict(config_dict)


def test_build_extraction_config_from_dict_v3_multiple_fields_throws_error() -> None:
    config_dict = {
        "ocr_backend": "tesseract",
        "chunk_content": True,
        "extract_tables": True,
    }
    with pytest.raises(ValidationError) as exc_info:
        build_extraction_config_from_dict(config_dict)
    assert "V3 configuration format detected" in str(exc_info.value)
    assert "ocr_backend" in exc_info.value.context["v3_fields_found"]
    assert "chunk_content" in exc_info.value.context["v3_fields_found"]
    assert "extract_tables" in exc_info.value.context["v3_fields_found"]


def test_build_extraction_config_from_dict_empty() -> None:
    config = build_extraction_config_from_dict({})
    assert isinstance(config, ExtractionConfig)
    assert isinstance(config.ocr, TesseractConfig)
    assert config.tables is None


def test_build_extraction_config_from_dict_with_all_features() -> None:
    config_dict = {
        "force_ocr": True,
        "ocr": {"backend": "tesseract", "language": "eng"},
        "tables": {"detection_threshold": 0.7},
        "chunking": {"max_chars": 1000},
        "keywords": {"count": 5},
        "entities": {},
        "language_detection": {},
    }
    config = build_extraction_config_from_dict(config_dict)
    assert config.force_ocr is True
    assert isinstance(config.ocr, TesseractConfig)
    assert isinstance(config.tables, TableExtractionConfig)
    assert isinstance(config.chunking, ChunkingConfig)
    assert isinstance(config.keywords, KeywordExtractionConfig)
    assert isinstance(config.entities, EntityExtractionConfig)
    assert isinstance(config.language_detection, LanguageDetectionConfig)


def test_find_config_file_kreuzberg_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")
    found = find_config_file(tmp_path)
    assert found == config_file


def test_find_config_file_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")
    found = find_config_file(tmp_path)
    assert found == config_file


def test_find_config_file_not_found(tmp_path: Path) -> None:
    found = find_config_file(tmp_path)
    assert found is None


def test_find_config_file_walks_up_directories(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")
    subdir = tmp_path / "sub" / "dir"
    subdir.mkdir(parents=True)
    found = find_config_file(subdir)
    assert found == config_file


def test_load_default_config_found(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
force_ocr = true

[ocr]
backend = "tesseract"
language = "eng"
""")
    config = load_default_config(tmp_path)
    assert config is not None
    assert config.force_ocr is True


def test_load_default_config_not_found(tmp_path: Path) -> None:
    config = load_default_config(tmp_path)
    assert config is None


def test_load_config_from_path(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("""
force_ocr = true

[ocr]
backend = "tesseract"
language = "eng"
""")
    config = load_config_from_path(config_file)
    assert config.force_ocr is True
    assert isinstance(config.ocr, TesseractConfig)


def test_discover_and_load_config_found(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
[ocr]
backend = "tesseract"
language = "eng"
""")
    config = discover_and_load_config(tmp_path)
    assert isinstance(config.ocr, TesseractConfig)


def test_discover_and_load_config_not_found(tmp_path: Path) -> None:
    with pytest.raises(ValidationError, match="No configuration file found"):
        discover_and_load_config(tmp_path)


def test_discover_config_found(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
[ocr]
backend = "tesseract"
language = "eng"
""")
    config = discover_config(tmp_path)
    assert config is not None
    assert isinstance(config.ocr, TesseractConfig)


def test_discover_config_not_found(tmp_path: Path) -> None:
    config = discover_config(tmp_path)
    assert config is None


def test_find_default_config() -> None:
    with tempfile.TemporaryDirectory():
        result = find_default_config()
        assert result is None or isinstance(result, Path)


def test_load_config_from_file_invalid_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("invalid toml {{{")
    with pytest.raises(ValidationError, match="Invalid TOML"):
        load_config_from_file(config_file)


def test_build_extraction_config_from_dict_unknown_fields_ignored() -> None:
    config_dict = {"ocr": {"backend": "tesseract", "invalid_field": "value"}}
    config = build_extraction_config_from_dict(config_dict)
    assert isinstance(config.ocr, TesseractConfig)


def test_load_config_from_path_v3_format_throws_error(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
ocr_backend = "tesseract"
chunk_content = true
""")
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        load_config_from_path(config_file)


def test_discover_config_v3_format_throws_error(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
ocr_backend = "tesseract"
extract_tables = true
""")
    with pytest.raises(ValidationError, match="V3 configuration format detected"):
        discover_config(tmp_path)
