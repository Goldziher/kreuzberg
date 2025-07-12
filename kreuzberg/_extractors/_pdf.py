from __future__ import annotations

import contextlib
from multiprocessing import cpu_count
from pathlib import Path
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
from kreuzberg._types import ExtractionResult, OcrBackendType
from kreuzberg._utils._pdf_lock import pypdfium_file_lock
from kreuzberg._utils._string import normalize_spaces
from kreuzberg._utils._sync import run_sync, run_taskgroup_batched
from kreuzberg._utils._tmp import create_temp_file
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:  # pragma: no cover
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

        if not self.config.force_ocr:
            content = await self._extract_pdf_searchable_text(path)
            if self._validate_extracted_text(content):
                result = ExtractionResult(content=content, mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}, chunks=[])

        if not result and self.config.ocr_backend is not None:
            result = await self._extract_pdf_text_with_ocr(path, self.config.ocr_backend)

        if not result:
            result = ExtractionResult(content="", mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}, chunks=[])

        result.metadata = await extract_pdf_metadata(content_bytes)

        if self.config.extract_tables:
            from kreuzberg._gmft import extract_tables

            result.tables = await extract_tables(path, self.config.gmft_config)

        return result

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Pure sync implementation of PDF extraction from bytes."""
        import os
        import tempfile

        fd, temp_path = tempfile.mkstemp(suffix=".pdf")
        try:
            with os.fdopen(fd, "wb") as f:
                f.write(content)

            result = self.extract_path_sync(Path(temp_path))

            from kreuzberg._playa import extract_pdf_metadata_sync

            metadata = extract_pdf_metadata_sync(content)
            result.metadata = metadata

            return result
        finally:
            with contextlib.suppress(OSError):
                Path(temp_path).unlink()

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Pure sync implementation of PDF extraction from path."""
        # Read PDF bytes for metadata extraction
        with path.open("rb") as f:
            content_bytes = f.read()

        text = self._extract_pdf_searchable_text_sync(path)

        if self.config.force_ocr or not self._validate_extracted_text(text):
            text = self._extract_pdf_with_ocr_sync(path)

        tables = []
        if self.config.extract_tables:
            try:
                from kreuzberg._gmft import extract_tables_sync

                tables = extract_tables_sync(path)
            except ImportError:
                pass

        text = normalize_spaces(text)

        # Extract metadata
        from kreuzberg._playa import extract_pdf_metadata_sync

        metadata = extract_pdf_metadata_sync(content_bytes)

        return ExtractionResult(
            content=text,
            mime_type=PLAIN_TEXT_MIME_TYPE,
            metadata=metadata,
            tables=tables,
            chunks=[],
        )

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
        from kreuzberg._utils._errors import create_error_context, should_retry

        document: pypdfium2.PdfDocument | None = None
        last_error = None

        for attempt in range(3):  # Try up to 3 times  # ~keep
            try:
                with pypdfium_file_lock(input_file):
                    document = await run_sync(pypdfium2.PdfDocument, str(input_file))
                    return [page.render(scale=4.25).to_pil() for page in cast("pypdfium2.PdfDocument", document)]
            except pypdfium2.PdfiumError as e:  # noqa: PERF203
                last_error = e
                if not should_retry(e, attempt + 1):
                    raise ParsingError(
                        "Could not convert PDF to images",
                        context=create_error_context(
                            operation="convert_pdf_to_images",
                            file_path=input_file,
                            error=e,
                            attempt=attempt + 1,
                        ),
                    ) from e
                # Wait before retry with exponential backoff  # ~keep
                await anyio.sleep(0.5 * (attempt + 1))
            finally:
                if document:
                    with pypdfium_file_lock(input_file), contextlib.suppress(Exception):
                        await run_sync(document.close)

        # All retries failed  # ~keep
        raise ParsingError(
            "Could not convert PDF to images after retries",
            context=create_error_context(
                operation="convert_pdf_to_images",
                file_path=input_file,
                error=last_error,
                attempts=3,
            ),
        ) from last_error

    async def _extract_pdf_text_with_ocr(self, input_file: Path, ocr_backend: OcrBackendType) -> ExtractionResult:
        """Extract text from a scanned PDF file using OCR.

        Args:
            input_file: The path to the PDF file.
            ocr_backend: The OCR backend to use.

        Returns:
            The extraction result with text content and metadata.
        """
        images = await self._convert_pdf_to_images(input_file)
        backend = get_ocr_backend(ocr_backend)
        ocr_results = await run_taskgroup_batched(
            *[backend.process_image(image, **self.config.get_config_dict()) for image in images],
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
        from kreuzberg._utils._errors import create_error_context

        document: pypdfium2.PdfDocument | None = None
        try:
            with pypdfium_file_lock(input_file):
                document = await run_sync(pypdfium2.PdfDocument, str(input_file))
                text_parts = []
                page_errors = []

                for i, page in enumerate(cast("pypdfium2.PdfDocument", document)):
                    try:
                        text_page = page.get_textpage()
                        text_parts.append(text_page.get_text_bounded())
                    except Exception as e:  # noqa: PERF203, BLE001
                        page_errors.append({"page": i + 1, "error": str(e)})
                        text_parts.append(f"[Error extracting page {i + 1}]")

                text = "\n".join(text_parts)

                if page_errors and text_parts:
                    return normalize_spaces(text)
                if not text_parts:
                    raise ParsingError(
                        "Could not extract any text from PDF",
                        context=create_error_context(
                            operation="extract_pdf_searchable_text",
                            file_path=input_file,
                            page_errors=page_errors,
                        ),
                    )

                return normalize_spaces(text)
        except pypdfium2.PdfiumError as e:
            raise ParsingError(
                "Could not extract text from PDF file",
                context=create_error_context(
                    operation="extract_pdf_searchable_text",
                    file_path=input_file,
                    error=e,
                ),
            ) from e
        finally:
            if document:
                with pypdfium_file_lock(input_file), contextlib.suppress(Exception):
                    await run_sync(document.close)

    def _extract_pdf_searchable_text_sync(self, path: Path) -> str:
        """Extract searchable text from PDF using pypdfium2 (sync version)."""
        pdf = None
        try:
            with pypdfium_file_lock(path):
                pdf = pypdfium2.PdfDocument(str(path))
                text_parts = []
                for page in pdf:
                    text_page = page.get_textpage()
                    text = text_page.get_text_bounded()
                    text_parts.append(text)
                    text_page.close()
                    page.close()
                return "".join(text_parts)
        except Exception as e:
            raise ParsingError(f"Failed to extract PDF text: {e}") from e
        finally:
            if pdf:
                with pypdfium_file_lock(path), contextlib.suppress(Exception):
                    pdf.close()

    def _extract_pdf_with_ocr_sync(self, path: Path) -> str:
        """Extract text from PDF using OCR (sync version)."""
        pdf = None
        try:
            images = []
            with pypdfium_file_lock(path):
                pdf = pypdfium2.PdfDocument(str(path))
                for page in pdf:
                    bitmap = page.render(scale=200 / 72)
                    pil_image = bitmap.to_pil()
                    images.append(pil_image)
                    bitmap.close()
                    page.close()

            import os
            import tempfile

            image_paths = []
            temp_files = []

            try:
                for i, img in enumerate(images):
                    fd, temp_path = tempfile.mkstemp(suffix=f"_page_{i}.png")
                    temp_files.append((fd, temp_path))
                    img.save(temp_path, format="PNG")
                    os.close(fd)
                    image_paths.append(temp_path)

                return self._process_pdf_images_with_ocr(image_paths)

            finally:
                for _, temp_path in temp_files:
                    with contextlib.suppress(OSError):
                        Path(temp_path).unlink()

        except Exception as e:
            raise ParsingError(f"Failed to OCR PDF: {e}") from e
        finally:
            if pdf:
                with pypdfium_file_lock(path), contextlib.suppress(Exception):
                    pdf.close()

    def _process_pdf_images_with_ocr(self, image_paths: list[str]) -> str:
        """Process PDF images with the configured OCR backend."""
        if self.config.ocr_backend == "tesseract":
            from kreuzberg._multiprocessing.sync_tesseract import process_batch_images_sync_pure
            from kreuzberg._ocr._tesseract import TesseractConfig

            tesseract_config = (
                self.config.ocr_config if isinstance(self.config.ocr_config, TesseractConfig) else TesseractConfig()
            )
            results = process_batch_images_sync_pure([str(p) for p in image_paths], tesseract_config)
            text_parts = [r.content for r in results]
            return "\n\n".join(text_parts)

        if self.config.ocr_backend == "paddleocr":
            from kreuzberg._multiprocessing.sync_paddleocr import process_image_sync_pure as paddle_process
            from kreuzberg._ocr._paddleocr import PaddleOCRConfig

            paddle_config = (
                self.config.ocr_config if isinstance(self.config.ocr_config, PaddleOCRConfig) else PaddleOCRConfig()
            )

            text_parts = []
            for image_path in image_paths:
                result = paddle_process(Path(image_path), paddle_config)
                text_parts.append(result.content)
            return "\n\n".join(text_parts)

        if self.config.ocr_backend == "easyocr":
            from kreuzberg._multiprocessing.sync_easyocr import process_image_sync_pure as easy_process
            from kreuzberg._ocr._easyocr import EasyOCRConfig

            easy_config = (
                self.config.ocr_config if isinstance(self.config.ocr_config, EasyOCRConfig) else EasyOCRConfig()
            )

            text_parts = []
            for image_path in image_paths:
                result = easy_process(Path(image_path), easy_config)
                text_parts.append(result.content)
            return "\n\n".join(text_parts)

        raise NotImplementedError(f"Sync OCR not implemented for {self.config.ocr_backend}")
