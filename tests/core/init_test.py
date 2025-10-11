from __future__ import annotations

import kreuzberg


def test_version() -> None:
    assert hasattr(kreuzberg, "__version__")
    assert isinstance(kreuzberg.__version__, str)
    # Version should be in format X.Y.Z or similar
    assert "." in kreuzberg.__version__


def test_exports() -> None:
    expected_exports = [
        "EasyOCRConfig",
        "Entity",
        "EntityExtractionConfig",
        "ExtractedImage",
        "ExtractionConfig",
        "ExtractionResult",
        "ExtractorRegistry",
        "TableExtractionConfig",
        "ImageExtractionConfig",
        "ImageOCRResult",
        "JSONExtractionConfig",
        "KreuzbergError",
        "LanguageDetectionConfig",
        "Metadata",
        "MissingDependencyError",
        "OCRError",
        "PSMMode",
        "PaddleOCRConfig",
        "ParsingError",
        "TableData",
        "TesseractConfig",
        "ValidationError",
        "batch_extract_bytes",
        "batch_extract_bytes_sync",
        "batch_extract_file",
        "batch_extract_file_sync",
        "extract_bytes",
        "extract_bytes_sync",
        "extract_file",
        "extract_file_sync",
    ]

    for export in expected_exports:
        assert hasattr(kreuzberg, export), f"Missing export: {export}"


def test_all_attribute() -> None:
    assert hasattr(kreuzberg, "__all__")
    assert isinstance(kreuzberg.__all__, list)
    assert "__version__" in kreuzberg.__all__

    for name in kreuzberg.__all__:
        assert hasattr(kreuzberg, name), f"Item in __all__ not importable: {name}"


def test_exception_hierarchy() -> None:
    assert issubclass(kreuzberg.MissingDependencyError, kreuzberg.KreuzbergError)
    assert issubclass(kreuzberg.OCRError, kreuzberg.KreuzbergError)
    assert issubclass(kreuzberg.ParsingError, kreuzberg.KreuzbergError)
    assert issubclass(kreuzberg.ValidationError, kreuzberg.KreuzbergError)


def test_config_classes() -> None:
    import msgspec

    assert isinstance(kreuzberg.ExtractionConfig, type)
    assert issubclass(kreuzberg.ExtractionConfig, msgspec.Struct)
    assert issubclass(kreuzberg.TesseractConfig, msgspec.Struct)
    assert issubclass(kreuzberg.EasyOCRConfig, msgspec.Struct)
    assert issubclass(kreuzberg.PaddleOCRConfig, msgspec.Struct)
    assert issubclass(kreuzberg.TableExtractionConfig, msgspec.Struct)


def test_extraction_functions_exist() -> None:
    assert callable(kreuzberg.extract_file)
    assert callable(kreuzberg.extract_file_sync)
    assert callable(kreuzberg.extract_bytes)
    assert callable(kreuzberg.extract_bytes_sync)
    assert callable(kreuzberg.batch_extract_file)
    assert callable(kreuzberg.batch_extract_file_sync)
    assert callable(kreuzberg.batch_extract_bytes)
    assert callable(kreuzberg.batch_extract_bytes_sync)
