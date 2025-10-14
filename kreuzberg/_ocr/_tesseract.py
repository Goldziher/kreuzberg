import io
from pathlib import Path
from typing import TYPE_CHECKING, Any, ClassVar

import anyio

from kreuzberg._internal_bindings import (
    ExtractionResultDTO,
    OCRProcessor,
    TesseractConfigDTO,
    validate_language_code,
    validate_tesseract_version,
)
from kreuzberg._mime_types import HTML_MIME_TYPE, MARKDOWN_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr._base import OCRBackend
from kreuzberg._types import ExtractionResult, TableData, TesseractConfig
from kreuzberg._utils._cache import get_ocr_cache
from kreuzberg._utils._sync import run_sync
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage

_BULLET_CHARS = {
    "*",
    "+",
    "\u2022",  # bullet
    "\u25aa",  # black small square
    "\u25e6",  # white bullet
    "\u25cf",  # black circle
    "\u2023",  # triangular bullet
    "\u2043",  # hyphen bullet (U+2043)
    "\u2219",  # bullet operator
    "\u00b7",  # middle dot
    "\u2027",  # hyphenation point
    "\uf0b7",  # private use bullet
}
_DASH_BULLET_CHARS = {"-", "\u2013", "\u2014", "\u2212"}  # hyphen, en dash, em dash, minus sign
_BULLET_MISREADS = {"e", "o", "0", "O"}


try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


def _bullet_body_from_line(line: str) -> str | None:
    first_char = line[0]
    if first_char in _DASH_BULLET_CHARS:
        if len(line) >= 2 and line[1].isspace():
            return line[2:].lstrip()
        return None
    if first_char in _BULLET_CHARS:
        if len(line) == 1:
            return ""
        next_char = line[1]
        if next_char.isspace() or not next_char.isalnum():
            return line[1:].lstrip()
        return None
    if len(line) >= 3 and first_char in _BULLET_MISREADS and line[1].isspace():
        candidate = line[2:].lstrip()
        if candidate and (candidate[0].isupper() or candidate[0].isdigit()):
            return candidate
    return None


def _merge_with_previous_bullet(normalized: list[str], line: str, pending_blank: bool) -> bool:
    if not normalized or pending_blank:
        return False
    previous = normalized[-1]
    if not previous.startswith("- "):
        return False
    if line.startswith(("- ", "* ")):
        return False
    normalized[-1] = f"{previous} {line}"
    return True


def _consume_short_fragment(normalized: list[str], line: str, pending_blank: bool) -> bool:
    if len(line) > 3 or not normalized or pending_blank:
        return False
    has_alpha = any(ch.isalpha() for ch in line)
    has_digit = any(ch.isdigit() for ch in line)
    if has_alpha:
        previous = normalized[-1].strip()
        if previous.isdigit():
            normalized.append(line)
        else:
            normalized[-1] = f"{normalized[-1]} {line}"
        return True
    if has_digit:
        previous = normalized[-1].strip()
        if len(line) <= 2 and len(previous) < 8:
            return True
        if previous.isdigit():
            normalized.append(line)
        elif len(previous) <= 6:
            normalized[-1] = f"{normalized[-1]} {line}"
        else:
            normalized.append(line)
        return True
    # punctuation-only fragments are dropped
    return True


def _merge_dash_fragment(normalized: list[str], line: str) -> bool:
    if normalized and line[0] in _DASH_BULLET_CHARS and len(line) > 1 and not line[1].isspace():
        normalized[-1] = f"{normalized[-1]} {line}"
        return True
    return False


def _is_duplicate_line(normalized: list[str], line: str) -> bool:
    if not normalized:
        return False
    return normalized[-1].strip().casefold() == line.casefold()


def _normalize_plain_text(content: str) -> str:
    lines = content.splitlines()
    normalized: list[str] = []
    pending_blank = False

    for line in lines:
        stripped = line.strip()
        if not stripped:
            if normalized:
                pending_blank = True
            continue

        bullet_body = _bullet_body_from_line(stripped)
        if bullet_body is not None:
            if pending_blank and normalized and not normalized[-1].startswith("- "):
                normalized.append("")
            pending_blank = False
            normalized.append(f"- {bullet_body}" if bullet_body else "-")
            continue

        if _merge_dash_fragment(normalized, stripped):
            pending_blank = False
            continue

        if _merge_with_previous_bullet(normalized, stripped, pending_blank):
            pending_blank = False
            continue

        if _consume_short_fragment(normalized, stripped, pending_blank):
            pending_blank = False
            continue

        if pending_blank:
            normalized.append("")
            pending_blank = False

        if _is_duplicate_line(normalized, stripped):
            continue

        normalized.append(stripped)

    if pending_blank and normalized:
        normalized.append("")
    return "\n".join(normalized).strip("\n")


class TesseractBackend(OCRBackend[TesseractConfig]):
    _version_checked: ClassVar[bool] = False

    def __init__(self) -> None:
        cache_dir_path = get_ocr_cache().cache_dir
        cache_dir = str(cache_dir_path) if cache_dir_path else None
        self._processor = OCRProcessor(cache_dir)

    def _config_to_dto(self, config: TesseractConfig) -> TesseractConfigDTO:
        try:
            validated_lang = validate_language_code(config.language)
        except ValueError as exc:
            raise ValidationError(
                f"Language code '{config.language}' is not supported by Tesseract",
                context={"language": config.language},
            ) from exc

        if hasattr(config.psm, "value"):
            psm_value = config.psm.value
        elif isinstance(config.psm, int):
            psm_value = config.psm
        else:
            psm_value = 3

        return TesseractConfigDTO(
            language=validated_lang,
            psm=psm_value,
            output_format=config.output_format,
            enable_table_detection=config.enable_table_detection,
            table_min_confidence=config.table_min_confidence,
            table_column_threshold=config.table_column_threshold,
            table_row_threshold_ratio=config.table_row_threshold_ratio,
            use_cache=True,
            classify_use_pre_adapted_templates=config.classify_use_pre_adapted_templates,
            language_model_ngram_on=config.language_model_ngram_on,
            tessedit_dont_blkrej_good_wds=config.tessedit_dont_blkrej_good_wds,
            tessedit_dont_rowrej_good_wds=config.tessedit_dont_rowrej_good_wds,
            tessedit_enable_dict_correction=config.tessedit_enable_dict_correction,
            tessedit_char_whitelist=config.tessedit_char_whitelist,
            tessedit_use_primary_params_model=config.tessedit_use_primary_params_model,
            textord_space_size_is_variable=config.textord_space_size_is_variable,
            thresholding_method=config.thresholding_method,
        )

    def _result_from_dto(self, dto: ExtractionResultDTO) -> ExtractionResult:
        mime_type_map = {
            "text/plain": PLAIN_TEXT_MIME_TYPE,
            "text/markdown": MARKDOWN_MIME_TYPE,
            "text/html": HTML_MIME_TYPE,
        }
        mime_type_str = mime_type_map.get(dto.mime_type, dto.mime_type)

        metadata_dict: dict[str, Any] = {}
        if dto.metadata:
            numeric_keys = {
                "tables_detected",
                "table_count",
                "table_rows",
                "table_cols",
            }
            for key, value in dto.metadata.items():
                if key in numeric_keys:
                    try:
                        metadata_dict[str(key)] = int(value)
                        continue
                    except (TypeError, ValueError):  # pragma: no cover - fallback to string
                        pass
                metadata_dict[str(key)] = value

        tables_data: list[TableData] = []
        if dto.tables:
            tables_data.extend(
                TableData(
                    cropped_image=None,
                    df=None,
                    page_number=table.page_number,
                    text=table.markdown,
                )
                for table in dto.tables
            )

        content = dto.content
        if mime_type_str == PLAIN_TEXT_MIME_TYPE and content:
            content = _normalize_plain_text(content)

        return ExtractionResult(
            content=content,
            mime_type=mime_type_str,
            metadata=metadata_dict,  # type: ignore[arg-type]
            chunks=[],
            tables=tables_data,
        )

    @classmethod
    async def _ensure_version_checked(cls) -> None:
        if not cls._version_checked:
            await anyio.to_thread.run_sync(validate_tesseract_version)
            cls._version_checked = True

    async def process_image(
        self,
        image: "PILImage",
        **kwargs: Unpack[TesseractConfig],
    ) -> ExtractionResult:
        await self._ensure_version_checked()

        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        save_image = image
        if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
            save_image = image.convert("RGB")

        image_buffer = io.BytesIO()
        await run_sync(save_image.save, image_buffer, format="PNG")
        image_bytes = image_buffer.getvalue()

        result_dto = await anyio.to_thread.run_sync(
            self._processor.process_image,
            image_bytes,
            config_dto,
        )

        return self._result_from_dto(result_dto)

    async def process_file(self, path: Path, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        await self._ensure_version_checked()

        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        result_dto = await anyio.to_thread.run_sync(
            self._processor.process_file,
            str(path),
            config_dto,
        )

        return self._result_from_dto(result_dto)

    def process_image_sync(self, image: "PILImage", **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        validate_tesseract_version()

        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        save_image = image
        if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
            save_image = image.convert("RGB")

        image_buffer = io.BytesIO()
        save_image.save(image_buffer, format="PNG")
        image_bytes = image_buffer.getvalue()

        result_dto = self._processor.process_image(image_bytes, config_dto)

        return self._result_from_dto(result_dto)

    def process_file_sync(self, path: Path, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        validate_tesseract_version()

        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        result_dto = self._processor.process_file(str(path), config_dto)

        return self._result_from_dto(result_dto)

    def process_batch_sync(self, paths: list[Path], **kwargs: Unpack[TesseractConfig]) -> list[ExtractionResult]:
        validate_tesseract_version()

        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        path_strings = [str(p) for p in paths]
        batch_results = self._processor.process_files_batch(path_strings, config_dto)

        results = []
        for batch_item in batch_results:
            if batch_item.success and batch_item.result:
                results.append(self._result_from_dto(batch_item.result))
            else:
                error_msg = batch_item.error if batch_item.error else "Unknown error"
                results.append(
                    ExtractionResult(
                        content=f"[OCR error: {error_msg}]",
                        mime_type=PLAIN_TEXT_MIME_TYPE,
                        metadata={"error": error_msg},
                        chunks=[],
                        tables=[],
                    )
                )
        return results

    async def process_batch(self, paths: list[Path], **kwargs: Unpack[TesseractConfig]) -> list[ExtractionResult]:
        await self._ensure_version_checked()

        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        path_strings = [str(p) for p in paths]
        batch_results = await anyio.to_thread.run_sync(
            self._processor.process_files_batch,
            path_strings,
            config_dto,
        )

        results = []
        for batch_item in batch_results:
            if batch_item.success and batch_item.result:
                results.append(self._result_from_dto(batch_item.result))
            else:
                error_msg = batch_item.error if batch_item.error else "Unknown error"
                results.append(
                    ExtractionResult(
                        content=f"[OCR error: {error_msg}]",
                        mime_type=PLAIN_TEXT_MIME_TYPE,
                        metadata={"error": error_msg},
                        chunks=[],
                        tables=[],
                    )
                )
        return results
