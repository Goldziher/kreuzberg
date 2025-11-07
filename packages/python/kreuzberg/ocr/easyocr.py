"""EasyOCR backend for document OCR processing.

This module provides integration with EasyOCR for optical character recognition.
EasyOCR supports 80+ languages and can run on CPU or GPU (CUDA).
"""

from __future__ import annotations

import logging
from typing import Any

from kreuzberg.exceptions import OCRError, ValidationError

logger = logging.getLogger(__name__)

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

    Args:
        languages: Language codes to enable (default: ``["en"]``).
        use_gpu: Whether to force GPU usage. If ``None``, CUDA availability is auto-detected.
        model_storage_directory: Directory used for EasyOCR model cache.
        beam_width: Beam width for recognition (higher values are slower but more accurate).

    Raises:
        ImportError: If the easyocr package is not installed.
        ValidationError: If any supplied language code is not supported.

    Note:
        All parameters are keyword-only. Python will raise TypeError if invalid
        parameters are passed, providing automatic validation.

    Installation:
        pip install "kreuzberg[easyocr]"

    Example:
        >>> from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig
        >>> # Register backend with custom options via extraction API
        >>> config = ExtractionConfig(ocr=OcrConfig(backend="easyocr", language="en"))
        >>> result = extract_file_sync("scanned.pdf", config=config, easyocr_kwargs={"use_gpu": True, "beam_width": 10})

    """

    def __init__(
        self,
        *,
        languages: list[str] | None = None,
        use_gpu: bool | None = None,
        model_storage_directory: str | None = None,
        beam_width: int = 5,
    ) -> None:
        try:
            import easyocr as easyocr_module  # noqa: PLC0415
        except ImportError as e:
            msg = "EasyOCR support requires the 'easyocr' package. Install with: pip install \"kreuzberg[easyocr]\""
            raise ImportError(msg) from e

        self._easyocr_module = easyocr_module

        self.languages = languages or ["en"]
        self.beam_width = beam_width
        self.model_storage_directory = model_storage_directory

        unsupported = [lang for lang in self.languages if lang not in SUPPORTED_LANGUAGES]
        if unsupported:
            msg = f"Unsupported EasyOCR language codes: {', '.join(unsupported)}"
            raise ValidationError(
                msg,
                context={
                    "unsupported_languages": unsupported,
                    "supported_languages": sorted(SUPPORTED_LANGUAGES),
                },
            )

        if use_gpu is None:
            self.use_gpu = self._is_cuda_available()
        else:
            self.use_gpu = use_gpu

        self._reader: Any | None = None

    def name(self) -> str:
        """Return backend name."""
        return "easyocr"

    def supported_languages(self) -> list[str]:
        """Return list of all supported language codes."""
        return sorted(SUPPORTED_LANGUAGES)

    def initialize(self) -> None:
        """Initialize EasyOCR reader (loads models)."""
        if self._reader is not None:
            return

        try:
            logger.info(
                "Initializing EasyOCR reader with languages=%s, GPU=%s",
                self.languages,
                self.use_gpu,
            )

            self._reader = self._easyocr_module.Reader(
                self.languages,
                gpu=self.use_gpu,
                verbose=False,
                model_storage_directory=self.model_storage_directory,
            )

            logger.info("EasyOCR reader initialized successfully")
        except Exception as e:
            msg = f"Failed to initialize EasyOCR: {e}"
            raise OCRError(msg) from e

    def shutdown(self) -> None:
        """Shutdown backend and cleanup resources."""
        self._reader = None
        logger.info("EasyOCR backend shutdown")

    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]:
        """Process image bytes and extract text using EasyOCR.

        Args:
            image_bytes: Raw image data.
            language: Language code (must be in ``supported_languages()``).

        Returns:
            Dictionary with the format:
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
            ValidationError: If the supplied language is not supported.
            RuntimeError: If EasyOCR fails to initialize.
            OCRError: If OCR processing fails.

        """
        if self._reader is None:
            self.initialize()

        if self._reader is None:
            msg = "EasyOCR reader failed to initialize"
            raise RuntimeError(msg)

        if language not in SUPPORTED_LANGUAGES:
            msg = f"Language '{language}' not supported by EasyOCR"
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

            result = self._reader.readtext(
                image_array,
                beamWidth=self.beam_width,
            )

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
            msg = f"EasyOCR processing failed: {e}"
            raise OCRError(msg) from e

    def process_file(self, path: str, language: str) -> dict[str, Any]:
        """Process image file using EasyOCR.

        Args:
            path: Path to the image file.
            language: Language code (must be in ``supported_languages()``).

        Returns:
            Dictionary in the same format as ``process_image()``.

        Note:
            Exceptions from :meth:`process_image` propagate unchanged.

        """
        from pathlib import Path  # noqa: PLC0415

        with Path(path).open("rb") as f:
            image_bytes = f.read()

        return self.process_image(image_bytes, language)

    @staticmethod
    def _process_easyocr_result(result: list[Any]) -> tuple[str, float, int]:
        if not result:
            return "", 0.0, 0

        if all(len(item) == 2 for item in result):
            text_parts = []
            total_confidence = 0.0
            for text, confidence in result:
                if text:
                    text_parts.append(text)
                    total_confidence += confidence

            content = "\n".join(text_parts)
            avg_confidence = total_confidence / len(result) if result else 0.0
            return content, avg_confidence, len(result)

        sorted_results = sorted(result, key=lambda x: x[0][0][1] + x[0][2][1])

        line_groups: list[list[Any]] = []
        current_line: list[Any] = []
        prev_y_center: float | None = None
        line_height_threshold = 20

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

        text_parts = []
        total_confidence = 0.0
        text_count = 0

        for line in line_groups:
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
        try:
            import torch  # noqa: PLC0415

            return bool(torch.cuda.is_available())
        except ImportError:
            return False
