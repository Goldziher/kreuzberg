from __future__ import annotations

from kreuzberg._ocr import get_ocr_backend
from kreuzberg._ocr._easyocr import EasyOCRBackend
from kreuzberg._ocr._paddleocr import PaddleBackend
from kreuzberg._ocr._tesseract import TesseractBackend


def test_get_ocr_backend_easyocr() -> None:
    backend = get_ocr_backend("easyocr")
    assert isinstance(backend, EasyOCRBackend)


def test_get_ocr_backend_paddleocr() -> None:
    backend = get_ocr_backend("paddleocr")
    assert isinstance(backend, PaddleBackend)


def test_get_ocr_backend_tesseract() -> None:
    backend = get_ocr_backend("tesseract")
    assert isinstance(backend, TesseractBackend)


def test_get_ocr_backend_caching() -> None:
    backend1 = get_ocr_backend("easyocr")
    backend2 = get_ocr_backend("easyocr")
    assert backend1 is backend2

    backend3 = get_ocr_backend("paddleocr")
    backend4 = get_ocr_backend("paddleocr")
    assert backend3 is backend4

    backend5 = get_ocr_backend("tesseract")
    backend6 = get_ocr_backend("tesseract")
    assert backend5 is backend6
