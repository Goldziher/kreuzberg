from typing import Any, TypedDict

class CacheStatsDict(TypedDict):
    total_files: int
    total_size_mb: float
    available_space_mb: float
    oldest_file_age_days: float
    newest_file_age_days: float

class DocumentCacheStatsDict(TypedDict):
    cached_documents: int
    processing_documents: int
    total_cache_size_mb: float

class CacheStats:
    total_files: int
    total_size_mb: float
    available_space_mb: float
    oldest_file_age_days: float
    newest_file_age_days: float

class GenericCache:
    def __init__(
        self,
        cache_type: str,
        cache_dir: str | None = None,
        max_age_days: float = 30.0,
        max_cache_size_mb: float = 500.0,
        min_free_space_mb: float = 1000.0,
    ) -> None: ...
    def get(self, cache_key: str, source_file: str | None = None) -> bytes | None: ...
    def set(self, cache_key: str, data: bytes, source_file: str | None = None) -> None: ...
    def is_processing(self, cache_key: str) -> bool: ...
    def mark_processing(self, cache_key: str) -> None: ...
    def mark_complete(self, cache_key: str) -> None: ...
    def clear(self) -> tuple[int, float]: ...
    def get_stats(self) -> CacheStats: ...
    @property
    def cache_dir(self) -> str: ...
    @property
    def cache_type_name(self) -> str: ...

class TextSplitter:
    def __init__(self, max_characters: int, overlap: int = 0, trim: bool = True) -> None: ...
    def chunks(self, text: str) -> list[str]: ...

class MarkdownSplitter:
    def __init__(self, max_characters: int, overlap: int = 0, trim: bool = True) -> None: ...
    def chunks(self, text: str) -> list[str]: ...

def generate_cache_key(**kwargs: Any) -> str: ...
def batch_generate_cache_keys(items: list[Any]) -> list[str]: ...
def fast_hash(data: bytes) -> int: ...
def validate_cache_key(key: str) -> bool: ...
def filter_old_cache_entries(cache_times: list[float], current_time: float, max_age_seconds: float) -> list[int]: ...
def sort_cache_by_access_time(entries: list[tuple[str, float]]) -> list[str]: ...
def get_available_disk_space(path: str) -> float: ...
def get_cache_metadata(cache_dir: str) -> CacheStats: ...
def cleanup_cache(
    cache_dir: str, max_age_days: float, max_size_mb: float, target_size_ratio: float
) -> tuple[int, float]: ...
def smart_cleanup_cache(
    cache_dir: str, max_age_days: float, max_size_mb: float, min_free_space_mb: float
) -> tuple[int, float]: ...
def is_cache_valid(cache_path: str, max_age_days: float) -> bool: ...
def clear_cache_directory(cache_dir: str) -> tuple[int, float]: ...
def batch_cleanup_caches(
    cache_dirs: list[str], max_age_days: float, max_size_mb: float, min_free_space_mb: float
) -> list[tuple[int, float]]: ...
def calculate_quality_score(text: str, metadata: dict[str, Any] | None = None) -> float: ...
def clean_extracted_text(text: str) -> str: ...
def normalize_spaces(text: str) -> str: ...
def safe_decode(data: bytes, encoding: str | None = None) -> str: ...
def convert_html_to_markdown(html: str, options: dict[str, Any] | None = ...) -> str: ...
def process_html(
    html: str,
    options: dict[str, Any] | None = ...,
    extract_images: bool = ...,
    max_image_size: int = ...,
) -> tuple[str, list[dict[str, Any]], list[str]]: ...
def batch_process_texts(texts: list[bytes]) -> list[str]: ...
def calculate_text_confidence(text: str) -> float: ...
def fix_mojibake(text: str) -> str: ...
def get_encoding_cache_key(data_hash: str, size: int) -> str: ...
def normalize_image_dpi(
    image_array: Any, config: ExtractionConfigDTO, dpi_info: dict[str, float] | None = None
) -> tuple[Any, ImagePreprocessingMetadataDTO]: ...
def batch_normalize_images(
    images: list[Any], config: ExtractionConfigDTO
) -> list[tuple[Any, ImagePreprocessingMetadataDTO]]: ...
def calculate_optimal_dpi(
    page_width: float, page_height: float, target_dpi: int, max_dimension: int, min_dpi: int = 72, max_dpi: int = 600
) -> int: ...
def load_image(data: bytes) -> tuple[bytes, int, int, str]: ...
def save_image(data: bytes, width: int, height: int, format: str) -> bytes: ...  # noqa: A002
def detect_image_format(data: bytes) -> str: ...
def load_image_as_numpy(data: bytes) -> Any: ...
def save_numpy_as_image(array: Any, format: str) -> bytes: ...  # noqa: A002
def compress_image_jpeg(data: bytes, width: int, height: int, quality: int = 85) -> bytes: ...
def compress_image_png(data: bytes, width: int, height: int, compression: int = 6) -> bytes: ...
def compress_image_auto(
    data: bytes, width: int, height: int, target_size_kb: int | None = None
) -> tuple[bytes, str]: ...
def rgb_to_grayscale(rgb_array: Any) -> Any: ...
def rgb_to_rgba(rgb_array: Any, alpha: int = 255) -> Any: ...
def rgba_to_rgb(rgba_array: Any) -> Any: ...
def convert_format(array: Any, to_format: str) -> Any: ...
def reduce_tokens(text: str, config: TokenReductionConfigDTO, language_hint: str | None = None) -> str: ...
def batch_reduce_tokens(
    texts: list[str], config: TokenReductionConfigDTO, language_hint: str | None = None
) -> list[str]: ...
def get_reduction_statistics(original: str, reduced: str) -> tuple[float, float, int, int, int, int]: ...
def table_from_arrow_to_markdown(arrow_bytes: bytes) -> str: ...
def read_excel_file(file_path: str) -> ExcelWorkbook: ...
def read_excel_bytes(data: bytes, file_extension: str) -> ExcelWorkbook: ...
def excel_to_markdown(file_path: str) -> str: ...
def benchmark_excel_reading(file_path: str, iterations: int) -> float: ...

class ImagePreprocessingMetadataDTO:
    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    scale_factor: float
    auto_adjusted: bool
    final_dpi: int
    new_dimensions: tuple[int, int] | None
    resample_method: str
    dimension_clamped: bool
    calculated_dpi: int | None
    skipped_resize: bool
    resize_error: str | None

class ExtractionConfigDTO:
    target_dpi: int
    max_image_dimension: int
    auto_adjust_dpi: bool
    min_dpi: int
    max_dpi: int

    def __init__(
        self,
        target_dpi: int,
        max_image_dimension: int,
        auto_adjust_dpi: bool,
        min_dpi: int,
        max_dpi: int,
    ) -> None: ...

class ReductionLevelDTO:
    Off: ReductionLevelDTO
    Light: ReductionLevelDTO
    Moderate: ReductionLevelDTO
    Aggressive: ReductionLevelDTO
    Maximum: ReductionLevelDTO

class TokenReductionConfigDTO:
    level: ReductionLevelDTO
    language_hint: str | None
    preserve_markdown: bool
    preserve_code: bool
    semantic_threshold: float
    enable_parallel: bool
    use_simd: bool
    custom_stopwords: dict[str, list[str]] | None
    preserve_patterns: list[str]
    target_reduction: float | None
    enable_semantic_clustering: bool

    def __init__(self) -> None: ...

class ExcelWorkbook:
    sheets: list[ExcelSheet]
    metadata: dict[str, str]

class ExcelSheet:
    name: str
    markdown: str
    row_count: int
    col_count: int
    cell_count: int

class PptxMetadata:
    title: str | None
    author: str | None
    description: str | None
    summary: str | None
    fonts: list[str]

class ExtractedImageDTO:
    data: bytes
    format: str
    slide_number: int | None

class PptxExtractionResult:
    content: str
    metadata: PptxMetadata
    slide_count: int
    image_count: int
    table_count: int
    images: list[ExtractedImageDTO]

class PptxExtractorDTO:
    def __init__(self, extract_images: bool) -> None: ...
    def extract_from_path(self, path: str) -> PptxExtractionResult: ...
    def extract_from_bytes(self, data: bytes) -> PptxExtractionResult: ...

class StreamingPptxExtractorDTO:
    def __init__(self, extract_images: bool | None = None, max_cache_mb: int | None = None) -> None: ...
    def extract_from_path(self, path: str) -> PptxExtractionResult: ...

class EmailAttachmentDTO:
    name: str | None
    filename: str | None
    mime_type: str | None
    size: int | None
    is_image: bool
    data: bytes | None

    def __init__(
        self,
        name: str | None,
        filename: str | None,
        mime_type: str | None,
        size: int | None,
        is_image: bool,
        data: bytes | None,
    ) -> None: ...

class EmailExtractionResultDTO:
    subject: str | None
    from_email: str | None
    to_emails: list[str]
    cc_emails: list[str]
    bcc_emails: list[str]
    date: str | None
    message_id: str | None
    plain_text: str | None
    html_content: str | None
    cleaned_text: str
    attachments: list[EmailAttachmentDTO]
    metadata: dict[str, str]

    def __init__(
        self,
        subject: str | None,
        from_email: str | None,
        to_emails: list[str],
        cc_emails: list[str],
        bcc_emails: list[str],
        date: str | None,
        message_id: str | None,
        plain_text: str | None,
        html_content: str | None,
        cleaned_text: str,
        attachments: list[EmailAttachmentDTO],
        metadata: dict[str, str],
    ) -> None: ...
    def to_dict(self) -> dict[str, Any]: ...

def extract_email_content(data: bytes, mime_type: str) -> EmailExtractionResultDTO: ...
def extract_eml_content(data: bytes) -> EmailExtractionResultDTO: ...
def extract_msg_content(data: bytes) -> EmailExtractionResultDTO: ...
def build_email_text_output(result: EmailExtractionResultDTO) -> str: ...

class XmlExtractionResult:
    content: str
    element_count: int
    unique_elements: list[str]

    def __init__(self, content: str, element_count: int, unique_elements: list[str]) -> None: ...

def parse_xml(xml_bytes: bytes, preserve_whitespace: bool) -> XmlExtractionResult: ...

class TextExtractionResult:
    content: str
    line_count: int
    word_count: int
    character_count: int
    headers: list[str] | None
    links: list[tuple[str, str]] | None
    code_blocks: list[tuple[str, str]] | None

    def __init__(
        self,
        content: str,
        line_count: int,
        word_count: int,
        character_count: int,
        headers: list[str] | None,
        links: list[tuple[str, str]] | None,
        code_blocks: list[tuple[str, str]] | None,
    ) -> None: ...

def parse_text(text_bytes: bytes, is_markdown: bool) -> TextExtractionResult: ...

class PSMMode:
    OsdOnly: int
    AutoOsd: int
    AutoOnly: int
    Auto: int
    SingleColumn: int
    SingleBlockVertical: int
    SingleBlock: int
    SingleLine: int
    SingleWord: int
    CircleWord: int
    SingleChar: int

    def __init__(self, value: int) -> None: ...

class TesseractConfigDTO:
    language: str
    psm: int
    output_format: str
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
    tessedit_use_primary_params_model: bool
    textord_space_size_is_variable: bool
    thresholding_method: bool

    def __init__(
        self,
        language: str = "eng",
        psm: int = 3,
        output_format: str = "markdown",
        enable_table_detection: bool = True,
        table_min_confidence: float = 0.0,
        table_column_threshold: int = 50,
        table_row_threshold_ratio: float = 0.5,
        use_cache: bool = True,
        classify_use_pre_adapted_templates: bool = True,
        language_model_ngram_on: bool = False,
        tessedit_dont_blkrej_good_wds: bool = True,
        tessedit_dont_rowrej_good_wds: bool = True,
        tessedit_enable_dict_correction: bool = True,
        tessedit_char_whitelist: str = "",
        tessedit_use_primary_params_model: bool = True,
        textord_space_size_is_variable: bool = True,
        thresholding_method: bool = False,
    ) -> None: ...

class TableDTO:
    cells: list[list[str]]
    markdown: str
    page_number: int

    def __init__(self, cells: list[list[str]], markdown: str, page_number: int) -> None: ...

class ExtractionResultDTO:
    content: str
    mime_type: str
    metadata: dict[str, str]
    tables: list[TableDTO]

    def __init__(
        self,
        content: str,
        mime_type: str,
        metadata: dict[str, str] | None = None,
        tables: list[TableDTO] | None = None,
    ) -> None: ...

class OCRCacheStats:
    total_files: int
    total_size_mb: float

class BatchItemResult:
    file_path: str
    success: bool
    result: ExtractionResultDTO | None
    error: str | None

    def __init__(
        self, file_path: str, success: bool, result: ExtractionResultDTO | None, error: str | None
    ) -> None: ...

class OCRProcessor:
    def __init__(self, cache_dir: str | None = None) -> None: ...
    def process_image(self, image_bytes: bytes, config: TesseractConfigDTO) -> ExtractionResultDTO: ...
    def process_file(self, file_path: str, config: TesseractConfigDTO) -> ExtractionResultDTO: ...
    def process_files_batch(self, file_paths: list[str], config: TesseractConfigDTO) -> list[BatchItemResult]: ...
    def clear_cache(self) -> None: ...
    def get_cache_stats(self) -> OCRCacheStats: ...

def validate_language_code(lang: str) -> str: ...
def validate_tesseract_version() -> None: ...
