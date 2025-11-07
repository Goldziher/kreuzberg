from __future__ import annotations

import hashlib
import re
from contextlib import suppress
from functools import lru_cache

import chardetng_py

_WHITESPACE_PATTERN = re.compile(r"[ \t\f\v\r\xa0\u2000-\u200b\u2028\u2029\u3000]+")
_NEWLINES_PATTERN = re.compile(r"\n+")
_MOJIBAKE_PATTERNS = {
    "hebrew_as_cyrillic": re.compile(r"[\u0400-\u04FF]{3,}"),
    "control_chars": re.compile(r"[\x00-\x08\x0B-\x0C\x0E-\x1F\x7F-\x9F]"),
    "replacement_chars": re.compile(r"\uFFFD+"),
    "isolated_combining": re.compile(r"[\u0300-\u036F](?![^\u0300-\u036F])"),
}

_encoding_cache: dict[str, str] = {}


@lru_cache(maxsize=128)
def _get_encoding_cache_key(data_hash: str, size: int) -> str:
    return f"{data_hash}:{size}"


def safe_decode(byte_data: bytes, encoding: str | None = None) -> str:
    if not byte_data:
        return ""

    if encoding:
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(encoding)
            return _fix_mojibake(decoded)

    data_hash = hashlib.sha256(byte_data[:1024]).hexdigest()[:16]
    cache_key = _get_encoding_cache_key(data_hash, len(byte_data))

    if cache_key in _encoding_cache:
        cached_encoding = _encoding_cache[cache_key]
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(cached_encoding)
            return _fix_mojibake(decoded)

    detected_encoding = chardetng_py.detect(byte_data)
    if detected_encoding:
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(detected_encoding)
            if len(_encoding_cache) < 1000:  # Prevent unlimited growth ~keep
                _encoding_cache[cache_key] = detected_encoding
            return _fix_mojibake(decoded)

    encodings_to_try = [
        "utf-8",
        "windows-1255",  # Hebrew ~keep
        "iso-8859-8",  # Hebrew ~keep
        "windows-1256",  # Arabic ~keep
        "iso-8859-6",  # Arabic ~keep
        "windows-1252",  # Western European ~keep
        "cp1251",  # Cyrillic ~keep
    ]

    best_result = None
    best_confidence = 0.0

    for enc in encodings_to_try:
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(enc)
            confidence = _calculate_text_confidence(decoded)
            if confidence > best_confidence:
                best_confidence = confidence
                best_result = decoded

    if best_result and best_confidence > 0.5:
        return _fix_mojibake(best_result)

    return byte_data.decode("latin-1", errors="replace")


def _calculate_text_confidence(text: str) -> float:
    if not text:
        return 0.0

    total_chars = len(text)
    if total_chars == 0:
        return 0.0

    replacement_count = len(_MOJIBAKE_PATTERNS["replacement_chars"].findall(text))
    control_count = len(_MOJIBAKE_PATTERNS["control_chars"].findall(text))

    penalty = (replacement_count + control_count * 2) / total_chars

    readable_chars = sum(1 for c in text if c.isprintable() or c.isspace())
    readability_score = readable_chars / total_chars

    cyrillic_matches = _MOJIBAKE_PATTERNS["hebrew_as_cyrillic"].findall(text)
    if cyrillic_matches:
        cyrillic_length = sum(len(match) for match in cyrillic_matches)
        if cyrillic_length > total_chars * 0.1:
            penalty += 0.3

    return max(0.0, min(1.0, readability_score - penalty))


def _fix_mojibake(text: str) -> str:
    if not text:
        return text

    text = _MOJIBAKE_PATTERNS["control_chars"].sub("", text)

    text = _MOJIBAKE_PATTERNS["replacement_chars"].sub("", text)

    text = _MOJIBAKE_PATTERNS["isolated_combining"].sub("", text)

    if _MOJIBAKE_PATTERNS["hebrew_as_cyrillic"].search(text):
        pass

    return text


def normalize_spaces(text: str) -> str:
    if not text or not text.strip():
        return ""

    paragraphs = text.split("\n\n")

    result_paragraphs = []

    for paragraph in paragraphs:
        cleaned = _WHITESPACE_PATTERN.sub(" ", paragraph)
        cleaned = _NEWLINES_PATTERN.sub("\n", cleaned)

        lines = []
        for line in cleaned.split("\n"):
            stripped_line = line.strip()
            if stripped_line:
                lines.append(stripped_line)

        if lines:
            result_paragraphs.append("\n".join(lines))

    return "\n\n".join(result_paragraphs)
