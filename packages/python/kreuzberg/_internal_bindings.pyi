from collections.abc import Awaitable
from pathlib import Path
from typing import Any, Literal, Protocol, TypedDict, overload

__all__ = [
    "ChunkingConfig",
    "EmbeddingConfig",
    "EmbeddingModelType",
    "EmbeddingPreset",
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
    "ValidatorProtocol",
    "batch_extract_bytes",
    "batch_extract_bytes_sync",
    "batch_extract_files",
    "batch_extract_files_sync",
    "clear_document_extractors",
    "clear_ocr_backends",
    "clear_post_processors",
    "clear_validators",
    "detect_mime_type_from_bytes",
    "detect_mime_type_from_path",
    "extract_bytes",
    "extract_bytes_sync",
    "extract_file",
    "extract_file_sync",
    "get_embedding_preset",
    "get_extensions_for_mime",
    "list_document_extractors",
    "list_embedding_presets",
    "list_ocr_backends",
    "list_post_processors",
    "list_validators",
    "register_ocr_backend",
    "register_post_processor",
    "register_validator",
    "unregister_document_extractor",
    "unregister_ocr_backend",
    "unregister_post_processor",
    "unregister_validator",
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

class ValidatorProtocol(Protocol):
    def name(self) -> str: ...
    def validate(self, result: dict[str, Any]) -> None: ...
    def priority(self) -> int: ...
    def should_validate(self, result: dict[str, Any]) -> bool: ...

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
    max_concurrent_extractions: int | None
    html_options: dict[str, Any] | None

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
        max_concurrent_extractions: int | None = None,
        html_options: dict[str, Any] | None = None,
    ) -> None: ...
    @staticmethod
    def from_file(path: str | Path) -> ExtractionConfig: ...

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

class EmbeddingModelType:
    @staticmethod
    def preset(name: str) -> EmbeddingModelType: ...
    @staticmethod
    def fastembed(model: str, dimensions: int) -> EmbeddingModelType: ...
    @staticmethod
    def custom(model_id: str, dimensions: int) -> EmbeddingModelType: ...

class EmbeddingConfig:
    model: EmbeddingModelType
    normalize: bool
    batch_size: int
    show_download_progress: bool
    cache_dir: str | None

    def __init__(
        self,
        *,
        model: EmbeddingModelType | None = None,
        normalize: bool | None = None,
        batch_size: int | None = None,
        show_download_progress: bool | None = None,
        cache_dir: str | None = None,
    ) -> None: ...

class EmbeddingPreset:
    name: str
    chunk_size: int
    overlap: int
    model_name: str
    dimensions: int
    description: str

class ChunkingConfig:
    max_chars: int
    max_overlap: int
    embedding: EmbeddingConfig | None
    preset: str | None

    def __init__(
        self,
        *,
        max_chars: int | None = None,
        max_overlap: int | None = None,
        embedding: EmbeddingConfig | None = None,
        preset: str | None = None,
    ) -> None: ...

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

class PdfMetadata(TypedDict, total=False):
    title: str
    subject: str
    authors: list[str]
    keywords: list[str]
    created_at: str
    modified_at: str
    created_by: str
    producer: str
    page_count: int
    pdf_version: str
    is_encrypted: bool
    width: int
    height: int
    summary: str

class ExcelMetadata(TypedDict, total=False):
    sheet_count: int
    sheet_names: list[str]

class EmailMetadata(TypedDict, total=False):
    from_email: str
    from_name: str
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str
    attachments: list[str]

class PptxMetadata(TypedDict, total=False):
    title: str
    author: str
    description: str
    summary: str
    fonts: list[str]

class ArchiveMetadata(TypedDict, total=False):
    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int

class ImageMetadata(TypedDict, total=False):
    width: int
    height: int
    format: str
    exif: dict[str, str]

class XmlMetadata(TypedDict, total=False):
    element_count: int
    unique_elements: list[str]

class TextMetadata(TypedDict, total=False):
    line_count: int
    word_count: int
    character_count: int
    headers: list[str]
    links: list[tuple[str, str]]
    code_blocks: list[tuple[str, str]]

class HtmlMetadata(TypedDict, total=False):
    title: str
    description: str
    keywords: str
    author: str
    canonical: str
    base_href: str
    og_title: str
    og_description: str
    og_image: str
    og_url: str
    og_type: str
    og_site_name: str
    twitter_card: str
    twitter_title: str
    twitter_description: str
    twitter_image: str
    twitter_site: str
    twitter_creator: str
    link_author: str
    link_license: str
    link_alternate: str

class OcrMetadata(TypedDict, total=False):
    language: str
    psm: int
    output_format: str
    table_count: int
    table_rows: int
    table_cols: int

class ImagePreprocessingMetadata(TypedDict, total=False):
    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    scale_factor: float
    auto_adjusted: bool
    final_dpi: int
    new_dimensions: tuple[int, int]
    resample_method: str
    dimension_clamped: bool
    calculated_dpi: int
    skipped_resize: bool
    resize_error: str

class ErrorMetadata(TypedDict, total=False):
    error_type: str
    message: str

class Metadata(TypedDict, total=False):
    language: str
    date: str
    subject: str

    format_type: Literal["pdf", "excel", "email", "pptx", "archive", "image", "xml", "text", "html", "ocr"]

    title: str
    authors: list[str]
    keywords: list[str]
    created_at: str
    modified_at: str
    created_by: str
    producer: str
    page_count: int
    pdf_version: str
    is_encrypted: bool
    width: int
    height: int
    summary: str

    sheet_count: int
    sheet_names: list[str]

    from_email: str
    from_name: str
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    message_id: str
    attachments: list[str]

    author: str
    description: str
    fonts: list[str]

    format: str
    file_count: int
    file_list: list[str]
    total_size: int
    compressed_size: int

    exif: dict[str, str]

    element_count: int
    unique_elements: list[str]

    line_count: int
    word_count: int
    character_count: int
    headers: list[str]
    links: list[tuple[str, str]]
    code_blocks: list[tuple[str, str]]

    canonical: str
    base_href: str
    og_title: str
    og_description: str
    og_image: str
    og_url: str
    og_type: str
    og_site_name: str
    twitter_card: str
    twitter_title: str
    twitter_description: str
    twitter_image: str
    twitter_site: str
    twitter_creator: str
    link_author: str
    link_license: str
    link_alternate: str

    psm: int
    output_format: str
    table_count: int
    table_rows: int
    table_cols: int

    image_preprocessing: ImagePreprocessingMetadata
    json_schema: dict[str, Any]
    error: ErrorMetadata

class ExtractedImage(TypedDict, total=False):
    data: bytes
    format: str
    image_index: int
    page_number: int
    width: int
    height: int
    colorspace: str
    bits_per_component: int
    is_mask: bool
    description: str
    ocr_result: ExtractionResult

class Chunk(TypedDict, total=False):
    content: str
    embedding: list[float] | None
    metadata: dict[str, Any]

class ExtractionResult:
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[ExtractedTable]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None

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
def register_ocr_backend(backend: OcrBackendProtocol) -> None: ...
def register_post_processor(processor: PostProcessorProtocol) -> None: ...
def clear_post_processors() -> None: ...
def unregister_post_processor(name: str) -> None: ...
def register_validator(validator: ValidatorProtocol) -> None: ...
def clear_validators() -> None: ...
def unregister_validator(name: str) -> None: ...
def list_embedding_presets() -> list[str]: ...
def get_embedding_preset(name: str) -> EmbeddingPreset | None: ...
def clear_document_extractors() -> None: ...
def clear_ocr_backends() -> None: ...
def detect_mime_type_from_bytes(data: bytes) -> str: ...
def detect_mime_type_from_path(path: str | Path) -> str: ...
def get_extensions_for_mime(mime_type: str) -> list[str]: ...
def list_document_extractors() -> list[str]: ...
def list_ocr_backends() -> list[str]: ...
def list_post_processors() -> list[str]: ...
def list_validators() -> list[str]: ...
def unregister_document_extractor(name: str) -> None: ...
def unregister_ocr_backend(name: str) -> None: ...
