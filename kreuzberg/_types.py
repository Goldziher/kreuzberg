from __future__ import annotations

import sys
from collections.abc import Awaitable, Callable
from dataclasses import asdict, dataclass, field
from typing import TYPE_CHECKING, Any, Literal, TypedDict

from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg.exceptions import ValidationError

if sys.version_info < (3, 11):  # pragma: no cover
    from typing_extensions import NotRequired
else:  # pragma: no cover
    from typing import NotRequired

if TYPE_CHECKING:
    from pandas import DataFrame
    from PIL.Image import Image

    from kreuzberg._entity_extraction import SpacyEntityExtractionConfig
    from kreuzberg._gmft import GMFTConfig
    from kreuzberg._language_detection import LanguageDetectionConfig
    from kreuzberg._ocr._easyocr import EasyOCRConfig
    from kreuzberg._ocr._paddleocr import PaddleOCRConfig
    from kreuzberg._ocr._tesseract import TesseractConfig

OcrBackendType = Literal["tesseract", "easyocr", "paddleocr"]
ExtractionBackendType = Literal["auto", "kreuzberg", "extractous", "hybrid"]


class TableData(TypedDict):
    """Table data, returned from table extraction."""

    cropped_image: Image
    """The cropped image of the table."""
    df: DataFrame
    """The table data as a pandas DataFrame."""
    page_number: int
    """The page number of the table."""
    text: str
    """The table text as a markdown string."""


class Metadata(TypedDict, total=False):
    """Base metadata common to all document types.

    All fields will only be included if they contain non-empty values.
    Any field that would be empty or None is omitted from the dictionary.
    """

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

    # Email-specific fields
    email_from: NotRequired[str]
    """Email sender (from field)."""
    email_to: NotRequired[list[str]]
    """Email recipients (to field)."""
    email_cc: NotRequired[list[str]]
    """Email carbon copy recipients."""
    email_bcc: NotRequired[list[str]]
    """Email blind carbon copy recipients."""

    # Backend tracking fields
    hybrid_backend: NotRequired[bool]
    """Whether this extraction used the hybrid backend."""
    primary_backend: NotRequired[str]
    """Name of the primary backend used for extraction."""
    fallback_backend: NotRequired[str]
    """Name of the fallback backend used if primary failed."""
    extraction_strategy: NotRequired[str]
    """Extraction strategy used by hybrid backend."""
    file_type: NotRequired[str]
    """Detected file type for backend routing."""
    mime_type: NotRequired[str]
    """MIME type used for backend selection."""
    primary_error: NotRequired[str]
    """Error message from primary backend if fallback was used."""
    backend: NotRequired[str]
    """Name of the backend used for extraction."""
    extraction_time: NotRequired[float]
    """Time taken for extraction in seconds."""
    word_count: NotRequired[int]
    """Number of words in extracted content."""
    char_count: NotRequired[int]
    """Number of characters in extracted content."""
    provided_mime_type: NotRequired[str]
    """MIME type provided as input to extraction."""
    sheet_count: NotRequired[int]
    """Number of sheets in spreadsheet documents."""


def normalize_metadata(data: dict[str, Any] | None) -> Metadata:
    """Normalize any dict to proper Metadata TypedDict.

    Filters out invalid keys and ensures type safety.
    """
    if not data:
        return {}

    # Get all valid Metadata keys from TypedDict annotations
    from typing import get_type_hints

    valid_keys = set(get_type_hints(Metadata).keys())

    # Filter and return only valid metadata
    normalized: Metadata = {}
    for key, value in data.items():
        if key in valid_keys and value is not None:
            normalized[key] = value  # type: ignore[literal-required]

    return normalized


@dataclass(frozen=True)
class Entity:
    """Represents an extracted entity with type, text, and position."""

    type: str
    """e.g., PERSON, ORGANIZATION, LOCATION, DATE, EMAIL, PHONE, or custom"""
    text: str
    """Extracted text"""
    start: int
    """Start character offset in the content"""
    end: int
    """End character offset in the content"""


@dataclass
class ExtractionResult:
    """The result of a file extraction."""

    content: str
    """The extracted content."""
    mime_type: str
    """The mime type of the extracted content. Is either text/plain or text/markdown."""
    metadata: Metadata
    """The metadata of the content."""
    tables: list[TableData] = field(default_factory=list)
    """Extracted tables. Is an empty list if 'extract_tables' is not set to True in the ExtractionConfig."""
    chunks: list[str] = field(default_factory=list)
    """The extracted content chunks. This is an empty list if 'chunk_content' is not set to True in the ExtractionConfig."""
    entities: list[Entity] | None = None
    """Extracted entities, if entity extraction is enabled."""
    keywords: list[tuple[str, float]] | None = None
    """Extracted keywords and their scores, if keyword extraction is enabled."""
    detected_languages: list[str] | None = None
    """Languages detected in the extracted content, if language detection is enabled."""

    def to_dict(self) -> dict[str, Any]:
        """Converts the ExtractionResult to a dictionary."""
        return asdict(self)


PostProcessingHook = Callable[[ExtractionResult], ExtractionResult | Awaitable[ExtractionResult]]
ValidationHook = Callable[[ExtractionResult], None | Awaitable[None]]


@dataclass(unsafe_hash=True)
class ExtractionConfig:
    """Represents configuration settings for an extraction process.

    This class encapsulates the configuration options for extracting text
    from images or documents using Optical Character Recognition (OCR). It
    provides options to customize the OCR behavior, select the backend
    engine, and configure engine-specific parameters.
    """

    extraction_backend: ExtractionBackendType = "auto"
    """The extraction backend to use.

    Options:
        - 'auto': Automatically select the best backend based on file type and availability
        - 'kreuzberg': Use only the vanilla Kreuzberg extractors
        - 'extractous': Use only Extractous (if available)
        - 'hybrid': Use intelligent hybrid routing between backends
    """
    force_ocr: bool = False
    """Whether to force OCR."""
    chunk_content: bool = False
    """Whether to chunk the content into smaller chunks."""
    extract_tables: bool = False
    """Whether to extract tables from the content. This requires the 'gmft' dependency."""
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
    language_detection_config: LanguageDetectionConfig | None = None
    """Configuration for language detection. If None, uses default settings."""
    spacy_entity_extraction_config: SpacyEntityExtractionConfig | None = None
    """Configuration for spaCy entity extraction. If None, uses default settings."""

    def __post_init__(self) -> None:
        if self.custom_entity_patterns is not None and isinstance(self.custom_entity_patterns, dict):
            object.__setattr__(self, "custom_entity_patterns", frozenset(self.custom_entity_patterns.items()))
        if self.post_processing_hooks is not None and isinstance(self.post_processing_hooks, list):
            object.__setattr__(self, "post_processing_hooks", tuple(self.post_processing_hooks))
        if self.validators is not None and isinstance(self.validators, list):
            object.__setattr__(self, "validators", tuple(self.validators))
        from kreuzberg._ocr._easyocr import EasyOCRConfig
        from kreuzberg._ocr._paddleocr import PaddleOCRConfig
        from kreuzberg._ocr._tesseract import TesseractConfig

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

    def get_config_dict(self) -> dict[str, Any]:
        """Returns the OCR configuration object based on the backend specified.

        Returns:
            A dict of the OCR configuration or an empty dict if no backend is provided.
        """
        if self.ocr_backend is not None:
            if self.ocr_config is not None:
                return asdict(self.ocr_config)
            if self.ocr_backend == "tesseract":
                from kreuzberg._ocr._tesseract import TesseractConfig

                return asdict(TesseractConfig())
            if self.ocr_backend == "easyocr":
                from kreuzberg._ocr._easyocr import EasyOCRConfig

                return asdict(EasyOCRConfig())
            from kreuzberg._ocr._paddleocr import PaddleOCRConfig

            return asdict(PaddleOCRConfig())
        return {}
