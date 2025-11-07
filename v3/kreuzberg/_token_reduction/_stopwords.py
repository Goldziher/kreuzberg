from __future__ import annotations

from functools import lru_cache
from pathlib import Path

import msgspec

from kreuzberg._utils._ref import Ref

_STOPWORDS_DIR = Path(__file__).parent / "stopwords"


@lru_cache(maxsize=16)
def _load_language_stopwords(lang_code: str) -> set[str]:
    """Load stopwords for a specific language from its JSON file."""
    if not lang_code or "/" in lang_code or "\\" in lang_code or ".." in lang_code:
        return set()

    file_path = _STOPWORDS_DIR / f"{lang_code}_stopwords.json"

    try:
        file_path = file_path.resolve()
        if not file_path.parent.samefile(_STOPWORDS_DIR):
            return set()
    except (OSError, ValueError):
        return set()

    if not file_path.exists():
        return set()

    try:
        with file_path.open("rb") as f:
            words: list[str] = msgspec.json.decode(f.read())
        return set(words)
    except (OSError, msgspec.DecodeError):
        return set()


def _get_available_languages() -> frozenset[str]:
    """Get list of available stopword languages by scanning directory."""
    try:
        if not _STOPWORDS_DIR.exists():
            return frozenset()

        languages = set()
        for file_path in _STOPWORDS_DIR.glob("*_stopwords.json"):
            lang_code = file_path.stem.replace("_stopwords", "")
            languages.add(lang_code)

        return frozenset(languages)
    except (OSError, ValueError):
        return frozenset()


_available_languages_ref = Ref("available_languages", _get_available_languages)


class StopwordsManager:
    """Manages stopwords for multiple languages with lazy loading."""

    def __init__(
        self,
        custom_stopwords: dict[str, list[str]] | None = None,
    ) -> None:
        """Initialize with optional custom stopwords.

        Args:
            custom_stopwords: Additional stopwords per language.

        """
        self._custom_stopwords: dict[str, set[str]] = {}

        if custom_stopwords:
            self._custom_stopwords = {lang: set(words) for lang, words in custom_stopwords.items()}

    def get_stopwords(self, language: str) -> set[str]:
        """Get stopwords for a language, combining default and custom."""
        result = _load_language_stopwords(language)

        if language in self._custom_stopwords:
            result = result | self._custom_stopwords[language]

        return result

    def has_language(self, language: str) -> bool:
        """Check if stopwords are available for a language."""
        available = _available_languages_ref.get()
        return language in available or language in self._custom_stopwords

    def supported_languages(self) -> list[str]:
        """Get sorted list of all supported languages."""
        available = _available_languages_ref.get()
        all_langs = set(available)
        all_langs.update(self._custom_stopwords.keys())
        return sorted(all_langs)

    def add_custom_stopwords(self, language: str, words: list[str] | set[str]) -> None:
        """Add custom stopwords for a language."""
        if language not in self._custom_stopwords:
            self._custom_stopwords[language] = set()

        if isinstance(words, list):
            words = set(words)

        self._custom_stopwords[language].update(words)


def _create_default_manager() -> StopwordsManager:
    return StopwordsManager()


_default_manager_ref = Ref("default_stopwords_manager", _create_default_manager)


def get_default_stopwords_manager() -> StopwordsManager:
    """Get the default global stopwords manager."""
    return _default_manager_ref.get()
