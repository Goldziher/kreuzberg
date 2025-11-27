from __future__ import annotations

import re
from functools import reduce
from itertools import chain
from typing import Any

_OCR_ARTIFACTS = {
    "scattered_chars": re.compile(r"\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b"),
    "repeated_punctuation": re.compile(r"[.]{3,}|[-]{3,}|[_]{3,}"),
    "isolated_punctuation": re.compile(r"\s[.,;:!?]\s"),
    "malformed_words": re.compile(r"\b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b"),
    "excessive_whitespace": re.compile(r"\s{3,}"),
    "broken_sentences": re.compile(r"[a-z]\s{3,}[A-Z][a-z]"),
}

_COMBINED_OCR_PATTERN = re.compile(
    r"(?P<scattered>\b[a-zA-Z]\s{2,}[a-zA-Z]\s{2,}[a-zA-Z]\b)|"
    r"(?P<repeated>[.]{3,}|[-]{3,}|[_]{3,})|"
    r"(?P<isolated>\s[.,;:!?]\s)|"
    r"(?P<malformed>\b[a-zA-Z]+[0-9]+[a-zA-Z]+[a-zA-Z0-9]*\b)|"
    r"(?P<whitespace>\s{3,})|"
    r"(?P<broken>[a-z]\s{3,}[A-Z][a-z])"
)

_WHITESPACE_NORMALIZE = re.compile(r"[ \t\f\v\r\xa0\u2000-\u200b\u2028\u2029\u3000]+")
_NEWLINE_NORMALIZE = re.compile(r"\n\s*\n\s*\n+")
_SENTENCE_DETECT = re.compile(r"[.!?]\s+[A-Z]")
_PUNCTUATION_DETECT = re.compile(r"[.!?]")

_SCRIPT_PATTERNS = {
    "js_functions": re.compile(r"function\s+\w+\s*\([^)]*\)\s*\{[^}]*\}", re.IGNORECASE),
    "css_rules": re.compile(r"\.[a-zA-Z][\w-]*\s*\{[^}]*\}", re.IGNORECASE),
    # Tempered dot to avoid catastrophic backtracking and over-greedy matches
    "script_tags": re.compile(r"<script\b[^>]*>(?:(?!</script>).)*</script>", re.DOTALL | re.IGNORECASE),
    "style_tags": re.compile(r"<style\b[^>]*>(?:(?!</style>).)*</style>", re.DOTALL | re.IGNORECASE),
}

_NAVIGATION_PATTERNS = {
    "nav_words": re.compile(r"\b(?:Skip to main content|Back to top|Main navigation|Site navigation)\b", re.IGNORECASE),
    "breadcrumbs": re.compile(r"(?:Home\s*[>»]\s*|[>»]\s*){2,}"),
    "pagination": re.compile(
        r"\b(?:Page \d+ of \d+|First page|Last page|Previous page|Next page|^\d+ of \d+$)\b", re.IGNORECASE
    ),
}


def calculate_quality_score(text: str, metadata: dict[str, Any] | None = None) -> float:
    if not text or not text.strip():
        return 0.0

    score = 1.0
    total_chars = len(text)

    ocr_penalty = _calculate_ocr_penalty(text, total_chars)
    score -= ocr_penalty * 0.3

    script_penalty = _calculate_script_penalty(text, total_chars)
    score -= script_penalty * 0.2

    nav_penalty = _calculate_navigation_penalty(text, total_chars)
    score -= nav_penalty * 0.1

    structure_bonus = _calculate_structure_bonus(text)
    score += structure_bonus * 0.2

    if metadata:
        metadata_bonus = _calculate_metadata_bonus(metadata)
        score += metadata_bonus * 0.1

    return max(0.0, min(1.0, score))


def clean_extracted_text(text: str) -> str:
    if not text:
        return text

    text = reduce(lambda t, pattern: pattern.sub(" ", t), _SCRIPT_PATTERNS.values(), text)

    text = _clean_ocr_artifacts(text)

    text = _clean_navigation_elements(text)

    text = _WHITESPACE_NORMALIZE.sub(" ", text)
    text = _NEWLINE_NORMALIZE.sub("\n\n", text)

    return text.strip()


def _calculate_ocr_penalty(text: str, total_chars: int) -> float:
    if total_chars == 0:
        return 0.0

    artifact_chars = sum(len(match.group()) for match in _COMBINED_OCR_PATTERN.finditer(text))
    return min(1.0, artifact_chars / total_chars)


def _calculate_script_penalty(text: str, total_chars: int) -> float:
    if total_chars == 0:
        return 0.0

    script_chars = sum(
        len(match) for match in chain.from_iterable(pattern.findall(text) for pattern in _SCRIPT_PATTERNS.values())
    )

    return min(1.0, script_chars / total_chars)


def _calculate_navigation_penalty(text: str, total_chars: int) -> float:
    if total_chars == 0:
        return 0.0

    nav_chars = sum(
        len(match) for match in chain.from_iterable(pattern.findall(text) for pattern in _NAVIGATION_PATTERNS.values())
    )

    return min(1.0, nav_chars / total_chars)


def _calculate_structure_bonus(text: str) -> float:
    if not text:
        return 0.0

    sentence_count = len(_SENTENCE_DETECT.findall(text))

    paragraph_count = len(text.split("\n\n"))

    words = len(text.split())
    if words == 0:
        return 0.0

    avg_words_per_sentence = words / max(1, sentence_count)
    avg_words_per_paragraph = words / max(1, paragraph_count)

    structure_score = 0.0

    if 10 <= avg_words_per_sentence <= 30:
        structure_score += 0.3

    if 50 <= avg_words_per_paragraph <= 300:
        structure_score += 0.3

    if paragraph_count > 1:
        structure_score += 0.2

    if _PUNCTUATION_DETECT.search(text):
        structure_score += 0.2

    return min(1.0, structure_score)


def _calculate_metadata_bonus(metadata: dict[str, Any]) -> float:
    if not metadata:
        return 0.0

    important_fields = {"title", "author", "subject", "description", "keywords"}
    present_fields = sum(1 for field in important_fields if metadata.get(field))

    return present_fields / len(important_fields)


def _clean_ocr_artifacts(text: str) -> str:
    text = _OCR_ARTIFACTS["scattered_chars"].sub(lambda m: m.group().replace(" ", ""), text)

    text = _OCR_ARTIFACTS["repeated_punctuation"].sub("...", text)

    text = _OCR_ARTIFACTS["isolated_punctuation"].sub(" ", text)

    text = _OCR_ARTIFACTS["malformed_words"].sub(" ", text)

    return _OCR_ARTIFACTS["excessive_whitespace"].sub(" ", text)


def _clean_navigation_elements(text: str) -> str:
    text = _NAVIGATION_PATTERNS["nav_words"].sub(" ", text)

    text = _NAVIGATION_PATTERNS["breadcrumbs"].sub(" ", text)

    return _NAVIGATION_PATTERNS["pagination"].sub(" ", text)
