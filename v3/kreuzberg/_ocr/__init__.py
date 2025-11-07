from functools import lru_cache
from typing import Any

from kreuzberg._ocr._base import OCRBackend
from kreuzberg._ocr._easyocr import EasyOCRBackend
from kreuzberg._ocr._paddleocr import PaddleBackend
from kreuzberg._ocr._tesseract import TesseractBackend, TesseractProcessPool
from kreuzberg._types import OcrBackendType

__all__ = [
    "EasyOCRBackend",
    "OCRBackend",
    "PaddleBackend",
    "TesseractBackend",
    "TesseractProcessPool",
    "get_ocr_backend",
]


@lru_cache
def get_ocr_backend(backend: OcrBackendType) -> OCRBackend[Any]:
    if backend == "easyocr":
        return EasyOCRBackend()
    if backend == "paddleocr":
        return PaddleBackend()
    return TesseractBackend()
