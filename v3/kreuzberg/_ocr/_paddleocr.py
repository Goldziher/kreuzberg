from __future__ import annotations

import os
import platform
import warnings
from importlib.util import find_spec
from typing import TYPE_CHECKING, Any, ClassVar, Final

from PIL import Image

from kreuzberg._mime_types import PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult, Metadata, PaddleOCRConfig
from kreuzberg._utils._device import DeviceInfo, validate_device_request
from kreuzberg._utils._ocr_cache import (
    build_cache_kwargs,
    cache_and_complete_async,
    cache_and_complete_sync,
    generate_image_hash,
    get_file_info,
    handle_cache_lookup_async,
    handle_cache_lookup_sync,
    mark_processing_complete,
)
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import MissingDependencyError, OCRError, ValidationError

if TYPE_CHECKING:
    from pathlib import Path

try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack

if TYPE_CHECKING:
    import numpy as np
    from paddleocr import PaddleOCR
else:
    np: Any = None
    PaddleOCR: Any = None

HAS_PADDLEOCR: bool = False


def _import_paddleocr() -> tuple[Any, Any]:
    global HAS_PADDLEOCR, np, PaddleOCR

    if HAS_PADDLEOCR:
        return np, PaddleOCR

    try:
        os.environ.setdefault("HUB_DATASET_ENDPOINT", "https://modelscope.cn/api/v1/datasets")

        import numpy as _np  # noqa: PLC0415, ICN001
        from paddleocr import PaddleOCR as _PaddleOCR  # noqa: PLC0415

        np = _np
        PaddleOCR = _PaddleOCR
        HAS_PADDLEOCR = True
        return np, PaddleOCR
    except ImportError:  # pragma: no cover
        return None, None


PADDLEOCR_SUPPORTED_LANGUAGE_CODES: Final[set[str]] = {"ch", "en", "french", "german", "japan", "korean"}


class PaddleBackend(OCRBackend[PaddleOCRConfig]):
    _paddle_ocr: ClassVar[Any] = None

    async def process_image(self, image: Image.Image, **kwargs: Unpack[PaddleOCRConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            image_hash = generate_image_hash(image)
            cache_kwargs = build_cache_kwargs("paddleocr", kwargs, image_hash=image_hash)

            cached_result = await handle_cache_lookup_async(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            await self._init_paddle_ocr(**kwargs)

            if image.mode != "RGB":
                image = image.convert("RGB")

            _np, _ = _import_paddleocr()
            if _np is None:
                raise MissingDependencyError.create_for_package(
                    dependency_group="paddleocr", functionality="PaddleOCR as an OCR backend", package_name="paddleocr"
                )
            image_np = _np.array(image)
            use_textline_orientation = kwargs.get("use_textline_orientation", kwargs.get("use_angle_cls", True))
            result = await run_sync(self._paddle_ocr.ocr, image_np, cls=use_textline_orientation)

            extraction_result = self._process_paddle_result(result, image)

            if use_cache and cache_kwargs:
                await cache_and_complete_async(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to OCR using PaddleOCR: {e}") from e

    async def process_file(self, path: Path, **kwargs: Unpack[PaddleOCRConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            file_info = get_file_info(path)
            cache_kwargs = build_cache_kwargs("paddleocr", kwargs, file_info=file_info)

            cached_result = await handle_cache_lookup_async(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            await self._init_paddle_ocr(**kwargs)
            image = await run_sync(Image.open, path)

            kwargs["use_cache"] = False
            extraction_result = await self.process_image(image, **kwargs)

            if use_cache and cache_kwargs:
                await cache_and_complete_async(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to load or process image using PaddleOCR: {e}") from e

    @staticmethod
    def _process_paddle_result(result: list[Any] | Any, image: Image.Image) -> ExtractionResult:
        text_content = ""
        confidence_sum = 0
        confidence_count = 0

        for page_result in result:
            if not page_result:
                continue

            # Group text boxes by lines based on Y coordinate  # ~keep
            sorted_boxes = sorted(page_result, key=lambda x: x[0][0][1])
            line_groups: list[list[Any]] = []
            current_line: list[Any] = []
            prev_y: float | None = None

            for box in sorted_boxes:
                box_points, (_, _) = box
                current_y = sum(point[1] for point in box_points) / 4
                min_box_distance = 20  # Minimum distance to consider as new line  # ~keep

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
                line_sorted = sorted(line, key=lambda x: x[0][0][0])  # Sort boxes by X coordinate within line  # ~keep

                for box in line_sorted:
                    _, (text, confidence) = box
                    if text:
                        text_content += text + " "
                        confidence_sum += confidence
                        confidence_count += 1

                text_content += "\n"

        if hasattr(image, "width") and hasattr(image, "height"):
            width = image.width
            height = image.height
        else:
            width, height = image.size
        metadata = Metadata(
            width=width,
            height=height,
        )

        return ExtractionResult(
            content=normalize_spaces(text_content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata=metadata
        )

    @classmethod
    def _is_mkldnn_supported(cls) -> bool:
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
        if cls._paddle_ocr is not None:
            return

        _np, _paddle_ocr = _import_paddleocr()
        if _paddle_ocr is None:
            raise MissingDependencyError.create_for_package(
                dependency_group="paddleocr", functionality="PaddleOCR as an OCR backend", package_name="paddleocr"
            )

        language = cls._validate_language_code(kwargs.pop("language", "en"))

        cls._resolve_device_config(**kwargs)

        bool(find_spec("paddlepaddle_gpu"))

        use_angle_cls = kwargs.pop("use_angle_cls", True)
        kwargs.setdefault("use_textline_orientation", use_angle_cls)

        det_db_thresh = kwargs.pop("det_db_thresh", 0.3)
        det_db_box_thresh = kwargs.pop("det_db_box_thresh", 0.5)
        det_db_unclip_ratio = kwargs.pop("det_db_unclip_ratio", 1.6)

        kwargs.setdefault("text_det_thresh", det_db_thresh)
        kwargs.setdefault("text_det_box_thresh", det_db_box_thresh)
        kwargs.setdefault("text_det_unclip_ratio", det_db_unclip_ratio)

        kwargs.pop("use_gpu", None)
        kwargs.pop("gpu_mem", None)
        kwargs.pop("gpu_memory_limit", None)

        kwargs.setdefault("enable_mkldnn", cls._is_mkldnn_supported())

        try:
            cls._paddle_ocr = await run_sync(_paddle_ocr, lang=language, **kwargs)
        except Exception as e:
            raise OCRError(f"Failed to initialize PaddleOCR: {e}") from e

    @classmethod
    def _resolve_device_config(cls, **kwargs: Unpack[PaddleOCRConfig]) -> DeviceInfo:
        use_gpu = kwargs.get("use_gpu", False)
        device = kwargs.get("device", "auto")
        memory_limit = kwargs.get("gpu_memory_limit")
        fallback_to_cpu = kwargs.get("fallback_to_cpu", True)

        if use_gpu and device == "auto":
            warnings.warn(
                "The 'use_gpu' parameter is deprecated and will be removed in a future version. "
                "Use 'device=\"cuda\"' or 'device=\"auto\"' instead.",
                DeprecationWarning,
                stacklevel=4,
            )

            device = "auto" if use_gpu else "cpu"
        elif use_gpu and device != "auto":
            warnings.warn(
                "Both 'use_gpu' and 'device' parameters specified. The 'use_gpu' parameter is deprecated. "
                "Using 'device' parameter value.",
                DeprecationWarning,
                stacklevel=4,
            )

        if device == "mps":
            warnings.warn(
                "PaddlePaddle does not support MPS (Apple Silicon) acceleration. Falling back to CPU.",
                UserWarning,
                stacklevel=4,
            )
            device = "cpu"

        try:
            return validate_device_request(
                device,
                "PaddleOCR",
                memory_limit=memory_limit,
                fallback_to_cpu=fallback_to_cpu,
            )
        except ValidationError:
            if not use_gpu and device == "cpu":
                return DeviceInfo(device_type="cpu", name="CPU")
            raise

    @staticmethod
    def _validate_language_code(lang_code: str) -> str:
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

    def process_image_sync(self, image: Image.Image, **kwargs: Unpack[PaddleOCRConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            image_hash = generate_image_hash(image)
            cache_kwargs = build_cache_kwargs("paddleocr", kwargs, image_hash=image_hash)

            cached_result = handle_cache_lookup_sync(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            self._init_paddle_ocr_sync(**kwargs)

            if image.mode != "RGB":
                image = image.convert("RGB")

            _np, _ = _import_paddleocr()
            if _np is None:
                raise MissingDependencyError.create_for_package(
                    dependency_group="paddleocr", functionality="PaddleOCR as an OCR backend", package_name="paddleocr"
                )
            image_np = _np.array(image)
            use_textline_orientation = kwargs.get("use_textline_orientation", kwargs.get("use_angle_cls", True))
            result = self._paddle_ocr.ocr(image_np, cls=use_textline_orientation)

            extraction_result = self._process_paddle_result(result, image)

            if use_cache and cache_kwargs:
                cache_and_complete_sync(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to OCR using PaddleOCR: {e}") from e

    def process_file_sync(self, path: Path, **kwargs: Unpack[PaddleOCRConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            file_info = get_file_info(path)
            cache_kwargs = build_cache_kwargs("paddleocr", kwargs, file_info=file_info)

            cached_result = handle_cache_lookup_sync(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            self._init_paddle_ocr_sync(**kwargs)
            image = Image.open(path)

            kwargs["use_cache"] = False
            extraction_result = self.process_image_sync(image, **kwargs)

            if use_cache and cache_kwargs:
                cache_and_complete_sync(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to load or process image using PaddleOCR: {e}") from e

    @classmethod
    def _init_paddle_ocr_sync(cls, **kwargs: Unpack[PaddleOCRConfig]) -> None:
        if cls._paddle_ocr is not None:
            return

        _np, _paddle_ocr = _import_paddleocr()
        if _paddle_ocr is None:
            raise MissingDependencyError.create_for_package(
                dependency_group="paddleocr", functionality="PaddleOCR as an OCR backend", package_name="paddleocr"
            )

        language = cls._validate_language_code(kwargs.pop("language", "en"))

        cls._resolve_device_config(**kwargs)

        bool(find_spec("paddlepaddle_gpu"))

        use_angle_cls = kwargs.pop("use_angle_cls", True)
        kwargs.setdefault("use_textline_orientation", use_angle_cls)

        det_db_thresh = kwargs.pop("det_db_thresh", 0.3)
        det_db_box_thresh = kwargs.pop("det_db_box_thresh", 0.5)
        det_db_unclip_ratio = kwargs.pop("det_db_unclip_ratio", 1.6)

        kwargs.setdefault("text_det_thresh", det_db_thresh)
        kwargs.setdefault("text_det_box_thresh", det_db_box_thresh)
        kwargs.setdefault("text_det_unclip_ratio", det_db_unclip_ratio)

        kwargs.pop("use_gpu", None)
        kwargs.pop("gpu_mem", None)
        kwargs.pop("gpu_memory_limit", None)

        kwargs.setdefault("enable_mkldnn", cls._is_mkldnn_supported())

        try:
            cls._paddle_ocr = _paddle_ocr(lang=language, **kwargs)
        except Exception as e:
            raise OCRError(f"Failed to initialize PaddleOCR: {e}") from e
