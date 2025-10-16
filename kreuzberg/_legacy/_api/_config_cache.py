from __future__ import annotations

from functools import lru_cache
from pathlib import Path
from typing import Any

from kreuzberg._config import discover_config
from kreuzberg._types import (
    EasyOCRConfig,
    EntityExtractionConfig,
    ExtractionConfig,
    HTMLToMarkdownConfig,
    LanguageDetectionConfig,
    PaddleOCRConfig,
    TableExtractionConfig,
    TesseractConfig,
)
from kreuzberg._utils._serialization import deserialize, serialize


@lru_cache(maxsize=16)
def _cached_discover_config(
    search_path: str,
    config_file_mtime: float,  # noqa: ARG001
    config_file_size: int,  # noqa: ARG001
) -> ExtractionConfig | None:
    return discover_config(Path(search_path))


def discover_config_cached(search_path: Path | str | None = None) -> ExtractionConfig | None:
    search_path = Path.cwd() if search_path is None else Path(search_path)

    config_files = ["kreuzberg.toml", "pyproject.toml"]
    for config_file_name in config_files:
        config_path = search_path / config_file_name
        if config_path.exists():
            try:
                stat = config_path.stat()
                return _cached_discover_config(
                    str(search_path),
                    stat.st_mtime,
                    stat.st_size,
                )
            except OSError:
                return discover_config(search_path)

    return _cached_discover_config(str(search_path), 0.0, 0)


@lru_cache(maxsize=128)
def _cached_create_ocr_config(
    config_type: str,
    config_json: str,
) -> TesseractConfig | EasyOCRConfig | PaddleOCRConfig:
    config_dict = deserialize(config_json, dict[str, Any], json=True)

    if config_type == "tesseract":
        return TesseractConfig(**config_dict)
    if config_type == "easyocr":
        return EasyOCRConfig(**config_dict)
    if config_type == "paddleocr":
        return PaddleOCRConfig(**config_dict)
    msg = f"Unknown OCR config type: {config_type}"
    raise ValueError(msg)


@lru_cache(maxsize=64)
def _cached_create_table_extraction_config(config_json: str) -> TableExtractionConfig:
    return TableExtractionConfig(**deserialize(config_json, dict[str, Any], json=True))


@lru_cache(maxsize=64)
def _cached_create_language_detection_config(config_json: str) -> LanguageDetectionConfig:
    return LanguageDetectionConfig(**deserialize(config_json, dict[str, Any], json=True))


@lru_cache(maxsize=64)
def _cached_create_entity_extraction_config(config_json: str) -> EntityExtractionConfig:
    return EntityExtractionConfig(**deserialize(config_json, dict[str, Any], json=True))


@lru_cache(maxsize=64)
def _cached_create_html_markdown_config(config_json: str) -> HTMLToMarkdownConfig:
    return HTMLToMarkdownConfig(**deserialize(config_json, dict[str, Any], json=True))


@lru_cache(maxsize=256)
def _cached_parse_header_config(header_value: str) -> dict[str, Any]:
    parsed_config: dict[str, Any] = deserialize(header_value, dict[str, Any], json=True)
    return parsed_config


def create_ocr_config_cached(
    ocr_backend: str | None, config_dict: dict[str, Any]
) -> TesseractConfig | EasyOCRConfig | PaddleOCRConfig:
    if not ocr_backend:
        return TesseractConfig()

    config_json = serialize(config_dict, json=True, sort_keys=True).decode()
    return _cached_create_ocr_config(ocr_backend, config_json)


def create_table_extraction_config_cached(config_dict: dict[str, Any]) -> TableExtractionConfig:
    config_json = serialize(config_dict, json=True, sort_keys=True).decode()
    return _cached_create_table_extraction_config(config_json)


def create_language_detection_config_cached(config_dict: dict[str, Any]) -> LanguageDetectionConfig:
    config_json = serialize(config_dict, json=True, sort_keys=True).decode()
    return _cached_create_language_detection_config(config_json)


def create_entity_extraction_config_cached(config_dict: dict[str, Any]) -> EntityExtractionConfig:
    config_json = serialize(config_dict, json=True, sort_keys=True).decode()
    return _cached_create_entity_extraction_config(config_json)


def create_html_markdown_config_cached(config_dict: dict[str, Any]) -> HTMLToMarkdownConfig:
    config_json = serialize(config_dict, json=True, sort_keys=True).decode()
    return _cached_create_html_markdown_config(config_json)


def parse_header_config_cached(header_value: str) -> dict[str, Any]:
    return _cached_parse_header_config(header_value)


def clear_all_caches() -> None:
    _cached_discover_config.cache_clear()
    _cached_create_ocr_config.cache_clear()
    _cached_create_table_extraction_config.cache_clear()
    _cached_create_language_detection_config.cache_clear()
    _cached_create_entity_extraction_config.cache_clear()
    _cached_create_html_markdown_config.cache_clear()
    _cached_parse_header_config.cache_clear()


def get_cache_stats() -> dict[str, dict[str, int | None]]:
    return {
        "discover_config": {
            "hits": _cached_discover_config.cache_info().hits,
            "misses": _cached_discover_config.cache_info().misses,
            "size": _cached_discover_config.cache_info().currsize,
            "max_size": _cached_discover_config.cache_info().maxsize,
        },
        "ocr_config": {
            "hits": _cached_create_ocr_config.cache_info().hits,
            "misses": _cached_create_ocr_config.cache_info().misses,
            "size": _cached_create_ocr_config.cache_info().currsize,
            "max_size": _cached_create_ocr_config.cache_info().maxsize,
        },
        "table_extraction_config": {
            "hits": _cached_create_table_extraction_config.cache_info().hits,
            "misses": _cached_create_table_extraction_config.cache_info().misses,
            "size": _cached_create_table_extraction_config.cache_info().currsize,
            "max_size": _cached_create_table_extraction_config.cache_info().maxsize,
        },
        "language_detection_config": {
            "hits": _cached_create_language_detection_config.cache_info().hits,
            "misses": _cached_create_language_detection_config.cache_info().misses,
            "size": _cached_create_language_detection_config.cache_info().currsize,
            "max_size": _cached_create_language_detection_config.cache_info().maxsize,
        },
        "entity_extraction_config": {
            "hits": _cached_create_entity_extraction_config.cache_info().hits,
            "misses": _cached_create_entity_extraction_config.cache_info().misses,
            "size": _cached_create_entity_extraction_config.cache_info().currsize,
            "max_size": _cached_create_entity_extraction_config.cache_info().maxsize,
        },
        "html_markdown_config": {
            "hits": _cached_create_html_markdown_config.cache_info().hits,
            "misses": _cached_create_html_markdown_config.cache_info().misses,
            "size": _cached_create_html_markdown_config.cache_info().currsize,
            "max_size": _cached_create_html_markdown_config.cache_info().maxsize,
        },
        "header_parsing": {
            "hits": _cached_parse_header_config.cache_info().hits,
            "misses": _cached_parse_header_config.cache_info().misses,
            "size": _cached_parse_header_config.cache_info().currsize,
            "max_size": _cached_parse_header_config.cache_info().maxsize,
        },
    }
