from __future__ import annotations

import json
import sys
import warnings
from collections.abc import Awaitable, Callable, Mapping
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal, NamedTuple, TypedDict

import langcodes
import msgspec
from html_to_markdown._html_to_markdown import (
    ConversionOptions as HTMLToMarkdownConversionOptions,
)
from html_to_markdown._html_to_markdown import (
    PreprocessingOptions as HTMLToMarkdownPreprocessingOptions,
)

from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg._utils._table import (
    enhance_table_markdown,
    export_table_to_csv,
    export_table_to_tsv,
    extract_table_structure_info,
)
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from kreuzberg._utils._device import DeviceType

if sys.version_info < (3, 11):  # pragma: no cover
    from typing_extensions import NotRequired
else:  # pragma: no cover
    from typing import NotRequired

if TYPE_CHECKING:
    from PIL.Image import Image
    from polars import DataFrame

OcrBackendType = Literal["tesseract", "easyocr", "paddleocr"]
OutputFormatType = Literal["text", "tsv", "hocr", "markdown"]
ErrorContextType = Literal["batch_processing", "optional_feature", "single_extraction", "unknown"]
class ConfigDict:
    def to_dict(self, include_none: bool = False) -> dict[str, Any]:
        result = msgspec.to_builtins(
            self,
            builtin_types=(type(None),),
            order="deterministic",
        )

        if include_none:
            return result  # type: ignore[no-any-return]

        return {k: v for k, v in result.items() if v is not None}


class PSMMode(Enum):
    OSD_ONLY = 0
    """Orientation and script detection only."""
    AUTO_OSD = 1
    """Automatic page segmentation with orientation and script detection."""
    AUTO_ONLY = 2
    """Automatic page segmentation without OSD."""
    AUTO = 3
    """Fully automatic page segmentation (default)."""
    SINGLE_COLUMN = 4
    """Assume a single column of text."""
    SINGLE_BLOCK_VERTICAL = 5
    """Assume a single uniform block of vertically aligned text."""
    SINGLE_BLOCK = 6
    """Assume a single uniform block of text."""
    SINGLE_LINE = 7
    """Treat the image as a single text line."""
    SINGLE_WORD = 8
    """Treat the image as a single word."""
    CIRCLE_WORD = 9
    """Treat the image as a single word in a circle."""
    SINGLE_CHAR = 10
    """Treat the image as a single character."""


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class TesseractConfig(ConfigDict):
    classify_use_pre_adapted_templates: bool = True
    """Whether to use pre-adapted templates during classification to improve recognition accuracy."""
    language: str = "eng"
    """Language code to use for OCR.
    Examples:
            -   'eng' for English
            -   'deu' for German
            -    multiple languages combined with '+', e.g. 'eng+deu'
    """
    language_model_ngram_on: bool = False
    """Enable or disable the use of n-gram-based language models for improved text recognition.
    Default is False for optimal performance on modern documents. Enable for degraded or historical text."""
    psm: PSMMode = PSMMode.AUTO
    """Page segmentation mode (PSM) to guide Tesseract on how to segment the image (e.g., single block, single line)."""
    tessedit_dont_blkrej_good_wds: bool = True
    """If True, prevents block rejection of words identified as good, improving text output quality."""
    tessedit_dont_rowrej_good_wds: bool = True
    """If True, prevents row rejection of words identified as good, avoiding unnecessary omissions."""
    tessedit_enable_dict_correction: bool = True
    """Enable or disable dictionary-based correction for recognized text to improve word accuracy."""
    tessedit_char_whitelist: str = ""
    """Whitelist of characters that Tesseract is allowed to recognize. Empty string means no restriction."""
    tessedit_use_primary_params_model: bool = True
    """If True, forces the use of the primary parameters model for text recognition."""
    textord_space_size_is_variable: bool = True
    """Allow variable spacing between words, useful for text with irregular spacing."""
    thresholding_method: bool = False
    """Enable or disable specific thresholding methods during image preprocessing for better OCR accuracy."""
    output_format: OutputFormatType = "markdown"
    """Output format: 'markdown' (default), 'text', 'tsv' (for structured data), or 'hocr' (HTML-based)."""
    enable_table_detection: bool = False
    """Enable table structure detection from TSV output."""
    table_column_threshold: int = 20
    """Pixel threshold for column clustering in table detection."""
    table_row_threshold_ratio: float = 0.5
    """Row threshold as ratio of mean text height for table detection."""
    table_min_confidence: float = 30.0
    """Minimum confidence score to include a word in table extraction."""


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class EasyOCRConfig(ConfigDict):
    add_margin: float = 0.1
    """Extend bounding boxes in all directions."""
    adjust_contrast: float = 0.5
    """Target contrast level for low contrast text."""
    beam_width: int = 5
    """Beam width for beam search in recognition."""
    canvas_size: int = 2560
    """Maximum image dimension for detection."""
    contrast_ths: float = 0.1
    """Contrast threshold for preprocessing."""
    decoder: Literal["greedy", "beamsearch", "wordbeamsearch"] = "greedy"
    """Decoder method. Options: 'greedy', 'beamsearch', 'wordbeamsearch'."""
    height_ths: float = 0.5
    """Maximum difference in box height for merging."""
    language: str | list[str] = "en"
    """Language or languages to use for OCR. Can be a single language code (e.g., 'en'),
    a comma-separated string of language codes (e.g., 'en,ch_sim'), or a list of language codes."""
    link_threshold: float = 0.4
    """Link confidence threshold."""
    low_text: float = 0.4
    """Text low-bound score."""
    mag_ratio: float = 1.0
    """Image magnification ratio."""
    min_size: int = 10
    """Minimum text box size in pixels."""
    rotation_info: list[int] | None = None
    """List of angles to try for detection."""
    slope_ths: float = 0.1
    """Maximum slope for merging text boxes."""
    text_threshold: float = 0.7
    """Text confidence threshold."""
    use_gpu: bool = False
    """Whether to use GPU for inference. DEPRECATED: Use 'device' parameter instead."""
    device: DeviceType = "auto"
    """Device to use for inference. Options: 'cpu', 'cuda', 'mps', 'auto'."""
    gpu_memory_limit: float | None = None
    """Maximum GPU memory to use in GB. None for no limit."""
    fallback_to_cpu: bool = True
    """Whether to fallback to CPU if requested device is unavailable."""
    width_ths: float = 0.5
    """Maximum horizontal distance for merging boxes."""
    x_ths: float = 1.0
    """Maximum horizontal distance for paragraph merging."""
    y_ths: float = 0.5
    """Maximum vertical distance for paragraph merging."""
    ycenter_ths: float = 0.5
    """Maximum shift in y direction for merging."""

    def __post_init__(self) -> None:
        if isinstance(self.language, list):
            object.__setattr__(self, "language", tuple(self.language))
        if isinstance(self.rotation_info, list):
            object.__setattr__(self, "rotation_info", tuple(self.rotation_info))


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class PaddleOCRConfig(ConfigDict):
    cls_image_shape: str = "3,48,192"
    """Image shape for classification algorithm in format 'channels,height,width'."""
    det_algorithm: Literal["DB", "EAST", "SAST", "PSE", "FCE", "PAN", "CT", "DB++", "Layout"] = "DB"
    """Detection algorithm."""
    det_db_box_thresh: float = 0.5
    """DEPRECATED in PaddleOCR 3.2.0+: Use 'text_det_box_thresh' instead. Score threshold for detected boxes."""
    det_db_thresh: float = 0.3
    """DEPRECATED in PaddleOCR 3.2.0+: Use 'text_det_thresh' instead. Binarization threshold for DB output map."""
    det_db_unclip_ratio: float = 2.0
    """DEPRECATED in PaddleOCR 3.2.0+: Use 'text_det_unclip_ratio' instead. Expansion ratio for detected text boxes."""
    det_east_cover_thresh: float = 0.1
    """Score threshold for EAST output boxes."""
    det_east_nms_thresh: float = 0.2
    """NMS threshold for EAST model output boxes."""
    det_east_score_thresh: float = 0.8
    """Binarization threshold for EAST output map."""
    det_max_side_len: int = 960
    """Maximum size of image long side. Images exceeding this will be proportionally resized."""
    det_model_dir: str | None = None
    """Directory for detection model. If None, uses default model location."""
    drop_score: float = 0.5
    """Filter recognition results by confidence score. Results below this are discarded."""
    enable_mkldnn: bool = False
    """Whether to enable MKL-DNN acceleration (Intel CPU only)."""
    gpu_mem: int = 8000
    """DEPRECATED in PaddleOCR 3.2.0+: Parameter no longer supported. GPU memory size (in MB) to use for initialization."""
    language: str = "en"
    """Language to use for OCR."""
    max_text_length: int = 25
    """Maximum text length that the recognition algorithm can recognize."""
    rec: bool = True
    """Enable text recognition when using the ocr() function."""
    rec_algorithm: Literal[
        "CRNN",
        "SRN",
        "NRTR",
        "SAR",
        "SEED",
        "SVTR",
        "SVTR_LCNet",
        "ViTSTR",
        "ABINet",
        "VisionLAN",
        "SPIN",
        "RobustScanner",
        "RFL",
    ] = "CRNN"
    """Recognition algorithm."""
    rec_image_shape: str = "3,32,320"
    """Image shape for recognition algorithm in format 'channels,height,width'."""
    rec_model_dir: str | None = None
    """Directory for recognition model. If None, uses default model location."""
    table: bool = True
    """Whether to enable table recognition."""
    use_angle_cls: bool = True
    """DEPRECATED in PaddleOCR 3.2.0+: Use 'use_textline_orientation' instead. Whether to use text orientation classification model."""
    use_gpu: bool = False
    """DEPRECATED in PaddleOCR 3.2.0+: Parameter no longer supported. Use hardware acceleration flags instead."""
    device: DeviceType = "auto"
    """Device to use for inference. Options: 'cpu', 'cuda', 'auto'. Note: MPS not supported by PaddlePaddle."""
    gpu_memory_limit: float | None = None
    """DEPRECATED in PaddleOCR 3.2.0+: Parameter no longer supported. Maximum GPU memory to use in GB."""
    fallback_to_cpu: bool = True
    """Whether to fallback to CPU if requested device is unavailable."""
    use_space_char: bool = True
    """Whether to recognize spaces."""
    use_zero_copy_run: bool = False
    """Whether to enable zero_copy_run for inference optimization."""

    text_det_thresh: float = 0.3
    """Binarization threshold for text detection output map (replaces det_db_thresh)."""
    text_det_box_thresh: float = 0.5
    """Score threshold for detected text boxes (replaces det_db_box_thresh)."""
    text_det_unclip_ratio: float = 2.0
    """Expansion ratio for detected text boxes (replaces det_db_unclip_ratio)."""
    use_textline_orientation: bool = True
    """Whether to use text line orientation classification model (replaces use_angle_cls)."""

# Backward-compatible alias for OCR configuration union used by prior releases.
OcrConfig = TesseractConfig | EasyOCRConfig | PaddleOCRConfig
OCRConfig = OcrConfig


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class GMFTConfig(ConfigDict):
    def __post_init__(self) -> None:
        warnings.warn(
            "GMFTConfig is deprecated and will be removed in Kreuzberg v4.0. "
            "Install `kreuzberg[gmft]` only if you still rely on GMFT. "
            "Future versions use native TATR-based table extraction via TableExtractionConfig.",
            FutureWarning,
            stacklevel=2,
        )

    verbosity: int = 0
    """
    Verbosity level for logging.

    0: errors only
    1: print warnings
    2: print warnings and info
    3: print warnings, info, and debug
    """
    formatter_base_threshold: float = 0.3
    """
    Base threshold for the confidence demanded of a table feature (row/column).

    Note that a low threshold is actually better, because overzealous rows means that generally, numbers are still aligned and there are just many empty rows (having fewer rows than expected merges cells, which is bad).
    """
    cell_required_confidence: dict[Literal[0, 1, 2, 3, 4, 5, 6], float] = field(
        default_factory=lambda: {
            0: 0.3,
            1: 0.3,
            2: 0.3,
            3: 0.3,
            4: 0.5,
            5: 0.5,
            6: 99,
        },
        hash=False,
    )
    """
    Confidences required (>=) for a row/column feature to be considered good. See TATRFormattedTable.id2label

    But low confidences may be better than too high confidence (see formatter_base_threshold)
    """
    detector_base_threshold: float = 0.9
    """Minimum confidence score required for a table"""
    remove_null_rows: bool = True
    """
    Flag to remove rows with no text.
    """
    enable_multi_header: bool = False
    """
    Enable multi-indices in the dataframe.

    If false, then multiple headers will be merged column-wise.
    """
    semantic_spanning_cells: bool = False
    """
    [Experimental] Enable semantic spanning cells, which often encode hierarchical multi-level indices.
    """
    semantic_hierarchical_left_fill: Literal["algorithm", "deep"] | None = "algorithm"
    """
    [Experimental] When semantic spanning cells is enabled, when a left header is detected which might represent a group of rows, that same value is reduplicated for each row.

    Possible values: 'algorithm', 'deep', None.
    """
    large_table_if_n_rows_removed: int = 8
    """
    If >= n rows are removed due to non-maxima suppression (NMS), then this table is classified as a large table.
    """
    large_table_threshold: int = 10
    """
    With large tables, table transformer struggles with placing too many overlapping rows. Luckily, with more rows, we have more info on the usual size of text, which we can use to make a guess on the height such that no rows are merged or overlapping.

    Large table assumption is only applied when (# of rows > large_table_threshold) AND (total overlap > large_table_row_overlap_threshold). Set 9999 to disable; set 0 to force large table assumption to run every time.
    """
    large_table_row_overlap_threshold: float = 0.2
    """
    With large tables, table transformer struggles with placing too many overlapping rows. Luckily, with more rows, we have more info on the usual size of text, which we can use to make a guess on the height such that no rows are merged or overlapping.

    Large table assumption is only applied when (# of rows > large_table_threshold) AND (total overlap > large_table_row_overlap_threshold).
    """
    large_table_maximum_rows: int = 1000
    """
    Maximum number of rows allowed for a large table.
    """
    force_large_table_assumption: bool | None = None
    """
    Force the large table assumption to be applied, regardless of the number of rows and overlap.
    """
    total_overlap_reject_threshold: float = 0.9
    """
    Reject if total overlap is > 90% of table area.
    """
    total_overlap_warn_threshold: float = 0.1
    """
    Warn if total overlap is > 10% of table area.
    """
    nms_warn_threshold: int = 5
    """
    Warn if non maxima suppression removes > 5 rows.
    """
    iob_reject_threshold: float = 0.05
    """
    Reject if iob between textbox and cell is < 5%.
    """
    iob_warn_threshold: float = 0.5
    """
    Warn if iob between textbox and cell is < 50%.
    """


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class ImageOCRConfig(ConfigDict):
    """Configuration for OCR processing of extracted images."""

    enabled: bool = False
    """Whether to perform OCR on extracted images."""
    backend: OcrBackendType | None = None
    """OCR backend for image OCR. Falls back to main ocr_backend when None."""
    backend_config: TesseractConfig | PaddleOCRConfig | EasyOCRConfig | None = None
    """Backend-specific configuration for image OCR."""
    min_dimensions: tuple[int, int] = (50, 50)
    """Minimum (width, height) in pixels for image OCR eligibility."""
    max_dimensions: tuple[int, int] = (10000, 10000)
    """Maximum (width, height) in pixels for image OCR eligibility."""
    allowed_formats: frozenset[str] = frozenset(
        {
            "jpg",
            "jpeg",
            "png",
            "gif",
            "bmp",
            "tiff",
            "tif",
            "webp",
            "jp2",
            "jpx",
            "jpm",
            "mj2",
            "pnm",
            "pbm",
            "pgm",
            "ppm",
        }
    )
    """Allowed image formats for OCR processing (lowercase, without dot)."""
    batch_size: int = 4
    """Number of images to process in parallel for OCR."""
    timeout_seconds: int = 30
    """Maximum time in seconds for OCR processing per image."""

    def __post_init__(self) -> None:
        if isinstance(self.allowed_formats, list):
            object.__setattr__(self, "allowed_formats", frozenset(self.allowed_formats))


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class LanguageDetectionConfig(ConfigDict):
    model: Literal["lite", "full", "auto"] = "auto"
    """Language detection model to use:
    - 'lite': Smaller, faster model with good accuracy
    - 'full': Larger model with highest accuracy
    - 'auto': Automatically choose based on memory availability (default)
    """
    top_k: int = 3
    """Maximum number of languages to return for multilingual detection."""
    multilingual: bool = False
    """If True, uses multilingual detection to handle mixed-language text.
    If False, uses single language detection."""
    cache_dir: str | None = None
    """Custom directory for model cache. If None, uses system default."""
    low_memory: bool = True
    """Deprecated. Use 'model' parameter instead. If True, uses 'lite' model."""


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class SpacyEntityExtractionConfig(ConfigDict):
    model_cache_dir: str | Path | None = None
    """Directory to cache spaCy models. If None, uses spaCy's default."""
    language_models: dict[str, str] | tuple[tuple[str, str], ...] | None = None
    """Mapping of language codes to spaCy model names.

    If None, uses default mappings:
    - en: en_core_web_sm
    - de: de_core_news_sm
    - fr: fr_core_news_sm
    - es: es_core_news_sm
    - pt: pt_core_news_sm
    - it: it_core_news_sm
    - nl: nl_core_news_sm
    - zh: zh_core_web_sm
    - ja: ja_core_news_sm
    """
    fallback_to_multilingual: bool = True
    """If True and language-specific model fails, try xx_ent_wiki_sm (multilingual)."""
    max_doc_length: int = 1000000
    """Maximum document length for spaCy processing."""
    batch_size: int = 1000
    """Batch size for processing multiple texts."""

    def __post_init__(self) -> None:
        if isinstance(self.model_cache_dir, Path):
            object.__setattr__(self, "model_cache_dir", str(self.model_cache_dir))

        if self.language_models is None:
            object.__setattr__(self, "language_models", self._get_default_language_models())

        if isinstance(self.language_models, dict):
            object.__setattr__(self, "language_models", tuple(sorted(self.language_models.items())))

    @staticmethod
    def _get_default_language_models() -> dict[str, str]:
        return {
            "en": "en_core_web_sm",
            "de": "de_core_news_sm",
            "fr": "fr_core_news_sm",
            "es": "es_core_news_sm",
            "pt": "pt_core_news_sm",
            "it": "it_core_news_sm",
            "nl": "nl_core_news_sm",
            "zh": "zh_core_web_sm",
            "ja": "ja_core_news_sm",
            "ko": "ko_core_news_sm",
            "ru": "ru_core_news_sm",
            "pl": "pl_core_news_sm",
            "ro": "ro_core_news_sm",
            "el": "el_core_news_sm",
            "da": "da_core_news_sm",
            "fi": "fi_core_news_sm",
            "nb": "nb_core_news_sm",
            "sv": "sv_core_news_sm",
            "ca": "ca_core_news_sm",
            "hr": "hr_core_news_sm",
            "lt": "lt_core_news_sm",
            "mk": "mk_core_news_sm",
            "sl": "sl_core_news_sm",
            "uk": "uk_core_news_sm",
            "xx": "xx_ent_wiki_sm",
        }

    def get_model_for_language(self, language_code: str) -> str | None:
        if not self.language_models:
            return None

        models_dict = dict(self.language_models) if isinstance(self.language_models, tuple) else self.language_models

        if language_code in models_dict:
            return models_dict[language_code]

        base_lang = language_code.split("-")[0].lower()
        if base_lang in models_dict:
            return models_dict[base_lang]

        return None

    def get_fallback_model(self) -> str | None:
        return "xx_ent_wiki_sm" if self.fallback_to_multilingual else None


class ProcessingErrorDict(TypedDict):
    feature: str
    """Name of the feature that failed (e.g., 'chunking', 'entity_extraction', 'keyword_extraction')."""
    error_type: str
    """Type of the exception that occurred (e.g., 'RuntimeError', 'ValidationError')."""
    error_message: str
    """Human-readable error message."""
    traceback: str
    """Full Python traceback for debugging."""


class BoundingBox(TypedDict):
    left: int
    """X coordinate of the left edge."""
    top: int
    """Y coordinate of the top edge."""
    width: int
    """Width of the bounding box."""
    height: int
    """Height of the bounding box."""


class TSVWord(TypedDict):
    level: int
    """Hierarchy level (1=page, 2=block, 3=para, 4=line, 5=word)."""
    page_num: int
    """Page number."""
    block_num: int
    """Block number within the page."""
    par_num: int
    """Paragraph number within the block."""
    line_num: int
    """Line number within the paragraph."""
    word_num: int
    """Word number within the line."""
    left: int
    """X coordinate of the left edge of the word."""
    top: int
    """Y coordinate of the top edge of the word."""
    width: int
    """Width of the word bounding box."""
    height: int
    """Height of the word bounding box."""
    conf: float
    """Confidence score (0-100)."""
    text: str
    """The recognized text content."""


class TableCell(TypedDict):
    row: int
    """Row index (0-based)."""
    col: int
    """Column index (0-based)."""
    text: str
    """Cell text content."""
    bbox: BoundingBox
    """Bounding box of the cell."""
    confidence: float
    """Average confidence of words in the cell."""


class TableData(TypedDict):
    cropped_image: Image
    """The cropped image of the table."""
    df: DataFrame | None
    """The table data as a polars DataFrame."""
    page_number: int
    """The page number of the table."""
    text: str
    """The table text as a markdown string."""


class ImagePreprocessingMetadata(NamedTuple):
    """Metadata about image preprocessing operations for OCR."""

    original_dimensions: tuple[int, int]
    """Original image dimensions (width, height) in pixels."""
    original_dpi: tuple[float, float]
    """Original image DPI (horizontal, vertical)."""
    target_dpi: int
    """Target DPI that was requested."""
    scale_factor: float
    """Scale factor applied to the image."""
    auto_adjusted: bool
    """Whether DPI was automatically adjusted due to size constraints."""
    final_dpi: int | None = None
    """Final DPI used after processing."""
    new_dimensions: tuple[int, int] | None = None
    """New image dimensions after processing (width, height) in pixels."""
    resample_method: str | None = None
    """Resampling method used (LANCZOS, BICUBIC, etc.)."""
    skipped_resize: bool = False
    """Whether resizing was skipped (no change needed)."""
    dimension_clamped: bool = False
    """Whether image was clamped to maximum dimension constraints."""
    calculated_dpi: int | None = None
    """DPI calculated during auto-adjustment."""
    resize_error: str | None = None
    """Error message if resizing failed."""


class Metadata(TypedDict, total=False):
    abstract: NotRequired[str]
    """Document abstract or summary."""
    authors: NotRequired[list[str]]
    """List of document authors."""
    categories: NotRequired[list[str]]
    """Categories or classifications."""
    citations: NotRequired[list[str]]
    """Citation identifiers."""
    comments: NotRequired[str]
    """General comments."""
    copyright: NotRequired[str]
    """Copyright information."""
    created_at: NotRequired[str]
    """Creation timestamp in ISO format."""
    created_by: NotRequired[str]
    """Document creator."""
    description: NotRequired[str]
    """Document description."""
    fonts: NotRequired[list[str]]
    """List of fonts used in the document."""
    height: NotRequired[int]
    """Height of the document page/slide/image, if applicable."""
    identifier: NotRequired[str]
    """Unique document identifier."""
    keywords: NotRequired[list[str]]
    """Keywords or tags."""
    languages: NotRequired[list[str]]
    """Document language code."""
    license: NotRequired[str]
    """License information."""
    modified_at: NotRequired[str]
    """Last modification timestamp in ISO format."""
    modified_by: NotRequired[str]
    """Username of last modifier."""
    organization: NotRequired[str | list[str]]
    """Organizational affiliation."""
    publisher: NotRequired[str]
    """Publisher or organization name."""
    references: NotRequired[list[str]]
    """Reference entries."""
    status: NotRequired[str]
    """Document status (e.g., draft, final)."""
    subject: NotRequired[str]
    """Document subject or topic."""
    subtitle: NotRequired[str]
    """Document subtitle."""
    summary: NotRequired[str]
    """Document Summary"""
    title: NotRequired[str]
    """Document title."""
    version: NotRequired[str]
    """Version identifier or revision number."""
    width: NotRequired[int]
    """Width of the document page/slide/image, if applicable."""
    email_from: NotRequired[str]
    """Email sender (from field)."""
    email_to: NotRequired[str]
    """Email recipient (to field)."""
    email_cc: NotRequired[str]
    """Email carbon copy recipients."""
    email_bcc: NotRequired[str]
    """Email blind carbon copy recipients."""
    date: NotRequired[str]
    """Email date or document date."""
    attachments: NotRequired[list[str]]
    """List of attachment names."""
    content: NotRequired[str]
    """Content metadata field."""
    parse_error: NotRequired[str]
    """Parse error information."""
    warning: NotRequired[str]
    """Warning messages."""
    table_count: NotRequired[int]
    """Number of tables extracted from the document."""
    tables_detected: NotRequired[int]
    """Number of tables detected in the document."""
    tables_summary: NotRequired[str]
    """Summary of table extraction results."""
    quality_score: NotRequired[float]
    """Quality score for extracted content (0.0-1.0)."""
    image_preprocessing: NotRequired[ImagePreprocessingMetadata]
    """Metadata about image preprocessing operations (DPI adjustments, scaling, etc.)."""
    source_format: NotRequired[str]
    """Source format of the extracted content."""
    error: NotRequired[str]
    """Error message if extraction failed."""
    error_context: NotRequired[dict[str, Any]]
    """Error context information for debugging."""
    json_schema: NotRequired[dict[str, Any]]
    """JSON schema information extracted from structured data."""
    notes: NotRequired[list[str]]
    """Notes or additional information extracted from documents."""
    note: NotRequired[str]
    """Single note or annotation."""
    name: NotRequired[str]
    """Name field from structured data."""
    body: NotRequired[str]
    """Body text content."""
    text: NotRequired[str]
    """Generic text content."""
    message: NotRequired[str]
    """Message or communication content."""
    attributes: NotRequired[dict[str, Any]]
    """Additional attributes extracted from structured data (e.g., custom text fields with dotted keys)."""
    token_reduction: NotRequired[dict[str, float]]
    """Token reduction statistics including reduction ratios and counts."""
    processing_errors: NotRequired[list[ProcessingErrorDict]]
    """List of processing errors that occurred during extraction."""
    extraction_error: NotRequired[dict[str, Any]]
    """Error information for critical extraction failures."""


_VALID_METADATA_KEYS = {
    "abstract",
    "authors",
    "categories",
    "citations",
    "comments",
    "content",
    "copyright",
    "created_at",
    "created_by",
    "description",
    "fonts",
    "height",
    "identifier",
    "keywords",
    "languages",
    "license",
    "modified_at",
    "modified_by",
    "organization",
    "parse_error",
    "publisher",
    "references",
    "status",
    "subject",
    "subtitle",
    "summary",
    "title",
    "version",
    "warning",
    "width",
    "email_from",
    "email_to",
    "email_cc",
    "email_bcc",
    "date",
    "attachments",
    "table_count",
    "tables_summary",
    "quality_score",
    "image_preprocessing",
    "source_format",
    "error",
    "error_context",
    "json_schema",
    "notes",
    "note",
    "name",
    "body",
    "text",
    "message",
    "attributes",
    "token_reduction",
    "processing_errors",
    "extraction_error",
}


def normalize_metadata(data: dict[str, Any] | None) -> Metadata:
    if not data:
        return {}

    normalized: Metadata = {}
    attributes: dict[str, Any] = {}

    for key, value in data.items():
        if value is not None:
            if key in _VALID_METADATA_KEYS:
                normalized[key] = value  # type: ignore[literal-required]
            elif "." in key and key.split(".")[-1] in {
                "title",
                "name",
                "subject",
                "description",
                "content",
                "body",
                "text",
                "message",
                "note",
                "abstract",
                "summary",
            }:
                attributes[key] = value

    if attributes:
        normalized["attributes"] = attributes

    return normalized


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class Entity:
    type: str
    """e.g., PERSON, ORGANIZATION, LOCATION, DATE, EMAIL, PHONE, or custom"""
    text: str
    """Extracted text"""
    start: int
    """Start character offset in the content"""
    end: int
    """End character offset in the content"""


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class ExtractedImage:
    data: bytes
    format: str
    filename: str | None = None
    page_number: int | None = None
    dimensions: tuple[int, int] | None = None
    colorspace: str | None = None
    bits_per_component: int | None = None
    is_mask: bool = False
    description: str | None = None


@dataclass(slots=True)
class ImageOCRResult:
    image: ExtractedImage
    ocr_result: ExtractionResult
    confidence_score: float | None = None
    processing_time: float | None = None
    skipped_reason: str | None = None


@dataclass(slots=True)
class ExtractionResult:
    content: str
    """The extracted content."""
    mime_type: str
    """The mime type of the extracted content. Is either text/plain or text/markdown."""
    metadata: Metadata = field(default_factory=lambda: Metadata())
    """The metadata of the content."""
    tables: list[TableData] = field(default_factory=list)
    """Extracted tables. Is an empty list if 'extract_tables' is not set to True in the ExtractionConfig."""
    chunks: list[str] = field(default_factory=list)
    """The extracted content chunks. This is an empty list if 'chunk_content' is not set to True in the ExtractionConfig."""
    images: list[ExtractedImage] = field(default_factory=list)
    """Extracted images. Empty list if 'extract_images' is not enabled."""
    image_ocr_results: list[ImageOCRResult] = field(default_factory=list)
    """OCR results from extracted images. Empty list if disabled or none processed."""
    entities: list[Entity] | None = None
    """Extracted entities, if entity extraction is enabled."""
    keywords: list[tuple[str, float]] | None = None
    """Extracted keywords and their scores, if keyword extraction is enabled."""
    detected_languages: list[str] | None = None
    """Languages detected in the extracted content, if language detection is enabled."""
    document_type: str | None = None
    """Detected document type, if document type detection is enabled."""
    document_type_confidence: float | None = None
    """Confidence of the detected document type."""
    layout: DataFrame | None = field(default=None, repr=False, hash=False)
    """Internal layout data from OCR, not for public use."""

    def to_dict(self, include_none: bool = False) -> dict[str, Any]:
        result = msgspec.to_builtins(
            self,
            builtin_types=(type(None),),
            order="deterministic",
        )

        if include_none:
            return result  # type: ignore[no-any-return]

        return {k: v for k, v in result.items() if v is not None}

    def export_tables_to_csv(self) -> list[str]:
        if not self.tables:  # pragma: no cover
            return []

        return [export_table_to_csv(table) for table in self.tables]

    def export_tables_to_tsv(self) -> list[str]:
        if not self.tables:  # pragma: no cover
            return []

        return [export_table_to_tsv(table) for table in self.tables]

    def get_table_summaries(self) -> list[dict[str, Any]]:
        if not self.tables:  # pragma: no cover
            return []

        return [extract_table_structure_info(table) for table in self.tables]

    def to_markdown(self, show_metadata: bool = False) -> str:
        """Render the extraction result as a Markdown document."""
        sections: list[str] = []

        content_block = self.content.rstrip()
        if content_block:
            sections.append(content_block)

        if self.tables:
            table_sections: list[str] = ["## Tables"]
            for index, table in enumerate(self.tables, start=1):
                table_heading = f"### Table {index}"
                table_markdown = enhance_table_markdown(table).strip()
                if not table_markdown and table.get("text"):
                    table_markdown = str(table.get("text", "")).strip()
                if not table_markdown:
                    table_markdown = "_No table data available._"
                table_sections.append(f"{table_heading}\n\n{table_markdown}")
            sections.append("\n\n".join(table_sections))

        if show_metadata and self.metadata:
            metadata_payload = json.dumps(self.metadata, indent=2, ensure_ascii=False)
            sections.append(f"## Metadata\n\n```json\n{metadata_payload}\n```")

        return "\n\n".join(part.strip() for part in sections if part.strip())


PostProcessingHook = Callable[[ExtractionResult], ExtractionResult | Awaitable[ExtractionResult]]
ValidationHook = Callable[[ExtractionResult], None | Awaitable[None]]


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class JSONExtractionConfig(ConfigDict):
    extract_schema: bool = False
    """Extract and include JSON schema information in metadata."""
    custom_text_field_patterns: frozenset[str] | None = None
    """Custom patterns to identify text fields beyond default keywords."""
    max_depth: int = 10
    """Maximum nesting depth to process in JSON structures."""
    array_item_limit: int = 1000
    """Maximum number of array items to process to prevent memory issues."""
    include_type_info: bool = False
    """Include data type information in extracted content."""
    flatten_nested_objects: bool = True
    """Flatten nested objects using dot notation for better text extraction."""

    def __post_init__(self) -> None:
        if self.max_depth <= 0:
            raise ValidationError("max_depth must be positive", context={"max_depth": self.max_depth})
        if self.array_item_limit <= 0:
            raise ValidationError(
                "array_item_limit must be positive", context={"array_item_limit": self.array_item_limit}
            )


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class ExtractionConfig(ConfigDict):
    force_ocr: bool = False
    """Whether to force OCR."""
    chunk_content: bool = False
    """Whether to chunk the content into smaller chunks."""
    extract_tables: bool = False
    """Whether to extract tables from the content. This requires the 'gmft' dependency."""
    extract_tables_from_ocr: bool = False
    """Extract tables from OCR output using TSV format (Tesseract only)."""
    extract_images: bool = False
    """Whether to extract images from documents."""
    deduplicate_images: bool = True
    """Whether to remove duplicate images using CRC32 checksums."""
    image_ocr_config: ImageOCRConfig | None = None
    """Configuration for OCR processing of extracted images."""
    ocr_extracted_images: bool = False
    """Deprecated: Use image_ocr_config.enabled instead."""
    image_ocr_backend: OcrBackendType | None = None
    """Deprecated: Use image_ocr_config.backend instead."""
    image_ocr_min_dimensions: tuple[int, int] = (50, 50)
    """Deprecated: Use image_ocr_config.min_dimensions instead."""
    image_ocr_max_dimensions: tuple[int, int] = (10000, 10000)
    """Deprecated: Use image_ocr_config.max_dimensions instead."""
    image_ocr_formats: frozenset[str] = frozenset(
        {
            "jpg",
            "jpeg",
            "png",
            "gif",
            "bmp",
            "tiff",
            "tif",
            "webp",
            "jp2",
            "jpx",
            "jpm",
            "mj2",
            "pnm",
            "pbm",
            "pgm",
            "ppm",
        }
    )
    """Deprecated: Use image_ocr_config.allowed_formats instead."""
    max_chars: int = DEFAULT_MAX_CHARACTERS
    """The size of each chunk in characters."""
    max_overlap: int = DEFAULT_MAX_OVERLAP
    """The overlap between chunks in characters."""
    ocr_backend: OcrBackendType | None = "tesseract"
    """The OCR backend to use.

    Notes:
        - If set to 'None', OCR will not be performed.
    """
    ocr_config: TesseractConfig | PaddleOCRConfig | EasyOCRConfig | None = None
    """Configuration to pass to the OCR backend."""
    gmft_config: GMFTConfig | None = None
    """GMFT configuration."""
    post_processing_hooks: list[PostProcessingHook] | None = None
    """Post processing hooks to call after processing is done and before the final result is returned."""
    validators: list[ValidationHook] | None = None
    """Validation hooks to call after processing is done and before post-processing and result return."""
    extract_entities: bool = False
    """Whether to extract named entities from the content."""
    extract_keywords: bool = False
    """Whether to extract keywords from the content."""
    keyword_count: int = 10
    """Number of keywords to extract if extract_keywords is True."""
    custom_entity_patterns: frozenset[tuple[str, str]] | None = None
    """Custom entity patterns as a frozenset of (entity_type, regex_pattern) tuples."""
    auto_detect_language: bool = False
    """Whether to automatically detect language and configure OCR accordingly."""
    language_detection_model: Literal["lite", "full", "auto"] = "auto"
    """Language detection model to use when auto_detect_language is True.
    - 'lite': Smaller, faster model with good accuracy
    - 'full': Larger model with highest accuracy
    - 'auto': Automatically choose based on memory availability (default)
    """
    language_detection_config: LanguageDetectionConfig | None = None
    """Configuration for language detection. If None, uses default settings with language_detection_model."""
    spacy_entity_extraction_config: SpacyEntityExtractionConfig | None = None
    """Configuration for spaCy entity extraction. If None, uses default settings."""
    auto_detect_document_type: bool = False
    """Whether to automatically detect the document type."""
    document_type_confidence_threshold: float = 0.5
    """Confidence threshold for document type detection."""
    document_classification_mode: Literal["text", "vision"] = "text"
    """The mode to use for document classification."""
    enable_quality_processing: bool = True
    """Whether to apply quality post-processing to improve extraction results."""
    pdf_password: str | list[str] = ""
    """Password(s) for encrypted PDF files. Can be a single password or list of passwords to try in sequence. Only used when crypto extra is installed."""
    html_to_markdown_config: HTMLToMarkdownConfig | None = None
    """Configuration for HTML to Markdown conversion. If None, uses default settings."""
    json_config: JSONExtractionConfig | None = None
    """Configuration for enhanced JSON extraction features. If None, uses standard JSON processing."""
    use_cache: bool = True
    """Whether to use caching for extraction results. Set to False to disable all caching."""
    target_dpi: int = 150
    """Target DPI for OCR processing. Images and PDF pages will be scaled to this DPI for optimal OCR results."""
    max_image_dimension: int = 25000
    """Maximum allowed pixel dimension (width or height) for processed images to prevent memory issues."""
    auto_adjust_dpi: bool = True
    """Whether to automatically adjust DPI based on image dimensions to stay within max_image_dimension limits."""
    min_dpi: int = 72
    """Minimum DPI threshold when auto-adjusting DPI."""
    max_dpi: int = 600
    """Maximum DPI threshold when auto-adjusting DPI."""
    token_reduction: TokenReductionConfig | None = None
    """Configuration for token reduction to optimize output size while preserving meaning."""

    def __post_init__(self) -> None:
        if self.custom_entity_patterns is not None and isinstance(self.custom_entity_patterns, dict):
            object.__setattr__(self, "custom_entity_patterns", frozenset(self.custom_entity_patterns.items()))
        if self.post_processing_hooks is not None and isinstance(self.post_processing_hooks, list):
            object.__setattr__(self, "post_processing_hooks", tuple(self.post_processing_hooks))
        if self.validators is not None and isinstance(self.validators, list):
            object.__setattr__(self, "validators", tuple(self.validators))

        if isinstance(self.pdf_password, list):
            object.__setattr__(self, "pdf_password", tuple(self.pdf_password))

        if isinstance(self.image_ocr_formats, list):
            object.__setattr__(self, "image_ocr_formats", frozenset(self.image_ocr_formats))

        if self.image_ocr_config is None and (
            self.ocr_extracted_images
            or self.image_ocr_backend is not None
            or self.image_ocr_min_dimensions != (50, 50)
            or self.image_ocr_max_dimensions != (10000, 10000)
            or self.image_ocr_formats
            != frozenset(
                {
                    "jpg",
                    "jpeg",
                    "png",
                    "gif",
                    "bmp",
                    "tiff",
                    "tif",
                    "webp",
                    "jp2",
                    "jpx",
                    "jpm",
                    "mj2",
                    "pnm",
                    "pbm",
                    "pgm",
                    "ppm",
                }
            )
        ):
            object.__setattr__(
                self,
                "image_ocr_config",
                ImageOCRConfig(
                    enabled=self.ocr_extracted_images,
                    backend=self.image_ocr_backend,
                    min_dimensions=self.image_ocr_min_dimensions,
                    max_dimensions=self.image_ocr_max_dimensions,
                    allowed_formats=self.image_ocr_formats,
                ),
            )

        if self.ocr_backend is None and self.ocr_config is not None:
            raise ValidationError("'ocr_backend' is None but 'ocr_config' is provided")

        if self.ocr_config is not None and (
            (self.ocr_backend == "tesseract" and not isinstance(self.ocr_config, TesseractConfig))
            or (self.ocr_backend == "easyocr" and not isinstance(self.ocr_config, EasyOCRConfig))
            or (self.ocr_backend == "paddleocr" and not isinstance(self.ocr_config, PaddleOCRConfig))
        ):
            raise ValidationError(
                "incompatible 'ocr_config' value provided for 'ocr_backend'",
                context={"ocr_backend": self.ocr_backend, "ocr_config": type(self.ocr_config).__name__},
            )

        if self.target_dpi <= 0:
            raise ValidationError("target_dpi must be positive", context={"target_dpi": self.target_dpi})
        if self.min_dpi <= 0:
            raise ValidationError("min_dpi must be positive", context={"min_dpi": self.min_dpi})
        if self.max_dpi <= 0:
            raise ValidationError("max_dpi must be positive", context={"max_dpi": self.max_dpi})
        if self.min_dpi >= self.max_dpi:
            raise ValidationError(
                "min_dpi must be less than max_dpi", context={"min_dpi": self.min_dpi, "max_dpi": self.max_dpi}
            )
        if self.max_image_dimension <= 0:
            raise ValidationError(
                "max_image_dimension must be positive", context={"max_image_dimension": self.max_image_dimension}
            )
        if not (self.min_dpi <= self.target_dpi <= self.max_dpi):
            raise ValidationError(
                "target_dpi must be between min_dpi and max_dpi",
                context={"target_dpi": self.target_dpi, "min_dpi": self.min_dpi, "max_dpi": self.max_dpi},
            )

    def get_config_dict(self) -> dict[str, Any]:
        match self.ocr_backend:
            case None:
                return {"use_cache": self.use_cache}
            case _ if self.ocr_config is not None:
                config_dict = asdict(self.ocr_config)
                config_dict["use_cache"] = self.use_cache
                return config_dict
            case "tesseract":
                config_dict = asdict(TesseractConfig())
            case "easyocr":
                config_dict = asdict(EasyOCRConfig())
            case _:
                config_dict = asdict(PaddleOCRConfig())

        config_dict["use_cache"] = self.use_cache
        return config_dict

    def to_dict(self, include_none: bool = False) -> dict[str, Any]:
        result = msgspec.to_builtins(
            self,
            builtin_types=(type(None),),
            order="deterministic",
        )

        for field_name, value in result.items():
            if hasattr(value, "to_dict"):
                result[field_name] = value.to_dict(include_none=include_none)

        if include_none:
            return result  # type: ignore[no-any-return]

        return {k: v for k, v in result.items() if v is not None}


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class HTMLToMarkdownConfig:
    heading_style: Literal["underlined", "atx", "atx_closed"] = "atx"
    """Style for markdown headings."""
    list_indent_type: Literal["spaces", "tabs"] = "spaces"
    """Type of indentation to use for lists."""
    list_indent_width: int = 4
    """Number of spaces per indentation level (use 2 for Discord/Slack)."""
    bullets: str = "*+-"
    """Characters to use for unordered list bullets."""
    strong_em_symbol: Literal["*", "_"] = "*"
    """Symbol to use for strong/emphasis formatting."""
    escape_asterisks: bool = False
    """Escape * characters to prevent unintended formatting."""
    escape_underscores: bool = False
    """Escape _ characters to prevent unintended formatting."""
    escape_misc: bool = False
    """Escape miscellaneous characters to prevent Markdown conflicts."""
    escape_ascii: bool = False
    """Escape all ASCII punctuation."""
    code_language: str = ""
    """Default language identifier for fenced code blocks."""
    code_language_callback: Callable[[Any], str] | None = field(default=None, compare=False, hash=False)
    """Legacy language callback (no longer used by v2 converter)."""
    autolinks: bool = True
    """Automatically convert valid URLs to Markdown links."""
    default_title: bool = False
    """Use default titles for elements like links."""
    keep_inline_images_in: tuple[str, ...] | None = None
    """Tags where inline images should be preserved."""
    br_in_tables: bool = False
    """Use <br> tags for line breaks in table cells instead of spaces."""
    highlight_style: Literal["double-equal", "html", "bold", "none"] = "double-equal"
    """Style for highlighting text."""
    extract_metadata: bool = True
    """Extract document metadata as comment header."""
    whitespace_mode: Literal["normalized", "strict"] = "normalized"
    """Whitespace handling mode."""
    strip_newlines: bool = False
    """Remove newlines from HTML input before processing."""
    wrap: bool = False
    """Enable text wrapping."""
    wrap_width: int = 80
    """Width for text wrapping."""
    convert_as_inline: bool = False
    """Treat content as inline elements only."""
    sub_symbol: str = ""
    """Symbol to use for subscript text."""
    sup_symbol: str = ""
    """Symbol to use for superscript text."""
    newline_style: Literal["spaces", "backslash"] = "spaces"
    """Style for line breaks in markdown."""
    code_block_style: Literal["indented", "backticks", "tildes"] = "backticks"
    """Style for fenced code blocks."""
    strip_tags: tuple[str, ...] | None = None
    """List of HTML tags to remove from output."""
    convert: tuple[str, ...] | None = None
    """Legacy list of tags to convert (no longer used by v2 converter)."""
    custom_converters: Mapping[str, Callable[..., str]] | None = field(default=None, compare=False, hash=False)
    """Legacy mapping of custom converters (ignored by v2 converter)."""
    preprocess_html: bool = False
    """Enable HTML preprocessing to clean messy HTML."""
    preprocessing_preset: Literal["minimal", "standard", "aggressive"] = "standard"
    """Preprocessing level for cleaning HTML."""
    remove_navigation: bool = True
    """Remove navigation elements during preprocessing."""
    remove_forms: bool = True
    """Remove form elements during preprocessing."""
    encoding: str = "utf-8"
    """Expected character encoding for the HTML input."""
    debug: bool = False
    """Enable debug diagnostics in the converter."""

    def __post_init__(self) -> None:
        if self.keep_inline_images_in is not None and not isinstance(self.keep_inline_images_in, tuple):
            object.__setattr__(self, "keep_inline_images_in", tuple(self.keep_inline_images_in))
        if self.strip_tags is not None and not isinstance(self.strip_tags, tuple):
            object.__setattr__(self, "strip_tags", tuple(self.strip_tags))
        if self.convert is not None and not isinstance(self.convert, tuple):
            object.__setattr__(self, "convert", tuple(self.convert))

    def to_options(self) -> tuple[HTMLToMarkdownConversionOptions, HTMLToMarkdownPreprocessingOptions]:
        """Build html_to_markdown ConversionOptions and PreprocessingOptions instances."""
        preprocessing = HTMLToMarkdownPreprocessingOptions(
            enabled=self.preprocess_html,
            preset=self.preprocessing_preset,
            remove_navigation=self.remove_navigation,
            remove_forms=self.remove_forms,
        )

        keep_inline_images_in = list(self.keep_inline_images_in) if self.keep_inline_images_in else []
        strip_tags = list(self.strip_tags) if self.strip_tags else []

        options = HTMLToMarkdownConversionOptions(
            heading_style=self.heading_style,
            list_indent_type=self.list_indent_type,
            list_indent_width=self.list_indent_width,
            bullets=self.bullets,
            strong_em_symbol=self.strong_em_symbol,
            escape_asterisks=self.escape_asterisks,
            escape_underscores=self.escape_underscores,
            escape_misc=self.escape_misc,
            escape_ascii=self.escape_ascii,
            code_language=self.code_language,
            autolinks=self.autolinks,
            default_title=self.default_title,
            keep_inline_images_in=keep_inline_images_in,
            br_in_tables=self.br_in_tables,
            highlight_style=self.highlight_style,
            extract_metadata=self.extract_metadata,
            whitespace_mode=self.whitespace_mode,
            strip_newlines=self.strip_newlines,
            wrap=self.wrap,
            wrap_width=self.wrap_width,
            convert_as_inline=self.convert_as_inline,
            sub_symbol=self.sub_symbol,
            sup_symbol=self.sup_symbol,
            newline_style=self.newline_style,
            code_block_style=self.code_block_style,
            strip_tags=strip_tags,
            debug=self.debug,
            encoding=self.encoding,
        )

        options.preprocessing = preprocessing
        return options, preprocessing

    def to_dict(self, include_none: bool = False) -> dict[str, Any]:
        result = msgspec.to_builtins(self, builtin_types=(type(None),), order="deterministic")
        if result.get("keep_inline_images_in") is not None:
            result["keep_inline_images_in"] = list(result["keep_inline_images_in"])
        if result.get("strip_tags") is not None:
            result["strip_tags"] = list(result["strip_tags"])
        if result.get("convert") is not None:
            result["convert"] = list(result["convert"])

        if include_none:
            return result  # type: ignore[no-any-return]

        return {k: v for k, v in result.items() if v is not None}


@dataclass(unsafe_hash=True, frozen=True, slots=True)
class TokenReductionConfig:
    mode: Literal["off", "light", "moderate"] = "off"
    preserve_markdown: bool = True
    custom_stopwords: dict[str, list[str]] | None = field(default=None, compare=False, hash=False)
    language_hint: str | None = None

    def __post_init__(self) -> None:
        if self.language_hint:
            hint = self.language_hint.strip()

            if not hint or len(hint) > 50 or any(c in hint for c in "\x00\r\n\t"):
                object.__setattr__(self, "language_hint", None)
                return

            try:
                normalized = langcodes.standardize_tag(hint)

                lang = langcodes.Language.get(normalized).language

                if lang and lang != hint:
                    object.__setattr__(self, "language_hint", lang)
            except (ValueError, AttributeError, TypeError):
                object.__setattr__(self, "language_hint", None)
