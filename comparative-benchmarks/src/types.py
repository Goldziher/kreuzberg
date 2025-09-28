from __future__ import annotations

from enum import Enum
from pathlib import Path
from typing import Any, Protocol

import msgspec

from .config_defaults import DefaultValues


class Framework(str, Enum):
    KREUZBERG_V3_SYNC = "kreuzberg_v3_sync"
    KREUZBERG_V3_ASYNC = "kreuzberg_v3_async"
    KREUZBERG_V4_SYNC = "kreuzberg_v4_sync"
    KREUZBERG_V4_ASYNC = "kreuzberg_v4_async"
    DOCLING = "docling"
    MARKITDOWN = "markitdown"
    UNSTRUCTURED = "unstructured"
    EXTRACTOUS = "extractous"


class FileType(str, Enum):
    PDF = "pdf"
    PDF_SCANNED = "pdf_scanned"
    DOCX = "docx"
    PPTX = "pptx"
    XLSX = "xlsx"
    XLS = "xls"
    ODT = "odt"
    HTML = "html"
    MARKDOWN = "md"
    TXT = "txt"
    RTF = "rtf"
    EPUB = "epub"
    MSG = "msg"
    EML = "eml"
    CSV = "csv"
    JSON = "json"
    YAML = "yaml"
    RST = "rst"
    ORG = "org"
    IMAGE_PNG = "png"
    IMAGE_JPG = "jpg"
    IMAGE_JPEG = "jpeg"
    IMAGE_BMP = "bmp"


class DocumentCategory(str, Enum):
    TINY = "tiny"
    SMALL = "small"
    MEDIUM = "medium"
    LARGE = "large"
    HUGE = "huge"

    PDF_STANDARD = "pdf_standard"
    PDF_SCANNED = "pdf_scanned"
    PDF_COMPLEX = "pdf_complex"
    OFFICE = "office"
    WEB = "web"
    TEXT = "text"
    EMAIL = "email"
    EBOOK = "ebook"
    DATA = "data"
    IMAGES = "images"

    ENGLISH = "english"
    UNICODE = "unicode"


class ExtractionStatus(str, Enum):
    SUCCESS = "success"
    PARTIAL = "partial"
    FAILED = "failed"
    TIMEOUT = "timeout"
    SKIPPED = "skipped"


class ExtractorProtocol(Protocol):
    def extract_text(self, file_path: str | Path) -> str: ...

    def extract_with_metadata(
        self, file_path: str | Path
    ) -> tuple[str, dict[str, Any]]: ...


class AsyncExtractorProtocol(Protocol):
    async def extract_text(self, file_path: str | Path) -> str: ...

    async def extract_with_metadata(
        self, file_path: str | Path
    ) -> tuple[str, dict[str, Any]]: ...


class ResourceMetrics(msgspec.Struct):
    timestamp: float
    cpu_percent: float
    memory_rss: int
    memory_vms: int
    num_threads: int
    open_files: int
    io_read_bytes: int | None = None
    io_write_bytes: int | None = None
    io_read_count: int | None = None
    io_write_count: int | None = None


class ExtractionResult(msgspec.Struct, kw_only=True):
    file_path: str
    file_size: int
    framework: Framework
    status: ExtractionStatus
    extraction_time: float | None = None
    extracted_text: str | None = None
    character_count: int | None = None
    word_count: int | None = None
    error_type: str | None = None
    error_message: str | None = None
    resource_metrics: list[ResourceMetrics] = msgspec.field(default_factory=list)
    extracted_metadata: dict[str, Any] | None = None


class BenchmarkResult(msgspec.Struct, kw_only=True):
    file_path: str
    file_size: int
    file_type: FileType
    category: DocumentCategory
    framework: Framework
    iteration: int

    extraction_time: float
    startup_time: float | None = None

    peak_memory_mb: float
    avg_memory_mb: float
    peak_cpu_percent: float
    avg_cpu_percent: float
    total_io_mb: float | None = None

    status: ExtractionStatus
    character_count: int | None = None
    word_count: int | None = None
    error_type: str | None = None
    error_message: str | None = None

    quality_metrics: dict[str, Any] | None = None
    overall_quality_score: float | None = None
    extracted_text: str | None = None

    extracted_metadata: dict[str, Any] | None = None

    attempts: int = 1
    timestamp: float = msgspec.field(default_factory=lambda: __import__("time").time())
    platform: str = msgspec.field(
        default_factory=lambda: __import__("platform").system()
    )
    python_version: str = msgspec.field(
        default_factory=lambda: __import__("platform").python_version()
    )


class BenchmarkSummary(msgspec.Struct, kw_only=True):
    framework: Framework
    category: DocumentCategory
    total_files: int
    successful_files: int
    failed_files: int
    partial_files: int
    timeout_files: int

    avg_extraction_time: float | None = None
    median_extraction_time: float | None = None
    min_extraction_time: float | None = None
    max_extraction_time: float | None = None
    std_extraction_time: float | None = None

    avg_peak_memory_mb: float | None = None
    avg_cpu_percent: float | None = None

    files_per_second: float | None = None
    mb_per_second: float | None = None

    success_rate: float

    avg_character_count: int | None = None
    avg_word_count: int | None = None


class AggregatedResults(msgspec.Struct, kw_only=True):
    total_runs: int
    total_files_processed: int
    total_time_seconds: float

    framework_summaries: dict[Framework, list[BenchmarkSummary]]

    category_summaries: dict[DocumentCategory, list[BenchmarkSummary]]

    framework_category_matrix: dict[str, BenchmarkSummary]

    failure_patterns: dict[str, int]
    timeout_files: list[str]

    performance_over_iterations: dict[Framework, list[float]]
    platform_results: dict[str, dict[Framework, BenchmarkSummary]]


class BenchmarkConfig(msgspec.Struct, kw_only=True):
    iterations: int = DefaultValues.DEFAULT_ITERATIONS
    warmup_runs: int = DefaultValues.DEFAULT_WARMUP_RUNS
    cooldown_seconds: int = DefaultValues.COOLDOWN_SECONDS

    timeout_seconds: int = DefaultValues.EXTRACTION_TIMEOUT_SECONDS
    max_run_duration_minutes: int = DefaultValues.MAX_RUN_DURATION_MINUTES
    max_memory_mb: int = DefaultValues.MAX_MEMORY_MB
    max_cpu_percent: int = DefaultValues.MAX_CPU_PERCENT

    max_retries: int = DefaultValues.MAX_RETRIES
    retry_backoff: float = 2.0
    continue_on_error: bool = True
    skip_on_repeated_failure: bool = True

    frameworks: list[Framework] = msgspec.field(default_factory=list)
    categories: list[DocumentCategory] = msgspec.field(default_factory=list)
    file_types: list[FileType] | None = None

    output_dir: Path = msgspec.field(default_factory=lambda: Path("results"))
    save_extracted_text: bool = False
    compression: bool = True
    detailed_errors: bool = True

    sampling_interval_ms: int = DefaultValues.SAMPLING_INTERVAL_MS
    table_extraction_only: bool = False

    enable_quality_assessment: bool = False
