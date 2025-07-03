from pathlib import Path

import pytest

from kreuzberg._mime_types import (
    EXT_TO_MIME_TYPE,
    HTML_MIME_TYPE,
    IMAGE_MIME_TYPES,
    MARKDOWN_MIME_TYPE,
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
    SUPPORTED_MIME_TYPES,
    validate_mime_type,
)
from kreuzberg.exceptions import ValidationError


def test_validate_mime_type_with_explicit_mime_type() -> None:
    assert (
        validate_mime_type(file_path="test.txt", mime_type=PLAIN_TEXT_MIME_TYPE, check_file_exists=False)
        == PLAIN_TEXT_MIME_TYPE
    )
    assert validate_mime_type(file_path="test.pdf", mime_type=PDF_MIME_TYPE, check_file_exists=False) == PDF_MIME_TYPE
    assert (
        validate_mime_type(file_path="test.html", mime_type=HTML_MIME_TYPE, check_file_exists=False) == HTML_MIME_TYPE
    )

    assert (
        validate_mime_type(file_path="test.txt", mime_type="text/plain; charset=utf-8", check_file_exists=False)
        == PLAIN_TEXT_MIME_TYPE
    )
    assert (
        validate_mime_type(file_path="test.pdf", mime_type="application/pdf; version=1.7", check_file_exists=False)
        == PDF_MIME_TYPE
    )
    assert (
        validate_mime_type(file_path="test.html", mime_type="text/html; charset=utf-8", check_file_exists=False)
        == HTML_MIME_TYPE
    )

    with pytest.raises(ValidationError) as exc_info:
        validate_mime_type(file_path="test.txt", mime_type="application/invalid", check_file_exists=False)
    assert "Unsupported mime type" in str(exc_info.value)


def test_validate_mime_type_extension_detection() -> None:
    assert validate_mime_type(file_path="document.txt", check_file_exists=False) == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type(file_path="document.md", check_file_exists=False) == MARKDOWN_MIME_TYPE
    assert validate_mime_type(file_path="presentation.pptx", check_file_exists=False) == POWER_POINT_MIME_TYPE
    assert validate_mime_type(file_path="document.pdf", check_file_exists=False) == PDF_MIME_TYPE

    assert validate_mime_type(file_path="image.PNG", check_file_exists=False) == "image/png"
    assert validate_mime_type(file_path="document.PDF", check_file_exists=False) == PDF_MIME_TYPE
    assert validate_mime_type(file_path="page.HTML", check_file_exists=False) == HTML_MIME_TYPE

    assert validate_mime_type(file_path=Path("document.txt"), check_file_exists=False) == PLAIN_TEXT_MIME_TYPE

    assert (
        validate_mime_type(file_path="document.txt", mime_type="text/plain; charset=utf-8", check_file_exists=False)
        == PLAIN_TEXT_MIME_TYPE
    )
    assert (
        validate_mime_type(file_path="document.html", mime_type="text/html; charset=utf-8", check_file_exists=False)
        == HTML_MIME_TYPE
    )


def test_validate_mime_type_image_extensions() -> None:
    image_files = {
        "photo.jpg": "image/jpeg",
        "photo.jpeg": "image/jpeg",
        "icon.png": "image/png",
        "picture.gif": "image/gif",
        "scan.tiff": "image/tiff",
        "graphic.webp": "image/webp",
        "image.bmp": "image/bmp",
    }

    for filename, expected_mime in image_files.items():
        assert validate_mime_type(file_path=filename, check_file_exists=False) == expected_mime
        assert expected_mime in IMAGE_MIME_TYPES

        parameterized_mime = f"{expected_mime}; charset=binary"
        assert (
            validate_mime_type(file_path=filename, mime_type=parameterized_mime, check_file_exists=False)
            == expected_mime
        )


def test_validate_mime_type_unknown_extension() -> None:
    with pytest.raises(ValidationError) as exc_info:
        validate_mime_type(file_path="file.unknown", check_file_exists=False)
    assert "Could not determine the mime type" in str(exc_info.value)
    assert "extension" in exc_info.value.context
    assert exc_info.value.context["extension"] == ".unknown"


def test_ext_to_mime_type_mapping_consistency() -> None:
    for mime_type in EXT_TO_MIME_TYPE.values():
        result = validate_mime_type(file_path="test.txt", mime_type=mime_type, check_file_exists=False)
        assert result in SUPPORTED_MIME_TYPES

        parameterized = f"{mime_type}; charset=utf-8"
        result = validate_mime_type(file_path="test.txt", mime_type=parameterized, check_file_exists=False)
        assert result in SUPPORTED_MIME_TYPES


def test_validate_mime_type_with_path_variants() -> None:
    assert validate_mime_type(file_path="./document.txt", check_file_exists=False) == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type(file_path="/path/to/document.pdf", check_file_exists=False) == PDF_MIME_TYPE
    assert validate_mime_type(file_path="relative/path/page.html", check_file_exists=False) == HTML_MIME_TYPE

    assert (
        validate_mime_type(
            file_path=Path("document.txt"), mime_type="text/plain; charset=utf-8", check_file_exists=False
        )
        == PLAIN_TEXT_MIME_TYPE
    )
    assert (
        validate_mime_type(
            file_path=Path("/absolute/path/document.pdf"),
            mime_type="application/pdf; version=1.7",
            check_file_exists=False,
        )
        == PDF_MIME_TYPE
    )
    assert (
        validate_mime_type(
            file_path=Path("./relative/path/page.html"), mime_type="text/html; charset=utf-8", check_file_exists=False
        )
        == HTML_MIME_TYPE
    )

    assert (
        validate_mime_type(
            file_path="./document.txt", mime_type="text/plain; charset=us-ascii", check_file_exists=False
        )
        == PLAIN_TEXT_MIME_TYPE
    )
    assert (
        validate_mime_type(
            file_path="/path/to/document.pdf", mime_type="application/pdf; version=1.5", check_file_exists=False
        )
        == PDF_MIME_TYPE
    )


def test_validate_mime_type_with_dots_in_name() -> None:
    assert validate_mime_type(file_path="my.backup.txt", check_file_exists=False) == PLAIN_TEXT_MIME_TYPE
    assert validate_mime_type(file_path="version.1.2.pdf", check_file_exists=False) == PDF_MIME_TYPE
    assert validate_mime_type(file_path="index.min.html", check_file_exists=False) == HTML_MIME_TYPE

    assert validate_mime_type(file_path="readme.v2.md", check_file_exists=False) == MARKDOWN_MIME_TYPE
    assert validate_mime_type(file_path="document.2023.02.14.pdf", check_file_exists=False) == PDF_MIME_TYPE

    assert (
        validate_mime_type(file_path="my.backup.txt", mime_type="text/plain; charset=utf-8", check_file_exists=False)
        == PLAIN_TEXT_MIME_TYPE
    )
    assert (
        validate_mime_type(file_path="index.min.html", mime_type="text/html; charset=utf-8", check_file_exists=False)
        == HTML_MIME_TYPE
    )
