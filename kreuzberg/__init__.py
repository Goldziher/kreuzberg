from importlib.metadata import version

from kreuzberg._lib import load_pdfium  # noqa: F401

# Note: ExtractorRegistry removed in v4 - all extraction now handled by Rust core
# from ._registry import ExtractorRegistry
from ._types import (
    ChunkingConfig,
    EasyOCRConfig,
    Entity,
    EntityExtractionConfig,
    ExtractedImage,
    ExtractionConfig,
    ExtractionResult,
    HTMLToMarkdownConfig,
    ImageExtractionConfig,
    ImageOCRResult,
    JSONExtractionConfig,
    KeywordExtractionConfig,
    LanguageDetectionConfig,
    Metadata,
    PaddleOCRConfig,
    PSMMode,
    TableData,
    TableExtractionConfig,
    TesseractConfig,
    TokenReductionConfig,
)
from .exceptions import (
    KreuzbergError,
    MemoryLimitError,
    MissingDependencyError,
    OCRError,
    ParsingError,
    ValidationError,
)
from .extraction import (
    batch_extract_bytes,
    batch_extract_bytes_sync,
    batch_extract_file,
    batch_extract_file_sync,
    extract_bytes,
    extract_bytes_sync,
    extract_file,
    extract_file_sync,
)

__version__ = version("kreuzberg")

__all__ = [
    "ChunkingConfig",
    "EasyOCRConfig",
    "Entity",
    "EntityExtractionConfig",
    "ExtractedImage",
    "ExtractionConfig",
    "ExtractionResult",
    # "ExtractorRegistry",  # Removed in v4 - extraction handled by Rust core
    "HTMLToMarkdownConfig",
    "ImageExtractionConfig",
    "ImageOCRResult",
    "JSONExtractionConfig",
    "KeywordExtractionConfig",
    "KreuzbergError",
    "LanguageDetectionConfig",
    "MemoryLimitError",
    "Metadata",
    "MissingDependencyError",
    "OCRError",
    "PSMMode",
    "PaddleOCRConfig",
    "ParsingError",
    "TableData",
    "TableExtractionConfig",
    "TesseractConfig",
    "TokenReductionConfig",
    "ValidationError",
    "__version__",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_file",
    "batch_extract_file_sync",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
]
