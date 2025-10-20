from collections.abc import Awaitable
from pathlib import Path
from typing import Any, Literal, Protocol, overload

__all__ = [
    "ChunkingConfig",
    "ExtractedTable",
    "ExtractionConfig",
    "ExtractionResult",
    "ImageExtractionConfig",
    "ImagePreprocessingConfig",
    "LanguageDetectionConfig",
    "OcrBackendProtocol",
    "OcrConfig",
    "PdfConfig",
    "PostProcessorConfig",
    "PostProcessorProtocol",
    "TesseractConfig",
    "TokenReductionConfig",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_files",
    "batch_extract_files_sync",
    "clear_post_processors",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
    "register_ocr_backend",
    "register_post_processor",
    "unregister_post_processor",
]

class OcrBackendProtocol(Protocol):
    def name(self) -> str: ...
    def supported_languages(self) -> list[str]: ...
    def process_image(self, image_bytes: bytes, language: str) -> dict[str, Any]: ...
    def process_file(self, path: str, language: str) -> dict[str, Any]: ...
    def initialize(self) -> None: ...
    def shutdown(self) -> None: ...
    def version(self) -> str: ...

class PostProcessorProtocol(Protocol):
    def name(self) -> str: ...
    def process(self, result: dict[str, Any]) -> dict[str, Any]: ...
    def processing_stage(self) -> Literal["early", "middle", "late"]: ...
    def initialize(self) -> None: ...
    def shutdown(self) -> None: ...

class ExtractionConfig:
    use_cache: bool
    enable_quality_processing: bool
    ocr: OcrConfig | None
    force_ocr: bool
    chunking: ChunkingConfig | None
    images: ImageExtractionConfig | None
    pdf_options: PdfConfig | None
    token_reduction: TokenReductionConfig | None
    language_detection: LanguageDetectionConfig | None
    postprocessor: PostProcessorConfig | None

    def __init__(
        self,
        *,
        use_cache: bool | None = None,
        enable_quality_processing: bool | None = None,
        ocr: OcrConfig | None = None,
        force_ocr: bool | None = None,
        chunking: ChunkingConfig | None = None,
        images: ImageExtractionConfig | None = None,
        pdf_options: PdfConfig | None = None,
        token_reduction: TokenReductionConfig | None = None,
        language_detection: LanguageDetectionConfig | None = None,
        postprocessor: PostProcessorConfig | None = None,
    ) -> None: ...

class OcrConfig:
    backend: str
    language: str
    tesseract_config: TesseractConfig | None

    def __init__(
        self,
        *,
        backend: str | None = None,
        language: str | None = None,
        tesseract_config: TesseractConfig | None = None,
    ) -> None: ...

class ChunkingConfig:
    max_chars: int
    max_overlap: int

    def __init__(self, *, max_chars: int | None = None, max_overlap: int | None = None) -> None: ...

class ImageExtractionConfig:
    extract_images: bool
    target_dpi: int
    max_image_dimension: int
    auto_adjust_dpi: bool
    min_dpi: int
    max_dpi: int

    def __init__(
        self,
        *,
        extract_images: bool | None = None,
        target_dpi: int | None = None,
        max_image_dimension: int | None = None,
        auto_adjust_dpi: bool | None = None,
        min_dpi: int | None = None,
        max_dpi: int | None = None,
    ) -> None: ...

class PdfConfig:
    extract_images: bool
    passwords: list[str] | None
    extract_metadata: bool

    def __init__(
        self,
        *,
        extract_images: bool | None = None,
        passwords: list[str] | None = None,
        extract_metadata: bool | None = None,
    ) -> None: ...

class TokenReductionConfig:
    mode: str
    preserve_important_words: bool

    def __init__(
        self,
        *,
        mode: Literal["off", "moderate", "aggressive"] | None = None,
        preserve_important_words: bool | None = None,
    ) -> None: ...

class LanguageDetectionConfig:
    enabled: bool
    min_confidence: float
    detect_multiple: bool

    def __init__(
        self,
        *,
        enabled: bool | None = None,
        min_confidence: float | None = None,
        detect_multiple: bool | None = None,
    ) -> None: ...

class PostProcessorConfig:
    enabled: bool
    enabled_processors: list[str] | None
    disabled_processors: list[str] | None

    def __init__(
        self,
        *,
        enabled: bool | None = None,
        enabled_processors: list[str] | None = None,
        disabled_processors: list[str] | None = None,
    ) -> None: ...

class ImagePreprocessingConfig:
    target_dpi: int
    auto_rotate: bool
    deskew: bool
    denoise: bool
    contrast_enhance: bool
    binarization_method: str
    invert_colors: bool

    def __init__(
        self,
        *,
        target_dpi: int | None = None,
        auto_rotate: bool | None = None,
        deskew: bool | None = None,
        denoise: bool | None = None,
        contrast_enhance: bool | None = None,
        binarization_method: str | None = None,
        invert_colors: bool | None = None,
    ) -> None: ...

class TesseractConfig:
    language: str
    psm: int
    output_format: str
    oem: int
    min_confidence: float
    preprocessing: ImagePreprocessingConfig | None
    enable_table_detection: bool
    table_min_confidence: float
    table_column_threshold: int
    table_row_threshold_ratio: float
    use_cache: bool
    classify_use_pre_adapted_templates: bool
    language_model_ngram_on: bool
    tessedit_dont_blkrej_good_wds: bool
    tessedit_dont_rowrej_good_wds: bool
    tessedit_enable_dict_correction: bool
    tessedit_char_whitelist: str
    tessedit_char_blacklist: str
    tessedit_use_primary_params_model: bool
    textord_space_size_is_variable: bool
    thresholding_method: bool

    def __init__(
        self,
        *,
        language: str | None = None,
        psm: int | None = None,
        output_format: str | None = None,
        oem: int | None = None,
        min_confidence: float | None = None,
        preprocessing: ImagePreprocessingConfig | None = None,
        enable_table_detection: bool | None = None,
        table_min_confidence: float | None = None,
        table_column_threshold: int | None = None,
        table_row_threshold_ratio: float | None = None,
        use_cache: bool | None = None,
        classify_use_pre_adapted_templates: bool | None = None,
        language_model_ngram_on: bool | None = None,
        tessedit_dont_blkrej_good_wds: bool | None = None,
        tessedit_dont_rowrej_good_wds: bool | None = None,
        tessedit_enable_dict_correction: bool | None = None,
        tessedit_char_whitelist: str | None = None,
        tessedit_char_blacklist: str | None = None,
        tessedit_use_primary_params_model: bool | None = None,
        textord_space_size_is_variable: bool | None = None,
        thresholding_method: bool | None = None,
    ) -> None: ...

class ExtractionResult:
    content: str
    mime_type: str
    metadata: dict[str, Any]
    tables: list[ExtractedTable]
    detected_languages: list[str] | None
    images: list[dict[str, Any]] | None

class ExtractedTable:
    cells: list[list[str]]
    markdown: str
    page_number: int

@overload
def extract_file_sync(
    path: str | Path | bytes,
    mime_type: None = None,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
@overload
def extract_file_sync(
    path: str | Path | bytes,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
def batch_extract_files_sync(
    paths: list[str | Path | bytes],
    config: ExtractionConfig = ...,
) -> list[ExtractionResult]: ...
def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig = ...,
) -> list[ExtractionResult]: ...
@overload
async def extract_file(
    path: str | Path | bytes,
    mime_type: None = None,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
@overload
async def extract_file(
    path: str | Path | bytes,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> ExtractionResult: ...
def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig = ...,
) -> Awaitable[ExtractionResult]: ...
def batch_extract_files(
    paths: list[str | Path | bytes],
    config: ExtractionConfig = ...,
) -> Awaitable[list[ExtractionResult]]: ...
def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig = ...,
) -> Awaitable[list[ExtractionResult]]: ...

# TODO: add registration for all supported plugins - we need to add register validator and register extractor
def register_ocr_backend(backend: OcrBackendProtocol) -> None: ...
def register_post_processor(processor: PostProcessorProtocol) -> None: ...
def clear_post_processors() -> None: ...
def unregister_post_processor(name: str) -> None: ...
