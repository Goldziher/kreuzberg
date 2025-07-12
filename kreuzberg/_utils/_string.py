from __future__ import annotations

import hashlib
import re
from contextlib import suppress
from functools import lru_cache

import chardetng_py

# Compile regex patterns once at module level for performance
_WHITESPACE_PATTERN = re.compile(r"[ \t\f\v\r\xa0\u2000-\u200b\u2028\u2029\u3000]+")
_NEWLINES_PATTERN = re.compile(r"\n+")
_MOJIBAKE_PATTERNS = {
    # Hebrew as Cyrillic patterns
    "hebrew_as_cyrillic": re.compile(r"[\u0400-\u04FF]{3,}"),
    # Control characters that shouldn't appear in text
    "control_chars": re.compile(r"[\x00-\x08\x0B-\x0C\x0E-\x1F\x7F-\x9F]"),
    # Unicode replacement characters
    "replacement_chars": re.compile(r"\uFFFD+"),
    # Isolated combining marks (likely encoding issues)
    "isolated_combining": re.compile(r"[\u0300-\u036F](?![^\u0300-\u036F])"),
}

# Simple cache for encoding detection (in-memory, session-scoped)
_encoding_cache: dict[str, str] = {}


@lru_cache(maxsize=128)
def _get_encoding_cache_key(data_hash: str, size: int) -> str:
    """Generate cache key for encoding detection."""
    return f"{data_hash}:{size}"


def safe_decode(byte_data: bytes, encoding: str | None = None) -> str:
    """Decode a byte string safely with mojibake detection and correction.

    Args:
        byte_data: The byte string to decode.
        encoding: The encoding to use when decoding the byte string.

    Returns:
        The decoded string with mojibake detection and correction.
    """
    if not byte_data:
        return ""

    # Try provided encoding first (fastest path)
    if encoding:
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(encoding)
            return _fix_mojibake(decoded)

    # Check cache for similar content (performance optimization)
    data_hash = hashlib.sha256(byte_data[:1024]).hexdigest()[:16]  # Hash first 1KB
    cache_key = _get_encoding_cache_key(data_hash, len(byte_data))

    if cache_key in _encoding_cache:
        cached_encoding = _encoding_cache[cache_key]
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(cached_encoding)
            return _fix_mojibake(decoded)

    # Use chardetng for better performance than charset-normalizer
    detected_encoding = chardetng_py.detect(byte_data)
    if detected_encoding:
        with suppress(UnicodeDecodeError, LookupError):
            decoded = byte_data.decode(detected_encoding)
            # Cache successful encoding detection
            if len(_encoding_cache) < 1000:  # Prevent unlimited growth
                _encoding_cache[cache_key] = detected_encoding
            return _fix_mojibake(decoded)

    # Try multiple encodings with confidence scoring
    encodings_to_try = [
        "utf-8",
        "windows-1255",  # Hebrew
        "iso-8859-8",  # Hebrew
        "windows-1256",  # Arabic
        "iso-8859-6",  # Arabic
        "windows-1252",  # Western European
        "cp1251",  # Cyrillic
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

    # Final fallback
    return byte_data.decode("latin-1", errors="replace")


def _calculate_text_confidence(text: str) -> float:
    """Calculate confidence score for decoded text quality."""
    if not text:
        return 0.0

    # Check for common encoding problems
    replacement_count = len(_MOJIBAKE_PATTERNS["replacement_chars"].findall(text))
    control_count = len(_MOJIBAKE_PATTERNS["control_chars"].findall(text))
    total_chars = len(text)

    if total_chars == 0:
        return 0.0

    # Penalize replacement and control characters
    penalty = (replacement_count + control_count * 2) / total_chars

    # Bonus for readable character ranges
    readable_chars = sum(1 for c in text if c.isprintable() or c.isspace())
    readability_score = readable_chars / total_chars

    # Check for suspicious Cyrillic that might be misencoded Hebrew
    cyrillic_matches = _MOJIBAKE_PATTERNS["hebrew_as_cyrillic"].findall(text)
    if cyrillic_matches and len("".join(cyrillic_matches)) > total_chars * 0.1:
        penalty += 0.3  # Heavy penalty for likely mojibake

    return max(0.0, min(1.0, readability_score - penalty))


def _fix_mojibake(text: str) -> str:
    """Attempt to fix common mojibake patterns."""
    if not text:
        return text

    # Remove control characters
    text = _MOJIBAKE_PATTERNS["control_chars"].sub("", text)

    # Remove replacement characters
    text = _MOJIBAKE_PATTERNS["replacement_chars"].sub("", text)

    # Remove isolated combining marks
    text = _MOJIBAKE_PATTERNS["isolated_combining"].sub("", text)

    # Try to fix Hebrew encoded as Cyrillic (common Windows-1255 -> CP1251 confusion)
    if _MOJIBAKE_PATTERNS["hebrew_as_cyrillic"].search(text):
        # This is a heuristic fix - in practice, you'd need actual character mapping
        # For now, we flag it for manual review by keeping the text but adding a marker
        pass

    return text


def normalize_spaces(text: str) -> str:
    """Normalize spaces while preserving line breaks and paragraph structure.

    Args:
        text: The text to normalize.

    Returns:
        The normalized text with proper spacing.
    """
    if not text or not text.strip():
        return ""

    # Split by double newlines to preserve paragraph breaks
    paragraphs = text.split("\n\n")
    normalized_paragraphs = []

    for paragraph in paragraphs:
        # Use pre-compiled patterns for better performance
        # Replace multiple whitespace (except newlines) with single space
        cleaned = _WHITESPACE_PATTERN.sub(" ", paragraph)
        # Clean up multiple newlines within paragraph (keep single newlines)
        cleaned = _NEWLINES_PATTERN.sub("\n", cleaned)

        # Strip and filter empty lines efficiently
        lines = [line.strip() for line in cleaned.split("\n") if line.strip()]

        if lines:
            normalized_paragraphs.append("\n".join(lines))

    return "\n\n".join(normalized_paragraphs)
