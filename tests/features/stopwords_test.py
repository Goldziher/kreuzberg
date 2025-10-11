from __future__ import annotations

from typing import TYPE_CHECKING

from kreuzberg._token_reduction._stopwords import (
    StopwordsManager,
    _get_available_languages,
    _load_language_stopwords,
    get_default_stopwords_manager,
)

if TYPE_CHECKING:
    from pathlib import Path

    import pytest


def test_load_language_stopwords_valid() -> None:
    stopwords = _load_language_stopwords("en")
    assert isinstance(stopwords, set)
    assert len(stopwords) > 0
    assert "the" in stopwords or "a" in stopwords


def test_load_language_stopwords_invalid_language() -> None:
    stopwords = _load_language_stopwords("invalid_lang_xyz")
    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_load_language_stopwords_path_traversal() -> None:
    stopwords = _load_language_stopwords("../../../etc/passwd")
    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_load_language_stopwords_with_slash() -> None:
    stopwords = _load_language_stopwords("en/../../etc/passwd")
    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_load_language_stopwords_with_backslash() -> None:
    stopwords = _load_language_stopwords("en\\..\\..\\etc\\passwd")
    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_load_language_stopwords_with_dotdot() -> None:
    stopwords = _load_language_stopwords("..en")
    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_load_language_stopwords_empty_string() -> None:
    stopwords = _load_language_stopwords("")
    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_load_language_stopwords_cached() -> None:
    stopwords1 = _load_language_stopwords("en")
    stopwords2 = _load_language_stopwords("en")
    assert stopwords1 is stopwords2


def test_load_language_stopwords_invalid_file(tmp_path: Path) -> None:
    from kreuzberg._token_reduction import _stopwords

    original_dir = _stopwords._STOPWORDS_DIR
    try:
        _stopwords._STOPWORDS_DIR = tmp_path
        invalid_file = tmp_path / "test_stopwords.json"
        invalid_file.write_text("invalid json {")

        _stopwords._load_language_stopwords.cache_clear()
        stopwords = _load_language_stopwords("test")
        assert isinstance(stopwords, set)
        assert len(stopwords) == 0
    finally:
        _stopwords._STOPWORDS_DIR = original_dir
        _stopwords._load_language_stopwords.cache_clear()


def test_get_available_languages() -> None:
    languages = _get_available_languages()
    assert isinstance(languages, frozenset)
    assert len(languages) > 0
    assert "en" in languages or "de" in languages


def test_get_available_languages_missing_dir(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    from kreuzberg._token_reduction import _stopwords

    missing_dir = tmp_path / "nonexistent"
    original_dir = _stopwords._STOPWORDS_DIR
    try:
        monkeypatch.setattr(_stopwords, "_STOPWORDS_DIR", missing_dir)
        languages = _get_available_languages()
        assert isinstance(languages, frozenset)
        assert len(languages) == 0
    finally:
        monkeypatch.setattr(_stopwords, "_STOPWORDS_DIR", original_dir)


def test_stopwords_manager_init_no_custom() -> None:
    manager = StopwordsManager()
    assert manager._custom_stopwords == {}


def test_stopwords_manager_init_with_custom() -> None:
    custom = {"en": ["foo", "bar"], "de": ["baz", "qux"]}
    manager = StopwordsManager(custom_stopwords=custom)
    assert "en" in manager._custom_stopwords
    assert "de" in manager._custom_stopwords
    assert "foo" in manager._custom_stopwords["en"]
    assert "bar" in manager._custom_stopwords["en"]


def test_stopwords_manager_get_stopwords_default() -> None:
    manager = StopwordsManager()
    stopwords = manager.get_stopwords("en")
    assert isinstance(stopwords, set)
    assert len(stopwords) > 0


def test_stopwords_manager_get_stopwords_with_custom() -> None:
    custom = {"en": ["custom1", "custom2"]}
    manager = StopwordsManager(custom_stopwords=custom)
    stopwords = manager.get_stopwords("en")
    assert "custom1" in stopwords
    assert "custom2" in stopwords


def test_stopwords_manager_get_stopwords_custom_only() -> None:
    custom = {"custom_lang": ["word1", "word2"]}
    manager = StopwordsManager(custom_stopwords=custom)
    stopwords = manager.get_stopwords("custom_lang")
    assert "word1" in stopwords
    assert "word2" in stopwords


def test_stopwords_manager_has_language_default() -> None:
    manager = StopwordsManager()
    assert manager.has_language("en")


def test_stopwords_manager_has_language_custom() -> None:
    custom = {"custom_lang": ["word1", "word2"]}
    manager = StopwordsManager(custom_stopwords=custom)
    assert manager.has_language("custom_lang")


def test_stopwords_manager_has_language_missing() -> None:
    manager = StopwordsManager()
    assert not manager.has_language("nonexistent_lang_xyz")


def test_stopwords_manager_supported_languages() -> None:
    custom = {"custom_lang": ["word1"], "en": ["custom_en"]}
    manager = StopwordsManager(custom_stopwords=custom)
    languages = manager.supported_languages()
    assert isinstance(languages, list)
    assert languages == sorted(languages)
    assert "custom_lang" in languages
    assert "en" in languages


def test_stopwords_manager_add_custom_stopwords_new_language() -> None:
    manager = StopwordsManager()
    manager.add_custom_stopwords("test_lang", ["word1", "word2"])
    assert "test_lang" in manager._custom_stopwords
    assert "word1" in manager._custom_stopwords["test_lang"]
    assert "word2" in manager._custom_stopwords["test_lang"]


def test_stopwords_manager_add_custom_stopwords_existing_language() -> None:
    manager = StopwordsManager(custom_stopwords={"en": ["existing"]})
    manager.add_custom_stopwords("en", ["new1", "new2"])
    assert "existing" in manager._custom_stopwords["en"]
    assert "new1" in manager._custom_stopwords["en"]
    assert "new2" in manager._custom_stopwords["en"]


def test_stopwords_manager_add_custom_stopwords_set() -> None:
    manager = StopwordsManager()
    manager.add_custom_stopwords("test_lang", {"word1", "word2"})
    assert "word1" in manager._custom_stopwords["test_lang"]
    assert "word2" in manager._custom_stopwords["test_lang"]


def test_get_default_stopwords_manager() -> None:
    manager1 = get_default_stopwords_manager()
    manager2 = get_default_stopwords_manager()
    assert manager1 is manager2


def test_default_manager_has_languages() -> None:
    manager = get_default_stopwords_manager()
    languages = manager.supported_languages()
    assert len(languages) > 0


def test_stopwords_manager_integration() -> None:
    manager = StopwordsManager(custom_stopwords={"en": ["custom_word"]})

    en_stopwords = manager.get_stopwords("en")
    assert len(en_stopwords) > 0
    assert "custom_word" in en_stopwords

    manager.add_custom_stopwords("de", ["deutsches_wort"])
    assert manager.has_language("de")

    de_stopwords = manager.get_stopwords("de")
    assert "deutsches_wort" in de_stopwords

    languages = manager.supported_languages()
    assert "en" in languages
    assert "de" in languages
