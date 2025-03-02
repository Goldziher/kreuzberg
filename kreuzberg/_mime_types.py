from __future__ import annotations

from mimetypes import guess_type
from pathlib import Path
from typing import TYPE_CHECKING, Final

from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:  # pragma: no cover
    from collections.abc import Mapping
    from os import PathLike

HTML_MIME_TYPE: Final = "text/html"
MARKDOWN_MIME_TYPE: Final = "text/markdown"
PDF_MIME_TYPE: Final = "application/pdf"
PLAIN_TEXT_MIME_TYPE: Final = "text/plain"
POWER_POINT_MIME_TYPE: Final = "application/vnd.openxmlformats-officedocument.presentationml.presentation"
DOCX_MIME_TYPE: Final = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
# Excel formats
EXCEL_MIME_TYPE: Final = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
EXCEL_BINARY_MIME_TYPE: Final = "application/vnd.ms-excel"
EXCEL_MACRO_MIME_TYPE: Final = "application/vnd.ms-excel.sheet.macroEnabled.12"
EXCEL_BINARY_2007_MIME_TYPE: Final = "application/vnd.ms-excel.sheet.binary.macroEnabled.12"
EXCEL_ADDON_MIME_TYPE: Final = "application/vnd.ms-excel.addin.macroEnabled.12"
EXCEL_TEMPLATE_MIME_TYPE: Final = "application/vnd.ms-excel.template.macroEnabled.12"

# OpenDocument spreadsheet format
OPENDOC_SPREADSHEET_MIME_TYPE: Final = "application/vnd.oasis.opendocument.spreadsheet"  # ods
PLAIN_TEXT_MIME_TYPES: Final[set[str]] = {PLAIN_TEXT_MIME_TYPE, MARKDOWN_MIME_TYPE}

IMAGE_MIME_TYPES: Final[set[str]] = {
    "image/bmp",
    "image/gif",
    "image/jp2",
    "image/jpeg",
    "image/jpm",
    "image/jpx",
    "image/mj2",
    "image/pjpeg",
    "image/png",
    "image/tiff",
    "image/webp",
    "image/x-bmp",
    "image/x-ms-bmp",
    "image/x-portable-anymap",
    "image/x-portable-bitmap",
    "image/x-portable-graymap",
    "image/x-portable-pixmap",
    "image/x-tiff",
}
IMAGE_MIME_TYPE_EXT_MAP: Final[Mapping[str, str]] = {
    "image/bmp": "bmp",
    "image/x-bmp": "bmp",
    "image/x-ms-bmp": "bmp",
    "image/gif": "gif",
    "image/jpeg": "jpg",
    "image/pjpeg": "jpg",
    "image/png": "png",
    "image/tiff": "tiff",
    "image/x-tiff": "tiff",
    "image/jp2": "jp2",
    "image/jpx": "jpx",
    "image/jpm": "jpm",
    "image/mj2": "mj2",
    "image/webp": "webp",
    "image/x-portable-anymap": "pnm",
    "image/x-portable-bitmap": "pbm",
    "image/x-portable-graymap": "pgm",
    "image/x-portable-pixmap": "ppm",
}
PANDOC_SUPPORTED_MIME_TYPES: Final[set[str]] = {
    "application/csl+json",
    "application/docbook+xml",
    "application/epub+zip",
    "application/rtf",
    "application/vnd.oasis.opendocument.text",
    DOCX_MIME_TYPE,
    "application/x-biblatex",
    "application/x-bibtex",
    "application/x-endnote+xml",
    "application/x-fictionbook+xml",
    "application/x-ipynb+json",
    "application/x-jats+xml",
    "application/x-latex",
    "application/x-opml+xml",
    "application/x-research-info-systems",
    "application/x-typst",
    "text/csv",
    "text/tab-separated-values",
    "text/troff",
    "text/x-commonmark",
    "text/x-dokuwiki",
    "text/x-gfm",
    "text/x-markdown",
    "text/x-markdown-extra",
    "text/x-mdoc",
    "text/x-multimarkdown",
    "text/x-org",
    "text/x-pod",
    "text/x-rst",
}

SPREADSHEET_MIME_TYPES: Final[set[str]] = {
    EXCEL_MIME_TYPE,
    EXCEL_BINARY_MIME_TYPE,
    EXCEL_MACRO_MIME_TYPE,
    EXCEL_BINARY_2007_MIME_TYPE,
    EXCEL_ADDON_MIME_TYPE,
    EXCEL_TEMPLATE_MIME_TYPE,
    OPENDOC_SPREADSHEET_MIME_TYPE,
}

EXT_TO_MIME_TYPE: Final[Mapping[str, str]] = {
    ".txt": PLAIN_TEXT_MIME_TYPE,
    ".md": MARKDOWN_MIME_TYPE,
    ".pdf": PDF_MIME_TYPE,
    ".html": HTML_MIME_TYPE,
    ".htm": HTML_MIME_TYPE,
    ".xlsx": EXCEL_MIME_TYPE,
    ".xls": EXCEL_BINARY_MIME_TYPE,
    ".xlsm": EXCEL_MACRO_MIME_TYPE,
    ".xlsb": EXCEL_BINARY_2007_MIME_TYPE,
    ".xlam": EXCEL_ADDON_MIME_TYPE,
    ".xla": EXCEL_TEMPLATE_MIME_TYPE,
    ".ods": OPENDOC_SPREADSHEET_MIME_TYPE,
    ".pptx": POWER_POINT_MIME_TYPE,
    ".bmp": "image/bmp",
    ".gif": "image/gif",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".png": "image/png",
    ".tiff": "image/tiff",
    ".tif": "image/tiff",
    ".webp": "image/webp",
    ".jp2": "image/jp2",
    ".jpx": "image/jpx",
    ".jpm": "image/jpm",
    ".mj2": "image/mj2",
    ".pnm": "image/x-portable-anymap",
    ".pbm": "image/x-portable-bitmap",
    ".pgm": "image/x-portable-graymap",
    ".ppm": "image/x-portable-pixmap",
    ".csv": "text/csv",
    ".tsv": "text/tab-separated-values",
    ".rst": "text/x-rst",
    ".org": "text/x-org",
    ".epub": "application/epub+zip",
    ".rtf": "application/rtf",
    ".odt": "application/vnd.oasis.opendocument.text",
    ".docx": DOCX_MIME_TYPE,
    ".bib": "application/x-bibtex",
    ".ipynb": "application/x-ipynb+json",
    ".tex": "application/x-latex",
}

SUPPORTED_MIME_TYPES: Final[set[str]] = (
    PLAIN_TEXT_MIME_TYPES
    | IMAGE_MIME_TYPES
    | PANDOC_SUPPORTED_MIME_TYPES
    | SPREADSHEET_MIME_TYPES
    | {PDF_MIME_TYPE, POWER_POINT_MIME_TYPE, HTML_MIME_TYPE}
)


def validate_mime_type(file_path: PathLike[str] | str, mime_type: str | None = None) -> str:
    """Validate and detect the MIME type for a given file.

    Args:
        file_path: The path to the file.
        mime_type: Optional explicit MIME type. If provided, this will be validated.
            If not provided, the function will attempt to detect the MIME type.

    Raises:
        ValidationError: If the MIME type is not supported or cannot be determined.

    Returns:
        The validated MIME type.
    """
    path = Path(file_path)

    if not mime_type:
        # Try to determine MIME type from file extension first
        ext = path.suffix.lower()
        mime_type = EXT_TO_MIME_TYPE.get(ext) or guess_type(path.name)[0]

        if not mime_type:  # pragma: no cover
            raise ValidationError(
                "Could not determine the mime type of the file. Please specify the mime_type parameter explicitly.",
                context={"input_file": str(path), "extension": ext},
            )

    if mime_type in SUPPORTED_MIME_TYPES:
        return mime_type

    for supported_mime_type in SUPPORTED_MIME_TYPES:
        if mime_type.startswith(supported_mime_type):
            return supported_mime_type

    raise ValidationError(
        f"Unsupported mime type: {mime_type}",
        context={"mime_type": mime_type, "supported_mimetypes": ",".join(sorted(SUPPORTED_MIME_TYPES))},
    )
