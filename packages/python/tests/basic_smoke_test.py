"""Basic smoke tests to verify package structure and imports work."""


def test_import_kreuzberg() -> None:
    """Test that kreuzberg can be imported."""
    import kreuzberg

    assert kreuzberg.__version__ is not None


def test_public_api_exports() -> None:
    """Test that all documented exports are available."""
    import kreuzberg

    # Configuration types
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

    # Result types
    assert hasattr(kreuzberg, "ExtractionResult")
    assert hasattr(kreuzberg, "ExtractedTable")
    assert hasattr(kreuzberg, "Metadata")

    # Exception types
    assert hasattr(kreuzberg, "KreuzbergError")
    assert hasattr(kreuzberg, "MissingDependencyError")
    assert hasattr(kreuzberg, "OCRError")
    assert hasattr(kreuzberg, "ParsingError")
    assert hasattr(kreuzberg, "ValidationError")

    # Extraction functions (sync)
    assert hasattr(kreuzberg, "extract_file_sync")
    assert hasattr(kreuzberg, "extract_bytes_sync")
    assert hasattr(kreuzberg, "batch_extract_files_sync")
    assert hasattr(kreuzberg, "batch_extract_bytes_sync")

    # Extraction functions (async)
    assert hasattr(kreuzberg, "extract_file")
    assert hasattr(kreuzberg, "extract_bytes")
    assert hasattr(kreuzberg, "batch_extract_files")
    assert hasattr(kreuzberg, "batch_extract_bytes")

    # PostProcessor management
    assert hasattr(kreuzberg, "PostProcessorProtocol")
    assert hasattr(kreuzberg, "register_post_processor")
    assert hasattr(kreuzberg, "unregister_post_processor")
    assert hasattr(kreuzberg, "clear_post_processors")

    # Version
    assert hasattr(kreuzberg, "__version__")
