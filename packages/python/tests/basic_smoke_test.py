"""Basic smoke tests to verify package structure and imports work."""


def test_import_kreuzberg():
    """Test that kreuzberg can be imported."""
    import kreuzberg

    assert kreuzberg.__version__ is not None


def test_public_api_exports():
    """Test that all documented exports are available."""
    import kreuzberg

    # Config types
    assert hasattr(kreuzberg, "ChunkingConfig")
    assert hasattr(kreuzberg, "ExtractionConfig")
    assert hasattr(kreuzberg, "ImageExtractionConfig")
    assert hasattr(kreuzberg, "LanguageDetectionConfig")
    assert hasattr(kreuzberg, "OcrConfig")
    assert hasattr(kreuzberg, "PdfConfig")
    assert hasattr(kreuzberg, "TokenReductionConfig")

    # Result types
    assert hasattr(kreuzberg, "ExtractionResult")
    assert hasattr(kreuzberg, "ExtractedTable")

    # Sync functions
    assert hasattr(kreuzberg, "extract_file_sync")
    assert hasattr(kreuzberg, "extract_bytes_sync")
    assert hasattr(kreuzberg, "batch_extract_files_sync")
    assert hasattr(kreuzberg, "batch_extract_bytes_sync")

    # Async functions
    assert hasattr(kreuzberg, "extract_file")
    assert hasattr(kreuzberg, "extract_bytes")
    assert hasattr(kreuzberg, "batch_extract_files")
    assert hasattr(kreuzberg, "batch_extract_bytes")

    # MIME utilities
    assert hasattr(kreuzberg, "detect_mime_type")
    assert hasattr(kreuzberg, "validate_mime_type")

    # Plugin functions
    assert hasattr(kreuzberg, "register_ocr_backend")
    assert hasattr(kreuzberg, "list_ocr_backends")
    assert hasattr(kreuzberg, "unregister_ocr_backend")
