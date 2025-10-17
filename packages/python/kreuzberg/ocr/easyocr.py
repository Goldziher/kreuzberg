"""EasyOCR backend for document OCR processing.

This module provides integration with EasyOCR for optical character recognition.
EasyOCR supports 80+ languages and can run on CPU or GPU (CUDA).
"""

from __future__ import annotations

import logging
from typing import Any

from kreuzberg.exceptions import MissingDependencyError, OCRError

logger = logging.getLogger(__name__)

# Supported EasyOCR language codes
SUPPORTED_LANGUAGES = {
    "abq",
    "ady",
    "af",
    "ang",
    "ar",
    "as",
    "ava",
    "az",
    "be",
    "bg",
    "bh",
    "bho",
    "bn",
    "bs",
    "ch_sim",
    "ch_tra",
    "che",
    "cs",
    "cy",
    "da",
    "dar",
    "de",
    "en",
    "es",
    "et",
    "fa",
    "fr",
    "ga",
    "gom",
    "hi",
    "hr",
    "hu",
    "id",
    "inh",
    "is",
    "it",
    "ja",
    "kbd",
    "kn",
    "ko",
    "ku",
    "la",
    "lbe",
    "lez",
    "lt",
    "lv",
    "mah",
    "mai",
    "mi",
    "mn",
    "mr",
    "ms",
    "mt",
    "ne",
    "new",
    "nl",
    "no",
    "oc",
    "pi",
    "pl",
    "pt",
    "ro",
    "ru",
    "rs_cyrillic",
    "rs_latin",
    "sck",
    "sk",
    "sl",
    "sq",
    "sv",
    "sw",
    "ta",
    "tab",
    "te",
    "th",
    "tjk",
    "tl",
    "tr",
    "ug",
    "uk",
    "ur",
    "uz",
    "vi",
}


class EasyOCRBackend:
    """EasyOCR backend for OCR processing.

    This backend uses EasyOCR for text extraction from images. It supports
    80+ languages and can run on both CPU and GPU (CUDA).

    Installation:
        pip install "kreuzberg[easyocr]"

    Example:
        >>> from kreuzberg import register_ocr_backend
        >>> from kreuzberg.ocr.easyocr import EasyOCRBackend
        >>>
        >>> backend = EasyOCRBackend(languages=["en"], use_gpu=True)
        >>> register_ocr_backend(backend)

    Attributes:
        languages: List of language codes to enable (default: ["en"])
        use_gpu: Whether to use GPU acceleration (default: True if available)
        model_storage_directory: Directory to cache downloaded models
        beam_width: Beam width for text recognition (higher = more accurate but slower)
    """

    def __init__(
        self,
        *,
        languages: list[str] | None = None,
        use_gpu: bool | None = None,
        model_storage_directory: str | None = None,
        beam_width: int = 5,
    ) -> None:
        """Initialize EasyOCR backend.

        Args:
            languages: List of language codes (default: ["en"])
            use_gpu: Use GPU if available (default: auto-detect)
            model_storage_directory: Directory to cache models
            beam_width: Beam width for recognition (default: 5)

        Raises:
            MissingDependencyError: If easyocr is not installed
            ValidationError: If unsupported language codes are provided
        """
        self.languages = languages or ["en"]
        self.beam_width = beam_width
        self.model_storage_directory = model_storage_directory

        # Validate languages
        unsupported = [lang for lang in self.languages if lang not in SUPPORTED_LANGUAGES]
        if unsupported:
            from kreuzberg.exceptions import ValidationError

            raise ValidationError(
                f"Unsupported EasyOCR language codes: {', '.join(unsupported)}",
                context={
                    "unsupported_languages": unsupported,
                    "supported_languages": sorted(SUPPORTED_LANGUAGES),
                },
            )

        # Determine GPU usage
        if use_gpu is None:
            # Auto-detect CUDA availability
            self.use_gpu = self._is_cuda_available()
        else:
            self.use_gpu = use_gpu

        # Reader instance (lazy loaded)
        self._reader: Any | None = None

    def name(self) -> str:
        """Return backend name."""
        return "easyocr"

    def version(self) -> str:
        """Return backend version."""
        try:
            import easyocr

            return getattr(easyocr, "__version__", "unknown")
        except ImportError:
            return "unknown"

    def supported_languages(self) -> list[str]:
        """Return list of all supported language codes."""
        return sorted(SUPPORTED_LANGUAGES)

    def initialize(self) -> None:
        """Initialize EasyOCR reader (loads models)."""
        if self._reader is not None:
            return

        try:
            import easyocr  # noqa: PLC0415
        except ImportError as e:
            raise MissingDependencyError(
                "EasyOCR backend requires easyocr package",
                context={
                    "install_command": 'pip install "kreuzberg[easyocr]"',
                    "package": "easyocr",
                },
            ) from e

        try:
            logger.info(
                "Initializing EasyOCR reader with languages=%s, GPU=%s",
                self.languages,
                self.use_gpu,
            )

            self._reader = easyocr.Reader(
                self.languages,
                gpu=self.use_gpu,
                verbose=False,
                model_storage_directory=self.model_storage_directory,
            )

            logger.info("EasyOCR reader initialized successfully")
        except Exception as e:
            raise OCRError(f"Failed to initialize EasyOCR: {e}") from e

    def shutdown(self) -> None:
        """Shutdown backend and cleanup resources."""
        self._reader = None
        logger.info("EasyOCR backend shutdown")

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        """Process image bytes and extract text using EasyOCR.

        Args:
            image_bytes: Raw image data
            language: Language code (must be in supported_languages())

        Returns:
            Dict with format:
            {
                "content": "extracted text",
                "metadata": {
                    "width": 800,
                    "height": 600,
                    "confidence": 0.95,
                    "text_regions": 42
                }
            }

        Raises:
            OCRError: If OCR processing fails
            ValidationError: If language is not supported
        """
        # Ensure reader is initialized
        if self._reader is None:
            self.initialize()

        # Validate language
        if language not in SUPPORTED_LANGUAGES:
            from kreuzberg.exceptions import ValidationError

            raise ValidationError(
                f"Language '{language}' not supported by EasyOCR",
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

            # Convert to numpy array for EasyOCR
            image_array = np.array(image)

            # Perform OCR
            result = self._reader.readtext(
                image_array,
                beamWidth=self.beam_width,
            )

            # Process results
            content, confidence, text_regions = self._process_easyocr_result(result)

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
            raise OCRError(f"EasyOCR processing failed: {e}") from e

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        """Process image file using EasyOCR.

        Args:
            path: Path to image file
            language: Language code

        Returns:
            Same format as process_image()
        """
        # Read file and call process_image
        with open(path, "rb") as f:
            image_bytes = f.read()

        return self.process_image(image_bytes, language)

    @staticmethod
    def _process_easyocr_result(result: list[Any]) -> tuple[str, float, int]:
        """Process EasyOCR result and extract text content.

        Args:
            result: EasyOCR result list

        Returns:
            Tuple of (content, average_confidence, text_region_count)
        """
        if not result:
            return "", 0.0, 0

        # Check result format
        # EasyOCR returns: [(bbox, text, confidence), ...]
        # Or with detail=0: [(text, confidence), ...]
        if all(len(item) == 2 for item in result):
            # Simplified format: (text, confidence)
            text_parts = []
            total_confidence = 0.0
            for text, confidence in result:
                if text:
                    text_parts.append(text)
                    total_confidence += confidence

            content = "\n".join(text_parts)
            avg_confidence = total_confidence / len(result) if result else 0.0
            return content, avg_confidence, len(result)

        # Full format with bounding boxes: (bbox, text, confidence)
        # Group by lines based on Y coordinate
        sorted_results = sorted(result, key=lambda x: x[0][0][1] + x[0][2][1])

        line_groups: list[list[Any]] = []
        current_line: list[Any] = []
        prev_y_center: float | None = None
        line_height_threshold = 20  # Pixels

        for item in sorted_results:
            box, text, confidence = item
            y_center = sum(point[1] for point in box) / 4

            if prev_y_center is None or abs(y_center - prev_y_center) > line_height_threshold:
                if current_line:
                    line_groups.append(current_line)
                current_line = [item]
            else:
                current_line.append(item)

            prev_y_center = y_center

        if current_line:
            line_groups.append(current_line)

        # Build text content
        text_parts = []
        total_confidence = 0.0
        text_count = 0

        for line in line_groups:
            # Sort by X coordinate within line
            line_sorted = sorted(line, key=lambda x: x[0][0][0])

            line_text = []
            for item in line_sorted:
                _, text, confidence = item
                if text:
                    line_text.append(text)
                    total_confidence += confidence
                    text_count += 1

            if line_text:
                text_parts.append(" ".join(line_text))

        content = "\n".join(text_parts)
        avg_confidence = total_confidence / text_count if text_count > 0 else 0.0

        return content, avg_confidence, text_count

    @staticmethod
    def _is_cuda_available() -> bool:
        """Check if CUDA is available for GPU acceleration."""
        try:
            import torch  # noqa: PLC0415

            return torch.cuda.is_available()
        except ImportError:
            return False
