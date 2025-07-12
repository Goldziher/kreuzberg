from __future__ import annotations

import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from kreuzberg._types import ExtractionConfig, ExtractionResult


DOCUMENT_CLASSIFIERS = {
    "invoice": [
        r"invoice",
        r"bill to",
        r"invoice number",
        r"total amount",
        r"tax id",
    ],
    "receipt": [
        r"receipt",
        r"cash receipt",
        r"payment",
        r"subtotal",
        r"total due",
    ],
    "contract": [
        r"agreement",
        r"contract",
        r"party a",
        r"party b",
        r"terms and conditions",
        r"signature",
    ],
    "report": [r"report", r"summary", r"analysis", r"findings", r"conclusion"],
    "form": [r"form", r"fill out", r"signature", r"date", r"submit"],
}


def classify_document(result: ExtractionResult, config: ExtractionConfig) -> tuple[str | None, float | None]:
    """Classifies the document type based on keywords and patterns.

    Args:
        result: The extraction result containing the content.
        config: The extraction configuration.

    Returns:
        A tuple containing the detected document type and the confidence score,
        or (None, None) if no type is detected with sufficient confidence.
    """
    content_lower = result.content.lower()
    scores = dict.fromkeys(DOCUMENT_CLASSIFIERS, 0)

    for doc_type, patterns in DOCUMENT_CLASSIFIERS.items():
        for pattern in patterns:
            if re.search(pattern, content_lower):
                scores[doc_type] += 1

    total_score = sum(scores.values())
    if total_score == 0:
        return None, None

    confidences = {doc_type: score / total_score for doc_type, score in scores.items()}

    best_type, best_confidence = max(confidences.items(), key=lambda item: item[1])

    if best_confidence >= config.type_confidence_threshold:
        return best_type, best_confidence

    return None, None


def classify_document_from_layout(
    result: ExtractionResult, config: ExtractionConfig
) -> tuple[str | None, float | None]:
    """Classifies the document type based on layout information from OCR.

    Args:
        result: The extraction result containing the layout data.
        config: The extraction configuration.

    Returns:
        A tuple containing the detected document type and the confidence score,
        or (None, None) if no type is detected with sufficient confidence.
    """
    if result.layout is None or result.layout.empty:
        return None, None

    layout_df = result.layout
    if not all(col in layout_df.columns for col in ["text", "top", "height"]):
        return None, None

    page_height = layout_df["top"].max() + layout_df["height"].max()
    scores = dict.fromkeys(DOCUMENT_CLASSIFIERS, 0.0)

    for doc_type, patterns in DOCUMENT_CLASSIFIERS.items():
        for pattern in patterns:
            found_words = layout_df[layout_df["text"].str.contains(pattern, case=False, na=False)]
            if not found_words.empty:
                scores[doc_type] += 1.0
                word_top = found_words.iloc[0]["top"]
                if word_top < page_height * 0.3:
                    scores[doc_type] += 0.5

    total_score = sum(scores.values())
    if total_score == 0:
        return None, None

    confidences = {doc_type: score / total_score for doc_type, score in scores.items()}

    best_type, best_confidence = max(confidences.items(), key=lambda item: item[1])

    if best_confidence >= config.type_confidence_threshold:
        return best_type, best_confidence

    return None, None
