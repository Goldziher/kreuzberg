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
    """Normalize the spaces in a string.

    Args:
        text: The text to sanitize.

    Returns:
        The sanitized text.
    """
    return " ".join(text.strip().split())
