from __future__ import annotations

from re import Pattern
from re import compile as compile_regex
from typing import TYPE_CHECKING, Final, cast

import pypdfium2
from anyio import Path as AsyncPath

from kreuzberg import ExtractionResult
from kreuzberg._mime_types import PLAIN_TEXT_MIME_TYPE
from kreuzberg._string import normalize_spaces
from kreuzberg._sync import run_sync
from kreuzberg._tesseract import PSMMode, batch_process_images
from kreuzberg.exceptions import ParsingError

if TYPE_CHECKING:  # pragma: no cover
    from pathlib import Path

    from PIL.Image import Image


# Pattern to detect common PDF text extraction corruption:
# - Control and non-printable characters
# - Unicode replacement and invalid characters
# - Zero-width spaces and other invisible characters
CORRUPTED_PATTERN: Final[Pattern[str]] = compile_regex(r"[\x00-\x08\x0B-\x0C\x0E-\x1F]|\uFFFD")
SHORT_TEXT_THRESHOLD: Final[int] = 50
MINIMUM_CORRUPTED_RESULTS: Final[int] = 2


def _validate_extracted_text(text: str, corruption_threshold: float = 0.05) -> bool:
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

    corruption_matches = CORRUPTED_PATTERN.findall(text)

    if len(text) < SHORT_TEXT_THRESHOLD:
        return len(corruption_matches) <= MINIMUM_CORRUPTED_RESULTS

    return (len(corruption_matches) / len(text)) < corruption_threshold


async def _convert_pdf_to_images(input_file: Path) -> list[Image]:
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
        return [page.render(scale=4.25).to_pil() for page in cast(pypdfium2.PdfDocument, document)]
    except pypdfium2.PdfiumError as e:
        raise ParsingError(
            "Could not convert PDF to images", context={"file_path": str(input_file), "error": str(e)}
        ) from e
    finally:
        if document:
            await run_sync(document.close)


async def _extract_pdf_text_with_ocr(
    input_file: Path,
    *,
    language: str = "eng",
    max_processes: int,
    psm: PSMMode = PSMMode.AUTO,
) -> ExtractionResult:
    """Extract text from a scanned PDF file using pytesseract.

    Args:
        input_file: The path to the PDF file.
        language: The language code for OCR. Defaults to "eng".
        max_processes: Maximum number of concurrent processes. Defaults to CPU count / 2 (minimum 1).
        psm: Page segmentation mode for Tesseract OCR. Defaults to PSMMode.AUTO.

    Returns:
        The extracted text.
    """
    images = await _convert_pdf_to_images(input_file)
    ocr_results = await batch_process_images(images, max_processes=max_processes, psm=psm, language=language)
    return ExtractionResult(
        content="\n".join([v.content for v in ocr_results]), mime_type=PLAIN_TEXT_MIME_TYPE, metadata={}
    )


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
        text = "\n".join(page.get_textpage().get_text_bounded() for page in cast(pypdfium2.PdfDocument, document))
        return normalize_spaces(text)
    except pypdfium2.PdfiumError as e:
        raise ParsingError(
            "Could not extract text from PDF file", context={"file_path": str(input_file), "error": str(e)}
        ) from e
    finally:
        if document:
            await run_sync(document.close)


async def extract_pdf_file(
    input_file: Path,
    *,
    force_ocr: bool,
    language: str = "eng",
    max_processes: int,
    psm: PSMMode = PSMMode.AUTO,
) -> ExtractionResult:
    """Extract text from a PDF file.

    Args:
        input_file: The path to the PDF file.
        force_ocr: Whether to force OCR on PDF files that have a text layer.
        language: The language code for OCR. Defaults to "eng".
        max_processes: Maximum number of concurrent processes. Defaults to CPU count / 2 (minimum 1).
        psm: Page segmentation mode for Tesseract OCR. Defaults to PSMMode.AUTO.

    Returns:
        The extracted text.
    """
    if not force_ocr:
        content = await _extract_pdf_searchable_text(input_file)
        if _validate_extracted_text(content):
            return ExtractionResult(content=content, mime_type=PLAIN_TEXT_MIME_TYPE, metadata={})
    return await _extract_pdf_text_with_ocr(input_file, max_processes=max_processes, language=language, psm=psm)


async def extract_pdf_content(
    content: bytes,
    *,
    force_ocr: bool,
    language: str = "eng",
    max_processes: int,
    psm: PSMMode = PSMMode.AUTO,
) -> ExtractionResult:
    """Extract text from a PDF file content.

    Args:
        content: The PDF file content.
        force_ocr: Whether to force OCR on PDF files that have a text layer.
        language: The language code for OCR. Defaults to "eng".
        max_processes: Maximum number of concurrent processes. Defaults to CPU count / 2 (minimum 1).
        psm: Page segmentation mode for Tesseract OCR. Defaults to PSMMode.AUTO.

    Returns:
        The extracted text.
    """
    from kreuzberg._tmp import create_temp_file

    file_path, unlink = await create_temp_file(".pdf")
    await AsyncPath(file_path).write_bytes(content)
    result = await extract_pdf_file(
        file_path, force_ocr=force_ocr, max_processes=max_processes, psm=psm, language=language
    )
    await unlink()
    return result
