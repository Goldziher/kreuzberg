from __future__ import annotations

from kreuzberg._internal_bindings import (
    calculate_text_confidence,
    fix_mojibake,
    get_encoding_cache_key,
    normalize_spaces,
    safe_decode,
)


def test_safe_decode_empty_bytes() -> None:
    assert safe_decode(b"") == ""


def test_safe_decode_with_encoding() -> None:
    text = "Hello, World! שלום עולם"
    utf8_bytes = text.encode("utf-8")
    assert safe_decode(utf8_bytes, "utf-8") == text


def test_safe_decode_with_invalid_encoding() -> None:
    text = "Hello, World!"
    utf8_bytes = text.encode("utf-8")
    result = safe_decode(utf8_bytes, "invalid-encoding")
    assert result == text


def test_safe_decode_with_windows_1252() -> None:
    text = "café résumé"
    windows_bytes = text.encode("windows-1252")
    result = safe_decode(windows_bytes)
    assert result == text


def test_safe_decode_with_hebrew() -> None:
    hebrew_text = "שלום עולם"
    hebrew_bytes = hebrew_text.encode("windows-1255")
    result = safe_decode(hebrew_bytes)
    assert "שלום" in result or "עולם" in result


def test_safe_decode_cache_functionality() -> None:
    text = "Hello, repeated content!"
    utf8_bytes = text.encode("utf-8")

    result1 = safe_decode(utf8_bytes)
    assert result1 == text

    result2 = safe_decode(utf8_bytes)
    assert result2 == text
    assert result1 == result2


def test_safe_decode_fallback_to_latin1() -> None:
    problematic_bytes = b"\xff\xfe\x00\x00\x01\x02\x03"
    result = safe_decode(problematic_bytes)
    assert isinstance(result, str)


def test_safe_decode_confidence_scoring() -> None:
    good_text = "This is normal English text with good characters."
    utf8_bytes = good_text.encode("utf-8")
    result = safe_decode(utf8_bytes)
    assert result == good_text


def test_calculate_text_confidence_empty() -> None:
    assert calculate_text_confidence("") == 0.0


def test_calculate_text_confidence_normal_text() -> None:
    text = "This is normal English text."
    confidence = calculate_text_confidence(text)
    assert confidence > 0.8


def test_calculate_text_confidence_with_replacement_chars() -> None:
    text = "Text with replacement \ufffd characters"
    confidence = calculate_text_confidence(text)
    assert confidence < 1.0


def test_calculate_text_confidence_with_control_chars() -> None:
    text = "Text with\x00control\x01chars"
    confidence = calculate_text_confidence(text)
    assert confidence < 0.8


def test_calculate_text_confidence_cyrillic_penalty() -> None:
    suspicious_text = "аваыврдвфгхькол" * 5
    confidence = calculate_text_confidence(suspicious_text)
    assert confidence <= 0.7


def test_fix_mojibake_empty() -> None:
    assert fix_mojibake("") == ""


def test_fix_mojibake_control_chars() -> None:
    text = "Text with\x00control\x01chars\x7f"
    cleaned = fix_mojibake(text)
    assert "\x00" not in cleaned
    assert "\x01" not in cleaned
    assert "\x7f" not in cleaned
    assert cleaned == "Text withcontrolchars"


def test_fix_mojibake_replacement_chars() -> None:
    text = "Text with\ufffd\ufffdreplacement chars"
    cleaned = fix_mojibake(text)
    assert "\ufffd" not in cleaned
    assert cleaned == "Text withreplacement chars"


def test_fix_mojibake_isolated_combining() -> None:
    text = "Text with\u0300\u0301isolated combining"
    cleaned = fix_mojibake(text)
    assert "\u0300" not in cleaned
    assert len(cleaned) < len(text)


def test_fix_mojibake_cyrillic_detection() -> None:
    cyrillic_text = "аваыврдвфгхькол"
    result = fix_mojibake(cyrillic_text)
    assert len(result) > 0


def test_normalize_spaces_empty() -> None:
    assert normalize_spaces("") == ""
    assert normalize_spaces("   ") == ""
    assert normalize_spaces("\n\n") == ""


def test_normalize_spaces_basic() -> None:
    text = "This   is    some  text"
    expected = "This is some text"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_preserve_paragraphs() -> None:
    text = "First paragraph.\n\nSecond paragraph."
    expected = "First paragraph.\n\nSecond paragraph."
    assert normalize_spaces(text) == expected


def test_normalize_spaces_multiple_whitespace() -> None:
    text = "Text\twith\fvarious\v\rwhitespace\xa0types"
    expected = "Text with various whitespace types"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_preserve_single_newlines() -> None:
    text = "Line 1\nLine 2\nLine 3"
    expected = "Line 1\nLine 2\nLine 3"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_clean_multiple_newlines() -> None:
    text = "Line 1\n\n\nLine 2"
    expected = "Line 1\n\nLine 2"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_remove_empty_lines() -> None:
    text = "Good line\n   \nAnother good line"
    expected = "Good line\nAnother good line"
    assert normalize_spaces(text) == expected


def test_normalize_spaces_complex_example() -> None:
    text = """
    First   paragraph with    extra spaces.



    Second paragraph\t\twith\ttabs.

    Third\n\n\nparagraph  with  newlines.
    """
    result = normalize_spaces(text)

    assert "First paragraph with extra spaces." in result
    assert "Second paragraph with tabs." in result
    assert "Third" in result
    assert "paragraph with newlines." in result

    paragraphs = result.split("\n\n")
    assert len(paragraphs) >= 2

    assert "   " not in result
    assert "\t\t" not in result


def test_get_encoding_cache_key() -> None:
    hash1 = "abcdef123456"
    size1 = 1024
    key1 = get_encoding_cache_key(hash1, size1)
    key2 = get_encoding_cache_key(hash1, size1)

    assert key1 == key2
    assert hash1 in key1
    assert str(size1) in key1


def test_get_encoding_cache_key_different_inputs() -> None:
    key1 = get_encoding_cache_key("hash1", 100)
    key2 = get_encoding_cache_key("hash2", 100)
    key3 = get_encoding_cache_key("hash1", 200)

    assert key1 != key2
    assert key1 != key3
    assert key2 != key3


def test_safe_decode_encoding_tries_fallback_encodings() -> None:
    text = "Simple ASCII text"
    ascii_bytes = text.encode("ascii")

    result = safe_decode(ascii_bytes)
    assert result == text


def test_safe_decode_caching_behavior() -> None:
    """Test that safe_decode produces consistent results for same input."""
    text = "Test caching functionality"
    utf8_bytes = text.encode("utf-8")

    results = [safe_decode(utf8_bytes) for _ in range(5)]
    assert all(r == text for r in results)

    text2 = "Different text"
    utf8_bytes2 = text2.encode("utf-8")
    result2 = safe_decode(utf8_bytes2)
    assert result2 == text2
    assert result2 != text


def test_safe_decode_handles_many_unique_inputs() -> None:
    """Test that safe_decode can handle many unique inputs without issues."""
    for i in range(100):
        unique_text = f"Unique text {i}"
        unique_bytes = unique_text.encode("utf-8")
        result = safe_decode(unique_bytes)
        assert result == unique_text
