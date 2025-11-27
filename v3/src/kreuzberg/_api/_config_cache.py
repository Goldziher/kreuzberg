"""API Configuration Caching Module.

This module provides LRU cached functions for API config operations to improve performance
by avoiding repeated file system operations and object creation.
"""

from __future__ import annotations

import hashlib
import json
from functools import lru_cache
from pathlib import Path
from typing import Any

from kreuzberg._config import discover_config
from kreuzberg._types import (
    EasyOCRConfig,
    ExtractionConfig,
    GMFTConfig,
    HTMLToMarkdownConfig,
    LanguageDetectionConfig,
    PaddleOCRConfig,
    SpacyEntityExtractionConfig,
    TesseractConfig,
)


@lru_cache(maxsize=16)
def _cached_discover_config(
    search_path: str,
    config_file_mtime: float,  # noqa: ARG001
    config_file_size: int,  # noqa: ARG001
) -> ExtractionConfig | None:
    """Cache config discovery with file modification time validation."""
    return discover_config(Path(search_path))


def discover_config_cached(search_path: Path | str | None = None) -> ExtractionConfig | None:
    """Cached version of discover_config with automatic invalidation.

    This function caches the result of discover_config() and automatically invalidates
    the cache when config files are modified.

    Args:
        search_path: Path to start searching for config files from

    Returns:
        ExtractionConfig if found, None otherwise

    """
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
    """Cache OCR config object creation."""
    config_dict = json.loads(config_json)

    if config_type == "tesseract":
        return TesseractConfig(**config_dict)
    if config_type == "easyocr":
        return EasyOCRConfig(**config_dict)
    if config_type == "paddleocr":
        return PaddleOCRConfig(**config_dict)
    msg = f"Unknown OCR config type: {config_type}"
    raise ValueError(msg)


@lru_cache(maxsize=64)
def _cached_create_gmft_config(config_json: str) -> GMFTConfig:
    """Cache GMFT config creation."""
    return GMFTConfig(**json.loads(config_json))


@lru_cache(maxsize=64)
def _cached_create_language_detection_config(config_json: str) -> LanguageDetectionConfig:
    """Cache language detection config creation."""
    return LanguageDetectionConfig(**json.loads(config_json))


@lru_cache(maxsize=64)
def _cached_create_spacy_config(config_json: str) -> SpacyEntityExtractionConfig:
    """Cache spaCy entity extraction config creation."""
    return SpacyEntityExtractionConfig(**json.loads(config_json))


@lru_cache(maxsize=64)
def _cached_create_html_markdown_config(config_json: str) -> HTMLToMarkdownConfig:
    """Cache HTML to Markdown config creation."""
    return HTMLToMarkdownConfig(**json.loads(config_json))


@lru_cache(maxsize=256)
def _cached_parse_header_config(header_value: str) -> dict[str, Any]:
    """Cache parsed header configurations."""
    parsed_config: dict[str, Any] = json.loads(header_value)
    return parsed_config


def create_ocr_config_cached(
    ocr_backend: str | None, config_dict: dict[str, Any]
) -> TesseractConfig | EasyOCRConfig | PaddleOCRConfig:
    """Cached version of OCR config creation.

    Args:
        ocr_backend: The OCR backend type
        config_dict: Configuration dictionary

    Returns:
        Configured OCR config object

    """
    if not ocr_backend:
        return TesseractConfig()

    config_json = json.dumps(config_dict, sort_keys=True)
    return _cached_create_ocr_config(ocr_backend, config_json)


def create_gmft_config_cached(config_dict: dict[str, Any]) -> GMFTConfig:
    """Cached version of GMFT config creation."""
    config_json = json.dumps(config_dict, sort_keys=True)
    return _cached_create_gmft_config(config_json)


def create_language_detection_config_cached(config_dict: dict[str, Any]) -> LanguageDetectionConfig:
    """Cached version of language detection config creation."""
    config_json = json.dumps(config_dict, sort_keys=True)
    return _cached_create_language_detection_config(config_json)


def create_spacy_config_cached(config_dict: dict[str, Any]) -> SpacyEntityExtractionConfig:
    """Cached version of spaCy config creation."""
    config_json = json.dumps(config_dict, sort_keys=True)
    return _cached_create_spacy_config(config_json)


def create_html_markdown_config_cached(config_dict: dict[str, Any]) -> HTMLToMarkdownConfig:
    """Cached version of HTML to Markdown config creation."""
    config_json = json.dumps(config_dict, sort_keys=True)
    return _cached_create_html_markdown_config(config_json)


def parse_header_config_cached(header_value: str) -> dict[str, Any]:
    """Cached version of header config parsing.

    Args:
        header_value: JSON string from X-Extraction-Config header

    Returns:
        Parsed configuration dictionary

    """
    return _cached_parse_header_config(header_value)


@lru_cache(maxsize=512)
def _cached_merge_configs(
    static_config_hash: str,
    query_params_hash: str,
    header_config_hash: str,
) -> ExtractionConfig:
    """Cache the complete config merging process.

    This is the ultimate optimization - cache the entire result of merge_configs()
    based on content hashes of all inputs.
    """
    msg = "Not implemented yet - use individual component caching"
    raise NotImplementedError(msg)


def _hash_dict(data: dict[str, Any] | None) -> str:
    """Create a hash string from a dictionary for cache keys."""
    if data is None:
        return "none"

    json_str = json.dumps(data, sort_keys=True, default=str)
    return hashlib.sha256(json_str.encode()).hexdigest()[:16]


def get_cache_stats() -> dict[str, Any]:
    """Get cache statistics for monitoring performance."""
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
        "header_parsing": {
            "hits": _cached_parse_header_config.cache_info().hits,
            "misses": _cached_parse_header_config.cache_info().misses,
            "size": _cached_parse_header_config.cache_info().currsize,
            "max_size": _cached_parse_header_config.cache_info().maxsize,
        },
        "gmft_config": {
            "hits": _cached_create_gmft_config.cache_info().hits,
            "misses": _cached_create_gmft_config.cache_info().misses,
            "size": _cached_create_gmft_config.cache_info().currsize,
            "max_size": _cached_create_gmft_config.cache_info().maxsize,
        },
        "language_detection_config": {
            "hits": _cached_create_language_detection_config.cache_info().hits,
            "misses": _cached_create_language_detection_config.cache_info().misses,
            "size": _cached_create_language_detection_config.cache_info().currsize,
            "max_size": _cached_create_language_detection_config.cache_info().maxsize,
        },
        "spacy_config": {
            "hits": _cached_create_spacy_config.cache_info().hits,
            "misses": _cached_create_spacy_config.cache_info().misses,
            "size": _cached_create_spacy_config.cache_info().currsize,
            "max_size": _cached_create_spacy_config.cache_info().maxsize,
        },
    }


def clear_all_caches() -> None:
    """Clear all API configuration caches."""
    _cached_discover_config.cache_clear()
    _cached_create_ocr_config.cache_clear()
    _cached_create_gmft_config.cache_clear()
    _cached_create_language_detection_config.cache_clear()
    _cached_create_spacy_config.cache_clear()
    _cached_create_html_markdown_config.cache_clear()
    _cached_parse_header_config.cache_clear()
