from importlib.metadata import version

from kreuzberg._gmft import GMFTConfig
from kreuzberg._ocr._easyocr import EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRConfig
from kreuzberg._ocr._tesseract import TesseractConfig

from ._ocr._tesseract import PSMMode
from ._registry import ExtractorRegistry
from ._types import ExtractionConfig, ExtractionResult, Metadata, TableData
from .exceptions import KreuzbergError, MissingDependencyError, OCRError, ParsingError, ValidationError
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
    "EasyOCRConfig",
    "ExtractionConfig",
    "ExtractionResult",
    "ExtractorRegistry",
    "GMFTConfig",
    "KreuzbergError",
    "Metadata",
    "MissingDependencyError",
    "OCRError",
    "PSMMode",
    "PaddleOCRConfig",
    "ParsingError",
    "TableData",
    "TesseractConfig",
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
