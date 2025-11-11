"""Basic smoke tests to verify package structure and imports work."""

from __future__ import annotations

import pytest


def test_import_kreuzberg() -> None:
    """Test that kreuzberg can be imported."""
    import kreuzberg

    assert kreuzberg.__version__ is not None


def test_public_api_exports() -> None:
    """Test that all documented exports are available."""
    import kreuzberg

    assert hasattr(kreuzberg, "ChunkingConfig")
    assert hasattr(kreuzberg, "ExtractionConfig")
    assert hasattr(kreuzberg, "ImageExtractionConfig")
    assert hasattr(kreuzberg, "ImagePreprocessingConfig")
    assert hasattr(kreuzberg, "LanguageDetectionConfig")
    assert hasattr(kreuzberg, "OcrConfig")
    assert hasattr(kreuzberg, "PdfConfig")
    assert hasattr(kreuzberg, "PostProcessorConfig")
    assert hasattr(kreuzberg, "TesseractConfig")
    assert hasattr(kreuzberg, "TokenReductionConfig")

    assert hasattr(kreuzberg, "ExtractionResult")
    assert hasattr(kreuzberg, "ExtractedTable")
    assert hasattr(kreuzberg, "Metadata")

    assert hasattr(kreuzberg, "KreuzbergError")
    assert hasattr(kreuzberg, "MissingDependencyError")
    assert hasattr(kreuzberg, "OCRError")
    assert hasattr(kreuzberg, "ParsingError")
    assert hasattr(kreuzberg, "ValidationError")

    assert hasattr(kreuzberg, "extract_file_sync")
    assert hasattr(kreuzberg, "extract_bytes_sync")
    assert hasattr(kreuzberg, "batch_extract_files_sync")
    assert hasattr(kreuzberg, "batch_extract_bytes_sync")

    assert hasattr(kreuzberg, "extract_file")
    assert hasattr(kreuzberg, "extract_bytes")
    assert hasattr(kreuzberg, "batch_extract_files")
    assert hasattr(kreuzberg, "batch_extract_bytes")

    assert hasattr(kreuzberg, "PostProcessorProtocol")
    assert hasattr(kreuzberg, "register_post_processor")
    assert hasattr(kreuzberg, "unregister_post_processor")
    assert hasattr(kreuzberg, "clear_post_processors")

    assert hasattr(kreuzberg, "__version__")


def test_extract_bytes_sync_smoke() -> None:
    """Ensure we can perform a simple synchronous extraction."""
    from kreuzberg import extract_bytes_sync

    result = extract_bytes_sync(b"Hello from smoke test!", "text/plain")

    assert result.content.startswith("Hello")
    assert result.mime_type == "text/plain"


@pytest.mark.asyncio
async def test_extract_bytes_async_smoke() -> None:
    """Ensure we can perform a simple asynchronous extraction."""
    from kreuzberg import extract_bytes

    result = await extract_bytes(b"Hello from async smoke!", "text/plain")

    assert result.content.startswith("Hello")
    assert result.mime_type == "text/plain"
