from __future__ import annotations

import tempfile
from pathlib import Path
from unittest.mock import Mock, patch

import pytest
from kreuzberg._mime_types import (
    DOCX_MIME_TYPE,
    EXCEL_MIME_TYPE,
    HTML_MIME_TYPE,
    PDF_MIME_TYPE,
    _detect_mime_type_uncached,
    _validate_explicit_mime_type,
    validate_mime_type,
)
from kreuzberg.exceptions import ValidationError


def test_validate_mime_type_with_explicit_mime_type() -> None:
    result = validate_mime_type(mime_type="application/pdf")
    assert result == "application/pdf"

    result = validate_mime_type(mime_type="text/html")
    assert result == "text/html"


def test_validate_mime_type_with_file_path_and_cache_hit() -> None:
    mock_cache = Mock()
    mock_cache.get.return_value = "application/pdf"

    with (
        patch("kreuzberg._mime_types.get_mime_cache", return_value=mock_cache),
        patch("kreuzberg._mime_types.Path") as mock_path_class,
    ):
        mock_path = Mock()
        mock_path.stat.return_value = Mock(st_size=1000, st_mtime=123456)
        mock_path.resolve.return_value = "/full/path/to/file.pdf"
        mock_path_class.return_value = mock_path

        result = validate_mime_type(file_path="test.pdf")

    assert result == "application/pdf"
    mock_cache.get.assert_called_once()
    mock_cache.set.assert_not_called()


def test_validate_mime_type_with_file_path_cache_miss() -> None:
    mock_cache = Mock()
    mock_cache.get.return_value = None

    with (
        patch("kreuzberg._mime_types.get_mime_cache", return_value=mock_cache),
        patch("kreuzberg._mime_types._detect_mime_type_uncached", return_value="application/pdf") as mock_detect,
        patch("kreuzberg._mime_types.Path") as mock_path_class,
    ):
        mock_path = Mock()
        mock_path.stat.return_value = Mock(st_size=1000, st_mtime=123456)
        mock_path.resolve.return_value = "/full/path/to/file.pdf"
        mock_path_class.return_value = mock_path

        result = validate_mime_type(file_path="test.pdf")

    assert result == "application/pdf"
    mock_cache.get.assert_called_once()
    mock_cache.set.assert_called_once_with(
        "application/pdf", file_info=mock_cache.get.call_args.kwargs["file_info"], detector="mime_type"
    )
    mock_detect.assert_called_once_with("test.pdf", True)


def test_validate_mime_type_with_file_path_os_error() -> None:
    mock_cache = Mock()
    mock_cache.get.return_value = None

    with (
        patch("kreuzberg._mime_types.get_mime_cache", return_value=mock_cache),
        patch("kreuzberg._mime_types._detect_mime_type_uncached", return_value="text/plain") as mock_detect,
        patch("kreuzberg._mime_types.Path") as mock_path_class,
    ):
        mock_path = Mock()
        mock_path.stat.side_effect = OSError("File not found")
        mock_path.__str__ = Mock(return_value="test.txt")  # type: ignore[method-assign]
        mock_path_class.return_value = mock_path

        result = validate_mime_type(file_path="test.txt")

    assert result == "text/plain"
    mock_cache.get.assert_called_once()
    mock_cache.set.assert_called_once()
    mock_detect.assert_called_once_with("test.txt", True)


def test_validate_mime_type_no_file_path_no_mime_type() -> None:
    with patch("kreuzberg._mime_types._detect_mime_type_uncached") as mock_detect:
        mock_detect.side_effect = ValidationError("Could not determine mime type.")

        with pytest.raises(ValidationError, match="Could not determine mime type"):
            validate_mime_type()


def test_validate_mime_type_with_file_no_check() -> None:
    mock_cache = Mock()
    mock_cache.get.return_value = None

    with (
        patch("kreuzberg._mime_types.get_mime_cache", return_value=mock_cache),
        patch("kreuzberg._mime_types._detect_mime_type_uncached", return_value="application/pdf") as mock_detect,
        patch("kreuzberg._mime_types.Path") as mock_path_class,
    ):
        mock_path = Mock()
        mock_path.stat.return_value = None
        mock_path.resolve.return_value = "/full/path/to/file.pdf"
        mock_path_class.return_value = mock_path

        result = validate_mime_type(file_path="test.pdf", check_file_exists=False)

    assert result == "application/pdf"
    mock_detect.assert_called_once_with("test.pdf", False)


def test_validate_explicit_mime_type_supported() -> None:
    result = _validate_explicit_mime_type("application/pdf")
    assert result == "application/pdf"

    result = _validate_explicit_mime_type("text/html")
    assert result == "text/html"


def test_validate_explicit_mime_type_with_prefix_match() -> None:
    result = _validate_explicit_mime_type("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    assert result == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"

    result = _validate_explicit_mime_type("text/plain")
    assert result == "text/plain"

    with patch("kreuzberg._mime_types.SUPPORTED_MIME_TYPES", {"application/", "text/html"}):
        result = _validate_explicit_mime_type("application/custom")
        assert result == "application/"


def test_validate_explicit_mime_type_unsupported() -> None:
    with pytest.raises(ValidationError, match="Unsupported mime type: application/unknown"):
        _validate_explicit_mime_type("application/unknown")


def test_detect_mime_type_uncached_no_file_path() -> None:
    with pytest.raises(ValidationError, match="Could not determine mime type"):
        _detect_mime_type_uncached(None)


def test_detect_mime_type_uncached_file_not_exists() -> None:
    with patch("kreuzberg._mime_types.Path") as mock_path_class:
        mock_path = Mock()
        mock_path.exists.return_value = False
        mock_path.__str__ = Mock(return_value="/nonexistent/file.pdf")  # type: ignore[method-assign]
        mock_path_class.return_value = mock_path

        with pytest.raises(ValidationError, match="The file does not exist"):
            _detect_mime_type_uncached("/nonexistent/file.pdf", check_file_exists=True)


def test_detect_mime_type_uncached_with_known_extension() -> None:
    with patch("kreuzberg._mime_types.Path") as mock_path_class:
        mock_path = Mock()
        mock_path.exists.return_value = True
        mock_path.suffix = ".pdf"
        mock_path.name = "document.pdf"
        mock_path_class.return_value = mock_path

        result = _detect_mime_type_uncached("document.pdf", check_file_exists=True)

    assert result == "application/pdf"


def test_detect_mime_type_uncached_with_guess_type() -> None:
    with (
        patch("kreuzberg._mime_types.Path") as mock_path_class,
        patch("kreuzberg._mime_types.guess_type") as mock_guess,
    ):
        mock_path = Mock()
        mock_path.exists.return_value = True
        mock_path.suffix = ".xyz"
        mock_path.name = "file.xyz"
        mock_path_class.return_value = mock_path

        mock_guess.return_value = ("text/html", None)

        result = _detect_mime_type_uncached("file.xyz", check_file_exists=True)

    assert result == "text/html"
    mock_guess.assert_called_once_with("file.xyz")


def test_detect_mime_type_uncached_no_check_file_exists() -> None:
    with patch("kreuzberg._mime_types.Path") as mock_path_class:
        mock_path = Mock()
        mock_path.suffix = ".docx"
        mock_path.name = "document.docx"
        mock_path_class.return_value = mock_path

        result = _detect_mime_type_uncached("document.docx", check_file_exists=False)

    assert result == DOCX_MIME_TYPE
    mock_path.exists.assert_not_called()


def test_detect_mime_type_uncached_unsupported_mime() -> None:
    with (
        patch("kreuzberg._mime_types.Path") as mock_path_class,
        patch("kreuzberg._mime_types.guess_type") as mock_guess,
    ):
        mock_path = Mock()
        mock_path.exists.return_value = True
        mock_path.suffix = ".xyz"
        mock_path.name = "file.xyz"
        mock_path_class.return_value = mock_path

        mock_guess.return_value = ("application/unsupported", None)

        with pytest.raises(ValidationError, match="Unsupported mime type"):
            _detect_mime_type_uncached("file.xyz", check_file_exists=True)


def test_real_file_mime_detection() -> None:
    with tempfile.NamedTemporaryFile(suffix=".txt", delete=False) as tmp:
        tmp.write(b"Hello World")
        tmp_path = tmp.name

    try:
        result = validate_mime_type(file_path=tmp_path)
        assert result == "text/plain"
    finally:
        Path(tmp_path).unlink()


def test_mime_type_constants() -> None:
    assert PDF_MIME_TYPE == "application/pdf"
    assert HTML_MIME_TYPE == "text/html"
    assert DOCX_MIME_TYPE == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    assert EXCEL_MIME_TYPE == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
