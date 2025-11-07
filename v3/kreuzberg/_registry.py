from __future__ import annotations

from functools import lru_cache
from typing import TYPE_CHECKING, ClassVar

from kreuzberg._extractors._email import EmailExtractor
from kreuzberg._extractors._html import HTMLExtractor
from kreuzberg._extractors._image import ImageExtractor
from kreuzberg._extractors._pandoc import (
    BibliographyExtractor,
    EbookExtractor,
    LaTeXExtractor,
    MarkdownExtractor,
    MiscFormatExtractor,
    OfficeDocumentExtractor,
    StructuredTextExtractor,
    TabularDataExtractor,
    XMLBasedExtractor,
)
from kreuzberg._extractors._pdf import PDFExtractor
from kreuzberg._extractors._presentation import PresentationExtractor
from kreuzberg._extractors._spread_sheet import SpreadSheetExtractor
from kreuzberg._extractors._structured import StructuredDataExtractor

if TYPE_CHECKING:
    from kreuzberg._extractors._base import Extractor
    from kreuzberg._types import ExtractionConfig


class ExtractorRegistry:
    """Registry for managing document extractors.

    This class maintains a registry of extractors for different file types and provides
    functionality to get the appropriate extractor for a given MIME type, as well as
    add or remove custom extractors.
    """

    _default_extractors: ClassVar[list[type[Extractor]]] = [
        PDFExtractor,
        OfficeDocumentExtractor,
        PresentationExtractor,
        SpreadSheetExtractor,
        HTMLExtractor,
        EmailExtractor,
        StructuredDataExtractor,
        MarkdownExtractor,
        ImageExtractor,
        BibliographyExtractor,
        EbookExtractor,
        LaTeXExtractor,
        MiscFormatExtractor,
        StructuredTextExtractor,
        TabularDataExtractor,
        XMLBasedExtractor,
    ]
    _registered_extractors: ClassVar[list[type[Extractor]]] = []

    @classmethod
    @lru_cache
    def get_extractor(cls, mime_type: str | None, config: ExtractionConfig) -> Extractor | None:
        """Get an appropriate extractor for the given MIME type.

        Args:
            mime_type: The MIME type to find an extractor for.
            config: The extraction configuration.

        Returns:
            An extractor instance if one supports the MIME type, None otherwise.

        """
        extractors: list[type[Extractor]] = [
            *cls._registered_extractors,
            *cls._default_extractors,
        ]
        if mime_type:
            for extractor in extractors:
                if extractor.supports_mimetype(mime_type):
                    return extractor(mime_type=mime_type, config=config)

        return None

    @classmethod
    def add_extractor(cls, extractor: type[Extractor]) -> None:
        """Add a custom extractor to the registry.

        Args:
            extractor: The extractor class to add to the registry.

        """
        cls._registered_extractors.append(extractor)
        cls.get_extractor.cache_clear()

    @classmethod
    def remove_extractor(cls, extractor: type[Extractor]) -> None:
        """Remove a custom extractor from the registry.

        Args:
            extractor: The extractor class to remove from the registry.

        """
        try:
            cls._registered_extractors.remove(extractor)
            cls.get_extractor.cache_clear()
        except ValueError:
            pass
