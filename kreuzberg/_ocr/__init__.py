from functools import lru_cache
from typing import Any

from kreuzberg._ocr._base import OCRBackend
from kreuzberg._ocr._easyocr import EasyOCRBackend, EasyOCRConfig
from kreuzberg._ocr._paddleocr import PaddleOCRBackend, PaddleOCRConfig
from kreuzberg._ocr._tesseract import TesseractBackend, TesseractConfig
from kreuzberg._types import OcrBackendType

__all__ = [
    "EasyOCRBackend",
    "EasyOCRConfig",
    "OCRBackend",
    "PaddleOCRBackend",
    "PaddleOCRConfig",
    "TesseractBackend",
    "TesseractConfig",
]


@lru_cache
def get_ocr_backend(backend: OcrBackendType) -> OCRBackend[Any]:
    if backend == "easyocr":
        return EasyOCRBackend(config=EasyOCRConfig())
    if backend == "paddleocr":
        return PaddleOCRBackend(config=PaddleOCRConfig())
    return TesseractBackend()
