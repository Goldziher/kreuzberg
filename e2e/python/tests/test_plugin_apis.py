# Auto-generated plugin API tests for Python binding.
"""
E2E tests for plugin registration and management APIs.

Tests all plugin types:
- Validators
- Post-processors
- OCR backends
- Document extractors

Tests all management operations:
- Registration
- Unregistration
- Listing
- Clearing
"""

from __future__ import annotations

import kreuzberg
from kreuzberg import ExtractionConfig


class TestValidatorAPIs:
    """Test validator registration and management APIs."""

    def test_list_validators(self) -> None:
        """List all registered validators."""
        validators = kreuzberg.list_validators()
        assert isinstance(validators, list)
        assert all(isinstance(v, str) for v in validators)

    def test_clear_validators(self) -> None:
        """Clear all validators."""
        # Should not raise
        kreuzberg.clear_validators()
        validators = kreuzberg.list_validators()
        assert len(validators) == 0


class TestPostProcessorAPIs:
    """Test post-processor registration and management APIs."""

    def test_list_post_processors(self) -> None:
        """List all registered post-processors."""
        processors = kreuzberg.list_post_processors()
        assert isinstance(processors, list)
        assert all(isinstance(p, str) for p in processors)

    def test_clear_post_processors(self) -> None:
        """Clear all post-processors."""
        # Should not raise
        kreuzberg.clear_post_processors()
        processors = kreuzberg.list_post_processors()
        assert len(processors) == 0


class TestOCRBackendAPIs:
    """Test OCR backend registration and management APIs."""

    def test_list_ocr_backends(self) -> None:
        """List all registered OCR backends."""
        backends = kreuzberg.list_ocr_backends()
        assert isinstance(backends, list)
        assert all(isinstance(b, str) for b in backends)
        # Should include built-in backends
        assert "tesseract" in backends

    def test_unregister_ocr_backend(self) -> None:
        """Unregister an OCR backend."""
        # Should handle nonexistent backend gracefully
        kreuzberg.unregister_ocr_backend("nonexistent-backend-xyz")

    def test_clear_ocr_backends(self) -> None:
        """Clear all OCR backends."""
        # Should not raise
        kreuzberg.clear_ocr_backends()
        backends = kreuzberg.list_ocr_backends()
        assert len(backends) == 0


class TestDocumentExtractorAPIs:
    """Test document extractor registration and management APIs."""

    def test_list_document_extractors(self) -> None:
        """List all registered document extractors."""
        extractors = kreuzberg.list_document_extractors()
        assert isinstance(extractors, list)
        assert all(isinstance(e, str) for e in extractors)
        # Should include built-in extractors
        assert any("pdf" in e.lower() for e in extractors)

    def test_unregister_document_extractor(self) -> None:
        """Unregister a document extractor."""
        # Should handle nonexistent extractor gracefully
        kreuzberg.unregister_document_extractor("nonexistent-extractor-xyz")

    def test_clear_document_extractors(self) -> None:
        """Clear all document extractors."""
        # Should not raise
        kreuzberg.clear_document_extractors()
        extractors = kreuzberg.list_document_extractors()
        assert len(extractors) == 0


class TestConfigAPIs:
    """Test configuration loading and management APIs."""

    def test_config_from_file(self, tmp_path) -> None:
        """Load configuration from a TOML file."""
        config_path = tmp_path / "test_config.toml"
        config_path.write_text("""
[chunking]
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

    def test_config_discover(self, tmp_path, monkeypatch) -> None:
        """Discover configuration from current or parent directories."""
        config_path = tmp_path / "kreuzberg.toml"
        config_path.write_text("""
[chunking]
max_chars = 50
""")

        # Change to subdirectory
        subdir = tmp_path / "subdir"
        subdir.mkdir()
        monkeypatch.chdir(subdir)

        config = ExtractionConfig.discover()
        assert config is not None
        assert config.chunking is not None
        assert config.chunking.max_chars == 50


class TestMIMEUtilities:
    """Test MIME type detection and utilities."""

    def test_detect_mime_from_bytes(self) -> None:
        """Detect MIME type from file bytes."""
        # PDF magic bytes
        pdf_bytes = b"%PDF-1.4\n"
        mime_type = kreuzberg.detect_mime_type(pdf_bytes)
        assert "pdf" in mime_type.lower()

    def test_detect_mime_from_path(self, tmp_path) -> None:
        """Detect MIME type from file path."""
        test_file = tmp_path / "test.txt"
        test_file.write_text("Hello, world!")

        mime_type = kreuzberg.detect_mime_type_from_path(str(test_file))
        assert "text" in mime_type.lower()

    def test_get_mime_extensions(self) -> None:
        """Get file extensions for a MIME type."""
        extensions = kreuzberg.get_extensions_for_mime("application/pdf")
        assert isinstance(extensions, list)
        assert "pdf" in extensions
