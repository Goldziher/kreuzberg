from __future__ import annotations

from pathlib import Path

import pytest

from kreuzberg import ExtractionConfig


FIXTURES_DIR = Path(__file__).parent.parent / "fixtures"


def test_from_file_toml_basic() -> None:
    config_path = FIXTURES_DIR / "config.toml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config is not None
    assert isinstance(config, ExtractionConfig)
    assert not config.use_cache
    assert config.enable_quality_processing
    assert config.force_ocr


def test_from_file_yaml_basic() -> None:
    config_path = FIXTURES_DIR / "config.yaml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config is not None
    assert isinstance(config, ExtractionConfig)
    assert config.use_cache
    assert not config.enable_quality_processing
    assert not config.force_ocr


def test_from_file_toml_with_ocr() -> None:
    config_path = FIXTURES_DIR / "config.toml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config.ocr is not None
    assert config.ocr.backend == "tesseract"
    assert config.ocr.language == "eng"


def test_from_file_yaml_with_chunking() -> None:
    config_path = FIXTURES_DIR / "config.yaml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config.chunking is not None
    assert config.chunking.max_chars == 2000
    assert config.chunking.max_overlap == 400


def test_from_file_toml_with_pdf_options() -> None:
    config_path = FIXTURES_DIR / "config.toml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config.pdf_options is not None
    assert config.pdf_options.extract_images
    assert config.pdf_options.extract_metadata
    assert config.pdf_options.passwords is not None
    assert len(config.pdf_options.passwords) == 2
    assert config.pdf_options.passwords[0] == "test123"
    assert config.pdf_options.passwords[1] == "secret"


def test_from_file_yaml_with_language_detection() -> None:
    config_path = FIXTURES_DIR / "config.yaml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config.language_detection is not None
    assert config.language_detection.enabled
    assert config.language_detection.min_confidence == 0.9
    assert config.language_detection.detect_multiple


def test_from_file_toml_with_images() -> None:
    config_path = FIXTURES_DIR / "config.toml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config.images is not None
    assert config.images.extract_images
    assert config.images.target_dpi == 300
    assert config.images.max_image_dimension == 4096


def test_from_file_nonexistent_file() -> None:
    with pytest.raises(Exception) as exc_info:
        ExtractionConfig.from_file("/nonexistent/path/config.toml")

    assert "Failed to read config file" in str(exc_info.value) or "ValidationError" in str(exc_info.value)


def test_from_file_invalid_toml() -> None:
    config_path = FIXTURES_DIR / "invalid.toml"
    with pytest.raises(Exception) as exc_info:
        ExtractionConfig.from_file(str(config_path))

    assert "Invalid TOML" in str(exc_info.value) or "ValidationError" in str(exc_info.value)


def test_from_file_invalid_yaml() -> None:
    config_path = FIXTURES_DIR / "invalid.yaml"
    with pytest.raises(Exception) as exc_info:
        ExtractionConfig.from_file(str(config_path))

    assert "Invalid YAML" in str(exc_info.value) or "ValidationError" in str(exc_info.value)


def test_from_file_with_path_object() -> None:
    config_path = FIXTURES_DIR / "config.toml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config is not None
    assert isinstance(config, ExtractionConfig)


def test_from_file_relative_path() -> None:
    import os

    original_cwd = os.getcwd()
    try:
        os.chdir(FIXTURES_DIR)
        config = ExtractionConfig.from_file("config.toml")

        assert config is not None
        assert isinstance(config, ExtractionConfig)
    finally:
        os.chdir(original_cwd)


def test_from_file_absolute_path() -> None:
    config_path = FIXTURES_DIR / "config.yaml"
    absolute_path = config_path.resolve()
    config = ExtractionConfig.from_file(str(absolute_path))

    assert config is not None
    assert isinstance(config, ExtractionConfig)


def test_from_file_auto_detects_toml_extension() -> None:
    config_path = FIXTURES_DIR / "config.toml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config is not None
    assert not config.use_cache


def test_from_file_auto_detects_yaml_extension() -> None:
    config_path = FIXTURES_DIR / "config.yaml"
    config = ExtractionConfig.from_file(str(config_path))

    assert config is not None
    assert config.use_cache
