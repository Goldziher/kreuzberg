from __future__ import annotations

from multiprocessing import cpu_count
from re import Pattern
from re import compile as compile_regex
from typing import TYPE_CHECKING, ClassVar, cast

import anyio
import pypdfium2
from anyio import Path as AsyncPath

from kreuzberg._extractors._base import Extractor
from kreuzberg._mime_types import PDF_MIME_TYPE, PLAIN_TEXT_MIME_TYPE
from kreuzberg._ocr import get_ocr_backend
from kreuzberg._playa import extract_pdf_metadata
from kreuzberg._types import ExtractionResult, OcrBackendType, ExtractionConfig
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync, run_taskgroup_batched
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ParsingError
from kreuzberg._language_detection import detect_languages

if TYPE_CHECKING:  # pragma: no cover
    from pathlib import Path

    from PIL.Image import Image


class PDFExtractor(Extractor):
    SUPPORTED_MIME_TYPES: ClassVar[set[str]] = {PDF_MIME_TYPE}
    CORRUPTED_PATTERN: ClassVar[Pattern[str]] = compile_regex(r"[\x00-\x08\x0B-\x0C\x0E-\x1F]|\uFFFD")
    SHORT_TEXT_THRESHOLD: ClassVar[int] = 50
    MINIMUM_CORRUPTED_RESULTS: ClassVar[int] = 2

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        file_path, unlink = await create_temp_file(".pdf")
        await AsyncPath(file_path).write_bytes(content)
        try:
            metadata = await extract_pdf_metadata(content)
            result = await self.extract_path_async(file_path)

            result.metadata = metadata
            return result
        finally:
            await unlink()

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        content_bytes = await AsyncPath(path).read_bytes()

        result: ExtractionResult | None = None
        detected_langs = None
        config = self.config
        if getattr(config, "auto_detect_language", False):
            # Try to extract searchable text and detect language
            content = await self._extract_pdf_searchable_text(path)
            if self._validate_extracted_text(content):
                detected_langs = detect_languages(content)
            else:
                # Fallback: convert first page to image, run quick OCR, detect language
                images = await self._convert_pdf_to_images(path)
                if images:
                    from kreuzberg._ocr import get_ocr_backend
                    ocr_result = await get_ocr_backend(config.ocr_backend).process_image(images[0], **config.get_config_dict())
                    detected_langs = detect_languages(ocr_result.content)
            if detected_langs:
                from dataclasses import replace
                # Always pass the full list for multi-language support
                config = replace(config, ocr_config=replace(config.ocr_config, language=detected_langs) if config.ocr_config else None)
        if not config.force_ocr:
            content = await self._extract_pdf_searchable_text(path)
            if self._validate_extracted_text(content):
                result = ExtractionResult(content=content, mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}, chunks=[])

        if not result and config.ocr_backend is not None:
            result = await self._extract_pdf_text_with_ocr(path, config.ocr_backend, config)

        if not result:
            result = ExtractionResult(content="", mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}, chunks=[])

        result.metadata = await extract_pdf_metadata(content_bytes)
        result.detected_languages = detected_langs if detected_langs else None

        if config.extract_tables:
            from kreuzberg._gmft import extract_tables

            result.tables = await extract_tables(path, config.gmft_config)

        return result

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        return anyio.run(self.extract_bytes_async, content)

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        return anyio.run(self.extract_path_async, path)

    def _validate_extracted_text(self, text: str, corruption_threshold: float = 0.05) -> bool:
        """Check if text extracted from PDF is valid or corrupted.

        This checks for indicators of corrupted PDF text extraction:
        1. Empty or whitespace-only text
        2. High concentration of control characters and null bytes
        3. High concentration of Unicode replacement characters

        Args:
            text: The extracted text to validate
            corruption_threshold: Maximum allowed percentage (0.0-1.0) of corrupted
                characters (default: 0.05 or 5%)

        Returns:
            True if the text appears valid, False if it seems corrupted
        """
        if not text or not text.strip():
            return False

        corruption_matches = self.CORRUPTED_PATTERN.findall(text)

        if len(text) < self.SHORT_TEXT_THRESHOLD:
            return len(corruption_matches) <= self.MINIMUM_CORRUPTED_RESULTS

        return (len(corruption_matches) / len(text)) < corruption_threshold

    async def _convert_pdf_to_images(self, input_file: Path) -> list[Image]:
        """Convert a PDF file to images.

        Args:
            input_file: The path to the PDF file.

        Raises:
            ParsingError: If the PDF file could not be converted to images.

        Returns:
            A list of Pillow Images.
        """
        document: pypdfium2.PdfDocument | None = None
        try:
            document = await run_sync(pypdfium2.PdfDocument, str(input_file))
            return [page.render(scale=4.25).to_pil() for page in cast("pypdfium2.PdfDocument", document)]
        except pypdfium2.PdfiumError as e:
            raise ParsingError(
                "Could not convert PDF to images", context={"file_path": str(input_file), "error": str(e)}
            ) from e
        finally:
            if document:
                await run_sync(document.close)

    async def _extract_pdf_text_with_ocr(self, input_file: Path, ocr_backend: OcrBackendType, config: ExtractionConfig) -> ExtractionResult:
        images = await self._convert_pdf_to_images(input_file)
        backend = get_ocr_backend(ocr_backend)
        ocr_results = await run_taskgroup_batched(
            *[backend.process_image(image, **config.get_config_dict()) for image in images],
            batch_size=cpu_count(),
        )
        return ExtractionResult(
            content="\n".join([v.content for v in ocr_results]), mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}, chunks=[]
        )

    @staticmethod
    async def _extract_pdf_searchable_text(input_file: Path) -> str:
        """Extract text from a searchable PDF file using pypdfium2.

        Args:
            input_file: The path to the PDF file.

        Raises:
            ParsingError: If the text could not be extracted from the PDF file.

        Returns:
            The extracted text.
        """
        document: pypdfium2.PdfDocument | None = None
        try:
            document = await run_sync(pypdfium2.PdfDocument, str(input_file))
            text = "\n".join(page.get_textpage().get_text_bounded() for page in cast("pypdfium2.PdfDocument", document))
            return normalize_spaces(text)
        except pypdfium2.PdfiumError as e:
            raise ParsingError(
                "Could not extract text from PDF file", context={"file_path": str(input_file), "error": str(e)}
            ) from e
        finally:
            if document:
                await run_sync(document.close)
