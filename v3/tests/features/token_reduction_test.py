from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from kreuzberg._token_reduction import StopwordsManager, get_reduction_stats, reduce_tokens
from kreuzberg._types import TokenReductionConfig
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from pathlib import Path


def test_reduce_tokens_off_mode_returns_original_text() -> None:
    config = TokenReductionConfig(mode="off")
    text = "This is a test with some stopwords and extra    spaces."

    result = reduce_tokens(text, config=config)

    assert result == text


def test_reduce_tokens_light_mode_normalizes_whitespace() -> None:
    config = TokenReductionConfig(mode="light", preserve_markdown=False)
    text = "This   has    multiple     spaces."

    result = reduce_tokens(text, config=config)

    assert result == "This has multiple spaces."


def test_reduce_tokens_light_mode_removes_html_comments() -> None:
    config = TokenReductionConfig(mode="light", preserve_markdown=False)
    text = "Text before <!-- comment --> text after."

    result = reduce_tokens(text, config=config)

    assert result == "Text before text after."


def test_reduce_tokens_light_mode_compresses_repeated_punctuation() -> None:
    config = TokenReductionConfig(mode="light", preserve_markdown=False)
    text = "Wait!!! What??? No way... Really,,,"

    result = reduce_tokens(text, config=config)

    assert result == "Wait! What? No way. Really,"


def test_reduce_tokens_light_mode_removes_excessive_newlines() -> None:
    config = TokenReductionConfig(mode="light", preserve_markdown=False)
    text = "Line 1\n\n\n\nLine 2\n\n\n\n\nLine 3"

    result = reduce_tokens(text, config=config)

    assert result == "Line 1\n\nLine 2\n\nLine 3"


def test_reduce_tokens_light_mode_preserves_markdown_when_enabled() -> None:
    config = TokenReductionConfig(mode="light", preserve_markdown=True)
    text = "# Header\n\nSome   text   with   spaces.\n\n```\ncode   block\n```"

    result = reduce_tokens(text, config=config)

    assert "# Header" in result
    assert "```\ncode   block\n```" in result
    assert "Some text with spaces." in result


def test_reduce_tokens_moderate_mode_removes_stopwords() -> None:
    config = TokenReductionConfig(mode="moderate", preserve_markdown=False)
    text = "The quick brown fox jumps over the lazy dog."

    result = reduce_tokens(text, config=config, language="en")

    assert "the" not in result.lower().split()
    assert "over" not in result.lower().split()
    assert "quick" in result.lower()
    assert "brown" in result.lower()
    assert "fox" in result.lower()


def test_reduce_tokens_moderate_mode_preserves_important_words() -> None:
    config = TokenReductionConfig(mode="moderate", preserve_markdown=False)
    text = "The API key is ABC123 and the URL is https://example.com"

    result = reduce_tokens(text, config=config, language="en")

    assert "API" in result
    assert "ABC123" in result
    assert "URL" in result


def test_reduce_tokens_moderate_mode_with_markdown_preservation() -> None:
    config = TokenReductionConfig(mode="moderate", preserve_markdown=True)
    text = """# Title

The quick brown fox jumps over the lazy dog.

- The first item
- The second item

```python
def the_function():
    pass
```"""

    result = reduce_tokens(text, config=config, language="en")

    assert "# Title" in result
    assert "- " in result
    assert "```python" in result
    assert "def the_function():" in result
    lines = result.split("\n")
    prose_line = next(line for line in lines if "fox" in line)
    assert "the" not in prose_line.lower().split()


def test_reduce_tokens_with_unsupported_language_falls_back_to_english() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "The quick brown fox jumps over the lazy dog."

    result = reduce_tokens(text, config=config, language="unsupported-lang")

    assert "the" not in result.lower().split()
    assert "quick" in result.lower()


def test_reduce_tokens_with_no_language_uses_english() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "The quick brown fox jumps over the lazy dog."

    result = reduce_tokens(text, config=config)

    assert "the" not in result.lower().split()
    assert "quick" in result.lower()


def test_get_reduction_stats_calculates_correctly() -> None:
    original = "The quick brown fox jumps over the lazy dog."
    reduced = "quick brown fox jumps lazy dog."

    stats = get_reduction_stats(original, reduced)

    assert stats["original_characters"] == len(original)
    assert stats["reduced_characters"] == len(reduced)
    assert stats["original_tokens"] == len(original.split())
    assert stats["reduced_tokens"] == len(reduced.split())
    assert stats["character_reduction_ratio"] > 0
    assert stats["token_reduction_ratio"] > 0


def test_get_reduction_stats_with_no_reduction() -> None:
    text = "Same text for both."

    stats = get_reduction_stats(text, text)

    assert stats["character_reduction_ratio"] == 0.0
    assert stats["token_reduction_ratio"] == 0.0
    assert stats["original_characters"] == stats["reduced_characters"]
    assert stats["original_tokens"] == stats["reduced_tokens"]


def test_get_reduction_stats_with_empty_original() -> None:
    stats = get_reduction_stats("", "")

    assert stats["character_reduction_ratio"] == 0.0
    assert stats["token_reduction_ratio"] == 0.0
    assert stats["original_characters"] == 0
    assert stats["reduced_characters"] == 0


def test_stopwords_manager_loads_english_stopwords() -> None:
    manager = StopwordsManager()

    stopwords = manager.get_stopwords("en")

    assert len(stopwords) > 0
    assert "the" in stopwords
    assert "and" in stopwords
    assert "is" in stopwords


def test_stopwords_manager_has_language_check() -> None:
    manager = StopwordsManager()

    assert manager.has_language("en") is True
    assert manager.has_language("nonexistent") is False


def test_stopwords_manager_supported_languages() -> None:
    manager = StopwordsManager()

    languages = manager.supported_languages()

    assert len(languages) > 0
    assert "en" in languages
    assert isinstance(languages, list)
    assert languages == sorted(languages)


def test_stopwords_manager_custom_stopwords() -> None:
    custom_stopwords = {"test": ["custom", "words"]}
    manager = StopwordsManager(custom_stopwords=custom_stopwords)

    stopwords = manager.get_stopwords("test")

    assert "custom" in stopwords
    assert "words" in stopwords


def test_reduce_tokens_empty_text_returns_empty() -> None:
    config = TokenReductionConfig(mode="light")

    result = reduce_tokens("", config=config)

    assert result == ""


def test_reduce_tokens_whitespace_only_text() -> None:
    config = TokenReductionConfig(mode="light")

    result = reduce_tokens("   \n\n   \t  ", config=config)

    assert result == ""


def test_reduce_tokens_moderate_mode_with_mixed_case() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "THE Quick BROWN fox"

    result = reduce_tokens(text, config=config, language="en")

    assert "Quick" in result
    assert "BROWN" in result
    assert "THE" in result.split()


def test_reduce_tokens_validation_raises_error_on_none_text() -> None:
    config = TokenReductionConfig(mode="light")

    with pytest.raises(ValidationError, match="Text cannot be None"):
        reduce_tokens(None, config=config)  # type: ignore[arg-type]


def test_reduce_tokens_validation_raises_error_on_none_config() -> None:
    with pytest.raises(ValidationError, match="Config cannot be None"):
        reduce_tokens("test", config=None)  # type: ignore[arg-type]


def test_reduce_tokens_validation_raises_error_on_invalid_text_type() -> None:
    config = TokenReductionConfig(mode="light")

    with pytest.raises(ValidationError, match="Text must be a string, got int"):
        reduce_tokens(123, config=config)  # type: ignore[arg-type]


def test_reduce_tokens_validation_raises_error_on_invalid_language_type() -> None:
    config = TokenReductionConfig(mode="moderate")

    with pytest.raises(ValidationError, match="Language must be a string or None, got int"):
        reduce_tokens("test", config=config, language=123)  # type: ignore[arg-type]


def test_reduce_tokens_validation_raises_error_on_empty_language() -> None:
    config = TokenReductionConfig(mode="moderate")

    with pytest.raises(ValidationError, match="Language cannot be empty or whitespace-only"):
        reduce_tokens("test", config=config, language="  ")


def test_reduce_tokens_handles_empty_text_gracefully() -> None:
    config = TokenReductionConfig(mode="light")

    result = reduce_tokens("", config=config)

    assert result == ""


def test_reduce_tokens_handles_whitespace_only_text_gracefully() -> None:
    config = TokenReductionConfig(mode="off")

    result = reduce_tokens("   \n\t  ", config=config)

    assert result == "   \n\t  "


def test_get_reduction_stats_validation_raises_error_on_none_original() -> None:
    with pytest.raises(ValidationError, match="Original text cannot be None"):
        get_reduction_stats(None, "test")  # type: ignore[arg-type]


def test_get_reduction_stats_validation_raises_error_on_none_reduced() -> None:
    with pytest.raises(ValidationError, match="Reduced text cannot be None"):
        get_reduction_stats("test", None)  # type: ignore[arg-type]


def test_get_reduction_stats_validation_raises_error_on_invalid_original_type() -> None:
    with pytest.raises(ValidationError, match="Original text must be a string, got int"):
        get_reduction_stats(123, "test")  # type: ignore[arg-type]


def test_get_reduction_stats_validation_raises_error_on_invalid_reduced_type() -> None:
    with pytest.raises(ValidationError, match="Reduced text must be a string, got list"):
        get_reduction_stats("test", ["test"])  # type: ignore[arg-type]


def test_reduce_tokens_handles_large_text_with_streaming() -> None:
    config = TokenReductionConfig(mode="light")
    large_text = "Hello world. " * 100_000

    result = reduce_tokens(large_text, config=config)
    assert isinstance(result, str)
    assert len(result) > 0
    assert "Hello world" in result


def test_reduce_tokens_security_raises_error_on_invalid_language_format() -> None:
    config = TokenReductionConfig(mode="moderate")

    with pytest.raises(ValidationError, match="Invalid language code format"):
        reduce_tokens("test", config=config, language="invalid/lang$code")


def test_reduce_tokens_security_accepts_valid_language_codes() -> None:
    config = TokenReductionConfig(mode="off")

    reduce_tokens("test", config=config, language="en")
    reduce_tokens("test", config=config, language="es")
    reduce_tokens("test", config=config, language="en-US")
    reduce_tokens("test", config=config, language="pt-BR")


def test_reduce_tokens_edge_case_unicode_characters() -> None:
    config = TokenReductionConfig(mode="light")
    text = "Hello 世界 🌍 café naïve résumé"

    result = reduce_tokens(text, config=config)

    assert "世界" in result
    assert "🌍" in result
    assert "café" in result


def test_reduce_tokens_edge_case_very_long_words() -> None:
    config = TokenReductionConfig(mode="moderate")
    long_word = "supercalifragilisticexpialidocious" * 10
    text = f"The {long_word} was amazing"

    result = reduce_tokens(text, config=config, language="en")

    assert long_word in result
    assert "the" not in result.lower()


def test_reduce_tokens_edge_case_numbers_and_symbols() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "The price is $123.45 and the date is 2024-01-01"

    result = reduce_tokens(text, config=config, language="en")

    assert "$123.45" in result
    assert "2024-01-01" in result
    assert "price" in result
    assert "the" not in result.lower()


def test_reduce_tokens_edge_case_mixed_languages() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "Hey mundo 世界 bonjour"

    result = reduce_tokens(text, config=config, language="en")

    assert "Hey" in result
    assert "mundo" in result
    assert "世界" in result
    assert "bonjour" in result


def test_reduce_tokens_edge_case_markdown_with_complex_nesting() -> None:
    config = TokenReductionConfig(mode="moderate", preserve_markdown=True)
    text = """# Header with the word the

This is a paragraph with the word the that should be reduced.

## Sub-header the

- Item the first
- Item the second
  - Nested item the third

```python
def the_function():
    # This the should be preserved
    return "the value"
```

| Column the | Another the |
|------------|-------------|
| Value the  | Data the    |
"""

    result = reduce_tokens(text, config=config, language="en")

    assert "# Header with the word the" in result
    assert "## Sub-header the" in result

    assert "def the_function():" in result
    assert "# This the should be preserved" in result

    assert "| Column the | Another the |" in result

    assert "- Item the first" in result

    lines = result.split("\n")
    prose_lines = [
        line
        for line in lines
        if line.strip() and not line.strip().startswith(("#", "-", "|", "```")) and "def " not in line
    ]
    for line in prose_lines:
        if "paragraph" in line.lower():
            assert "the" not in line.lower().split()


def test_reduce_tokens_edge_case_only_stopwords() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "the and or but"

    result = reduce_tokens(text, config=config, language="en")

    assert len(result.strip()) == 0 or result.strip() in ["", "and", "or", "but"]


def test_reduce_tokens_edge_case_repeated_punctuation_edge_cases() -> None:
    config = TokenReductionConfig(mode="light")
    text = "What?!?!?! No way.... Really,,, Wow!!! Amazing..."

    result = reduce_tokens(text, config=config)

    assert "?!?!?!" not in result
    assert "...." not in result
    assert ",,," not in result
    assert "!!!" not in result

    assert "What?" in result or "What!" in result
    assert "way." in result
    assert "Really," in result


def test_get_reduction_stats_edge_case_empty_strings() -> None:
    stats = get_reduction_stats("", "")

    assert stats["character_reduction_ratio"] == 0.0
    assert stats["token_reduction_ratio"] == 0.0
    assert stats["original_characters"] == 0
    assert stats["reduced_characters"] == 0
    assert stats["original_tokens"] == 0
    assert stats["reduced_tokens"] == 0


def test_get_reduction_stats_edge_case_expansion() -> None:
    original = "Hi"
    reduced = "Hello there"

    stats = get_reduction_stats(original, reduced)

    assert stats["character_reduction_ratio"] < 0
    assert stats["token_reduction_ratio"] < 0
    assert stats["original_characters"] == 2
    assert stats["reduced_characters"] == 11
    assert stats["original_tokens"] == 1
    assert stats["reduced_tokens"] == 2


def test_stopwords_manager_concurrent_access() -> None:
    import concurrent.futures

    manager = StopwordsManager()
    languages = ["en", "es", "fr", "de", "it"]

    def load_language(lang: str) -> int:
        stopwords = manager.get_stopwords(lang)
        return len(stopwords)

    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
        futures = [executor.submit(load_language, lang) for lang in languages * 3]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]

    assert all(r > 0 for r in results)
    assert len(results) == 15


def test_stopwords_manager_handles_corrupted_file(tmp_path: Path) -> None:
    from unittest.mock import patch

    corrupted_dir = tmp_path / "stopwords"
    corrupted_dir.mkdir()
    corrupted_file = corrupted_dir / "xx_stopwords.json"
    corrupted_file.write_text("not valid json {[}")

    with patch("kreuzberg._token_reduction._stopwords._STOPWORDS_DIR", corrupted_dir):
        manager = StopwordsManager()

        stopwords = manager.get_stopwords("xx")
        assert isinstance(stopwords, set)
        assert len(stopwords) == 0


def test_stopwords_manager_handles_missing_file() -> None:
    manager = StopwordsManager()

    stopwords = manager.get_stopwords("zz_nonexistent")

    assert isinstance(stopwords, set)
    assert len(stopwords) == 0


def test_reduce_tokens_thread_safety() -> None:
    import concurrent.futures

    config = TokenReductionConfig(mode="moderate")
    test_texts = [
        "The quick brown fox jumps over the lazy dog.",
        "This is another test sentence with stopwords.",
        "Multiple threads should handle this correctly.",
        "Each thread processes its own text independently.",
        "Thread safety is important for production use.",
    ]

    def process_text(text: str) -> str:
        return reduce_tokens(text, config=config, language="en")

    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
        futures = [executor.submit(process_text, text) for text in test_texts * 3]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]

    assert len(results) == 15
    assert all(isinstance(r, str) for r in results)
    assert all(len(r) > 0 for r in results)


def test_stopwords_manager_lru_cache_size() -> None:
    manager = StopwordsManager()

    languages = [
        "en",
        "es",
        "fr",
        "de",
        "it",
        "pt",
        "nl",
        "sv",
        "no",
        "da",
        "fi",
        "pl",
        "cs",
        "sk",
        "hu",
        "ro",
        "bg",
        "hr",
        "sr",
    ]

    for lang in languages:
        if manager.has_language(lang):
            manager.get_stopwords(lang)

    stopwords = manager.get_stopwords("en")
    assert "the" in stopwords
    assert len(stopwords) > 0


def test_reduce_tokens_with_custom_stopwords_thread_safety() -> None:
    import concurrent.futures

    config = TokenReductionConfig(
        mode="moderate",
        custom_stopwords={
            "en": ["customword", "special"],
            "es": ["personalizado", "especial"],
        },
    )

    test_cases = [
        ("Python customword programming has special features", "en"),
        ("Python personalizado programación tiene especial características", "es"),
    ]

    def process_with_custom(text: str, lang: str) -> str:
        return reduce_tokens(text, config=config, language=lang)

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(process_with_custom, text, lang) for text, lang in test_cases * 5]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]

    assert len(results) == 10

    for result in results:
        result_lower = result.lower()
        assert "customword" not in result_lower
        assert "special" not in result_lower
        assert "personalizado" not in result_lower
        assert "especial" not in result_lower

        assert "python" in result_lower
        assert (
            "programming" in result_lower
            or "programación" in result_lower
            or "features" in result_lower
            or "características" in result_lower
        )


def test_short_stopwords_are_removed() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "It is an excellent document of the organization"

    result = reduce_tokens(text, config=config, language="en")

    assert " is " not in f" {result} "
    assert " it " not in f" {result.lower()} "
    assert " an " not in f" {result.lower()} "
    assert " of " not in f" {result.lower()} "

    assert "excellent" in result
    assert "document" in result
    assert "organization" in result


def test_single_letter_words_preserved() -> None:
    config = TokenReductionConfig(mode="moderate")
    text = "I need a solution"

    result = reduce_tokens(text, config=config, language="en")

    assert "I" in result or "i" in result.lower()
    assert " a " in f" {result.lower()} " or result.lower().startswith("a ")

    assert "solution" in result


def test_markdown_table_detection_improved() -> None:
    config = TokenReductionConfig(mode="moderate", preserve_markdown=True)

    table_text = """
| Column 1 | Column 2 |
|----------|----------|
| Value 1  | Value 2  |
"""

    not_table = "This text has a pipe | but it's not a table"

    table_result = reduce_tokens(table_text, config=config, language="en")
    not_table_result = reduce_tokens(not_table, config=config, language="en")

    assert "| Column 1 | Column 2 |" in table_result
    assert "|----------|----------|" in table_result

    assert "pipe" in not_table_result
    assert "table" in not_table_result
    assert "but" not in not_table_result


def test_path_traversal_protection() -> None:
    manager = StopwordsManager()

    dangerous_codes = [
        "../../../etc/passwd",
        "..\\..\\windows\\system32",
        "en/../../../etc/passwd",
        "en/../../secret",
    ]

    for dangerous_code in dangerous_codes:
        stopwords = manager.get_stopwords(dangerous_code)
        assert stopwords == set()


def test_empty_result_handling() -> None:
    config = TokenReductionConfig(mode="moderate", custom_stopwords={"en": ["everything", "removed"]})
    text = "everything is removed"

    result = reduce_tokens(text, config=config, language="en")

    assert result == ""


def test_thread_safe_ref_initialization() -> None:
    import concurrent.futures
    import time

    from kreuzberg._utils._ref import Ref

    call_count = 0

    def slow_factory() -> str:
        nonlocal call_count
        call_count += 1
        time.sleep(0.01)
        return f"initialized_{call_count}"

    ref = Ref("test_ref", slow_factory)

    ref.clear()

    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
        futures = [executor.submit(ref.get) for _ in range(10)]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]

    assert call_count == 1
    assert all(r == "initialized_1" for r in results)

    ref.clear()


def test_language_normalization_in_config() -> None:
    from kreuzberg._types import TokenReductionConfig

    config = TokenReductionConfig(language_hint="eng")
    assert config.language_hint == "en"

    config = TokenReductionConfig(language_hint="fra")
    assert config.language_hint == "fr"

    config = TokenReductionConfig(language_hint="deu")
    assert config.language_hint == "de"

    config = TokenReductionConfig(language_hint="en-US")
    assert config.language_hint == "en"

    config = TokenReductionConfig(language_hint="en_GB")
    assert config.language_hint == "en"

    config = TokenReductionConfig(language_hint="zh-Hans-CN")
    assert config.language_hint == "zh"

    config = TokenReductionConfig(language_hint="EN")
    assert config.language_hint == "en"

    config = TokenReductionConfig(language_hint="ENG")
    assert config.language_hint == "en"

    config = TokenReductionConfig(language_hint="en")
    assert config.language_hint == "en"

    config = TokenReductionConfig(language_hint="fr")
    assert config.language_hint == "fr"


def test_unicode_normalization() -> None:
    from kreuzberg._types import TokenReductionConfig

    text_combining = "cafe\u0301"
    text_precomposed = "café"

    config = TokenReductionConfig(mode="light")

    result1 = reduce_tokens(text_combining, config=config)
    result2 = reduce_tokens(text_precomposed, config=config)

    assert result1 == result2


def test_punctuation_preservation_with_stopwords() -> None:
    from kreuzberg._types import TokenReductionConfig

    config = TokenReductionConfig(mode="moderate")

    text = "The cat is on the mat."
    result = reduce_tokens(text, config=config, language="en")
    assert result.endswith(".")
    assert "cat" in result
    assert "mat" in result
    assert "the" not in result.lower()

    text = "Is the cat on the mat?"
    result = reduce_tokens(text, config=config, language="en")
    assert result.endswith("?")

    text = "The cat is amazing!"
    result = reduce_tokens(text, config=config, language="en")
    assert result.endswith("!")

    text = "The cat, the dog, and the bird."
    result = reduce_tokens(text, config=config, language="en")
    assert "cat" in result
    assert "dog" in result
    assert "bird" in result


def test_performance_pre_lowercase_stopwords() -> None:
    import time

    from kreuzberg._types import TokenReductionConfig

    config = TokenReductionConfig(mode="moderate")

    text = " ".join(["The quick brown fox jumps over the lazy dog"] * 1000)

    start = time.perf_counter()
    result = reduce_tokens(text, config=config, language="en")
    elapsed = time.perf_counter() - start

    assert elapsed < 1.0
    assert "quick" in result
    assert "brown" in result
    assert "fox" in result
    assert "the" not in result.lower()
