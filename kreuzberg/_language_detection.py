"""
Language detection utilities for Kreuzberg.
"""
from typing import Optional, List
from functools import lru_cache

from kreuzberg.exceptions import MissingDependencyError

try:
    from fast_langdetect import detect_langs
except ImportError as e:
    detect_langs = None


def _require_fast_langdetect():
    if detect_langs is None:
        raise MissingDependencyError(
            "fast-langdetect is required for language detection. Install with: pip install 'kreuzberg[language-detection]'"
        )


@lru_cache(maxsize=128)
def detect_languages(text: str, top_n: int = 3) -> List[str]:
    """
    Detects the most probable languages in the given text using fast-langdetect.
    Returns a list of language codes (e.g., ['en', 'de']).
    """
    _require_fast_langdetect()
    try:
        results = detect_langs(text)
        return [r.lang for r in results[:top_n]]
    except Exception:
        return []


def is_language_detection_available() -> bool:
    """Returns True if fast-langdetect is available, False otherwise."""
    return detect_langs is not None 