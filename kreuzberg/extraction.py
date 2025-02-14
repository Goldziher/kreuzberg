"""This module provides functions to extract textual content from files.

It includes vendored code:

- The extract PPTX logic is based on code vendored from `markitdown` to extract text from PPTX files.
    See: https://github.com/microsoft/markitdown/blob/main/src/markitdown/_markitdown.py
    Refer to the markitdown repository for it's license (MIT).
"""

from __future__ import annotations

from mimetypes import guess_type
from pathlib import Path
from tempfile import NamedTemporaryFile
from typing import NamedTuple

from anyio import Path as AsyncPath

from kreuzberg._extractors import (
    extract_content_with_pandoc,
    extract_file_with_pandoc,
    extract_html_string,
    extract_pdf,
    extract_pptx_file,
    extract_xlsx_file,
)
from kreuzberg._mime_types import (
    EXCEL_MIME_TYPE,
    HTML_MIME_TYPE,
    IMAGE_MIME_TYPE_EXT_MAP,
    IMAGE_MIME_TYPES,
    MARKDOWN_MIME_TYPE,
    PANDOC_SUPPORTED_MIME_TYPES,
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
    SUPPORTED_MIME_TYPES,
)
from kreuzberg._string import safe_decode
from kreuzberg._tesseract import process_image_with_tesseract
from kreuzberg.exceptions import ValidationError


class ExtractionResult(NamedTuple):
    """The result of a file extraction."""

    content: str
    """The extracted content."""
    mime_type: str
    """The mime type of the content."""


async def extract_bytes(content: bytes, mime_type: str, force_ocr: bool = False) -> ExtractionResult:
    """Extract the textual content from a given byte string representing a file's contents.

    Args:
        content: The content to extract.
        mime_type: The mime type of the content.
        force_ocr: Whether or not to force OCR on PDF files that have a text layer. Default = false.

    Raises:
        ValidationError: If the mime type is not supported.

    Returns:
        The extracted content and the mime type of the content.
    """
    if mime_type not in SUPPORTED_MIME_TYPES or not any(mime_type.startswith(value) for value in SUPPORTED_MIME_TYPES):
        raise ValidationError(
            f"Unsupported mime type: {mime_type}",
            context={"mime_type": mime_type, "supported_mimetypes": ",".join(sorted(SUPPORTED_MIME_TYPES))},
        )

    if mime_type == PDF_MIME_TYPE or mime_type.startswith(PDF_MIME_TYPE):
        return ExtractionResult(content=await extract_pdf(content, force_ocr), mime_type=PLAIN_TEXT_MIME_TYPE)

    if mime_type == EXCEL_MIME_TYPE or mime_type.startswith(EXCEL_MIME_TYPE):
        return ExtractionResult(content=await extract_xlsx_file(content), mime_type=MARKDOWN_MIME_TYPE)

    if mime_type in IMAGE_MIME_TYPES or any(mime_type.startswith(value) for value in IMAGE_MIME_TYPES):
        with NamedTemporaryFile(suffix=IMAGE_MIME_TYPE_EXT_MAP[mime_type], delete=False) as temp_file:
            try:
                temp_file.write(content)
                return ExtractionResult(
                    content=await process_image_with_tesseract(temp_file.name), mime_type=PLAIN_TEXT_MIME_TYPE
                )
            finally:
                temp_file.close()
                await AsyncPath(temp_file.name).unlink()

    if mime_type in PANDOC_SUPPORTED_MIME_TYPES or any(
        mime_type.startswith(value) for value in PANDOC_SUPPORTED_MIME_TYPES
    ):
        return ExtractionResult(
            content=await extract_content_with_pandoc(content, mime_type), mime_type=MARKDOWN_MIME_TYPE
        )

    if mime_type == POWER_POINT_MIME_TYPE or mime_type.startswith(POWER_POINT_MIME_TYPE):
        return ExtractionResult(content=await extract_pptx_file(content), mime_type=MARKDOWN_MIME_TYPE)

    if mime_type == HTML_MIME_TYPE or mime_type.startswith(HTML_MIME_TYPE):
        return ExtractionResult(content=await extract_html_string(content), mime_type=MARKDOWN_MIME_TYPE)

    return ExtractionResult(
        content=safe_decode(content),
        mime_type=mime_type,
    )


async def extract_file(
    file_path: Path | str, mime_type: str | None = None, force_ocr: bool = False
) -> ExtractionResult:
    """Extract the textual content from a given file.

    Args:
        file_path: The path to the file.
        mime_type: The mime type of the file.
        force_ocr: Whether or not to force OCR on PDF files that have a text layer. Default = false.

    Raises:
        ValidationError: If the mime type is not supported.

    Returns:
        The extracted content and the mime type of the content.
    """
    file_path = Path(file_path)
    mime_type = mime_type or guess_type(file_path.name)[0]
    if not mime_type:  # pragma: no cover
        raise ValidationError("Could not determine the mime type of the file.", context={"file_path": str(file_path)})

    if mime_type not in SUPPORTED_MIME_TYPES or not any(mime_type.startswith(value) for value in SUPPORTED_MIME_TYPES):
        raise ValidationError(
            f"Unsupported mime type: {mime_type}",
            context={"mime_type": mime_type, "supported_mimetypes": ",".join(sorted(SUPPORTED_MIME_TYPES))},
        )

    if not await AsyncPath(file_path).exists():
        raise ValidationError("The file does not exist.", context={"file_path": str(file_path)})

    if mime_type == PDF_MIME_TYPE or mime_type.startswith(PDF_MIME_TYPE):
        return ExtractionResult(content=await extract_pdf(file_path, force_ocr), mime_type=PLAIN_TEXT_MIME_TYPE)

    if mime_type == EXCEL_MIME_TYPE or mime_type.startswith(EXCEL_MIME_TYPE):
        return ExtractionResult(content=await extract_xlsx_file(file_path), mime_type=MARKDOWN_MIME_TYPE)

    if mime_type in IMAGE_MIME_TYPES or any(mime_type.startswith(value) for value in IMAGE_MIME_TYPES):
        return ExtractionResult(content=await process_image_with_tesseract(file_path), mime_type=PLAIN_TEXT_MIME_TYPE)

    if mime_type in PANDOC_SUPPORTED_MIME_TYPES or any(
        mime_type.startswith(value) for value in PANDOC_SUPPORTED_MIME_TYPES
    ):
        return ExtractionResult(
            content=await extract_file_with_pandoc(file_path, mime_type), mime_type=MARKDOWN_MIME_TYPE
        )

    if mime_type == POWER_POINT_MIME_TYPE or mime_type.startswith(POWER_POINT_MIME_TYPE):
        return ExtractionResult(content=await extract_pptx_file(file_path), mime_type=MARKDOWN_MIME_TYPE)

    if mime_type == HTML_MIME_TYPE or mime_type.startswith(HTML_MIME_TYPE):
        return ExtractionResult(content=await extract_html_string(file_path), mime_type=MARKDOWN_MIME_TYPE)

    return ExtractionResult(content=await AsyncPath(file_path).read_text(), mime_type=mime_type)
