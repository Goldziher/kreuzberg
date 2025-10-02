from __future__ import annotations

import sys
from collections.abc import Awaitable, Callable, Mapping
from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING, Any, Literal, NamedTuple, TypedDict

import msgspec

from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg._utils._device import DeviceType  # noqa: TC001  # Needed at runtime for msgspec deserialization
from kreuzberg._utils._table import (
    export_table_to_csv,
    export_table_to_tsv,
    extract_table_structure_info,
)

if TYPE_CHECKING:
    from PIL.Image import Image
    from polars import DataFrame

if sys.version_info < (3, 11):  # pragma: no cover
    from typing_extensions import NotRequired
else:  # pragma: no cover
    from typing import NotRequired

OcrBackendType = Literal["tesseract", "easyocr", "paddleocr"]
OutputFormatType = Literal["text", "tsv", "hocr", "markdown"]
ErrorContextType = Literal["batch_processing", "optional_feature", "single_extraction", "unknown"]


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


class OCRBackendConfig(msgspec.Struct, tag_field="backend", kw_only=True, frozen=True):
    """Base class for OCR backend configurations.

    Uses msgspec tagged unions for automatic backend discrimination during serialization.
    """


class TesseractConfig(OCRBackendConfig, tag="tesseract", kw_only=True, frozen=True):
    """Tesseract OCR configuration.

    V4 defaults optimized for best quality:
    - Markdown output format with table extraction enabled
    - English language
    - Auto page segmentation
    """

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
    enable_table_detection: bool = True
    """Enable table structure detection from TSV output (enabled by default in V4)."""
    table_column_threshold: int = 20
    """Pixel threshold for column clustering in table detection."""
    table_row_threshold_ratio: float = 0.5
    """Row threshold as ratio of mean text height for table detection."""
    table_min_confidence: float = 30.0
    """Minimum confidence score to include a word in table extraction."""


class EasyOCRConfig(OCRBackendConfig, tag="easyocr", kw_only=True, frozen=True):
    """EasyOCR backend configuration."""

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
    language: str | tuple[str, ...] = "en"
    """Language or languages to use for OCR. Can be a single language code (e.g., 'en'),
    or a tuple of language codes (e.g., ('en', 'ch_sim')). Lists will be automatically converted to tuples."""
    link_threshold: float = 0.4
    """Link confidence threshold."""
    low_text: float = 0.4
    """Text low-bound score."""
    mag_ratio: float = 1.0
    """Image magnification ratio."""
    min_size: int = 10
    """Minimum text box size in pixels."""
    rotation_info: tuple[int, ...] | None = None
    """Tuple of angles to try for detection. Lists will be automatically converted to tuples."""
    slope_ths: float = 0.1
    """Maximum slope for merging text boxes."""
    text_threshold: float = 0.7
    """Text confidence threshold."""
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


class PaddleOCRConfig(OCRBackendConfig, tag="paddleocr", kw_only=True, frozen=True):
    """PaddleOCR backend configuration."""

    cls_image_shape: str = "3,48,192"
    """Image shape for classification algorithm in format 'channels,height,width'."""
    det_algorithm: Literal["DB", "EAST", "SAST", "PSE", "FCE", "PAN", "CT", "DB++", "Layout"] = "DB"
    """Detection algorithm."""
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
    device: DeviceType = "auto"
    """Device to use for inference. Options: 'cpu', 'cuda', 'auto'. Note: MPS not supported by PaddlePaddle."""
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


# Feature Extraction Configs


class ChunkingConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for text chunking.

    When enabled via ExtractionConfig.chunking, splits extracted text into smaller chunks
    for downstream processing (e.g., LLM context windows, vector databases).
    """

    max_chars: int = DEFAULT_MAX_CHARACTERS
    """Maximum size of each chunk in characters."""
    max_overlap: int = DEFAULT_MAX_OVERLAP
    """Overlap between consecutive chunks in characters."""


class TableExtractionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for table extraction using vision models.

    Extracts tables from documents using Microsoft's Table Transformer models.
    Requires the 'gmft' optional dependency.
    """

    extract_from_ocr: bool = False
    """Extract tables from Tesseract OCR TSV output (Tesseract backend only)."""
    detection_model: str = "microsoft/table-transformer-detection"
    """HuggingFace model path for table detection."""
    structure_model: str = "microsoft/table-transformer-structure-recognition-v1.1-all"
    """HuggingFace model path for table structure recognition."""
    model_cache_dir: str | None = None
    """Custom cache directory for model downloads. If None, uses HuggingFace default."""
    detection_threshold: float = 0.7
    """Confidence threshold for table detection (0.0-1.0)."""
    detection_device: str = "auto"
    """Device for detection model ('auto', 'cpu', 'cuda', 'cuda:0', etc)."""
    structure_threshold: float = 0.5
    """Confidence threshold for structure elements (rows/columns)."""
    structure_device: str = "auto"
    """Device for structure model ('auto', 'cpu', 'cuda', 'cuda:0', etc)."""
    crop_padding: int = 20
    """Pixels to add around detected tables when cropping."""
    min_table_area: int = 1000
    """Minimum table area in pixels² to process."""
    max_table_area: int | None = None
    """Maximum table area in pixels² to process. None = no limit."""
    cell_confidence_table: float = 0.3
    """Confidence threshold for table cells."""
    cell_confidence_column: float = 0.3
    """Confidence threshold for columns."""
    cell_confidence_row: float = 0.3
    """Confidence threshold for rows."""
    cell_confidence_column_header: float = 0.3
    """Confidence threshold for column headers."""
    cell_confidence_projected_row_header: float = 0.5
    """Confidence threshold for projected row headers."""
    cell_confidence_spanning_cell: float = 0.5
    """Confidence threshold for spanning cells."""
    total_overlap_reject_threshold: float = 0.9
    """Reject table if total overlap > this fraction of table area."""
    total_overlap_warn_threshold: float = 0.1
    """Warn if total overlap > this fraction of table area."""
    iob_reject_threshold: float = 0.05
    """Reject if intersection-over-box between text and cell < this value."""
    iob_warn_threshold: float = 0.5
    """Warn if intersection-over-box between text and cell < this value."""
    large_table_threshold: int = 10
    """Row count threshold to trigger large table handling."""
    large_table_row_overlap_threshold: float = 0.2
    """Overlap threshold to trigger large table handling."""
    large_table_maximum_rows: int = 1000
    """Maximum rows allowed in a large table."""
    force_large_table_assumption: bool | None = None
    """Force large table handling regardless of thresholds."""
    remove_null_rows: bool = True
    """Remove rows with no text content."""
    enable_multi_header: bool = False
    """Enable multi-level column headers in output."""
    semantic_spanning_cells: bool = False
    """Enable semantic interpretation of spanning cells."""
    enable_model_caching: bool = True
    """Cache loaded models for reuse."""
    batch_size: int = 1
    """Batch size for processing multiple tables."""
    mixed_precision: bool = False
    """Use mixed precision (FP16) when available for faster inference."""
    verbosity: int = 1
    """Verbosity level (0=errors, 1=warnings, 2=info, 3=debug)."""

    @property
    def cell_required_confidence(self) -> dict[int, float]:
        """Get cell confidence thresholds as dict for internal use."""
        return {
            0: self.cell_confidence_table,
            1: self.cell_confidence_column,
            2: self.cell_confidence_row,
            3: self.cell_confidence_column_header,
            4: self.cell_confidence_projected_row_header,
            5: self.cell_confidence_spanning_cell,
            6: 99.0,
        }


class ImageExtractionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for image extraction from documents.

    Controls how images embedded in documents are extracted and optionally processed with OCR.
    """

    deduplicate: bool = True
    """Remove duplicate images using CRC32 checksums."""
    ocr_min_dimensions: tuple[int, int] | None = None
    """Minimum (width, height) in pixels for OCR on extracted images. None = disabled."""
    ocr_max_dimensions: tuple[int, int] = (10000, 10000)
    """Maximum (width, height) in pixels for OCR on extracted images."""
    ocr_allowed_formats: frozenset[str] = frozenset(
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
    ocr_batch_size: int = 4
    """Number of images to process in parallel for OCR."""
    ocr_timeout_seconds: int = 30
    """Maximum time in seconds for OCR processing per image."""


class LanguageDetectionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for automatic language detection."""

    model: Literal["lite", "full", "auto"] = "auto"
    """Language detection model: 'lite' (fast), 'full' (accurate), 'auto' (choose based on memory)."""
    top_k: int = 3
    """Maximum number of languages to return for multilingual detection."""
    multilingual: bool = False
    """Enable multilingual detection to handle mixed-language text."""
    cache_dir: str | None = None
    """Custom directory for model cache. If None, uses system default."""


class KeywordExtractionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for keyword extraction."""

    count: int = 10
    """Number of keywords to extract."""


class EntityExtractionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for named entity extraction using spaCy."""

    model_cache_dir: str | None = None
    """Directory to cache spaCy models. If None, uses spaCy's default. Can be a string path."""
    language_models: tuple[tuple[str, str], ...] | None = None
    """Mapping of language codes to spaCy model names. If None, uses default mappings."""
    fallback_to_multilingual: bool = True
    """If True and language-specific model fails, try xx_ent_wiki_sm (multilingual)."""
    max_doc_length: int = 1000000
    """Maximum document length for spaCy processing."""
    batch_size: int = 1000
    """Batch size for processing multiple texts."""

    @staticmethod
    def get_default_language_models() -> dict[str, str]:
        """Get default spaCy model mappings for supported languages."""
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
        """Get the spaCy model name for a given language code."""
        models_dict = self.get_default_language_models() if not self.language_models else dict(self.language_models)

        if language_code in models_dict:
            return models_dict[language_code]

        base_lang = language_code.split("-")[0].lower()
        if base_lang in models_dict:
            return models_dict[base_lang]

        return None

    def get_fallback_model(self) -> str | None:
        """Get the fallback multilingual model if enabled."""
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
    character_count: NotRequired[int]
    """Number of characters in text content."""
    citations: NotRequired[list[str]]
    """Citation identifiers."""
    code_blocks: NotRequired[list[dict[str, str]]]
    """Code blocks extracted from markdown (language and code)."""
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
    headers: NotRequired[list[str]]
    """Headers extracted from markdown content."""
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
    line_count: NotRequired[int]
    """Number of lines in text content."""
    links: NotRequired[list[dict[str, str]]]
    """Links extracted from markdown (text and url)."""
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
    sheet_count: NotRequired[str]
    """Number of sheets in spreadsheet."""
    sheet_names: NotRequired[str]
    """Names of sheets in spreadsheet."""
    title: NotRequired[str]
    """Document title."""
    total_cells: NotRequired[str]
    """Total number of cells in spreadsheet."""
    version: NotRequired[str]
    """Version identifier or revision number."""
    width: NotRequired[int]
    """Width of the document page/slide/image, if applicable."""
    word_count: NotRequired[int]
    """Number of words in text content."""
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
    converted_via: NotRequired[str]
    """Tool used to convert the document (e.g., 'libreoffice', 'pandoc')."""
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
    element_count: NotRequired[int]
    """Total number of XML elements encountered."""
    unique_elements: NotRequired[int]
    """Number of unique XML element names."""
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
    "character_count",
    "citations",
    "code_blocks",
    "comments",
    "content",
    "copyright",
    "created_at",
    "created_by",
    "description",
    "fonts",
    "headers",
    "height",
    "identifier",
    "keywords",
    "languages",
    "license",
    "line_count",
    "links",
    "modified_at",
    "modified_by",
    "organization",
    "parse_error",
    "publisher",
    "references",
    "sheet_count",
    "sheet_names",
    "status",
    "subject",
    "subtitle",
    "summary",
    "title",
    "total_cells",
    "version",
    "warning",
    "width",
    "word_count",
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
    "converted_via",
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
    "element_count",
    "unique_elements",
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


PostProcessingHook = Callable[[ExtractionResult], ExtractionResult | Awaitable[ExtractionResult]]
ValidationHook = Callable[[ExtractionResult], None | Awaitable[None]]


class JSONExtractionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for enhanced JSON extraction."""

    extract_schema: bool = False
    """Extract and include JSON schema information in metadata."""
    custom_text_field_patterns: frozenset[str] | None = None
    """Custom patterns to identify text fields beyond default keywords."""
    max_depth: int = 10
    """Maximum nesting depth to process in JSON structures (must be positive)."""
    array_item_limit: int = 1000
    """Maximum number of array items to process to prevent memory issues (must be positive)."""
    include_type_info: bool = False
    """Include data type information in extracted content."""
    flatten_nested_objects: bool = True
    """Flatten nested objects using dot notation for better text extraction."""


class ExtractionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """V4 extraction configuration with flat structure and tagged unions.

    Breaking changes from V3:
    - OCR: Use tagged union (ocr=TesseractConfig()) instead of ocr_backend + ocr_config
    - Features: Use None = disabled pattern (tables=None) instead of extract_tables=False
    - Simplified: No more redundant flags, cleaner structure
    """

    # Model cache
    model_cache_dir: str | None = None
    """Global cache directory for all ML models (OCR, vision-tables, spaCy, etc.).
    Overrides individual model cache settings. Can also be set via KREUZBERG_MODEL_CACHE or HF_HOME."""

    # OCR configuration (tagged union)
    ocr: TesseractConfig | EasyOCRConfig | PaddleOCRConfig | None = TesseractConfig()
    """OCR backend configuration. None = OCR disabled. Default: Tesseract with markdown + table detection."""
    force_ocr: bool = False
    """Force OCR even for searchable PDFs and text-based documents."""

    # Feature configurations (None = feature disabled)
    chunking: ChunkingConfig | None = None
    """Text chunking configuration. None = chunking disabled."""
    tables: TableExtractionConfig | None = None
    """Table extraction configuration. None = table extraction disabled."""
    images: ImageExtractionConfig | None = None
    """Image extraction configuration. None = image extraction disabled."""
    language_detection: LanguageDetectionConfig | None = None
    """Language detection configuration. None = language detection disabled."""
    entities: EntityExtractionConfig | None = None
    """Named entity extraction configuration. None = entity extraction disabled."""
    keywords: KeywordExtractionConfig | None = None
    """Keyword extraction configuration. None = keyword extraction disabled."""

    # Format-specific configurations
    html_to_markdown: HTMLToMarkdownConfig | None = None
    """HTML to Markdown conversion configuration. None = use default settings."""
    json_extraction: JSONExtractionConfig | None = None
    """JSON extraction configuration. None = use standard JSON processing."""
    token_reduction: TokenReductionConfig | None = None
    """Token reduction configuration. None = token reduction disabled."""

    # Global settings
    pdf_password: str | tuple[str, ...] = ""
    """Password(s) for encrypted PDFs. Single string or tuple of passwords to try."""
    custom_entity_patterns: frozenset[tuple[str, str]] | None = None
    """Custom entity patterns as frozenset of (entity_type, regex_pattern) tuples."""
    post_processing_hooks: tuple[PostProcessingHook, ...] | None = None
    """Post-processing hooks called after extraction, before final result."""
    validators: tuple[ValidationHook, ...] | None = None
    """Validation hooks called after extraction, before post-processing."""
    use_cache: bool = True
    """Enable caching for extraction results. False = disable all caching."""
    enable_quality_processing: bool = True
    """Apply quality post-processing to improve extraction results."""

    # DPI and image settings
    target_dpi: int = 150
    """Target DPI for OCR processing. Images/PDFs scaled to this DPI."""
    max_image_dimension: int = 25000
    """Maximum pixel dimension (width or height) to prevent memory issues."""
    auto_adjust_dpi: bool = True
    """Auto-adjust DPI based on dimensions to stay within max_image_dimension."""
    min_dpi: int = 72
    """Minimum DPI threshold when auto-adjusting."""
    max_dpi: int = 600
    """Maximum DPI threshold when auto-adjusting."""

    # Document classification (deprecated, may be removed in future)
    auto_detect_document_type: bool = False
    """Auto-detect document type (deprecated)."""
    document_type_confidence_threshold: float = 0.5
    """Confidence threshold for document type detection (deprecated)."""
    document_classification_mode: Literal["text", "vision"] = "text"
    """Document classification mode (deprecated)."""


class HTMLToMarkdownConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for HTML to Markdown conversion."""

    autolinks: bool = True
    """Automatically convert valid URLs to Markdown links."""
    br_in_tables: bool = False
    """Use <br> tags for line breaks in table cells instead of spaces."""
    bullets: str = "*+-"
    """Characters to use for unordered list bullets."""
    code_language: str = ""
    """Default language identifier for fenced code blocks."""
    code_language_callback: Callable[[Any], str] | None = None
    """Function to dynamically determine code block language."""
    convert: tuple[str, ...] | None = None
    """Tuple of HTML tags to convert (None = all supported tags)."""
    convert_as_inline: bool = False
    """Treat content as inline elements only."""
    custom_converters: Mapping[str, Callable[..., str]] | None = None
    """Mapping of HTML tag names to custom converter functions."""
    default_title: bool = False
    """Use default titles for elements like links."""
    escape_asterisks: bool = False
    """Escape * characters to prevent unintended formatting."""
    escape_misc: bool = False
    """Escape miscellaneous characters to prevent Markdown conflicts."""
    escape_underscores: bool = False
    """Escape _ characters to prevent unintended formatting."""
    extract_metadata: bool = True
    """Extract document metadata as comment header."""
    heading_style: Literal["underlined", "atx", "atx_closed"] = "underlined"
    """Style for markdown headings."""
    highlight_style: Literal["double-equal", "html", "bold"] = "double-equal"
    """Style for highlighting text."""
    keep_inline_images_in: tuple[str, ...] | None = None
    """Tags where inline images should be preserved."""
    list_indent_type: Literal["spaces", "tabs"] = "spaces"
    """Type of indentation to use for lists."""
    list_indent_width: int = 4
    """Number of spaces per indentation level (use 2 for Discord/Slack)."""
    newline_style: Literal["spaces", "backslash"] = "spaces"
    """Style for line breaks in markdown."""
    preprocess_html: bool = False
    """Enable HTML preprocessing to clean messy HTML."""
    preprocessing_preset: Literal["minimal", "standard", "aggressive"] = "standard"
    """Preprocessing level for cleaning HTML."""
    remove_forms: bool = True
    """Remove form elements during preprocessing."""
    remove_navigation: bool = True
    """Remove navigation elements during preprocessing."""
    strip: tuple[str, ...] | None = None
    """Tuple of HTML tags to remove from output."""
    strip_newlines: bool = False
    """Remove newlines from HTML input before processing."""
    strong_em_symbol: Literal["*", "_"] = "*"
    """Symbol to use for strong/emphasis formatting."""
    sub_symbol: str = ""
    """Symbol to use for subscript text."""
    sup_symbol: str = ""
    """Symbol to use for superscript text."""
    whitespace_mode: Literal["normalized", "strict"] = "normalized"
    """Whitespace handling mode."""
    wrap: bool = False
    """Enable text wrapping."""
    wrap_width: int = 80
    """Width for text wrapping."""


class TokenReductionConfig(msgspec.Struct, kw_only=True, frozen=True):
    """Configuration for token reduction to optimize output size while preserving meaning."""

    mode: Literal["off", "light", "moderate", "aggressive"] = "off"
    """Token reduction mode: off (disabled), light, moderate, or aggressive."""
    preserve_markdown: bool = True
    """Preserve markdown formatting during token reduction."""
    custom_stopwords: dict[str, tuple[str, ...]] | None = None
    """Custom stopwords per language for token reduction."""
    language_hint: str | None = None
    """Language hint for token reduction. Will be normalized to language code."""
