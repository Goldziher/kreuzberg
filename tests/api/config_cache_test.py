import json
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

from kreuzberg._api._config_cache import (
    clear_all_caches,
    create_gmft_config_cached,
    create_html_markdown_config_cached,
    create_language_detection_config_cached,
    create_ocr_config_cached,
    create_spacy_config_cached,
    discover_config_cached,
    get_cache_stats,
    parse_header_config_cached,
)
from kreuzberg._types import (
    EasyOCRConfig,
    GMFTConfig,
    HTMLToMarkdownConfig,
    LanguageDetectionConfig,
    PaddleOCRConfig,
    SpacyEntityExtractionConfig,
    TesseractConfig,
)


def test_discover_config_cached_no_config_file(tmp_path: Path) -> None:
    result = discover_config_cached(tmp_path)
    assert result is None


def test_discover_config_cached_with_kreuzberg_toml(tmp_path: Path) -> None:
    result = discover_config_cached(tmp_path)
    assert result is None


def test_discover_config_cached_with_pyproject_toml(tmp_path: Path) -> None:
    result = discover_config_cached(tmp_path)
    assert result is None


def test_discover_config_cached_os_error_fallback(tmp_path: Path) -> None:
    with patch("kreuzberg._api._config_cache.discover_config") as mock_discover:
        mock_discover.return_value = None
        result = discover_config_cached(tmp_path)
        assert result is None


def test_discover_config_cached_uses_current_dir_when_none() -> None:
    with patch("kreuzberg._api._config_cache.Path.cwd") as mock_cwd:
        fake_dir = Path("/fake/dir")
        mock_cwd.return_value = fake_dir
        with patch("kreuzberg._api._config_cache._cached_discover_config") as mock_cached:
            mock_cached.return_value = None

            result = discover_config_cached(None)
            mock_cached.assert_called_with(str(fake_dir), 0.0, 0)
            assert result is None


def test_create_ocr_config_cached_tesseract() -> None:
    config_dict = {"language": "eng"}
    result = create_ocr_config_cached("tesseract", config_dict)
    assert isinstance(result, TesseractConfig)
    assert result.language == "eng"


def test_create_ocr_config_cached_easyocr() -> None:
    config_dict = {"language": "en"}
    result = create_ocr_config_cached("easyocr", config_dict)
    assert isinstance(result, EasyOCRConfig)


def test_create_ocr_config_cached_paddleocr() -> None:
    config_dict = {"language": "en"}
    result = create_ocr_config_cached("paddleocr", config_dict)
    assert isinstance(result, PaddleOCRConfig)


def test_create_ocr_config_cached_no_backend() -> None:
    config_dict = {"psm_mode": "auto"}
    result = create_ocr_config_cached(None, config_dict)
    assert isinstance(result, TesseractConfig)


def test_create_ocr_config_cached_empty_backend() -> None:
    config_dict = {"psm_mode": "auto"}
    result = create_ocr_config_cached("", config_dict)
    assert isinstance(result, TesseractConfig)


def test_create_ocr_config_cached_invalid_backend() -> None:
    config_dict: dict[str, Any] = {}
    with pytest.raises(ValueError, match="Unknown OCR config type: invalid"):
        create_ocr_config_cached("invalid", config_dict)


def test_create_gmft_config_cached() -> None:
    config_dict = {"verbosity": 1}
    result = create_gmft_config_cached(config_dict)
    assert isinstance(result, GMFTConfig)
    assert result.verbosity == 1


def test_create_language_detection_config_cached() -> None:
    config_dict = {"top_k": 5}
    result = create_language_detection_config_cached(config_dict)
    assert isinstance(result, LanguageDetectionConfig)
    assert result.top_k == 5


def test_create_spacy_config_cached() -> None:
    config_dict = {"language_models": {"en": "en_core_web_sm"}}
    result = create_spacy_config_cached(config_dict)
    assert isinstance(result, SpacyEntityExtractionConfig)


def test_create_html_markdown_config_cached() -> None:
    config_dict = {"autolinks": False}
    result = create_html_markdown_config_cached(config_dict)
    assert isinstance(result, HTMLToMarkdownConfig)
    assert result.autolinks is False


def test_parse_header_config_cached() -> None:
    header_value = '{"use_cache": false, "extract_tables": true}'
    result = parse_header_config_cached(header_value)
    assert result == {"use_cache": False, "extract_tables": True}


def test_parse_header_config_cached_invalid_json() -> None:
    header_value = "invalid json"
    with pytest.raises(json.JSONDecodeError):
        parse_header_config_cached(header_value)


def test_get_cache_stats() -> None:
    clear_all_caches()

    create_ocr_config_cached("tesseract", {})
    create_gmft_config_cached({})
    parse_header_config_cached("{}")

    stats = get_cache_stats()

    expected_sections = [
        "discover_config",
        "ocr_config",
        "header_parsing",
        "gmft_config",
        "language_detection_config",
        "spacy_config",
    ]

    for section in expected_sections:
        assert section in stats
        assert "hits" in stats[section]
        assert "misses" in stats[section]
        assert "size" in stats[section]
        assert "max_size" in stats[section]


def test_clear_all_caches() -> None:
    create_ocr_config_cached("tesseract", {})
    create_gmft_config_cached({})

    clear_all_caches()

    stats = get_cache_stats()
    for section in stats.values():
        assert section["size"] == 0


def test_caching_behavior_ocr_config() -> None:
    clear_all_caches()

    config_dict = {"language": "deu"}

    result1 = create_ocr_config_cached("tesseract", config_dict)
    stats_after_first = get_cache_stats()
    assert stats_after_first["ocr_config"]["misses"] == 1
    assert stats_after_first["ocr_config"]["hits"] == 0

    result2 = create_ocr_config_cached("tesseract", config_dict)
    stats_after_second = get_cache_stats()
    assert stats_after_second["ocr_config"]["misses"] == 1
    assert stats_after_second["ocr_config"]["hits"] == 1

    assert result1.__dict__ == result2.__dict__


def test_caching_behavior_header_parsing() -> None:
    clear_all_caches()

    header_value = '{"test": "value"}'

    result1 = parse_header_config_cached(header_value)
    stats_after_first = get_cache_stats()
    assert stats_after_first["header_parsing"]["misses"] == 1

    result2 = parse_header_config_cached(header_value)
    stats_after_second = get_cache_stats()
    assert stats_after_second["header_parsing"]["hits"] == 1

    assert result1 == result2


def test_config_dict_serialization_consistency() -> None:
    clear_all_caches()

    config1 = {"verbosity": 2, "detection_threshold": 0.8}
    config2 = {"detection_threshold": 0.8, "verbosity": 2}

    result1 = create_gmft_config_cached(config1)
    result2 = create_gmft_config_cached(config2)

    stats = get_cache_stats()
    assert stats["gmft_config"]["hits"] == 1
    assert stats["gmft_config"]["misses"] == 1

    assert result1.__dict__ == result2.__dict__
