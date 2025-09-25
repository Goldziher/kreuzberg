from collections.abc import Mapping
from enum import Enum
from typing import Any

# Quality and text utilities
def calculate_quality_score(text: str, metadata: Mapping[str, Any] | None = None) -> float: ...
def clean_extracted_text(text: str) -> str: ...
def normalize_spaces(text: str) -> str: ...
def safe_decode(byte_data: bytes, encoding: str | None = None) -> str: ...
def calculate_text_confidence(text: str) -> float: ...
def fix_mojibake(text: str) -> str: ...
def get_encoding_cache_key(data_hash: str, size: int) -> str: ...
def batch_process_texts(texts: list[str]) -> list[str]: ...

# Image preprocessing
def normalize_image_dpi(
    image_array: Any,  # numpy array
    config: ExtractionConfig,
    dpi_info: dict[str, Any] | None = None,
) -> tuple[Any, ImagePreprocessingMetadata]: ...  # returns numpy array
def batch_normalize_images(
    images: list[Any],  # list of numpy arrays
    config: ExtractionConfig,
) -> list[tuple[Any, ImagePreprocessingMetadata]]: ...  # returns list of numpy arrays

class ImagePreprocessingMetadata:
    original_dimensions: tuple[int, int]
    original_dpi: tuple[float, float]
    target_dpi: int
    final_dpi: int
    new_dimensions: tuple[int, int]
    scale_factor: float
    auto_adjusted: bool
    calculated_dpi: float | None
    dimension_clamped: bool
    resample_method: str
    resize_error: str | None
    skipped_resize: bool

class ExtractionConfig:
    target_dpi: int
    max_image_dimension: int
    auto_adjust_dpi: bool
    min_dpi: int
    max_dpi: int
    def __init__(
        self,
        target_dpi: int = 300,
        max_image_dimension: int = 4096,
        auto_adjust_dpi: bool = False,
        min_dpi: int = 72,
        max_dpi: int = 600,
    ) -> None: ...

# Token reduction
class ReductionLevel(Enum):
    Off = 0
    Light = 1
    Moderate = 2
    Aggressive = 3
    Maximum = 4

class TokenReductionConfig:
    level: ReductionLevel
    language_hint: str | None
    preserve_markdown: bool
    preserve_code: bool
    semantic_threshold: float
    enable_parallel: bool
    use_simd: bool
    custom_stopwords: dict[str, list[str]] | None
    preserve_patterns: list[str] | None
    target_reduction: float | None
    enable_semantic_clustering: bool

    def __init__(
        self,
        level: ReductionLevel = ...,
        language_hint: str | None = None,
        preserve_markdown: bool = False,
        preserve_code: bool = True,
        semantic_threshold: float = 0.3,
        enable_parallel: bool = True,
        use_simd: bool = True,
        custom_stopwords: dict[str, list[str]] | None = None,
        preserve_patterns: list[str] | None = None,
        target_reduction: float | None = None,
        enable_semantic_clustering: bool = False,
    ) -> None: ...

def reduce_tokens(
    text: str,
    config: TokenReductionConfig,
    language: str | None = None,
) -> str: ...
def batch_reduce_tokens(
    texts: list[str],
    config: TokenReductionConfig,
    language: str | None = None,
) -> list[str]: ...
def get_reduction_statistics(
    original: str,
    reduced: str,
) -> tuple[float, float, int, int, int, int]: ...
