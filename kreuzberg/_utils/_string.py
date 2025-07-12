from __future__ import annotations

from contextlib import suppress

import chardetng_py


def safe_decode(byte_data: bytes, encoding: str | None = None) -> str:
    """Decode a byte string safely, removing invalid sequences.

    Args:
        byte_data: The byte string to decode.
        encoding: The encoding to use when decoding the byte string.

    Returns:
        The decoded string.
    """
    if not byte_data:
        return ""

    # Use chardetng for detection - it's more accurate for web content
    detected_encoding = chardetng_py.detect(byte_data)

    encodings = [encoding, detected_encoding, "utf-8"]

    for enc in [e for e in encodings if e]:
        with suppress(UnicodeDecodeError, LookupError):
            return byte_data.decode(enc)

    return byte_data.decode("latin-1", errors="replace")


def normalize_spaces(text: str) -> str:
    """Normalize the spaces in a string while preserving meaningful structure.

    This function:
    - Preserves line breaks and paragraph structure
    - Removes excessive whitespace within lines
    - Cleans up multiple consecutive empty lines
    - Trims whitespace from line ends

    Args:
        text: The text to sanitize.

    Returns:
        The sanitized text with improved formatting.
    """
    import re

    if not text:
        return text

    # Split into lines and process each line
    lines = text.split("\n")
    processed_lines = []

    for line in lines:
        # Normalize spaces within each line (collapse multiple spaces/tabs to single space)
        # Convert non-breaking spaces and other Unicode spaces to regular spaces
        # but don't remove the line entirely if it's just whitespace
        normalized_line = re.sub(r"[ \t\u00A0\u2000-\u200B\u2028\u2029\u202F\u205F\u3000]+", " ", line.strip())
        processed_lines.append(normalized_line)

    # Join lines back together
    result = "\n".join(processed_lines)

    # Clean up excessive empty lines (more than 2 consecutive empty lines)
    result = re.sub(r"\n{4,}", "\n\n\n", result)

    # Clean up trailing/leading whitespace from the entire text
    return result.strip()
