"""Python OCR backend implementations.

These backends can be imported and manually registered, or they will be
auto-registered when kreuzberg is imported (if their dependencies are installed).

Each backend has a separate optional dependency group:
- EasyOCR: pip install "kreuzberg[easyocr]"
- PaddleOCR: pip install "kreuzberg[paddleocr]"
"""

from __future__ import annotations

__all__ = ["EasyOCRBackend", "OcrBackendProtocol", "PaddleOCRBackend"]

from kreuzberg.ocr.protocol import OcrBackendProtocol

# Make backends available for direct import (if dependencies are installed)
try:
    from kreuzberg.ocr.easyocr import EasyOCRBackend
except ImportError:
    EasyOCRBackend = None  # type: ignore[assignment,misc]

try:
    from kreuzberg.ocr.paddleocr import PaddleOCRBackend
except ImportError:
    PaddleOCRBackend = None  # type: ignore[assignment,misc]
