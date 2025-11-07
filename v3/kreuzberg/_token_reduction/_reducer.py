from __future__ import annotations

import re
import unicodedata
from functools import lru_cache
from typing import TYPE_CHECKING, Any, TypedDict

from kreuzberg._token_reduction._stopwords import get_default_stopwords_manager
from kreuzberg.exceptions import ValidationError

if TYPE_CHECKING:
    from collections.abc import Callable

    from kreuzberg._types import TokenReductionConfig


class ReductionStats(TypedDict):
    """Statistics about token reduction operation."""

    character_reduction_ratio: float
    token_reduction_ratio: float
    original_characters: int
    reduced_characters: int
    original_tokens: int
    reduced_tokens: int


HTML_COMMENT_PATTERN = re.compile(r"<!--.*?-->", re.DOTALL)

PUNCTUATION_CLEANUP_PATTERN = re.compile(
    r"([!?.])(?:\1)+"
    r"|(,)(?:,)+"
    r"|[!?]+\.+[!?]*|[?!]{3,}"
)

WHITESPACE_CLEANUP_PATTERN = re.compile(r"\n{3,}|[ \t]+")

MARKDOWN_LIST_PATTERNS = (
    re.compile(r"^\s*[-*+]\s"),
    re.compile(r"^\s*\d+\.\s"),
)

WORD_CLEAN_PATTERN = re.compile(r"[^\w]", re.UNICODE)
LANGUAGE_CODE_PATTERN = re.compile(r"^[a-zA-Z0-9-]+$")

WORD_SPLIT_PATTERN = re.compile(r"\S+")
WORD_BOUNDARY_PATTERN = re.compile(r"^(\W*)(.*?)(\W*)$", re.UNICODE)

STREAMING_THRESHOLD = 1_000_000


def _normalize_unicode(text: str) -> str:
    """Normalize Unicode text to NFC form for consistent processing."""
    return unicodedata.normalize("NFC", text)


def _normalize_newlines(text: str) -> str:
    """Remove excessive newlines, keeping at most double newlines."""
    return WHITESPACE_CLEANUP_PATTERN.sub(lambda m: "\n\n" if m.group().startswith("\n") else " ", text)


def _process_text_streaming(
    text: str, processor_func: Callable[..., str], chunk_size: int = 100_000, **kwargs: Any
) -> str:
    """Process large text in chunks to optimize memory usage."""
    if len(text) <= chunk_size:
        return processor_func(text, **kwargs)

    chunks = []
    start = 0

    while start < len(text):
        end = min(start + chunk_size, len(text))

        if end < len(text):
            search_start = max(start, end - 1000)
            for i in range(end - 1, search_start - 1, -1):
                if text[i] in ".!?\n":
                    end = i + 1
                    break

        chunk = text[start:end]
        processed_chunk = processor_func(chunk, **kwargs)
        chunks.append(processed_chunk)
        start = end

    return " ".join(chunks).strip()


def _is_markdown_structural_line(line: str, in_code_block: bool) -> bool:
    """Check if a line contains markdown structural elements that should be preserved."""
    if in_code_block:
        return True

    stripped = line.strip()

    if stripped.startswith("#"):
        return True

    if "|" in line:
        pipe_count = line.count("|")
        if pipe_count >= 2 and (line.strip().startswith("|") or line.strip().endswith("|") or " | " in line):
            return True

    return MARKDOWN_LIST_PATTERNS[0].match(line) is not None or MARKDOWN_LIST_PATTERNS[1].match(line) is not None


@lru_cache(maxsize=64)
def _get_stopwords_with_custom(language: str, custom_words_tuple: tuple[str, ...] | None = None) -> set[str]:
    """Get stopwords for a language, optionally with custom additions."""
    manager = get_default_stopwords_manager()
    base_stopwords = manager.get_stopwords(language)

    if custom_words_tuple:
        return base_stopwords | set(custom_words_tuple)
    return base_stopwords


@lru_cache(maxsize=64)
def _get_lowercase_stopwords(language: str, custom_words_tuple: tuple[str, ...] | None = None) -> set[str]:
    """Get pre-lowercased stopwords for faster comparison."""
    stopwords = _get_stopwords_with_custom(language, custom_words_tuple)
    return {sw.lower() for sw in stopwords}


def reduce_tokens(
    text: str,
    *,
    config: TokenReductionConfig,
    language: str | None = None,
) -> str:
    """Reduce tokens in text based on the specified configuration.

    Args:
        text: The text to reduce.
        config: Configuration for token reduction.
        language: Optional language code for stopword selection.

    Returns:
        The reduced text.

    Raises:
        ValidationError: If inputs are invalid.

    """
    if config is None:
        raise ValidationError("Config cannot be None")

    if text is None:
        raise ValidationError("Text cannot be None")

    if not isinstance(text, str):
        raise ValidationError(f"Text must be a string, got {type(text).__name__}")

    if language is not None and not isinstance(language, str):
        raise ValidationError(f"Language must be a string or None, got {type(language).__name__}")

    if language is not None and len(language.strip()) == 0:
        raise ValidationError("Language cannot be empty or whitespace-only")

    if config.mode == "off":
        return text

    use_streaming = len(text) > STREAMING_THRESHOLD

    if language and not LANGUAGE_CODE_PATTERN.match(language):
        raise ValidationError(f"Invalid language code format: {language}")

    if not text or not text.strip():
        return ""

    text = _normalize_unicode(text)

    if config.mode == "light":
        return _apply_light_reduction(text, preserve_markdown=config.preserve_markdown, use_streaming=use_streaming)

    if config.mode == "moderate":
        return _apply_moderate_reduction(
            text,
            config=config,
            language=language,
            use_streaming=use_streaming,
        )

    return text


def _apply_light_reduction(text: str, *, preserve_markdown: bool, use_streaming: bool = False) -> str:
    """Apply light reduction (formatting only)."""
    if use_streaming:
        if preserve_markdown:
            return str(_process_text_streaming(text, _apply_light_reduction_markdown_aware))
        return str(_process_text_streaming(text, _apply_light_reduction_plain))

    if preserve_markdown:
        return _apply_light_reduction_markdown_aware(text)
    return _apply_light_reduction_plain(text)


def _apply_light_reduction_plain(text: str) -> str:
    """Apply light reduction to plain text."""
    text = HTML_COMMENT_PATTERN.sub("", text)

    def punctuation_replacer(match: re.Match[str]) -> str:
        if match.group(1):
            return match.group(1)
        if match.group(2):
            return ","
        return "?"

    text = PUNCTUATION_CLEANUP_PATTERN.sub(punctuation_replacer, text)

    def whitespace_replacer(match: re.Match[str]) -> str:
        if match.group().startswith("\n"):
            return "\n\n"
        return " "

    text = WHITESPACE_CLEANUP_PATTERN.sub(whitespace_replacer, text)

    return text.strip()


def _apply_light_reduction_markdown_aware(text: str) -> str:
    """Apply light reduction preserving markdown structure."""
    lines = text.split("\n")
    processed_lines = []
    in_code_block = False

    for line in lines:
        if line.strip().startswith("```"):
            in_code_block = not in_code_block
            processed_lines.append(line)
            continue

        if _is_markdown_structural_line(line, in_code_block) or in_code_block:
            processed_lines.append(line)
            continue

        if line.strip():
            reduced = _apply_light_reduction_plain(line)
            processed_lines.append(reduced)
        else:
            processed_lines.append(line)

    result = "\n".join(processed_lines)

    lines = result.split("\n")
    normalized_lines = []
    in_code_block = False
    consecutive_empty = 0

    for line in lines:
        if line.strip().startswith("```"):
            in_code_block = not in_code_block
            normalized_lines.append(line)
            consecutive_empty = 0
            continue

        if in_code_block:
            normalized_lines.append(line)
            consecutive_empty = 0
        elif not line.strip():
            consecutive_empty += 1
            if consecutive_empty <= 2:
                normalized_lines.append(line)
        else:
            normalized_lines.append(line)
            consecutive_empty = 0

    return "\n".join(normalized_lines).strip()


def _apply_moderate_reduction(
    text: str,
    *,
    config: TokenReductionConfig,
    language: str | None = None,
    use_streaming: bool = False,
) -> str:
    """Apply moderate reduction (formatting + stopwords)."""
    text = _apply_light_reduction(text, preserve_markdown=config.preserve_markdown, use_streaming=use_streaming)

    lang = language or config.language_hint or "en"

    manager = get_default_stopwords_manager()
    if not manager.has_language(lang):
        lang = "en"
        if not manager.has_language("en"):
            return text

    custom_words_tuple = None
    if config.custom_stopwords and lang in config.custom_stopwords:
        custom_words_tuple = tuple(sorted(config.custom_stopwords[lang]))

    if use_streaming:
        if config.preserve_markdown:
            return str(
                _process_text_streaming(
                    text,
                    _apply_stopword_reduction_markdown_aware,
                    stopwords=_get_lowercase_stopwords(lang, custom_words_tuple),
                )
            )
        return str(
            _process_text_streaming(
                text, _apply_stopword_reduction_plain, stopwords=_get_lowercase_stopwords(lang, custom_words_tuple)
            )
        )

    stopwords = _get_lowercase_stopwords(lang, custom_words_tuple)

    if config.preserve_markdown:
        return _apply_stopword_reduction_markdown_aware(text, stopwords=stopwords)
    return _apply_stopword_reduction_plain(text, stopwords=stopwords)


def _apply_stopword_reduction_plain(text: str, *, stopwords: set[str]) -> str:
    """Apply stopword reduction to plain text.

    Args:
        text: Text to process
        stopwords: Pre-lowercased stopwords set for faster comparison

    """
    words = WORD_SPLIT_PATTERN.findall(text)
    if not words:
        return ""

    filtered_words = []

    for word in words:
        if len(word) <= 3 and word.isalpha():
            if word.lower() not in stopwords or word.isupper() or len(word) == 1:
                filtered_words.append(word)
            continue

        match = WORD_BOUNDARY_PATTERN.match(word)
        if not match:
            filtered_words.append(word)
            continue

        _prefix_punct, core_word, suffix_punct = match.groups()

        if not core_word:
            filtered_words.append(word)
            continue

        clean_word = core_word.lower() if core_word.isalpha() else WORD_CLEAN_PATTERN.sub("", core_word).lower()

        if not clean_word:
            filtered_words.append(word)
            continue

        is_stopword = clean_word in stopwords
        should_keep = (
            not is_stopword
            or len(clean_word) <= 1
            or (len(core_word) > 1 and core_word.isupper())
            or any(c.isdigit() for c in core_word)
        )

        if should_keep:
            filtered_words.append(word)
        elif (
            suffix_punct
            and suffix_punct in ".,;:!?"
            and filtered_words
            and not filtered_words[-1].endswith(suffix_punct)
        ):
            filtered_words[-1] += suffix_punct

    return " ".join(filtered_words) if filtered_words else ""


def _apply_stopword_reduction_markdown_aware(text: str, *, stopwords: set[str]) -> str:
    """Apply stopword reduction preserving markdown structure."""
    lines = text.split("\n")
    processed_lines = []
    in_code_block = False

    for line in lines:
        if line.strip().startswith("```"):
            in_code_block = not in_code_block
            processed_lines.append(line)
            continue

        if _is_markdown_structural_line(line, in_code_block):
            processed_lines.append(line)
            continue

        if line.strip():
            reduced = _apply_stopword_reduction_plain(line, stopwords=stopwords)
            processed_lines.append(reduced)
        else:
            processed_lines.append(line)

    result = "\n".join(processed_lines)
    return _normalize_newlines(result).strip()


def get_reduction_stats(original: str, reduced: str) -> ReductionStats:
    """Get detailed statistics about the reduction.

    Args:
        original: The original text.
        reduced: The reduced text.

    Returns:
        Statistics about the reduction.

    Raises:
        ValidationError: If inputs are invalid.

    """
    if original is None:
        raise ValidationError("Original text cannot be None")

    if reduced is None:
        raise ValidationError("Reduced text cannot be None")

    if not isinstance(original, str):
        raise ValidationError(f"Original text must be a string, got {type(original).__name__}")

    if not isinstance(reduced, str):
        raise ValidationError(f"Reduced text must be a string, got {type(reduced).__name__}")

    original_chars = len(original)
    reduced_chars = len(reduced)
    original_tokens = len(original.split()) if original else 0
    reduced_tokens = len(reduced.split()) if reduced else 0

    char_reduction = (original_chars - reduced_chars) / original_chars if original_chars > 0 else 0.0
    token_reduction = (original_tokens - reduced_tokens) / original_tokens if original_tokens > 0 else 0.0

    return ReductionStats(
        character_reduction_ratio=char_reduction,
        token_reduction_ratio=token_reduction,
        original_characters=original_chars,
        reduced_characters=reduced_chars,
        original_tokens=original_tokens,
        reduced_tokens=reduced_tokens,
    )
