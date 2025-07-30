"""Comprehensive tests for kreuzberg._config module."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Any
from unittest.mock import Mock, patch

import pytest

from kreuzberg._config import (
    _build_ocr_config_from_cli,
    _configure_gmft,
    _configure_ocr_backend,
    _merge_cli_args,
    _merge_file_config,
    build_extraction_config,
    build_extraction_config_from_dict,
    discover_and_load_config,
    find_config_file,
    find_default_config,
    load_config_from_file,
    load_config_from_path,
    load_default_config,
    merge_configs,
    parse_ocr_backend_config,
    try_discover_config,
)
from kreuzberg._gmft import GMFTConfig
from kreuzberg._ocr._easyocr import EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRConfig
from kreuzberg._ocr._tesseract import PSMMode, TesseractConfig
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import ValidationError


class TestLoadConfigFromFile:
    """Test load_config_from_file function."""

    def test_load_kreuzberg_toml(self, tmp_path: Path) -> None:
        """Test loading from kreuzberg.toml file."""
        config_path = tmp_path / "kreuzberg.toml"
        config_content = """
force_ocr = true
chunk_content = false
max_chars = 1000
ocr_backend = "tesseract"
"""
        config_path.write_text(config_content)

        result = load_config_from_file(config_path)

        assert result["force_ocr"] is True
        assert result["chunk_content"] is False
        assert result["max_chars"] == 1000
        assert result["ocr_backend"] == "tesseract"

    def test_load_pyproject_toml(self, tmp_path: Path) -> None:
        """Test loading from pyproject.toml file."""
        config_path = tmp_path / "pyproject.toml"
        config_content = """
[tool.kreuzberg]
force_ocr = true
extract_tables = true
max_chars = 2000
ocr_backend = "easyocr"
"""
        config_path.write_text(config_content)

        result = load_config_from_file(config_path)

        assert result["force_ocr"] is True
        assert result["extract_tables"] is True
        assert result["max_chars"] == 2000
        assert result["ocr_backend"] == "easyocr"

    def test_load_pyproject_toml_no_tool_section(self, tmp_path: Path) -> None:
        """Test loading from pyproject.toml without [tool.kreuzberg] section."""
        config_path = tmp_path / "pyproject.toml"
        config_content = """
[project]
name = "test"
"""
        config_path.write_text(config_content)

        result = load_config_from_file(config_path)

        assert result == {}

    def test_load_file_not_found(self, tmp_path: Path) -> None:
        """Test loading from non-existent file."""
        config_path = tmp_path / "nonexistent.toml"

        with pytest.raises(ValidationError, match="Configuration file not found"):
            load_config_from_file(config_path)

    def test_load_invalid_toml(self, tmp_path: Path) -> None:
        """Test loading from file with invalid TOML."""
        config_path = tmp_path / "invalid.toml"
        config_path.write_text("invalid toml content [")

        with pytest.raises(ValidationError, match="Invalid TOML in configuration file"):
            load_config_from_file(config_path)


class TestMergeConfigs:
    """Test merge_configs function."""

    def test_merge_simple_configs(self) -> None:
        """Test merging simple configuration dictionaries."""
        base = {"a": 1, "b": 2}
        override = {"b": 3, "c": 4}

        result = merge_configs(base, override)

        assert result == {"a": 1, "b": 3, "c": 4}

    def test_merge_nested_configs(self) -> None:
        """Test merging nested configuration dictionaries."""
        base = {"level1": {"a": 1, "b": 2}, "level2": {"x": 10}}
        override = {"level1": {"b": 3, "c": 4}, "level3": {"y": 20}}

        result = merge_configs(base, override)

        assert result == {
            "level1": {"a": 1, "b": 3, "c": 4},
            "level2": {"x": 10},
            "level3": {"y": 20},
        }

    def test_merge_non_dict_override(self) -> None:
        """Test merging when override contains non-dict values."""
        base = {"nested": {"a": 1, "b": 2}}
        override = {"nested": "string_value"}

        result = merge_configs(base, override)

        assert result == {"nested": "string_value"}

    def test_merge_empty_configs(self) -> None:
        """Test merging empty configurations."""
        base = {"a": 1}
        override: dict[str, Any] = {}

        result = merge_configs(base, override)

        assert result == {"a": 1}

        # Test reverse
        result = merge_configs({}, {"b": 2})
        assert result == {"b": 2}


class TestParseOcrBackendConfig:
    """Test parse_ocr_backend_config function."""

    def test_parse_tesseract_config(self) -> None:
        """Test parsing Tesseract configuration."""
        config_dict = {
            "tesseract": {
                "language": "eng+fra",
                "psm": 6,
                "tessedit_char_whitelist": "0123456789",
            }
        }

        result = parse_ocr_backend_config(config_dict, "tesseract")

        assert isinstance(result, TesseractConfig)
        assert result.language == "eng+fra"
        assert result.psm == PSMMode.SINGLE_BLOCK
        assert result.tessedit_char_whitelist == "0123456789"

    def test_parse_tesseract_config_psm_enum(self) -> None:
        """Test parsing Tesseract config with PSM as enum."""
        config_dict = {
            "tesseract": {
                "language": "eng",
                "psm": PSMMode.SINGLE_LINE,
            }
        }

        result = parse_ocr_backend_config(config_dict, "tesseract")

        assert isinstance(result, TesseractConfig)
        assert result.psm == PSMMode.SINGLE_LINE

    def test_parse_easyocr_config(self) -> None:
        """Test parsing EasyOCR configuration."""
        config_dict = {
            "easyocr": {
                "language": "en",
                "canvas_size": 2048,
            }
        }

        result = parse_ocr_backend_config(config_dict, "easyocr")

        assert isinstance(result, EasyOCRConfig)
        assert result.language == "en"
        assert result.canvas_size == 2048

    def test_parse_paddleocr_config(self) -> None:
        """Test parsing PaddleOCR configuration."""
        config_dict = {
            "paddleocr": {
                "language": "ch",
                "det_db_thresh": 0.4,
            }
        }

        result = parse_ocr_backend_config(config_dict, "paddleocr")

        assert isinstance(result, PaddleOCRConfig)
        assert result.language == "ch"
        assert result.det_db_thresh == 0.4

    def test_parse_backend_not_in_config(self) -> None:
        """Test parsing when backend is not in config."""
        config_dict: dict[str, Any] = {"other_backend": {}}

        result = parse_ocr_backend_config(config_dict, "tesseract")

        assert result is None

    def test_parse_backend_config_not_dict(self) -> None:
        """Test parsing when backend config is not a dictionary."""
        config_dict = {"tesseract": "not_a_dict"}

        result = parse_ocr_backend_config(config_dict, "tesseract")

        assert result is None


class TestBuildExtractionConfigFromDict:
    """Test build_extraction_config_from_dict function."""

    def test_build_basic_config(self) -> None:
        """Test building basic extraction config."""
        config_dict = {
            "force_ocr": True,
            "chunk_content": False,
            "max_chars": 1000,
            "max_overlap": 100,
            "ocr_backend": "tesseract",
        }

        result = build_extraction_config_from_dict(config_dict)

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True
        assert result.chunk_content is False
        assert result.max_chars == 1000
        assert result.max_overlap == 100
        assert result.ocr_backend == "tesseract"

    def test_build_config_with_ocr_backend_config(self) -> None:
        """Test building config with OCR backend configuration."""
        config_dict = {
            "ocr_backend": "tesseract",
            "tesseract": {
                "language": "eng",
                "psm": 6,
            },
        }

        result = build_extraction_config_from_dict(config_dict)

        assert result.ocr_backend == "tesseract"
        assert isinstance(result.ocr_config, TesseractConfig)
        assert result.ocr_config.language == "eng"

    def test_build_config_with_gmft(self) -> None:
        """Test building config with GMFT configuration."""
        config_dict = {
            "extract_tables": True,
            "gmft": {
                "verbosity": 1,
                "detector_base_threshold": 0.8,
            },
        }

        result = build_extraction_config_from_dict(config_dict)

        assert result.extract_tables is True
        assert isinstance(result.gmft_config, GMFTConfig)
        assert result.gmft_config.verbosity == 1
        assert result.gmft_config.detector_base_threshold == 0.8

    def test_build_config_ocr_backend_none(self) -> None:
        """Test building config with ocr_backend set to 'none'."""
        config_dict = {"ocr_backend": "none"}

        result = build_extraction_config_from_dict(config_dict)

        assert result.ocr_backend is None

    def test_build_config_partial_fields(self) -> None:
        """Test building config with only some fields."""
        config_dict = {"extract_entities": True, "extract_keywords": True}

        result = build_extraction_config_from_dict(config_dict)

        assert result.extract_entities is True
        assert result.extract_keywords is True
        # Other fields should have defaults


class TestFindConfigFile:
    """Test find_config_file function."""

    def test_find_kreuzberg_toml(self, tmp_path: Path) -> None:
        """Test finding kreuzberg.toml file."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = find_config_file(tmp_path)

        assert result == config_file

    def test_find_pyproject_toml_with_kreuzberg_section(self, tmp_path: Path) -> None:
        """Test finding pyproject.toml with [tool.kreuzberg] section."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")

        result = find_config_file(tmp_path)

        assert result == config_file

    def test_find_pyproject_toml_without_kreuzberg_section(self, tmp_path: Path) -> None:
        """Test pyproject.toml without [tool.kreuzberg] section is ignored."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[project]
name = "test"
""")

        result = find_config_file(tmp_path)

        assert result is None

    def test_find_prefers_kreuzberg_toml(self, tmp_path: Path) -> None:
        """Test that kreuzberg.toml is preferred over pyproject.toml."""
        kreuzberg_toml = tmp_path / "kreuzberg.toml"
        pyproject_toml = tmp_path / "pyproject.toml"

        kreuzberg_toml.write_text("force_ocr = true")
        pyproject_toml.write_text("""
[tool.kreuzberg]
force_ocr = false
""")

        result = find_config_file(tmp_path)

        assert result == kreuzberg_toml

    def test_find_config_searches_up_tree(self, tmp_path: Path) -> None:
        """Test that config file search goes up the directory tree."""
        parent_dir = tmp_path / "parent"
        child_dir = parent_dir / "child"
        child_dir.mkdir(parents=True)

        config_file = parent_dir / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = find_config_file(child_dir)

        assert result == config_file

    def test_find_config_no_file_found(self, tmp_path: Path) -> None:
        """Test when no config file is found."""
        result = find_config_file(tmp_path)

        assert result is None

    def test_find_config_default_start_path(self) -> None:
        """Test find_config_file with default start path."""
        with patch("pathlib.Path.cwd") as mock_cwd:
            mock_cwd.return_value = Path("/fake/path")

            # Mock the file system traversal
            with patch.object(Path, "exists", return_value=False):
                result = find_config_file()
                assert result is None

    def test_find_config_invalid_pyproject_toml(self, tmp_path: Path) -> None:
        """Test handling invalid pyproject.toml file."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("invalid toml [")

        result = find_config_file(tmp_path)

        assert result is None


class TestLoadDefaultConfig:
    """Test load_default_config function."""

    def test_load_default_config_success(self, tmp_path: Path) -> None:
        """Test successful loading of default config."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("""
force_ocr = true
chunk_content = false
""")

        result = load_default_config(tmp_path)

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True
        assert result.chunk_content is False

    def test_load_default_config_no_file(self, tmp_path: Path) -> None:
        """Test loading when no config file exists."""
        result = load_default_config(tmp_path)

        assert result is None

    def test_load_default_config_empty_file(self, tmp_path: Path) -> None:
        """Test loading when config file is empty."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("")

        result = load_default_config(tmp_path)

        assert result is None

    def test_load_default_config_invalid_file(self, tmp_path: Path) -> None:
        """Test loading when config file is invalid."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("invalid content [")

        result = load_default_config(tmp_path)

        assert result is None


class TestLoadConfigFromPath:
    """Test load_config_from_path function."""

    def test_load_config_from_path_success(self, tmp_path: Path) -> None:
        """Test successful loading from specific path."""
        config_file = tmp_path / "pyproject.toml"
        config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
max_chars = 2000
ocr_backend = "tesseract"
""")

        result = load_config_from_path(config_file)

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True
        assert result.max_chars == 2000
        assert result.ocr_backend == "tesseract"

    def test_load_config_from_path_string(self, tmp_path: Path) -> None:
        """Test loading from string path."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = load_config_from_path(str(config_file))

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True

    def test_load_config_from_path_not_found(self, tmp_path: Path) -> None:
        """Test loading from non-existent path."""
        config_file = tmp_path / "nonexistent.toml"

        with pytest.raises(ValidationError, match="Configuration file not found"):
            load_config_from_path(config_file)


class TestDiscoverAndLoadConfig:
    """Test discover_and_load_config function."""

    def test_discover_and_load_config_success(self, tmp_path: Path) -> None:
        """Test successful discovery and loading."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = discover_and_load_config(tmp_path)

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True

    def test_discover_and_load_config_string_path(self, tmp_path: Path) -> None:
        """Test discovery with string path."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = discover_and_load_config(str(tmp_path))

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True

    def test_discover_and_load_config_no_file(self, tmp_path: Path) -> None:
        """Test discovery when no config file found."""
        with pytest.raises(ValidationError, match="No configuration file found"):
            discover_and_load_config(tmp_path)

    def test_discover_and_load_config_empty_file(self, tmp_path: Path) -> None:
        """Test discovery when config file is empty."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("")

        with pytest.raises(ValidationError, match="contains no Kreuzberg configuration"):
            discover_and_load_config(tmp_path)


class TestTryDiscoverConfig:
    """Test try_discover_config function."""

    def test_try_discover_config_success(self, tmp_path: Path) -> None:
        """Test successful discovery."""
        config_file = tmp_path / "kreuzberg.toml"
        config_file.write_text("force_ocr = true")

        result = try_discover_config(tmp_path)

        assert isinstance(result, ExtractionConfig)
        assert result.force_ocr is True

    def test_try_discover_config_no_file(self, tmp_path: Path) -> None:
        """Test discovery when no file found returns None."""
        result = try_discover_config(tmp_path)

        assert result is None


class TestLegacyFunctions:
    """Test legacy configuration functions."""

    def test_merge_file_config(self) -> None:
        """Test _merge_file_config function."""
        config_dict: dict[str, Any] = {"existing": "value"}
        file_config: dict[str, Any] = {
            "force_ocr": True,
            "chunk_content": False,
            "unknown_field": "ignored",
        }

        _merge_file_config(config_dict, file_config)

        assert config_dict["force_ocr"] is True
        assert config_dict["chunk_content"] is False
        assert "unknown_field" not in config_dict
        assert config_dict["existing"] == "value"

    def test_merge_file_config_empty(self) -> None:
        """Test _merge_file_config with empty file config."""
        config_dict: dict[str, Any] = {"existing": "value"}

        _merge_file_config(config_dict, {})

        assert config_dict == {"existing": "value"}

    def test_merge_cli_args(self) -> None:
        """Test _merge_cli_args function."""
        config_dict: dict[str, Any] = {}
        cli_args: dict[str, Any] = {
            "force_ocr": True,
            "chunk_content": None,  # Should be ignored
            "extract_tables": False,
            "unknown_field": "ignored",
        }

        _merge_cli_args(config_dict, cli_args)

        assert config_dict["force_ocr"] is True
        assert "chunk_content" not in config_dict
        assert config_dict["extract_tables"] is False
        assert "unknown_field" not in config_dict

    def test_build_ocr_config_from_cli_tesseract(self) -> None:
        """Test building Tesseract config from CLI."""
        cli_args: dict[str, Any] = {
            "tesseract_config": {
                "language": "eng+fra",
                "psm": PSMMode.SINGLE_BLOCK,  # Pass as enum directly
            }
        }

        result = _build_ocr_config_from_cli("tesseract", cli_args)

        assert isinstance(result, TesseractConfig)
        assert result.language == "eng+fra"
        assert result.psm == PSMMode.SINGLE_BLOCK

    def test_build_ocr_config_from_cli_easyocr(self) -> None:
        """Test building EasyOCR config from CLI."""
        cli_args: dict[str, Any] = {
            "easyocr_config": {
                "language": "en",
                "canvas_size": 1024,
            }
        }

        result = _build_ocr_config_from_cli("easyocr", cli_args)

        assert isinstance(result, EasyOCRConfig)
        assert result.language == "en"
        assert result.canvas_size == 1024

    def test_build_ocr_config_from_cli_paddleocr(self) -> None:
        """Test building PaddleOCR config from CLI."""
        cli_args: dict[str, Any] = {
            "paddleocr_config": {
                "language": "en",
                "det_db_thresh": 0.5,
            }
        }

        result = _build_ocr_config_from_cli("paddleocr", cli_args)

        assert isinstance(result, PaddleOCRConfig)
        assert result.language == "en"
        assert result.det_db_thresh == 0.5

    def test_build_ocr_config_from_cli_no_config(self) -> None:
        """Test building OCR config when no config in CLI args."""
        cli_args: dict[str, Any] = {}

        result = _build_ocr_config_from_cli("tesseract", cli_args)

        assert result is None

    def test_configure_ocr_backend(self) -> None:
        """Test _configure_ocr_backend function."""
        config_dict = {"ocr_backend": "tesseract"}
        file_config: dict[str, Any] = {
            "tesseract": {
                "language": "eng",
                "psm": 6,
            }
        }
        cli_args: dict[str, Any] = {}

        _configure_ocr_backend(config_dict, file_config, cli_args)

        assert "ocr_config" in config_dict
        assert isinstance(config_dict["ocr_config"], TesseractConfig)

    def test_configure_ocr_backend_cli_priority(self) -> None:
        """Test that CLI config takes priority over file config."""
        config_dict = {"ocr_backend": "tesseract"}
        file_config: dict[str, Any] = {"tesseract": {"language": "eng"}}
        cli_args: dict[str, Any] = {"tesseract_config": {"language": "fra"}}

        _configure_ocr_backend(config_dict, file_config, cli_args)

        assert isinstance(config_dict["ocr_config"], TesseractConfig)
        assert config_dict["ocr_config"].language == "fra"

    def test_configure_ocr_backend_none(self) -> None:
        """Test OCR backend configuration when backend is None."""
        config_dict = {"ocr_backend": "none"}
        file_config: dict[str, Any] = {}
        cli_args: dict[str, Any] = {}

        _configure_ocr_backend(config_dict, file_config, cli_args)

        assert "ocr_config" not in config_dict

    def test_configure_gmft(self) -> None:
        """Test _configure_gmft function."""
        config_dict = {"extract_tables": True}
        file_config: dict[str, Any] = {
            "gmft": {
                "verbosity": 2,
                "detector_base_threshold": 0.8,
            }
        }
        cli_args: dict[str, Any] = {}

        _configure_gmft(config_dict, file_config, cli_args)

        assert "gmft_config" in config_dict
        assert isinstance(config_dict["gmft_config"], GMFTConfig)
        assert config_dict["gmft_config"].verbosity == 2

    def test_configure_gmft_cli_priority(self) -> None:
        """Test that CLI GMFT config takes priority."""
        config_dict = {"extract_tables": True}
        file_config: dict[str, Any] = {"gmft": {"detector_base_threshold": 0.5}}
        cli_args: dict[str, Any] = {"gmft_config": {"detector_base_threshold": 0.9}}

        _configure_gmft(config_dict, file_config, cli_args)

        assert isinstance(config_dict["gmft_config"], GMFTConfig)
        assert config_dict["gmft_config"].detector_base_threshold == 0.9

    def test_configure_gmft_no_extract_tables(self) -> None:
        """Test GMFT configuration when extract_tables is False."""
        config_dict = {"extract_tables": False}
        file_config: dict[str, Any] = {"gmft": {}}
        cli_args: dict[str, Any] = {}

        _configure_gmft(config_dict, file_config, cli_args)

        assert "gmft_config" not in config_dict

    def test_build_extraction_config_full(self) -> None:
        """Test building full extraction config from file and CLI."""
        file_config: dict[str, Any] = {
            "force_ocr": False,
            "extract_tables": True,
            "ocr_backend": "tesseract",
            "tesseract": {"language": "eng"},
            "gmft": {"detector_base_threshold": 0.5},
        }
        cli_args: dict[str, Any] = {
            "force_ocr": True,  # Should override file config
            "tesseract_config": {"language": "fra"},  # Should override file config
        }

        result = build_extraction_config(file_config, cli_args)

        assert result.force_ocr is True  # CLI override
        assert result.extract_tables is True  # From file
        assert result.ocr_config is not None
        assert isinstance(result.ocr_config, TesseractConfig)
        assert result.ocr_config.language == "fra"  # CLI override

    def test_find_default_config_deprecated(self) -> None:
        """Test deprecated find_default_config function."""
        with patch("kreuzberg._config.find_config_file") as mock_find:
            mock_find.return_value = Path("/fake/config.toml")

            result = find_default_config()

            assert result == Path("/fake/config.toml")
            mock_find.assert_called_once_with()
