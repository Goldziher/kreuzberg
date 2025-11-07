from __future__ import annotations

import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, Any
from unittest.mock import patch

import pytest
from kreuzberg import ExtractionConfig
from kreuzberg._config import (
    _build_ocr_config_from_cli,
    _configure_gmft,
    _configure_ocr_backend,
    _create_ocr_config,
    _merge_cli_args,
    _merge_file_config,
    build_extraction_config,
    build_extraction_config_from_dict,
    discover_and_load_config,
    discover_config,
    find_config_file,
    find_default_config,
    load_config_from_file,
    load_config_from_path,
    load_default_config,
    merge_configs,
    parse_ocr_backend_config,
)
from kreuzberg._types import (
    EasyOCRConfig,
    GMFTConfig,
    HTMLToMarkdownConfig,
    PaddleOCRConfig,
    PSMMode,
    TesseractConfig,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from collections.abc import MutableMapping


def test_merge_file_config_empty() -> None:
    config_dict = {"existing": "value"}
    _merge_file_config(config_dict, {})
    assert config_dict == {"existing": "value"}


def test_merge_file_config_with_values() -> None:
    config_dict: dict[str, Any] = {}
    file_config: dict[str, Any] = {
        "force_ocr": True,
        "chunk_content": True,
        "extract_tables": True,
        "max_chars": 1000,
        "max_overlap": 200,
        "ocr_backend": "tesseract",
        "extract_entities": True,
        "extract_keywords": True,
        "auto_detect_language": True,
        "keyword_count": 5,
        "not_a_config_field": "ignored",
    }
    _merge_file_config(config_dict, file_config)
    assert config_dict["force_ocr"] is True
    assert config_dict["chunk_content"] is True
    assert config_dict["extract_tables"] is True
    assert config_dict["max_chars"] == 1000
    assert config_dict["keyword_count"] == 5
    assert "not_a_config_field" not in config_dict


def test_merge_cli_args_none_values() -> None:
    config_dict = {"force_ocr": False}
    cli_args: MutableMapping[str, Any] = {"force_ocr": None, "chunk_content": None}
    _merge_cli_args(config_dict, cli_args)
    assert config_dict == {"force_ocr": False}


def test_merge_cli_args_with_values() -> None:
    config_dict = {"force_ocr": False}
    cli_args: MutableMapping[str, Any] = {
        "force_ocr": True,
        "chunk_content": True,
        "extract_tables": False,
        "max_chars": 2000,
        "not_a_config_field": "ignored",
    }
    _merge_cli_args(config_dict, cli_args)
    assert config_dict["force_ocr"] is True
    assert config_dict["chunk_content"] is True
    assert config_dict["extract_tables"] is False
    assert config_dict["max_chars"] == 2000
    assert "not_a_config_field" not in config_dict


def test_build_ocr_config_from_cli_tesseract() -> None:
    cli_args: MutableMapping[str, Any] = {"tesseract_config": {"language": "eng", "psm": 3}}
    config = _build_ocr_config_from_cli("tesseract", cli_args)
    assert isinstance(config, TesseractConfig)
    assert config.language == "eng"
    assert config.psm == PSMMode.AUTO


def test_build_ocr_config_from_cli_easyocr() -> None:
    cli_args: MutableMapping[str, Any] = {"easyocr_config": {"language": ["en", "fr"], "beam_width": 10}}
    config = _build_ocr_config_from_cli("easyocr", cli_args)
    assert isinstance(config, EasyOCRConfig)
    assert config.language == ("en", "fr")  # type: ignore[comparison-overlap]
    assert config.beam_width == 10


def test_build_ocr_config_from_cli_paddleocr() -> None:
    cli_args: MutableMapping[str, Any] = {"paddleocr_config": {"language": "en", "use_angle_cls": True}}
    config = _build_ocr_config_from_cli("paddleocr", cli_args)
    assert isinstance(config, PaddleOCRConfig)
    assert config.language == "en"
    assert config.use_angle_cls is True


def test_build_ocr_config_from_cli_unknown_backend() -> None:
    cli_args: MutableMapping[str, Any] = {"unknown_config": {}}
    config = _build_ocr_config_from_cli("unknown", cli_args)
    assert config is None


def test_build_ocr_config_from_cli_no_config() -> None:
    cli_args: MutableMapping[str, Any] = {}
    config = _build_ocr_config_from_cli("tesseract", cli_args)
    assert config is None


def test_build_ocr_config_from_cli_invalid_config() -> None:
    cli_args: MutableMapping[str, Any] = {"tesseract_config": {"invalid_field": "value"}}
    with pytest.raises(ValidationError) as exc_info:
        _build_ocr_config_from_cli("tesseract", cli_args)
    assert "Invalid tesseract configuration" in str(exc_info.value)


def test_configure_ocr_backend_none() -> None:
    config_dict: dict[str, Any] = {"ocr_backend": None}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {}
    _configure_ocr_backend(config_dict, file_config, cli_args)
    assert "ocr_config" not in config_dict


def test_configure_ocr_backend_from_cli() -> None:
    config_dict: dict[str, Any] = {"ocr_backend": "tesseract"}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {"tesseract_config": {"language": "eng"}}
    _configure_ocr_backend(config_dict, file_config, cli_args)
    assert isinstance(config_dict["ocr_config"], TesseractConfig)
    assert config_dict["ocr_config"].language == "eng"


def test_configure_ocr_backend_from_file() -> None:
    config_dict: dict[str, Any] = {"ocr_backend": "easyocr"}
    file_config: dict[str, Any] = {"easyocr": {"language": ["en"], "beam_width": 5}}
    cli_args: MutableMapping[str, Any] = {}
    _configure_ocr_backend(config_dict, file_config, cli_args)
    assert isinstance(config_dict["ocr_config"], EasyOCRConfig)
    assert config_dict["ocr_config"].language == ("en",)  # type: ignore[comparison-overlap]


def test_configure_gmft_disabled() -> None:
    config_dict: dict[str, Any] = {"extract_tables": False}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {}
    _configure_gmft(config_dict, file_config, cli_args)
    assert "gmft_config" not in config_dict


def test_configure_gmft_from_cli() -> None:
    config_dict: dict[str, Any] = {"extract_tables": True}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {"gmft_config": {"verbosity": 2, "detector_base_threshold": 0.8}}
    _configure_gmft(config_dict, file_config, cli_args)
    assert isinstance(config_dict["gmft_config"], GMFTConfig)
    assert config_dict["gmft_config"].verbosity == 2


def test_configure_gmft_from_file() -> None:
    config_dict: dict[str, Any] = {"extract_tables": True}
    file_config: dict[str, Any] = {"gmft": {"verbosity": 1, "formatter_base_threshold": 0.9}}
    cli_args: MutableMapping[str, Any] = {}
    _configure_gmft(config_dict, file_config, cli_args)
    assert isinstance(config_dict["gmft_config"], GMFTConfig)
    assert config_dict["gmft_config"].verbosity == 1


def test_configure_gmft_invalid_config() -> None:
    config_dict: dict[str, Any] = {"extract_tables": True}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {"gmft_config": {"invalid_field": "value"}}
    with pytest.raises(ValidationError) as exc_info:
        _configure_gmft(config_dict, file_config, cli_args)
    assert "Invalid GMFT configuration" in str(exc_info.value)


def test_create_ocr_config_tesseract() -> None:
    backend_config = {"language": "eng", "psm": 6}
    config = _create_ocr_config("tesseract", backend_config)
    assert isinstance(config, TesseractConfig)
    assert config.language == "eng"
    assert config.psm == PSMMode.SINGLE_BLOCK


def test_create_ocr_config_tesseract_psm_int() -> None:
    backend_config = {"psm": 3}
    config = _create_ocr_config("tesseract", backend_config)
    assert isinstance(config, TesseractConfig)
    assert config.psm == PSMMode.AUTO


def test_create_ocr_config_tesseract_invalid_psm() -> None:
    backend_config = {"psm": 99}
    with pytest.raises(ValidationError) as exc_info:
        _create_ocr_config("tesseract", backend_config)
    assert "Invalid PSM mode value" in str(exc_info.value)


def test_create_ocr_config_easyocr() -> None:
    backend_config = {"language": ["en"], "beam_width": 3}
    config = _create_ocr_config("easyocr", backend_config)
    assert isinstance(config, EasyOCRConfig)


def test_create_ocr_config_paddleocr() -> None:
    backend_config = {"language": "en"}
    config = _create_ocr_config("paddleocr", backend_config)
    assert isinstance(config, PaddleOCRConfig)


def test_create_ocr_config_unknown() -> None:
    with pytest.raises(ValueError, match="Unknown backend: unknown"):
        _create_ocr_config("unknown", {})


def test_load_config_from_file_kreuzberg_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("""
force_ocr = true
chunk_content = false
max_chars = 1000
""")
    config = load_config_from_file(config_file)
    assert config["force_ocr"] is True
    assert config["chunk_content"] is False
    assert config["max_chars"] == 1000


def test_load_config_from_file_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
chunk_content = false
""")
    config = load_config_from_file(config_file)
    assert config["force_ocr"] is True
    assert config["chunk_content"] is False


def test_load_config_from_file_other_toml_with_tool(tmp_path: Path) -> None:
    config_file = tmp_path / "other.toml"
    config_file.write_text("""
[tool.kreuzberg]
force_ocr = true
""")
    config = load_config_from_file(config_file)
    assert config["force_ocr"] is True


def test_load_config_from_file_other_toml_direct(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("""
force_ocr = true
""")
    config = load_config_from_file(config_file)
    assert config["force_ocr"] is True


def test_load_config_from_file_not_found() -> None:
    with pytest.raises(ValidationError) as exc_info:
        load_config_from_file(Path("/nonexistent/file.toml"))
    assert "Configuration file not found" in str(exc_info.value)


def test_load_config_from_file_invalid_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "invalid.toml"
    config_file.write_text("invalid toml content {")
    with pytest.raises(ValidationError) as exc_info:
        load_config_from_file(config_file)
    assert "Invalid TOML" in str(exc_info.value)


def test_merge_configs() -> None:
    base = {"a": 1, "b": {"c": 2, "d": 3}}
    override = {"b": {"c": 4, "e": 5}, "f": 6}
    result = merge_configs(base, override)
    assert result == {"a": 1, "b": {"c": 4, "d": 3, "e": 5}, "f": 6}


def test_merge_configs_non_dict_override() -> None:
    base = {"a": {"b": 1}}
    override = {"a": "string"}
    result = merge_configs(base, override)
    assert result == {"a": "string"}


def test_parse_ocr_backend_config_not_present() -> None:
    config_dict = {"other": "value"}
    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert result is None


def test_parse_ocr_backend_config_tesseract() -> None:
    config_dict = {"tesseract": {"language": "eng"}}
    result = parse_ocr_backend_config(config_dict, "tesseract")
    assert isinstance(result, TesseractConfig)
    assert result.language == "eng"


def test_parse_ocr_backend_config_invalid_type() -> None:
    config_dict = {"tesseract": "not a dict"}
    with pytest.raises(ValidationError) as exc_info:
        parse_ocr_backend_config(config_dict, "tesseract")
    assert "expected dict, got str" in str(exc_info.value)


def test_parse_ocr_backend_config_invalid_values() -> None:
    config_dict = {"tesseract": {"invalid": "field"}}
    with pytest.raises(ValidationError) as exc_info:
        parse_ocr_backend_config(config_dict, "tesseract")
    assert "Invalid configuration for OCR backend" in str(exc_info.value)


def test_build_extraction_config_from_dict_basic() -> None:
    config_dict = {
        "force_ocr": True,
        "chunk_content": True,
        "max_chars": 2000,
    }
    config = build_extraction_config_from_dict(config_dict)
    assert config.force_ocr is True
    assert config.chunk_content is True
    assert config.max_chars == 2000


def test_build_extraction_config_from_dict_with_ocr() -> None:
    config_dict = {
        "ocr_backend": "tesseract",
        "tesseract": {"language": "eng"},
    }
    config = build_extraction_config_from_dict(config_dict)
    assert config.ocr_backend == "tesseract"
    assert isinstance(config.ocr_config, TesseractConfig)


def test_build_extraction_config_from_dict_invalid_ocr_backend() -> None:
    config_dict = {"ocr_backend": "invalid"}
    with pytest.raises(ValidationError) as exc_info:
        build_extraction_config_from_dict(config_dict)
    assert "Invalid OCR backend: invalid" in str(exc_info.value)


def test_build_extraction_config_from_dict_with_gmft() -> None:
    config_dict = {
        "extract_tables": True,
        "gmft": {"verbosity": 1},
    }
    with pytest.warns(FutureWarning):
        config = build_extraction_config_from_dict(config_dict)
    assert config.extract_tables is True
    assert isinstance(config.gmft_config, GMFTConfig)


def test_build_extraction_config_from_dict_invalid_gmft() -> None:
    config_dict = {
        "extract_tables": True,
        "gmft": {"invalid": "field"},
    }
    with pytest.raises(ValidationError) as exc_info:
        build_extraction_config_from_dict(config_dict)
    assert "Invalid GMFT configuration" in str(exc_info.value)


def test_build_extraction_config_from_dict_with_html_to_markdown() -> None:
    config_dict = {"html_to_markdown": {"strip_tags": ["script", "style"]}}
    config = build_extraction_config_from_dict(config_dict)
    assert isinstance(config.html_to_markdown_config, HTMLToMarkdownConfig)
    assert config.html_to_markdown_config.strip_tags == ("script", "style")


def test_build_extraction_config_from_dict_invalid_html_to_markdown() -> None:
    config_dict = {"html_to_markdown": {"invalid": "field"}}
    with pytest.raises(ValidationError) as exc_info:
        build_extraction_config_from_dict(config_dict)
    assert "Invalid HTML to Markdown configuration" in str(exc_info.value)


def test_build_extraction_config_from_dict_ocr_backend_none() -> None:
    config_dict = {"ocr_backend": "none"}
    config = build_extraction_config_from_dict(config_dict)
    assert config.ocr_backend is None


def test_build_extraction_config_from_dict_invalid() -> None:
    config_dict = {"invalid_field": "value"}
    config = build_extraction_config_from_dict(config_dict)
    assert config is not None
    assert hasattr(config, "force_ocr")


def test_build_extraction_config() -> None:
    file_config: dict[str, Any] = {"force_ocr": False}
    cli_args: MutableMapping[str, Any] = {"force_ocr": True}
    config = build_extraction_config(file_config, cli_args)
    assert config.force_ocr is True


def test_build_extraction_config_ocr_backend_none() -> None:
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {"ocr_backend": "none"}
    config = build_extraction_config(file_config, cli_args)
    assert config.ocr_backend is None


def test_build_extraction_config_invalid() -> None:
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {"invalid": "value"}
    config = build_extraction_config(file_config, cli_args)
    assert config is not None
    assert hasattr(config, "force_ocr")


def test_find_config_file_kreuzberg_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.touch()

    with patch("pathlib.Path.cwd", return_value=tmp_path):
        result = find_config_file(tmp_path)
    assert result == config_file


def test_find_config_file_pyproject_toml(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("[tool.kreuzberg]\nforce_ocr = true")

    with patch("pathlib.Path.cwd", return_value=tmp_path):
        result = find_config_file(tmp_path)
    assert result == config_file


def test_find_config_file_pyproject_toml_no_kreuzberg(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("[tool.other]\nkey = 'value'")

    with patch("pathlib.Path.cwd", return_value=tmp_path):
        result = find_config_file(tmp_path)
    assert result is None


def test_find_config_file_pyproject_toml_read_error(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.touch()

    with patch("pathlib.Path.open", side_effect=OSError("Read error")), pytest.raises(ValidationError) as exc_info:
        find_config_file(tmp_path)
    assert "Failed to read pyproject.toml" in str(exc_info.value)


def test_find_config_file_pyproject_toml_invalid(tmp_path: Path) -> None:
    config_file = tmp_path / "pyproject.toml"
    config_file.write_text("invalid toml {")

    with pytest.raises(ValidationError) as exc_info:
        find_config_file(tmp_path)
    assert "Invalid TOML in pyproject.toml" in str(exc_info.value)


def test_find_config_file_parent_directory(tmp_path: Path) -> None:
    parent = tmp_path / "parent"
    child = parent / "child"
    child.mkdir(parents=True)
    config_file = parent / "kreuzberg.toml"
    config_file.touch()

    result = find_config_file(child)
    assert result == config_file


def test_find_config_file_not_found() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        result = find_config_file(tmp_path)
        assert result is None


def test_load_default_config(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        config = load_default_config(tmp_path)
    assert config is not None
    assert config.force_ocr is True


def test_load_default_config_not_found() -> None:
    with patch("kreuzberg._config.find_config_file", return_value=None):
        config = load_default_config()
    assert config is None


def test_load_default_config_empty(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        config = load_default_config(tmp_path)
    assert config is None


def test_load_config_from_path_string(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("force_ocr = true")

    config = load_config_from_path(str(config_file))
    assert config.force_ocr is True


def test_load_config_from_path_path(tmp_path: Path) -> None:
    config_file = tmp_path / "config.toml"
    config_file.write_text("chunk_content = true")

    config = load_config_from_path(config_file)
    assert config.chunk_content is True


def test_discover_and_load_config(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        config = discover_and_load_config(tmp_path)
    assert config.force_ocr is True


def test_discover_and_load_config_not_found() -> None:
    with patch("kreuzberg._config.find_config_file", return_value=None), pytest.raises(ValidationError) as exc_info:
        discover_and_load_config()
    assert "No configuration file found" in str(exc_info.value)


def test_discover_and_load_config_empty(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        with pytest.raises(ValidationError) as exc_info:
            discover_and_load_config(tmp_path)
    assert "contains no Kreuzberg configuration" in str(exc_info.value)


def test_discover_config(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("force_ocr = true")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        config = discover_config(tmp_path)
    assert config is not None
    assert config.force_ocr is True


def test_discover_config_not_found() -> None:
    with patch("kreuzberg._config.find_config_file", return_value=None):
        config = discover_config()
    assert config is None


def test_discover_config_empty(tmp_path: Path) -> None:
    config_file = tmp_path / "kreuzberg.toml"
    config_file.write_text("")

    with patch("kreuzberg._config.find_config_file", return_value=config_file):
        config = discover_config(tmp_path)
    assert config is None


def test_find_default_config() -> None:
    mock_path = Path("/mock/path/kreuzberg.toml")
    with patch("kreuzberg._config.find_config_file", return_value=mock_path):
        result = find_default_config()
    assert result == mock_path


def test_find_default_config_none() -> None:
    with patch("kreuzberg._config.find_config_file", return_value=None):
        result = find_default_config()
    assert result is None


def test_configure_gmft_with_cli_config() -> None:
    config_dict: dict[str, Any] = {"extract_tables": True}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {"gmft_config": {"verbosity": 2}}

    _configure_gmft(config_dict, file_config, cli_args)

    assert "gmft_config" in config_dict
    assert isinstance(config_dict["gmft_config"], GMFTConfig)
    assert config_dict["gmft_config"].verbosity == 2


def test_configure_gmft_with_file_config() -> None:
    config_dict: dict[str, Any] = {"extract_tables": True}
    file_config = {"gmft": {"verbosity": 1}}
    cli_args: MutableMapping[str, Any] = {}

    _configure_gmft(config_dict, file_config, cli_args)

    assert "gmft_config" in config_dict
    assert isinstance(config_dict["gmft_config"], GMFTConfig)
    assert config_dict["gmft_config"].verbosity == 1


def test_configure_ocr_backend_no_ocr_config_from_cli_or_file() -> None:
    from kreuzberg._config import _configure_ocr_backend

    config_dict: dict[str, Any] = {"ocr_backend": "tesseract"}
    file_config: dict[str, Any] = {}
    cli_args: MutableMapping[str, Any] = {}

    _configure_ocr_backend(config_dict, file_config, cli_args)
    assert "ocr_config" not in config_dict


def test_build_extraction_config_from_dict_no_ocr_config() -> None:
    from kreuzberg._config import build_extraction_config_from_dict

    config_dict = {
        "ocr_backend": "tesseract",
        "force_ocr": True,
    }

    config = build_extraction_config_from_dict(config_dict)
    assert config.ocr_backend == "tesseract"
    assert config.force_ocr is True
    assert config.ocr_config is None


def test_build_extraction_config_from_dict_no_gmft_config() -> None:
    from kreuzberg._config import build_extraction_config_from_dict

    config_dict = {
        "extract_tables": True,
    }

    config = build_extraction_config_from_dict(config_dict)
    assert config.extract_tables is True
    assert config.gmft_config is None


def test_extract_images_config_default() -> None:
    cfg = ExtractionConfig()
    assert cfg.extract_images is False
    assert cfg.ocr_extracted_images is False


def test_extract_images_config_enabled() -> None:
    cfg = ExtractionConfig(extract_images=True, ocr_extracted_images=True)
    assert cfg.extract_images is True
    assert cfg.ocr_extracted_images is True
