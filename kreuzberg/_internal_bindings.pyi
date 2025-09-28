from typing import Any

class CacheStats:
    total_files: int
    total_size_mb: float
    available_space_mb: float
    oldest_file_age_days: float
    newest_file_age_days: float

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
