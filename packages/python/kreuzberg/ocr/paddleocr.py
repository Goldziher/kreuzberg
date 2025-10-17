"""PaddleOCR backend for document OCR processing.

This module provides integration with PaddleOCR for optical character recognition.
PaddleOCR supports 80+ languages and is optimized for production deployments.
"""

from __future__ import annotations

import logging
from typing import Any

from kreuzberg.exceptions import MissingDependencyError, OCRError

logger = logging.getLogger(__name__)

# Supported PaddleOCR language codes
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
        use_angle_cls: Whether to enable angle classification for rotated text.
        show_log: Whether to emit PaddleOCR logs.

    Raises:
        ValidationError: If an unsupported language code is provided.

    Installation:
        pip install "kreuzberg[paddleocr]"

    Example:
        >>> from kreuzberg import register_ocr_backend
        >>> from kreuzberg.ocr.paddleocr import PaddleOCRBackend
        >>>
        >>> backend = PaddleOCRBackend(lang="en", use_gpu=True)
        >>> register_ocr_backend(backend)

    """

    def __init__(
        self,
        *,
        lang: str = "en",
        use_gpu: bool | None = None,
        use_angle_cls: bool = True,
        show_log: bool = False,
    ) -> None:
        # Validate language
        if lang not in SUPPORTED_LANGUAGES:
            from kreuzberg.exceptions import ValidationError

            msg = f"Unsupported PaddleOCR language code: {lang}"
            raise ValidationError(
                msg,
                context={
                    "language": lang,
                    "supported_languages": sorted(SUPPORTED_LANGUAGES),
                },
            )

        self.lang = lang
        self.use_angle_cls = use_angle_cls
        self.show_log = show_log

        # Determine GPU usage
        if use_gpu is None:
            self.use_gpu = self._is_cuda_available()
        else:
            self.use_gpu = use_gpu

        # OCR instance (lazy loaded)
        self._ocr: Any | None = None

    def name(self) -> str:
        """Return backend name."""
        return "paddleocr"

    def version(self) -> str:
        """Return backend version."""
        try:
            import paddleocr

            return getattr(paddleocr, "__version__", "unknown")
        except ImportError:
            return "unknown"

    def supported_languages(self) -> list[str]:
        """Return list of all supported language codes."""
        return sorted(SUPPORTED_LANGUAGES)

    def initialize(self) -> None:
        """Initialize PaddleOCR (loads models)."""
        if self._ocr is not None:
            return

        try:
            from paddleocr import PaddleOCR  # noqa: PLC0415
        except ImportError as e:
            msg = "PaddleOCR backend requires paddleocr package"
            raise MissingDependencyError(
                msg,
                context={
                    "install_command": 'pip install "kreuzberg[paddleocr]"',
                    "package": "paddleocr",
                },
            ) from e

        try:
            logger.info(
                "Initializing PaddleOCR with lang=%s, GPU=%s",
                self.lang,
                self.use_gpu,
            )

            self._ocr = PaddleOCR(
                lang=self.lang,
                use_gpu=self.use_gpu,
                use_angle_cls=self.use_angle_cls,
                show_log=self.show_log,
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
        # Ensure OCR is initialized
        if self._ocr is None:
            self.initialize()

        if self._ocr is None:
            msg = "PaddleOCR failed to initialize"
            raise RuntimeError(msg)

        # Validate language
        if language not in SUPPORTED_LANGUAGES:
            from kreuzberg.exceptions import ValidationError

            msg = f"Language '{language}' not supported by PaddleOCR"
            raise ValidationError(
                msg,
                context={"language": language, "supported_languages": sorted(SUPPORTED_LANGUAGES)},
            )

        try:
            # Import dependencies
            import io  # noqa: PLC0415

            import numpy as np  # noqa: PLC0415
            from PIL import Image  # noqa: PLC0415

            # Convert bytes to PIL Image
            image = Image.open(io.BytesIO(image_bytes))
            width, height = image.size

            # Convert to numpy array for PaddleOCR
            image_array = np.array(image)

            # Perform OCR
            result = self._ocr.ocr(image_array, cls=self.use_angle_cls)

            # Process results
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

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        """Process image file using PaddleOCR.

        Args:
            path: Path to the image file.
            language: Language code (must be in ``supported_languages()``).

        Returns:
            Dictionary in the same format as ``process_image()``.

        Raises:
            RuntimeError: If PaddleOCR fails to initialize.
            OCRError: If OCR processing fails.

        """
        # PaddleOCR can process files directly
        if self._ocr is None:
            self.initialize()

        if self._ocr is None:
            msg = "PaddleOCR failed to initialize"
            raise RuntimeError(msg)

        try:
            from PIL import Image  # noqa: PLC0415

            # Load image to get dimensions
            image = Image.open(path)
            width, height = image.size

            # Perform OCR directly on file
            result = self._ocr.ocr(path, cls=self.use_angle_cls)

            # Process results
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
        """Process PaddleOCR result and extract text content.

        Args:
            result: PaddleOCR result list

        Returns:
            Tuple of (content, average_confidence, text_region_count)

        """
        if not result or result[0] is None:
            return "", 0.0, 0

        # PaddleOCR returns: [[[bbox, (text, confidence)], ...]]
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
        """Check if CUDA is available for GPU acceleration."""
        try:
            import paddle  # noqa: PLC0415

            return paddle.device.is_compiled_with_cuda()
        except (ImportError, AttributeError):
            return False
