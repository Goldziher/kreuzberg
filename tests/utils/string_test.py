from __future__ import annotations

import pytest

from kreuzberg._utils._string import normalize_spaces, safe_decode


@pytest.mark.parametrize(
    ("byte_data", "encoding", "expected"),
    [
        (b"hello", "utf-8", "hello"),
        (b"hello", None, "hello"),
        (b"caf\xc3\xa9", "utf-8", "café"),
        (b"caf\xe9", "latin-1", "café"),
        (b"", "utf-8", ""),
        (b"", None, ""),
    ],
)
def test_safe_decode(byte_data: bytes, encoding: str | None, expected: str) -> None:
    assert safe_decode(byte_data, encoding) == expected


def test_safe_decode_with_invalid_encoding_value() -> None:
    test_bytes = b"Hello World"
    result = safe_decode(test_bytes, encoding="invalid-encoding")
    assert result == "Hello World"


def test_safe_decode_with_detected_encoding() -> None:
    text = "Hello 世界"
    byte_data = text.encode("utf-8")
    assert safe_decode(byte_data) == text


def test_safe_decode_with_all_encodings_failing() -> None:
    invalid_bytes = bytes([0xFF, 0xFE, 0xFD])
    result = safe_decode(invalid_bytes, encoding="invalid-encoding")
    assert result is not None
    assert isinstance(result, str)
    assert len(result) == 3


def test_safe_decode_with_invalid_encoding() -> None:
    byte_data = bytes([0xFF, 0xFE, 0xFD])
    result = safe_decode(byte_data)

    assert isinstance(result, str)
    assert len(result) == 3

    assert safe_decode(byte_data) == result


def test_safe_decode_with_fallback_encodings() -> None:
    text = "Hello World"
    byte_data = text.encode("utf-8")
    assert safe_decode(byte_data) == text


@pytest.mark.parametrize(
    ("input_text", "expected"),
    [
        ("hello  world", "hello world"),
        ("  hello   world  ", "hello world"),
        ("\thello\t\tworld\n", "hello world"),
        ("hello      world", "hello world"),
        ("", ""),
        ("   ", ""),
    ],
)
def test_normalize_spaces(input_text: str, expected: str) -> None:
    assert normalize_spaces(input_text) == expected
