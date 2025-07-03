from __future__ import annotations

import sys
from collections.abc import Awaitable
from dataclasses import asdict, dataclass, field
from typing import TYPE_CHECKING, Any, Callable, Literal, TypedDict, Union

from kreuzberg._constants import DEFAULT_MAX_CHARACTERS, DEFAULT_MAX_OVERLAP
from kreuzberg.exceptions import ValidationError

if sys.version_info < (3, 11):  # pragma: no cover
    from typing_extensions import NotRequired
else:  # pragma: no cover
    from typing import NotRequired

if TYPE_CHECKING:
    from pandas import DataFrame
    from PIL.Image import Image

    from kreuzberg._gmft import GMFTConfig
    from kreuzberg._ocr._easyocr import EasyOCRConfig
    from kreuzberg._ocr._paddleocr import PaddleOCRConfig
    from kreuzberg._ocr._tesseract import TesseractConfig

OcrBackendType = Literal["tesseract", "easyocr", "paddleocr"]


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
    detected_languages: list[str] | None = None
    """Languages detected in the extracted content, if language detection is enabled."""


PostProcessingHook = Callable[[ExtractionResult], Union[ExtractionResult, Awaitable[ExtractionResult]]]
ValidationHook = Callable[[ExtractionResult], Union[None, Awaitable[None]]]


@dataclass(unsafe_hash=True)
class ExtractionConfig:
    """Represents configuration settings for an extraction process.

    This class encapsulates the configuration options for extracting text
    from images or documents using Optical Character Recognition (OCR). It
    provides options to customize the OCR behavior, select the backend
    engine, and configure engine-specific parameters.
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
    auto_detect_language: bool = False
    """Whether to automatically detect language and configure OCR accordingly."""

    def __post_init__(self) -> None:
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
