# Auto-generated from fixtures/plugin_api/ - DO NOT EDIT
"""
E2E tests for plugin/config/utility APIs.

Generated from plugin API fixtures.
To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang python
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import kreuzberg
from kreuzberg import ExtractionConfig

if TYPE_CHECKING:
    from pathlib import Path

# Configuration Tests


def test_config_discover(tmp_path: Path, monkeypatch) -> None:
    """Discover configuration from current or parent directories"""
    config_path = tmp_path / "kreuzberg.toml"
    config_path.write_text("""[chunking]
max_chars = 50
""")

    subdir = tmp_path / "subdir"
    subdir.mkdir()
    monkeypatch.chdir(subdir)

    config = ExtractionConfig.discover()
    assert config is not None

    assert config.chunking is not None
    assert config.chunking.max_chars == 50


def test_config_from_file(tmp_path: Path) -> None:
    """Load configuration from a TOML file"""
    config_path = tmp_path / "test_config.toml"
    config_path.write_text("""[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
""")

    config = ExtractionConfig.from_file(str(config_path))

    assert config.chunking is not None
    assert config.chunking.max_chars == 100
    assert config.chunking.max_overlap == 20
    assert config.language_detection is not None
    assert config.language_detection.enabled is False


# Document Extractor Management Tests


def test_extractors_clear() -> None:
    """Clear all document extractors and verify list is empty"""
    kreuzberg.clear_document_extractors()
    result = kreuzberg.list_document_extractors()
    assert len(result) == 0


def test_extractors_list() -> None:
    """List all registered document extractors"""
    result = kreuzberg.list_document_extractors()
    assert isinstance(result, list)
    assert all(isinstance(item, str) for item in result)


def test_extractors_unregister() -> None:
    """Unregister nonexistent document extractor gracefully"""
    kreuzberg.unregister_document_extractor("nonexistent-extractor-xyz")


# Mime Utilities Tests


def test_mime_detect_bytes() -> None:
    """Detect MIME type from file bytes"""
    test_bytes = b"%PDF-1.4\n"
    result = kreuzberg.detect_mime_type(test_bytes)

    assert "pdf" in result.lower()


def test_mime_detect_path(tmp_path: Path) -> None:
    """Detect MIME type from file path"""
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello, world!")

    result = kreuzberg.detect_mime_type_from_path(str(test_file))

    assert "text" in result.lower()


def test_mime_get_extensions() -> None:
    """Get file extensions for a MIME type"""
    result = kreuzberg.get_extensions_for_mime("application/pdf")
    assert isinstance(result, list)
    assert "pdf" in result


# Ocr Backend Management Tests


def test_ocr_backends_clear() -> None:
    """Clear all OCR backends and verify list is empty"""
    kreuzberg.clear_ocr_backends()
    result = kreuzberg.list_ocr_backends()
    assert len(result) == 0


def test_ocr_backends_list() -> None:
    """List all registered OCR backends"""
    result = kreuzberg.list_ocr_backends()
    assert isinstance(result, list)
    assert all(isinstance(item, str) for item in result)
    assert "tesseract" in result


def test_ocr_backends_unregister() -> None:
    """Unregister nonexistent OCR backend gracefully"""
    kreuzberg.unregister_ocr_backend("nonexistent-backend-xyz")


# Post Processor Management Tests


def test_post_processors_clear() -> None:
    """Clear all post-processors and verify list is empty"""
    kreuzberg.clear_post_processors()
    result = kreuzberg.list_post_processors()
    assert len(result) == 0


def test_post_processors_list() -> None:
    """List all registered post-processors"""
    result = kreuzberg.list_post_processors()
    assert isinstance(result, list)
    assert all(isinstance(item, str) for item in result)


# Validator Management Tests


def test_validators_clear() -> None:
    """Clear all validators and verify list is empty"""
    kreuzberg.clear_validators()
    result = kreuzberg.list_validators()
    assert len(result) == 0


def test_validators_list() -> None:
    """List all registered validators"""
    result = kreuzberg.list_validators()
    assert isinstance(result, list)
    assert all(isinstance(item, str) for item in result)
