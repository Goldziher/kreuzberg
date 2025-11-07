from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import Mock, patch

import pytest
from kreuzberg._language_detection import detect_languages
from kreuzberg._types import LanguageDetectionConfig
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from collections.abc import Generator


@pytest.fixture(autouse=True)
def clear_language_detection_cache() -> Generator[None, None, None]:
    detect_languages.cache_clear()
    yield
    detect_languages.cache_clear()


def test_detect_languages_when_library_missing() -> None:
    text = "This is some English text."

    with patch.dict("sys.modules", {"fast_langdetect": None}):
        with pytest.raises(MissingDependencyError) as exc_info:
            detect_languages(text)

        error = exc_info.value
        assert "fast-langdetect" in str(error)
        assert "language detection" in str(error)


def test_detect_languages_single_language_success() -> None:
    text = "This is some English text."
    mock_detect_result = [{"lang": "EN", "score": 0.99}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text)

    assert result == ["en"]
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_single_language_no_lang_key() -> None:
    text = "This is some text."
    mock_detect_result = [{"score": 0.50}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text)

    assert result is None
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_single_language_empty_lang() -> None:
    text = "This is some text."
    mock_detect_result = [{"lang": "", "score": 0.50}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text)

    assert result is None
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_single_language_none_result() -> None:
    text = "This is some text."
    mock_detect = Mock(return_value=None)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text)

    assert result is None
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_multilingual_success() -> None:
    text = "Hello world. Bonjour le monde."
    config = LanguageDetectionConfig(multilingual=True, top_k=3)

    mock_multilingual_results = [
        {"lang": "EN", "score": 0.8},
        {"lang": "FR", "score": 0.7},
        {"lang": "ES", "score": 0.1},
    ]

    mock_detect = Mock(return_value=mock_multilingual_results)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en", "fr", "es"]
    mock_detect.assert_called_once_with(text, model="auto", k=3)


def test_detect_languages_multilingual_with_top_k() -> None:
    text = "Hello world. Bonjour le monde."
    config = LanguageDetectionConfig(multilingual=True, top_k=2)

    mock_multilingual_results = [{"lang": "EN", "score": 0.8}, {"lang": "FR", "score": 0.7}]

    mock_detect = Mock(return_value=mock_multilingual_results)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en", "fr"]
    mock_detect.assert_called_once_with(text, model="auto", k=2)


def test_detect_languages_multilingual_results_missing_lang() -> None:
    text = "Mixed language text."
    config = LanguageDetectionConfig(multilingual=True)

    mock_multilingual_results = [
        {"lang": "EN", "score": 0.8},
        {"score": 0.6},
        {"lang": "", "score": 0.4},
        {"lang": "FR", "score": 0.3},
    ]

    mock_detect = Mock(return_value=mock_multilingual_results)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en", "fr"]


def test_detect_languages_with_default_config() -> None:
    text = "This is English text."
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config=None)

    assert result == ["en"]
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_single_language_with_config() -> None:
    text = "This is English text."
    config = LanguageDetectionConfig(multilingual=False)
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en"]
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_exception_handling() -> None:
    text = "This is some text."

    mock_detect = Mock(side_effect=RuntimeError("Detection failed"))

    with patch("fast_langdetect.detect", mock_detect), pytest.raises(RuntimeError):
        detect_languages(text)


def test_detect_languages_multilingual_exception_handling() -> None:
    text = "Mixed language text."
    config = LanguageDetectionConfig(multilingual=True)

    mock_detect = Mock(side_effect=ValueError("Multilingual detection failed"))

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result is None
    mock_detect.assert_called_once_with(text, model="auto", k=3)


def test_detect_languages_caching_behavior() -> None:
    text = "This is English text."
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        config = LanguageDetectionConfig()
        result1 = detect_languages(text, config)
        result2 = detect_languages(text, config)

    assert result1 == ["en"]
    assert result2 == ["en"]
    mock_detect.assert_called_once_with(text, model="auto", k=1)


def test_detect_languages_cache_different_configs() -> None:
    text = "This is English text."
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        config1 = LanguageDetectionConfig(multilingual=False)
        config2 = LanguageDetectionConfig(multilingual=True, top_k=2)

        result1 = detect_languages(text, config1)
        result2 = detect_languages(text, config2)

    assert result1 == ["en"]
    assert result2 == ["en"]
    assert mock_detect.call_count == 2
    mock_detect.assert_any_call(text, model="auto", k=1)
    mock_detect.assert_any_call(text, model="auto", k=2)


def test_detect_languages_real_single_language() -> None:
    text = "This is definitely an English text with multiple sentences. It should be detected as English."
    result = detect_languages(text)

    assert result is not None
    assert len(result) == 1
    assert result[0] == "en"


def test_detect_languages_real_multilingual() -> None:
    text = "Hello world. Bonjour le monde. Hola mundo. Ciao mondo."
    config = LanguageDetectionConfig(multilingual=True, top_k=4)
    result = detect_languages(text, config)

    assert result is not None
    assert len(result) >= 1
    assert all(isinstance(lang, str) for lang in result)
    assert all(len(lang) == 2 for lang in result)


def test_detect_languages_real_empty_text() -> None:
    text = ""
    result = detect_languages(text)

    assert result is None or (isinstance(result, list) and len(result) <= 1)


def test_detect_languages_real_with_config() -> None:
    text = "This is an English sentence."
    config = LanguageDetectionConfig(multilingual=False)
    result = detect_languages(text, config)

    assert result is not None
    assert len(result) == 1
    assert result[0] == "en"


def test_detect_languages_real_french_text() -> None:
    text = "Ceci est un texte en français. Il devrait être détecté comme français."
    result = detect_languages(text)

    assert result is not None
    assert len(result) == 1
    assert isinstance(result[0], str)
    assert len(result[0]) == 2


def test_detect_languages_real_german_text() -> None:
    text = "Dies ist ein deutscher Text. Es sollte als Deutsch erkannt werden."
    result = detect_languages(text)

    assert result is not None
    assert len(result) == 1
    assert isinstance(result[0], str)
    assert len(result[0]) == 2


def test_detect_languages_real_spanish_text() -> None:
    text = "Este es un texto en español. Debería ser detectado como español."
    result = detect_languages(text)

    assert result is not None
    assert len(result) == 1
    assert isinstance(result[0], str)
    assert len(result[0]) == 2


def test_detect_languages_real_mixed_languages_with_top_k() -> None:
    text = "English text. Texte français. Deutscher Text. Texto español."
    config = LanguageDetectionConfig(multilingual=True, top_k=2)
    result = detect_languages(text, config)

    assert result is not None
    assert 1 <= len(result) <= 2
    assert all(isinstance(lang, str) for lang in result)
    assert all(len(lang) == 2 for lang in result)


def test_detect_languages_with_lite_model() -> None:
    text = "This is English text."
    config = LanguageDetectionConfig(model="lite")
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en"]
    mock_detect.assert_called_once_with(text, model="lite", k=1)


def test_detect_languages_with_full_model() -> None:
    text = "This is English text."
    config = LanguageDetectionConfig(model="full")
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en"]
    mock_detect.assert_called_once_with(text, model="full", k=1)


def test_detect_languages_with_auto_model() -> None:
    text = "This is English text."
    config = LanguageDetectionConfig(model="auto")
    mock_detect_result = [{"lang": "EN", "score": 0.95}]

    mock_detect = Mock(return_value=mock_detect_result)

    with patch("fast_langdetect.detect", mock_detect):
        result = detect_languages(text, config)

    assert result == ["en"]
    mock_detect.assert_called_once_with(text, model="auto", k=1)
