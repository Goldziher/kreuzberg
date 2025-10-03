import io
from pathlib import Path
from typing import TYPE_CHECKING, ClassVar

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

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage

try:  # pragma: no cover
    from typing import Unpack  # type: ignore[attr-defined]
except ImportError:  # pragma: no cover
    from typing_extensions import Unpack


class TesseractBackend(OCRBackend[TesseractConfig]):
    _version_checked: ClassVar[bool] = False

    def __init__(self) -> None:
        cache_dir_path = get_ocr_cache().cache_dir
        cache_dir = str(cache_dir_path) if cache_dir_path else None
        self._processor = OCRProcessor(cache_dir)

    def _config_to_dto(self, config: TesseractConfig) -> TesseractConfigDTO:
        """Convert Python TesseractConfig to Rust TesseractConfigDTO."""
        # Validate language code
        validated_lang = validate_language_code(config.language)

        # Convert PSMMode to integer
        if hasattr(config.psm, "value"):
            psm_value = config.psm.value
        elif isinstance(config.psm, int):
            psm_value = config.psm
        else:
            psm_value = 3  # Default to AUTO

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
        """Convert Rust ExtractionResultDTO to Python ExtractionResult."""
        # Determine MIME type
        mime_type_map = {
            "text/plain": PLAIN_TEXT_MIME_TYPE,
            "text/markdown": MARKDOWN_MIME_TYPE,
            "text/html": HTML_MIME_TYPE,
        }
        mime_type_str = mime_type_map.get(dto.mime_type, dto.mime_type)

        # Convert metadata dict - Rust HashMap to Python dict
        metadata_dict: dict[str, str] = {}
        if dto.metadata:
            for key, value in dto.metadata.items():
                metadata_dict[str(key)] = str(value)

        # Convert TableDTO objects to TableData
        # For now, we can't create full TableData with images and DataFrames
        # since we don't have the original image or bounding box information in OCR context
        # Tables will be converted later by the extraction pipeline if needed
        tables_data: list[TableData] = []
        # Note: The tables field in ExtractionResult expects TableData objects with images/DataFrames
        # Since Tesseract processes individual images without PDF page context,
        # we store the table structures in metadata and return empty tables list for now
        # The table markdown is already included in the content when output_format='markdown'

        return ExtractionResult(
            content=dto.content,
            mime_type=mime_type_str,
            metadata=metadata_dict,  # type: ignore[arg-type]
            chunks=[],
            tables=tables_data,
        )

    @classmethod
    async def _ensure_version_checked(cls) -> None:
        """Ensure Tesseract version is validated."""
        if not cls._version_checked:
            await anyio.to_thread.run_sync(validate_tesseract_version)
            cls._version_checked = True

    async def process_image(
        self,
        image: "PILImage",
        **kwargs: Unpack[TesseractConfig],
    ) -> ExtractionResult:
        """Process an image with Tesseract OCR."""
        await self._ensure_version_checked()

        # Convert TesseractConfig kwargs to TesseractConfig object
        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # Convert PIL Image to bytes
        save_image = image
        if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
            save_image = image.convert("RGB")

        image_buffer = io.BytesIO()
        await run_sync(save_image.save, image_buffer, format="PNG")
        image_bytes = image_buffer.getvalue()

        # Process with Rust implementation
        result_dto = await anyio.to_thread.run_sync(
            self._processor.process_image,
            image_bytes,
            config_dto,
        )

        return self._result_from_dto(result_dto)

    async def process_file(self, path: Path, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        """Process a file with Tesseract OCR."""
        await self._ensure_version_checked()

        # Convert TesseractConfig kwargs to TesseractConfig object
        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # Process with Rust implementation
        # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        result_dto = await anyio.to_thread.run_sync(
            self._processor.process_file,
            str(path),
            config_dto,
        )

        return self._result_from_dto(result_dto)

    def process_image_sync(self, image: "PILImage", **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        """Process an image with Tesseract OCR (synchronous)."""
        validate_tesseract_version()

        # Convert TesseractConfig kwargs to TesseractConfig object
        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # Convert PIL Image to bytes
        save_image = image
        if image.mode not in ("RGB", "RGBA", "L", "LA", "P", "1"):
            save_image = image.convert("RGB")

        image_buffer = io.BytesIO()
        save_image.save(image_buffer, format="PNG")
        image_bytes = image_buffer.getvalue()

        # Process with Rust implementation
        result_dto = self._processor.process_image(image_bytes, config_dto)

        return self._result_from_dto(result_dto)

    def process_file_sync(self, path: Path, **kwargs: Unpack[TesseractConfig]) -> ExtractionResult:
        """Process a file with Tesseract OCR (synchronous)."""
        validate_tesseract_version()

        # Convert TesseractConfig kwargs to TesseractConfig object
        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # Process with Rust implementation
        # OSError/RuntimeError must bubble up - system errors need user reports ~keep
        result_dto = self._processor.process_file(str(path), config_dto)

        return self._result_from_dto(result_dto)

    def process_batch_sync(self, paths: list[Path], **kwargs: Unpack[TesseractConfig]) -> list[ExtractionResult]:
        """Process multiple files in batch (synchronous)."""
        validate_tesseract_version()

        # Convert TesseractConfig kwargs to TesseractConfig object
        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # Process batch with Rust implementation
        path_strings = [str(p) for p in paths]
        batch_results = self._processor.process_files_batch(path_strings, config_dto)

        # Convert results
        results = []
        for batch_item in batch_results:
            if batch_item.success and batch_item.result:
                results.append(self._result_from_dto(batch_item.result))
            else:
                # Return error as content
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
        """Process multiple files in batch (asynchronous)."""
        await self._ensure_version_checked()

        # Convert TesseractConfig kwargs to TesseractConfig object
        config = TesseractConfig(**kwargs) if kwargs else TesseractConfig()
        config_dto = self._config_to_dto(config)

        # Process batch with Rust implementation (run in thread since it uses rayon)
        path_strings = [str(p) for p in paths]
        batch_results = await anyio.to_thread.run_sync(
            self._processor.process_files_batch,
            path_strings,
            config_dto,
        )

        # Convert results
        results = []
        for batch_item in batch_results:
            if batch_item.success and batch_item.result:
                results.append(self._result_from_dto(batch_item.result))
            else:
                # Return error as content
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
