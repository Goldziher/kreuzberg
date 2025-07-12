"""Tests for string utilities."""

from __future__ import annotations

from unittest.mock import patch

from kreuzberg._utils._string import normalize_spaces, safe_decode


def test_safe_decode_with_valid_utf8() -> None:
    """Test safe_decode with valid UTF-8 bytes."""
    result = safe_decode(b"Hello, world!")
    assert result == "Hello, world!"


def test_safe_decode_with_empty_bytes() -> None:
    """Test safe_decode with empty bytes."""
    result = safe_decode(b"")
    assert result == ""


def test_safe_decode_with_specified_encoding() -> None:
    """Test safe_decode with specified encoding."""

    latin_bytes = "café".encode("latin-1")
    result = safe_decode(latin_bytes, encoding="latin-1")
    assert result == "café"


def test_safe_decode_with_invalid_encoding() -> None:
    """Test safe_decode with invalid encoding name."""
    result = safe_decode(b"Hello", encoding="invalid-encoding")
    assert result == "Hello"


def test_safe_decode_fallback_to_latin1() -> None:
    """Test safe_decode fallback to latin-1 when all other encodings fail."""

    problematic_bytes = b"\xff\xfe\x00\x01\x02"

    with patch("kreuzberg._utils._string.chardetng_py.detect") as mock_detect:
        mock_detect.return_value = None

        # This should trigger the latin-1 fallback on line 27  # ~keep
        result = safe_decode(problematic_bytes)

        # Latin-1 can decode any byte sequence, so we should get a result  # ~keep
        assert isinstance(result, str)
        assert len(result) == len(problematic_bytes)


def test_normalize_spaces_basic() -> None:
    """Test normalize_spaces with basic text."""
    result = normalize_spaces("  hello   world  ")
    assert result == "hello world"


def test_normalize_spaces_with_newlines() -> None:
    """Test normalize_spaces with newlines and tabs."""
    result = normalize_spaces("hello\n\t world\r\n  test")
    assert result == "hello\nworld\ntest"


def test_normalize_spaces_empty_string() -> None:
    """Test normalize_spaces with empty string."""
    result = normalize_spaces("")
    assert result == ""


def test_normalize_spaces_only_whitespace() -> None:
    """Test normalize_spaces with only whitespace."""
    result = normalize_spaces("   \n\t\r  ")
    assert result == ""


def test_normalize_spaces_single_word() -> None:
    """Test normalize_spaces with single word."""
    result = normalize_spaces("  hello  ")
    assert result == "hello"


def test_normalize_spaces_multiple_types() -> None:
    """Test normalize_spaces with multiple whitespace types."""
    result = normalize_spaces("word1\u00a0\u2000word2\u2003\u2009word3")
    assert result == "word1 word2 word3"
