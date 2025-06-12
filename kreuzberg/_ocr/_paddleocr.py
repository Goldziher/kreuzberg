from __future__ import annotations

import logging
import platform
from dataclasses import dataclass, field
from importlib.util import find_spec
from typing import TYPE_CHECKING, Any, Final, Literal

from PIL import Image

from kreuzberg._mime_types import PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult, Metadata
from kreuzberg._utils._device import DeviceType, get_device_memory_info, set_memory_limit, validate_device
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

logger = logging.getLogger(__name__)
if TYPE_CHECKING:
    from pathlib import Path


try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


PADDLEOCR_SUPPORTED_LANGUAGE_CODES: Final[set[str]] = {"ch", "en", "french", "german", "japan", "korean"}


@dataclass(unsafe_hash=True, frozen=True)
class PaddleOCRConfig:
    """Configuration options for PaddleOCR.

    This TypedDict provides type hints and documentation for all PaddleOCR parameters.
    """

    cls_image_shape: str = "3,48,192"
    """Image shape for classification algorithm in format 'channels,height,width'."""
    det_algorithm: Literal["DB", "EAST", "SAST", "PSE", "FCE", "PAN", "CT", "DB++", "Layout"] = "DB"
    """Detection algorithm."""
    det_db_box_thresh: float = 0.5
    """Score threshold for detected boxes. Boxes below this value are discarded."""
    det_db_thresh: float = 0.3
    """Binarization threshold for DB output map."""
    det_db_unclip_ratio: float = 2.0
    """Expansion ratio for detected text boxes."""
    det_east_cover_thresh: float = 0.1
    """Score threshold for EAST output boxes."""
    det_east_nms_thresh: float = 0.2
    """NMS threshold for EAST model output boxes."""
    det_east_score_thresh: float = 0.8
    """Binarization threshold for EAST output map."""
    det_max_side_len: int = 960
    """Maximum size of image long side. Images exceeding this will be proportionally resized."""
    det_model_dir: str | None = None
    """Directory for detection model. If None, uses default model location."""
    drop_score: float = 0.5
    """Filter recognition results by confidence score. Results below this are discarded."""
    enable_mkldnn: bool = False
    """Whether to enable MKL-DNN acceleration (Intel CPU only)."""
    gpu_mem: int = 8000
    """GPU memory size (in MB) to use for initialization."""
    language: list[str] = field(default_factory=lambda: ["en"])
    """List of languages to use for OCR."""
    max_text_length: int = 25
    """Maximum text length that the recognition algorithm can recognize."""
    rec: bool = True
    """Enable text recognition when using the ocr() function."""
    rec_algorithm: Literal[
        "CRNN",
        "SRN",
        "NRTR",
        "SAR",
        "SEED",
        "SVTR",
        "SVTR_LCNet",
        "ViTSTR",
        "ABINet",
        "VisionLAN",
        "SPIN",
        "RobustScanner",
        "RFL",
    ] = "CRNN"
    """Recognition algorithm."""
    rec_image_shape: str = "3,32,320"
    """Image shape for recognition algorithm in format 'channels,height,width'."""
    rec_model_dir: str | None = None
    """Directory for recognition model. If None, uses default model location."""
    table: bool = True
    """Whether to enable table recognition."""
    use_angle_cls: bool = True
    """Whether to use text orientation classification model."""
    use_gpu: bool = False
    """Whether to use GPU for inference. Requires installing the paddlepaddle-gpu package"""
    use_space_char: bool = True
    """Whether to recognize spaces."""
    use_zero_copy_run: bool = False
    """Whether to enable zero_copy_run for inference optimization."""
    gpu_memory_limit: float | None = None
    """GPU memory limit in GB. None means no limit."""
    device: DeviceType = "auto"
    """Device to use for computation. Can be 'cpu', 'cuda', 'mps', or 'auto'."""
    image_dir: str | None = None
    """Directory containing images to process."""
    det_limit_side_len: int = 960
    """Maximum size of image long side for detection."""
    det_limit_type: Literal["min", "max"] = "max"
    """Type of size limit for detection."""
    max_batch_size: int = 10
    """Maximum batch size for detection."""
    use_dilation: bool = False
    """Whether to use dilation for detection."""
    det_db_score_mode: Literal["fast", "slow"] = "fast"
    """Score mode for DB detection."""
    rec_batch_size: int = 6
    """Batch size for recognition."""
    rec_char_dict_path: str | None = None
    """Path to character dictionary for recognition."""
    use_mp: bool = False
    """Whether to use multiprocessing."""
    total_process_num: int = 1
    """Total number of processes to use."""


class PaddleOCRBackend(OCRBackend[PaddleOCRConfig]):
    """PaddleOCR backend implementation."""

    _paddle_ocr: Any | None = None
    """Class-level PaddleOCR instance."""

    @classmethod
    def get_paddle_ocr(cls) -> Any | None:
        """Get the class-level PaddleOCR instance."""
        return cls._paddle_ocr

    @classmethod
    def set_paddle_ocr(cls, instance: Any | None) -> None:
        """Set the class-level PaddleOCR instance."""
        cls._paddle_ocr = instance

    def __init__(self, config: PaddleOCRConfig) -> None:
        self.config = config
        self._ocr: Any | None = None
        self._device = validate_device(config.device, "paddleocr")

        if config.use_gpu and config.gpu_memory_limit is not None:
            set_memory_limit(self._device, config.gpu_memory_limit)

    def _initialize(self) -> None:
        """Initialize the PaddleOCR reader."""
        if self._ocr is None:
            try:
                from paddleocr import PaddleOCR
            except ImportError as e:
                raise MissingDependencyError.create_for_package(
                    dependency_group="paddleocr", functionality="OCR processing", package_name="paddleocr"
                ) from e

            self._ocr = PaddleOCR(
                use_angle_cls=self.config.use_angle_cls,
                lang=self.config.language[0],  # PaddleOCR only supports one language at a time
                use_gpu=self.config.use_gpu,
                gpu_mem=self.config.gpu_mem,
                image_dir=self.config.image_dir,
                det_algorithm=self.config.det_algorithm,
                det_model_dir=self.config.det_model_dir,
                det_limit_side_len=self.config.det_limit_side_len,
                det_limit_type=self.config.det_limit_type,
                det_db_thresh=self.config.det_db_thresh,
                det_db_box_thresh=self.config.det_db_box_thresh,
                det_db_unclip_ratio=self.config.det_db_unclip_ratio,
                max_batch_size=self.config.max_batch_size,
                use_dilation=self.config.use_dilation,
                det_db_score_mode=self.config.det_db_score_mode,
                rec_algorithm=self.config.rec_algorithm,
                rec_model_dir=self.config.rec_model_dir,
                rec_image_shape=self.config.rec_image_shape,
                rec_batch_size=self.config.rec_batch_size,
                rec_char_dict_path=self.config.rec_char_dict_path,
                use_space_char=self.config.use_space_char,
                drop_score=self.config.drop_score,
                use_mp=self.config.use_mp,
                total_process_num=self.config.total_process_num,
                enable_mkldnn=self.config.enable_mkldnn,
                use_zero_copy_run=self.config.use_zero_copy_run,
                device=self._device,
            )
            self.__class__.set_paddle_ocr(self._ocr)

    def _process_image(self, image: Image.Image) -> str:
        """Process an image using PaddleOCR.

        Args:
            image: PIL Image to process

        Returns:
            str: Extracted text from the image

        Raises:
            OCRError: If OCR processing fails
        """
        if self._ocr is None:
            raise OCRError("PaddleOCR not initialized")

        try:
            results = self._ocr.ocr(image, cls=True)

            if self.config.gpu_memory_limit is not None:
                memory_info = get_device_memory_info("cuda")
                if memory_info:
                    logger.debug(
                        "GPU memory usage: %.2f GB allocated, %.2f GB free",
                        memory_info["allocated"] / 1024**3,
                        memory_info["free"] / 1024**3,
                    )

            text: list[str] = []
            for line in results:
                text.extend(word_info[1][0] for word_info in line)  # Get the recognized text

            return "\n".join(text)

        except Exception as e:
            raise OCRError(f"PaddleOCR processing failed: {e!s}") from e

    async def process_file(self, path: Path, **kwargs: Unpack[PaddleOCRConfig]) -> ExtractionResult:
        """Asynchronously process a file and extract its text and metadata using PaddleOCR.

        Args:
            path: A Path object representing the file to be processed.
            **kwargs: Configuration parameters for PaddleOCR including language, detection thresholds, etc.

        Returns:
            ExtractionResult: The extraction result containing text content, mime type, and metadata.

        Raises:
            OCRError: If file loading or OCR processing fails.
        """
        await self._init_paddle_ocr(**kwargs)
        try:
            image = await run_sync(Image.open, path)
            return await self.process_image(image, **kwargs)
        except Exception as e:
            raise OCRError(f"Failed to load or process image using PaddleOCR: {e}") from e

    async def process_image(self, image: Image.Image, **kwargs: Unpack[PaddleOCRConfig]) -> ExtractionResult:
        """Asynchronously process an image and extract its text and metadata using PaddleOCR.

        Args:
            image: An instance of PIL.Image representing the input image.
            **kwargs: Configuration parameters for PaddleOCR including language, detection thresholds, etc.

        Returns:
            ExtractionResult: The extraction result containing text content, mime type, and metadata.

        Raises:
            OCRError: If OCR processing fails.
        """
        await self._init_paddle_ocr(**kwargs)
        try:
            text = await run_sync(self._process_image, image)
            return ExtractionResult(
                content=normalize_spaces(text),
                mime_type=PLAIN_TEXT_MIME_TYPE,
                metadata=Metadata(width=image.width, height=image.height),
                chunks=[],
            )
        except Exception as e:
            raise OCRError(f"Failed to OCR using PaddleOCR: {e}") from e

    @staticmethod
    def _process_paddle_result(result: list[Any], image: Image.Image) -> ExtractionResult:
        """Process PaddleOCR result into an ExtractionResult with metadata.

        Args:
            result: The raw result from PaddleOCR.
            image: The original PIL image.

        Returns:
            ExtractionResult: The extraction result containing text content, mime type, and metadata.
        """
        text_content = ""
        confidence_sum = 0
        confidence_count = 0

        for page_result in result:
            if not page_result:
                continue

            sorted_boxes = sorted(page_result, key=lambda x: x[0][0][1])
            line_groups: list[list[Any]] = []
            current_line: list[Any] = []
            prev_y: float | None = None

            for box in sorted_boxes:
                box_points, (_, _) = box
                current_y = sum(point[1] for point in box_points) / 4
                min_box_distance = 20

                if prev_y is None or abs(current_y - prev_y) > min_box_distance:
                    if current_line:
                        line_groups.append(current_line)
                    current_line = [box]
                else:
                    current_line.append(box)

                prev_y = current_y

            if current_line:
                line_groups.append(current_line)

            for line in line_groups:
                line_sorted = sorted(line, key=lambda x: x[0][0][0])

                for box in line_sorted:
                    _, (text, confidence) = box
                    if text:
                        text_content += text + " "
                        confidence_sum += confidence
                        confidence_count += 1

                text_content += "\n"

        width, height = image.size
        metadata = Metadata(
            width=width,
            height=height,
        )

        return ExtractionResult(
            content=normalize_spaces(text_content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata=metadata, chunks=[]
        )

    @classmethod
    def _is_mkldnn_supported(cls) -> bool:
        """Check if the current architecture supports MKL-DNN optimization.

        Returns:
            True if MKL-DNN is supported on this architecture.
        """
        system = platform.system().lower()
        processor = platform.processor().lower()
        machine = platform.machine().lower()

        if system in ("linux", "windows"):
            return "intel" in processor or "x86" in machine or "amd64" in machine or "x86_64" in machine

        if system == "darwin":
            return machine == "x86_64"

        return False

    @classmethod
    async def _init_paddle_ocr(cls, **kwargs: Unpack[PaddleOCRConfig]) -> None:
        """Initialize PaddleOCR with the provided configuration.

        Args:
            **kwargs: Configuration parameters for PaddleOCR including language, detection thresholds, etc.

        Raises:
            MissingDependencyError: If PaddleOCR is not installed.
            OCRError: If initialization fails.
        """
        if cls.get_paddle_ocr() is not None:
            return

        try:
            from paddleocr import PaddleOCR
        except ImportError as e:
            raise MissingDependencyError.create_for_package(
                dependency_group="paddleocr", functionality="OCR processing", package_name="paddleocr"
            ) from e

        language = cls._validate_language_code(kwargs.pop("language", "en"))
        has_gpu_package = bool(find_spec("paddlepaddle_gpu"))
        kwargs.setdefault("use_angle_cls", True)
        kwargs.setdefault("use_gpu", has_gpu_package)
        kwargs.setdefault("enable_mkldnn", cls._is_mkldnn_supported() and not has_gpu_package)
        kwargs.setdefault("det_db_thresh", 0.3)
        kwargs.setdefault("det_db_box_thresh", 0.5)
        kwargs.setdefault("det_db_unclip_ratio", 1.6)

        try:
            cls.set_paddle_ocr(await run_sync(PaddleOCR, lang=language, show_log=False, **kwargs))
        except Exception as e:
            raise OCRError(f"Failed to initialize PaddleOCR: {e}") from e

    @staticmethod
    def _validate_language_code(lang_code: str) -> str:
        """Convert a language code to PaddleOCR format.

        Args:
            lang_code: ISO language code or language name

        Raises:
            ValidationError: If the language is not supported by PaddleOCR

        Returns:
            Language code compatible with PaddleOCR
        """
        normalized = lang_code.lower()
        if normalized in PADDLEOCR_SUPPORTED_LANGUAGE_CODES:
            return normalized

        raise ValidationError(
            "The provided language code is not supported by PaddleOCR",
            context={
                "language_code": lang_code,
                "supported_languages": ",".join(sorted(PADDLEOCR_SUPPORTED_LANGUAGE_CODES)),
            },
        )
