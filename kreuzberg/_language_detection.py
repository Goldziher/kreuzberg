from typing import List, Optional
from functools import lru_cache
from kreuzberg.exceptions import MissingDependencyError

try:
    from fast_langdetect import detect_langs
except ImportError as e:
    detect_langs = None

_CACHE_SIZE = 128

@lru_cache(maxsize=_CACHE_SIZE)
def detect_languages(text: str, top_k: int = 3) -> Optional[List[str]]:
    """Detect the most probable languages in the given text using fast-langdetect.

    Args:
        text: The text to analyze.
        top_k: The maximum number of languages to return.

    Returns:
        A list of detected language codes, or None if detection fails.
    """
    if detect_langs is None:
        raise MissingDependencyError(
            "fast-langdetect is required for language detection. Install with: pip install 'kreuzberg[language-detection]'"
        )
    try:
        results = detect_langs(text)
        return [lang.lang for lang in results[:top_k]]
    except Exception:
        return None 