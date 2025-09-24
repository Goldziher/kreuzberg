from __future__ import annotations

import warnings
from typing import TYPE_CHECKING, Any, ClassVar, Final

from PIL import Image

from kreuzberg._mime_types import PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import EasyOCRConfig, ExtractionResult, Metadata
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
    import easyocr
    import torch
else:
    easyocr: Any = None
    torch: Any = None

HAS_EASYOCR: bool = False


def _import_easyocr() -> tuple[Any, Any]:
    global HAS_EASYOCR, easyocr, torch

    if easyocr is not None:
        return easyocr, torch

    if not HAS_EASYOCR and easyocr is None:
        return None, None

    try:
        import easyocr as _easyocr  # noqa: PLC0415

        try:
            import torch as _torch  # noqa: PLC0415
        except ImportError:  # pragma: no cover
            _torch = None

        easyocr = _easyocr
        torch = _torch
        HAS_EASYOCR = True
        return easyocr, torch
    except ImportError:  # pragma: no cover
        return None, None


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


class EasyOCRBackend(OCRBackend[EasyOCRConfig]):
    _reader: ClassVar[Any] = None

    async def process_image(self, image: Image.Image, **kwargs: Unpack[EasyOCRConfig]) -> ExtractionResult:
        try:
            import numpy as np  # noqa: PLC0415
        except ImportError as e:  # pragma: no cover
            raise MissingDependencyError("EasyOCR requires numpy: pip install 'kreuzberg[easyocr]'") from e

        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            image_hash = generate_image_hash(image)
            cache_kwargs = build_cache_kwargs("easyocr", kwargs, image_hash=image_hash)

            cached_result = await handle_cache_lookup_async(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            await self._init_easyocr(**kwargs)

            beam_width = kwargs.pop("beam_width", 5)

            kwargs.pop("language", None)
            kwargs.pop("use_gpu", None)
            kwargs.pop("device", None)
            kwargs.pop("gpu_memory_limit", None)
            kwargs.pop("fallback_to_cpu", None)

            result = await run_sync(
                self._reader.readtext,
                np.array(image),
                beamWidth=beam_width,
                **kwargs,
            )

            extraction_result = self._process_easyocr_result(result, image)

            if use_cache and cache_kwargs:
                await cache_and_complete_async(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to OCR using EasyOCR: {e}") from e

    async def process_file(self, path: Path, **kwargs: Unpack[EasyOCRConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            file_info = get_file_info(path)
            cache_kwargs = build_cache_kwargs("easyocr", kwargs, file_info=file_info)

            cached_result = await handle_cache_lookup_async(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            await self._init_easyocr(**kwargs)
            image = await run_sync(Image.open, path)

            kwargs["use_cache"] = False
            extraction_result = await self.process_image(image, **kwargs)

            if use_cache and cache_kwargs:
                await cache_and_complete_async(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to load or process image using EasyOCR: {e}") from e

    @staticmethod
    def _process_easyocr_result(result: list[Any], image: Image.Image) -> ExtractionResult:
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
                content=normalize_spaces(text_content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata=metadata
            )

        # Group text boxes by lines based on Y coordinate  # ~keep
        sorted_results = sorted(result, key=lambda x: x[0][0][1] + x[0][2][1])
        line_groups: list[list[Any]] = []
        current_line: list[Any] = []
        prev_y_center: float | None = None
        line_height_threshold = 20  # Minimum distance to consider as new line  # ~keep

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
            line_sorted = sorted(line, key=lambda x: x[0][0][0])  # Sort boxes by X coordinate within line  # ~keep

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
            content=normalize_spaces(text_content), mime_type=PLAIN_TEXT_MIME_TYPE, metadata=metadata
        )

    @classmethod
    def _is_gpu_available(cls) -> bool:
        if torch is None:
            return False
        return bool(torch.cuda.is_available())

    @classmethod
    async def _init_easyocr(cls, **kwargs: Unpack[EasyOCRConfig]) -> None:
        if cls._reader is not None:
            return

        languages = cls._validate_language_code(kwargs.pop("language", "en"))

        easyocr_module, _ = _import_easyocr()
        if easyocr_module is None:
            raise MissingDependencyError.create_for_package(
                dependency_group="easyocr", functionality="EasyOCR as an OCR backend", package_name="easyocr"
            )

        device_info = cls._resolve_device_config(**kwargs)
        use_gpu = device_info.device_type in ("cuda", "mps")

        kwargs.setdefault("detector", True)
        kwargs.setdefault("recognizer", True)
        kwargs.setdefault("download_enabled", True)
        kwargs.setdefault("recog_network", "standard")

        try:
            cls._reader = await run_sync(
                easyocr_module.Reader,
                languages,
                gpu=use_gpu,
                verbose=False,
            )
        except Exception as e:
            raise OCRError(f"Failed to initialize EasyOCR: {e}") from e

    @classmethod
    def _resolve_device_config(cls, **kwargs: Unpack[EasyOCRConfig]) -> DeviceInfo:
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

        try:
            return validate_device_request(
                device,
                "EasyOCR",
                memory_limit=memory_limit,
                fallback_to_cpu=fallback_to_cpu,
            )
        except ValidationError:
            if not use_gpu and device == "cpu":
                return DeviceInfo(device_type="cpu", name="CPU")
            raise

    @staticmethod
    def _validate_language_code(language_codes: str | list[str]) -> list[str]:
        if isinstance(language_codes, str):
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

    def process_image_sync(self, image: Image.Image, **kwargs: Unpack[EasyOCRConfig]) -> ExtractionResult:
        try:
            import numpy as np  # noqa: PLC0415
        except ImportError as e:  # pragma: no cover
            raise MissingDependencyError("EasyOCR requires numpy: pip install 'kreuzberg[easyocr]'") from e

        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            image_hash = generate_image_hash(image)
            cache_kwargs = build_cache_kwargs("easyocr", kwargs, image_hash=image_hash)

            cached_result = handle_cache_lookup_sync(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            self._init_easyocr_sync(**kwargs)

            beam_width = kwargs.pop("beam_width", 5)
            kwargs.pop("language", None)
            kwargs.pop("use_gpu", None)
            kwargs.pop("device", None)
            kwargs.pop("gpu_memory_limit", None)
            kwargs.pop("fallback_to_cpu", None)

            result = self._reader.readtext(
                np.array(image),
                beamWidth=beam_width,
                **kwargs,
            )

            extraction_result = self._process_easyocr_result(result, image)

            if use_cache and cache_kwargs:
                cache_and_complete_sync(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to OCR using EasyOCR: {e}") from e

    def process_file_sync(self, path: Path, **kwargs: Unpack[EasyOCRConfig]) -> ExtractionResult:
        use_cache = kwargs.pop("use_cache", True)

        cache_kwargs = None
        if use_cache:
            file_info = get_file_info(path)
            cache_kwargs = build_cache_kwargs("easyocr", kwargs, file_info=file_info)

            cached_result = handle_cache_lookup_sync(cache_kwargs)
            if cached_result:
                return cached_result

        try:
            self._init_easyocr_sync(**kwargs)
            image = Image.open(path)

            kwargs["use_cache"] = False
            extraction_result = self.process_image_sync(image, **kwargs)

            if use_cache and cache_kwargs:
                cache_and_complete_sync(extraction_result, cache_kwargs, use_cache)

            return extraction_result
        except Exception as e:
            if use_cache and cache_kwargs:
                mark_processing_complete(cache_kwargs)
            raise OCRError(f"Failed to load or process image using EasyOCR: {e}") from e

    @classmethod
    def _init_easyocr_sync(cls, **kwargs: Unpack[EasyOCRConfig]) -> None:
        if cls._reader is not None:
            return

        languages = cls._validate_language_code(kwargs.pop("language", "en"))

        easyocr_module, _ = _import_easyocr()
        if easyocr_module is None:
            raise MissingDependencyError.create_for_package(
                dependency_group="easyocr", functionality="EasyOCR as an OCR backend", package_name="easyocr"
            )

        device_info = cls._resolve_device_config(**kwargs)
        use_gpu = device_info.device_type in ("cuda", "mps")

        kwargs.setdefault("detector", True)
        kwargs.setdefault("recognizer", True)
        kwargs.setdefault("download_enabled", True)
        kwargs.setdefault("recog_network", "standard")

        try:
            cls._reader = easyocr_module.Reader(
                languages,
                gpu=use_gpu,
                verbose=False,
            )
        except Exception as e:
            raise OCRError(f"Failed to initialize EasyOCR: {e}") from e
