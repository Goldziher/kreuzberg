"""PaddleOCR backend for document OCR processing.

This module provides integration with PaddleOCR for optical character recognition.
PaddleOCR supports 80+ languages and is optimized for production deployments.
"""

from __future__ import annotations

import logging
from typing import Any

from kreuzberg.exceptions import OCRError, ValidationError

logger = logging.getLogger(__name__)

SUPPORTED_LANGUAGES = {
    "ch",
    "en",
    "french",
    "german",
    "korean",
    "japan",
    "chinese_cht",
    "ta",
    "te",
    "ka",
    "latin",
    "arabic",
    "cyrillic",
    "devanagari",
}


class PaddleOCRBackend:
    """PaddleOCR backend for OCR processing.

    This backend uses PaddleOCR for text extraction from images. It supports
    80+ languages and can run on CPU or GPU (CUDA).

    Args:
        lang: Language code (default: "en").
        use_gpu: Whether to force GPU usage. If ``None``, CUDA availability is auto-detected.
        use_textline_orientation: Whether to enable orientation classification for rotated text.

    Raises:
        ImportError: If the paddleocr package is not installed.
        ValidationError: If an unsupported language code is provided.

    Note:
        All parameters are keyword-only. Python will raise TypeError if invalid
        parameters are passed, providing automatic validation.

    Installation:
        pip install "kreuzberg[paddleocr]"

    Example:
        >>> from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig
        >>> # Register backend with custom options via extraction API
        >>> config = ExtractionConfig(ocr=OcrConfig(backend="paddleocr", language="ch"))
        >>> result = extract_file_sync("scanned.pdf", config=config, paddleocr_kwargs={"use_gpu": True})

    """

    def __init__(
        self,
        *,
        lang: str = "en",
        use_gpu: bool | None = None,
        use_textline_orientation: bool = True,
    ) -> None:
        if lang not in SUPPORTED_LANGUAGES:
            msg = f"Unsupported PaddleOCR language code: {lang}"
            raise ValidationError(
                msg,
                context={
                    "language": lang,
                    "supported_languages": sorted(SUPPORTED_LANGUAGES),
                },
            )

        try:
            from paddleocr import PaddleOCR as PaddleOCRClass  # noqa: PLC0415
        except ImportError as e:
            msg = (
                "PaddleOCR support requires the 'paddleocr' package. Install with: pip install \"kreuzberg[paddleocr]\""
            )
            raise ImportError(msg) from e

        self._paddleocr_cls = PaddleOCRClass

        self.lang = lang
        self.use_textline_orientation = use_textline_orientation

        if use_gpu is None:
            self.device = "gpu" if self._is_cuda_available() else "cpu"
        else:
            self.device = "gpu" if use_gpu else "cpu"

        self._ocr: Any | None = None

    def name(self) -> str:
        """Return backend name."""
        return "paddleocr"

    def supported_languages(self) -> list[str]:
        """Return list of all supported language codes."""
        return sorted(SUPPORTED_LANGUAGES)

    def initialize(self) -> None:
        """Initialize PaddleOCR (loads models)."""
        if self._ocr is not None:
            return

        try:
            logger.info(
                "Initializing PaddleOCR with lang=%s, device=%s",
                self.lang,
                self.device,
            )

            self._ocr = self._paddleocr_cls(
                lang=self.lang,
                device=self.device,
                use_textline_orientation=self.use_textline_orientation,
            )

            logger.info("PaddleOCR initialized successfully")
        except Exception as e:
            msg = f"Failed to initialize PaddleOCR: {e}"
            raise OCRError(msg) from e

    def shutdown(self) -> None:
        """Shutdown backend and cleanup resources."""
        self._ocr = None
        logger.info("PaddleOCR backend shutdown")

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        """Process image bytes and extract text using PaddleOCR.

        Args:
            image_bytes: Raw image data.
            language: Language code (must be in ``supported_languages()``).

        Returns:
            Dictionary with the structure:
            {
                "content": "extracted text",  # Concatenated text content.
                "metadata": {
                    "width": 800,
                    "height": 600,
                    "confidence": 0.95,
                    "text_regions": 42
                }
            }

        Raises:
            ValidationError: If the supplied language is not supported.
            RuntimeError: If PaddleOCR fails to initialize.
            OCRError: If OCR processing fails.

        """
        if self._ocr is None:
            self.initialize()

        if self._ocr is None:
            msg = "PaddleOCR failed to initialize"
            raise RuntimeError(msg)

        if language not in SUPPORTED_LANGUAGES:
            msg = f"Language '{language}' not supported by PaddleOCR"
            raise ValidationError(
                msg,
                context={"language": language, "supported_languages": sorted(SUPPORTED_LANGUAGES)},
            )

        try:
            import io  # noqa: PLC0415

            import numpy as np  # noqa: PLC0415
            from PIL import Image  # noqa: PLC0415

            image = Image.open(io.BytesIO(image_bytes))
            width, height = image.size

            image_array = np.array(image)

            result = self._ocr.predict(image_array)

            content, confidence, text_regions = self._process_paddleocr_result(result)

            return {
                "content": content,
                "metadata": {
                    "width": width,
                    "height": height,
                    "confidence": confidence,
                    "text_regions": text_regions,
                },
            }

        except Exception as e:
            msg = f"PaddleOCR processing failed: {e}"
            raise OCRError(msg) from e

    def process_file(self, path: str, _language: str) -> dict[str, Any]:
        """Process image file using PaddleOCR.

        Args:
            path: Path to the image file.
            _language: Language code (unused - PaddleOCR uses language from initialization).

        Returns:
            Dictionary in the same format as ``process_image()``.

        Raises:
            RuntimeError: If PaddleOCR fails to initialize.
            OCRError: If OCR processing fails.

        """
        if self._ocr is None:
            self.initialize()

        if self._ocr is None:
            msg = "PaddleOCR failed to initialize"
            raise RuntimeError(msg)

        try:
            from PIL import Image  # noqa: PLC0415

            image = Image.open(path)
            width, height = image.size

            result = self._ocr.predict(path)

            content, confidence, text_regions = self._process_paddleocr_result(result)

            return {
                "content": content,
                "metadata": {
                    "width": width,
                    "height": height,
                    "confidence": confidence,
                    "text_regions": text_regions,
                },
            }

        except Exception as e:
            msg = f"PaddleOCR file processing failed: {e}"
            raise OCRError(msg) from e

    @staticmethod
    def _process_paddleocr_result(result: list[Any] | None) -> tuple[str, float, int]:
        if not result or result[0] is None:
            return "", 0.0, 0

        page_result = result[0]

        text_parts = []
        total_confidence = 0.0
        text_count = 0

        for line in page_result:
            if isinstance(line, (list, tuple)) and len(line) >= 2:
                text_info = line[1]
                if isinstance(text_info, (list, tuple)) and len(text_info) >= 2:
                    text, confidence = text_info[0], text_info[1]
                    if text:
                        text_parts.append(str(text))
                        total_confidence += float(confidence)
                        text_count += 1

        content = "\n".join(text_parts)
        avg_confidence = total_confidence / text_count if text_count > 0 else 0.0

        return content, avg_confidence, text_count

    @staticmethod
    def _is_cuda_available() -> bool:
        try:
            import paddle  # noqa: PLC0415

            return paddle.device.is_compiled_with_cuda()
        except (ImportError, AttributeError):
            return False
