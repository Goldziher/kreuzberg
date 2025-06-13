from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Final, Literal

import numpy as np
from PIL import Image

from kreuzberg._mime_types import PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult, Metadata
from kreuzberg._utils._device import DeviceType, set_memory_limit, validate_device
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path

try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


EASYOCR_SUPPORTED_LANGUAGE_CODES: Final[set[str]] = {
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
    "inh",  # codespell:ignore
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
    "te",  # codespell:ignore
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


@dataclass(unsafe_hash=True, frozen=True)
class EasyOCRConfig:
    """Configuration options for EasyOCR."""

    add_margin: float = 0.1
    """Extend bounding boxes in all directions."""
    adjust_contrast: float = 0.5
    """Target contrast level for low contrast text."""
    beam_width: int = 5
    """Beam width for beam search in recognition."""
    canvas_size: int = 2560
    """Maximum image dimension for detection."""
    contrast_ths: float = 0.1
    """Contrast threshold for preprocessing."""
    decoder: Literal["greedy", "beamsearch", "wordbeamsearch"] = "greedy"
    """Decoder method. Options: 'greedy', 'beamsearch', 'wordbeamsearch'."""
    height_ths: float = 0.5
    """Maximum difference in box height for merging."""
    language: list[str] = field(default_factory=lambda: ["en"])
    """Language or languages to use for OCR. Can be a single language code (e.g., 'en'),
    a comma-separated string of language codes (e.g., 'en,ch_sim'), or a list of language codes."""
    link_threshold: float = 0.4
    """Link confidence threshold."""
    low_text: float = 0.4
    """Text low-bound score."""
    mag_ratio: float = 1.0
    """Image magnification ratio."""
    min_size: int = 10
    """Minimum text box size in pixels."""
    rotation_info: list[int] | None = None
    """List of angles to try for detection."""
    slope_ths: float = 0.1
    """Maximum slope for merging text boxes."""
    text_threshold: float = 0.7
    """Text confidence threshold."""
    use_gpu: bool = False
    """Whether to use GPU acceleration."""
    width_ths: float = 0.5
    """Maximum horizontal distance for merging boxes."""
    x_ths: float = 1.0
    """Maximum horizontal distance for paragraph merging."""
    y_ths: float = 0.5
    """Maximum vertical distance for paragraph merging."""
    ycenter_ths: float = 0.5
    """Maximum shift in y direction for merging."""
    gpu_memory_limit: float | None = None
    """GPU memory limit in GB. None means no limit."""
    device: DeviceType = "auto"
    """Device to use for computation. Can be 'cpu', 'cuda', 'mps', or 'auto'."""


class EasyOCRBackend(OCRBackend[EasyOCRConfig]):
    """EasyOCR backend implementation."""

    __slots__ = ("_device", "_reader", "config")

    def __init__(self, config: EasyOCRConfig) -> None:
        self.config = config
        self._reader = None
        self._device = validate_device(config.device, "easyocr")

        if config.use_gpu and config.gpu_memory_limit is not None:
            set_memory_limit(self._device, config.gpu_memory_limit)

    def _initialize(self) -> None:
        """Initialize the EasyOCR reader."""
        if self._reader is None:
            try:
                import importlib.util

                if importlib.util.find_spec("easyocr") is None:
                    raise ImportError("easyocr not found")
                import easyocr
            except ImportError as e:
                raise MissingDependencyError.create_for_package(
                    dependency_group="easyocr", functionality="EasyOCR as an OCR backend", package_name="easyocr"
                ) from e

            self._reader = easyocr.Reader(self.config.language, gpu=self.config.use_gpu)

    async def process_image(self, image: Image.Image, **kwargs: Unpack[EasyOCRConfig]) -> ExtractionResult:
        """Asynchronously process an image and extract its text and metadata using EasyOCR.

        Args:
            image: An instance of PIL.Image representing the input image.
            **kwargs: Configuration parameters for EasyOCR including language, detection thresholds, etc.

        Returns:
            ExtractionResult: The extraction result containing text content, mime type, and metadata.

        Raises:
            OCRError: If OCR processing fails.
        """
        await self._init_easyocr(**kwargs)

        if self._reader is None:
            raise OCRError("EasyOCR reader not initialized")

        beam_width = kwargs.pop("beam_width")

        kwargs.pop("language", None)
        kwargs.pop("use_gpu", None)

        try:
            result = await run_sync(
                self._reader.readtext,
                np.array(image),
                beamWidth=beam_width,
                **kwargs,
            )

            return self._process_easyocr_result(result, image)
        except Exception as e:
            raise OCRError(f"Failed to OCR using EasyOCR: {e}") from e

    async def process_file(self, path: Path, **kwargs: Unpack[EasyOCRConfig]) -> ExtractionResult:
        """Asynchronously process a file and extract its text and metadata using EasyOCR.

        Args:
            path: A Path object representing the file to be processed.
            **kwargs: Configuration parameters for EasyOCR including language, detection thresholds, etc.

        Returns:
            ExtractionResult: The extraction result containing text content, mime type, and metadata.

        Raises:
            OCRError: If file loading or OCR processing fails.
        """
        await self._init_easyocr(**kwargs)
        try:
            image = await run_sync(Image.open, path)
            return await self.process_image(image, **kwargs)
        except Exception as e:
            raise OCRError(f"Failed to load or process image using EasyOCR: {e}") from e

    @staticmethod
    def _process_easyocr_result(result: list[Any], image: Image.Image) -> ExtractionResult:
        """Process EasyOCR result into an ExtractionResult with metadata.

        Args:
            result: The raw result from EasyOCR.
            image: The original PIL image.

        Returns:
            ExtractionResult: The extraction result containing text content, mime type, and metadata.
        """
        if not result:
            return ExtractionResult(
                content="",
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=Metadata(width=image.width, height=image.height),
                chunks=[],
            )

        expected_tuple_length = 2

        if all(len(item) == expected_tuple_length for item in result):
            text_content = ""
            confidence_sum = 0
            confidence_count = 0

            for text, confidence in result:
                if text:
                    text_content += text + "\n"
                    confidence_sum += confidence
                    confidence_count += 1

            metadata = Metadata(
                width=image.width,
                height=image.height,
            )

            return ExtractionResult(
                content=normalize_spaces(text_content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata=metadata, chunks=[]
            )

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

        text_content = ""
        confidence_sum = 0
        confidence_count = 0

        for line in line_groups:
            line_sorted = sorted(line, key=lambda x: x[0][0][0])

            for item in line_sorted:
                _, text, confidence = item
                if text:
                    text_content += text + " "
                    confidence_sum += confidence
                    confidence_count += 1

            text_content += "\n"

        metadata = Metadata(
            width=image.width,
            height=image.height,
        )

        return ExtractionResult(
            content=normalize_spaces(text_content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata=metadata, chunks=[]
        )

    @classmethod
    def _is_gpu_available(cls) -> bool:
        """Check if GPU is available for EasyOCR.

        Returns:
            bool: True if GPU support is available.
        """
        try:
            import torch

            return torch.cuda.is_available()
        except ImportError:
            return False

    @classmethod
    async def _init_easyocr(cls, **kwargs: Unpack[EasyOCRConfig]) -> None:
        """Initialize EasyOCR with the provided configuration.

        Args:
            **kwargs: Configuration parameters for EasyOCR including language, etc.

        Raises:
            MissingDependencyError: If EasyOCR is not installed or if there's an error importing it.
            OCRError: If initialization fails after EasyOCR is successfully imported.
            Exception: For any other unexpected errors during initialization.
        """
        if not cls._is_gpu_available() and kwargs.get("use_gpu", False):
            kwargs["use_gpu"] = False
            kwargs["device"] = "cpu"

        try:
            import importlib.util

            easyocr_spec = importlib.util.find_spec("easyocr")
            if easyocr_spec is None:
                raise MissingDependencyError.create_for_package(
                    dependency_group="easyocr", functionality="EasyOCR as an OCR backend", package_name="easyocr"
                )
        except Exception as e:
            if isinstance(e, MissingDependencyError):
                raise
            raise MissingDependencyError.create_for_package(
                dependency_group="easyocr", functionality="EasyOCR as an OCR backend", package_name="easyocr"
            ) from e

        try:
            instance = cls(EasyOCRConfig(**kwargs))
            instance._initialize()
        except Exception as e:
            raise OCRError(f"Failed to initialize EasyOCR: {e}") from e

    @staticmethod
    def _validate_language_code(language_codes: str | list[str]) -> list[str]:
        """Validate and normalize provided language codes.

        Args:
            language_codes: The language code(s), either as a string (single or comma-separated) or a list.

        Raises:
            ValidationError: If any of the languages are not supported by EasyOCR

        Returns:
            A list with the normalized language codes.
        """
        if isinstance(language_codes, str):
            # Handle comma-separated language codes
            languages = [lang.strip().lower() for lang in language_codes.split(",")]
        else:
            languages = [lang.lower() for lang in language_codes]

        unsupported_langs = [lang for lang in languages if lang not in EASYOCR_SUPPORTED_LANGUAGE_CODES]
        if unsupported_langs:
            raise ValidationError(
                "The provided language codes are not supported by EasyOCR",
                context={
                    "language_code": ",".join(unsupported_langs),
                    "supported_languages": ",".join(sorted(EASYOCR_SUPPORTED_LANGUAGE_CODES)),
                },
            )

        return languages
