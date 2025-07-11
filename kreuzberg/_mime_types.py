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

EML_MIME_TYPE: Final = "message/rfc822"
MSG_MIME_TYPE: Final = "application/vnd.ms-outlook"
JSON_MIME_TYPE: Final = "application/json"
YAML_MIME_TYPE: Final = "application/x-yaml"
TOML_MIME_TYPE: Final = "application/toml"

EXCEL_MIME_TYPE: Final = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
EXCEL_BINARY_MIME_TYPE: Final = "application/vnd.ms-excel"
EXCEL_MACRO_MIME_TYPE: Final = "application/vnd.ms-excel.sheet.macroEnabled.12"
EXCEL_BINARY_2007_MIME_TYPE: Final = "application/vnd.ms-excel.sheet.binary.macroEnabled.12"
EXCEL_ADDON_MIME_TYPE: Final = "application/vnd.ms-excel.addin.macroEnabled.12"
EXCEL_TEMPLATE_MIME_TYPE: Final = "application/vnd.ms-excel.template.macroEnabled.12"


OPENDOC_SPREADSHEET_MIME_TYPE: Final = "application/vnd.oasis.opendocument.spreadsheet"
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
    ".eml": EML_MIME_TYPE,
    ".msg": MSG_MIME_TYPE,
    ".json": JSON_MIME_TYPE,
    ".yaml": YAML_MIME_TYPE,
    ".yml": YAML_MIME_TYPE,
    ".toml": TOML_MIME_TYPE,
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
    | {
        PDF_MIME_TYPE,
        POWER_POINT_MIME_TYPE,
        HTML_MIME_TYPE,
        EML_MIME_TYPE,
        MSG_MIME_TYPE,
        JSON_MIME_TYPE,
        YAML_MIME_TYPE,
        TOML_MIME_TYPE,
        "text/json",
        "text/yaml",
        "text/x-yaml",
        "application/yaml",
        "text/toml",
    }
)


def validate_mime_type(
    *, file_path: PathLike[str] | str | None = None, mime_type: str | None = None, check_file_exists: bool = True
) -> str:
    """Validate and detect the MIME type for a given file.

    Args:
        file_path: The path to the file.
        mime_type: Optional explicit MIME type. If provided, this will be validated.
            If not provided, the function will attempt to detect the MIME type.
        check_file_exists: Whether to check if the file exists. Default is True.
            Set to False in tests where you want to validate a mime type without an actual file.

    Raises:
        ValidationError: If the MIME type is not supported or cannot be determined.

    Returns:
        The validated MIME type.
    """
    if mime_type:
        return _validate_explicit_mime_type(mime_type)

    if file_path:
        from kreuzberg._utils._cache import get_mime_cache

        path = Path(file_path)

        try:
            stat = path.stat() if check_file_exists else None
            file_info = {
                "path": str(path.resolve()),
                "size": stat.st_size if stat else 0,
                "mtime": stat.st_mtime if stat else 0,
                "check_file_exists": check_file_exists,
            }
        except OSError:
            file_info = {
                "path": str(path),
                "size": 0,
                "mtime": 0,
                "check_file_exists": check_file_exists,
            }

        cache_kwargs = {"file_info": str(sorted(file_info.items())), "detector": "mime_type"}

        mime_cache = get_mime_cache()
        cached_result = mime_cache.get(**cache_kwargs)
        if cached_result is not None:
            return cached_result

        detected_mime_type = _detect_mime_type_uncached(file_path, check_file_exists)

        mime_cache.set(detected_mime_type, **cache_kwargs)

        return detected_mime_type

    return _detect_mime_type_uncached(file_path, check_file_exists)


def _validate_explicit_mime_type(mime_type: str) -> str:
    """Validate an explicitly provided MIME type."""
    if mime_type in SUPPORTED_MIME_TYPES:
        return mime_type

    for supported_mime_type in SUPPORTED_MIME_TYPES:
        if mime_type.startswith(supported_mime_type):
            return supported_mime_type

    raise ValidationError(
        f"Unsupported mime type: {mime_type}",
        context={"mime_type": mime_type, "supported_mimetypes": ",".join(sorted(SUPPORTED_MIME_TYPES))},
    )


def _detect_mime_type_uncached(file_path: PathLike[str] | str | None = None, check_file_exists: bool = True) -> str:
    """Detect MIME type without caching (internal function)."""
    if file_path and check_file_exists:
        path = Path(file_path)
        if not path.exists():
            raise ValidationError("The file does not exist", context={"file_path": str(path)})

    if not file_path:
        raise ValidationError(
            "Could not determine mime type.",
        )

    path = Path(file_path)
    ext = path.suffix.lower()
    mime_type = EXT_TO_MIME_TYPE.get(ext) or guess_type(path.name)[0]

    if not mime_type:  # pragma: no cover
        raise ValidationError(
            "Could not determine the mime type of the file. Please specify the mime_type parameter explicitly.",
            context={"input_file": str(path), "extension": ext},
        )

    return _validate_explicit_mime_type(mime_type)
