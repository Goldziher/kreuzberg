from __future__ import annotations

from contextlib import suppress

from charset_normalizer import detect


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

    encodings = [encoding, detect(byte_data).get("encoding", ""), "utf-8"]

    for enc in [e for e in encodings if e]:  # pragma: no cover
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
