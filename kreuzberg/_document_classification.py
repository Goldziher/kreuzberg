from __future__ import annotations

import re
from typing import TYPE_CHECKING

import polars as pl

from kreuzberg._ocr import get_ocr_backend
from kreuzberg._types import ExtractionConfig, ExtractionResult  # noqa: TC001
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from pathlib import Path


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


def _get_translated_text(result: ExtractionResult) -> str:
    text_to_classify = result.content
    if result.metadata:
        metadata_text = " ".join(str(value) for value in result.metadata.values() if value)
        text_to_classify = f"{text_to_classify} {metadata_text}"

    try:
        from deep_translator import GoogleTranslator  # noqa: PLC0415  # type: ignore[import-untyped]
    except ImportError as e:  # pragma: no cover
        raise MissingDependencyError(
            "The 'deep-translator' library is not installed. Please install it with: pip install 'kreuzberg[document-classification]'"
        ) from e

    try:
        return str(GoogleTranslator(source="auto", target="en").translate(text_to_classify).lower())
    except Exception:  # noqa: BLE001
        return text_to_classify.lower()


def classify_document(result: ExtractionResult, config: ExtractionConfig) -> tuple[str | None, float | None]:
    if not config.auto_detect_document_type:
        return None, None

    translated_text = _get_translated_text(result)
    scores = {
        doc_type: sum(1 for pattern in patterns if re.search(pattern, translated_text))
        for doc_type, patterns in DOCUMENT_CLASSIFIERS.items()
    }

    total_score = sum(scores.values())
    if total_score == 0:
        return None, None

    confidences = {doc_type: score / total_score for doc_type, score in scores.items()}

    best_type, best_confidence = max(confidences.items(), key=lambda item: item[1])

    if best_confidence >= config.document_type_confidence_threshold:
        return best_type, best_confidence

    return None, None


def classify_document_from_layout(
    result: ExtractionResult, config: ExtractionConfig
) -> tuple[str | None, float | None]:
    if not config.auto_detect_document_type:
        return None, None

    if result.layout is None or result.layout.is_empty():
        return None, None

    layout_df = result.layout
    if not all(col in layout_df.columns for col in ["text", "top", "height"]):
        return None, None

    layout_text = " ".join(layout_df["text"].cast(str).to_list())

    text_to_classify = layout_text
    if result.metadata:
        metadata_text = " ".join(str(value) for value in result.metadata.values() if value)
        text_to_classify = f"{text_to_classify} {metadata_text}"

    try:
        from deep_translator import GoogleTranslator  # noqa: PLC0415  # type: ignore[import-untyped]

        translated_text = str(GoogleTranslator(source="auto", target="en").translate(text_to_classify).lower())
    except Exception:  # noqa: BLE001
        translated_text = text_to_classify.lower()

    layout_df = layout_df.with_columns(pl.lit(translated_text).alias("translated_text"))

    try:
        layout_df = layout_df.with_columns(
            [pl.col("top").cast(pl.Float64, strict=False), pl.col("height").cast(pl.Float64, strict=False)]
        )

        page_height_val = layout_df.select(pl.col("top").max() + pl.col("height").max()).item()
        if page_height_val is None:
            page_height_val = 0.0
        page_height = float(page_height_val)
    except Exception:  # noqa: BLE001
        page_height = 1000.0
    scores = dict.fromkeys(DOCUMENT_CLASSIFIERS, 0.0)

    for doc_type, patterns in DOCUMENT_CLASSIFIERS.items():
        for pattern in patterns:
            found_words = layout_df.filter(layout_df["translated_text"].str.contains(pattern))
            if not found_words.is_empty():
                scores[doc_type] += 1.0
                word_top = found_words[0, "top"]
                if word_top is not None and word_top < page_height * 0.3:
                    scores[doc_type] += 0.5

    total_score = sum(scores.values())
    if total_score == 0:
        return None, None

    confidences = {doc_type: score / total_score for doc_type, score in scores.items()}

    best_type, best_confidence = max(confidences.items(), key=lambda item: item[1])

    if best_confidence >= config.document_type_confidence_threshold:
        return best_type, best_confidence

    return None, None


def auto_detect_document_type(
    result: ExtractionResult, config: ExtractionConfig, file_path: Path | None = None
) -> ExtractionResult:
    if config.document_classification_mode == "vision" and file_path:
        layout_result = get_ocr_backend("tesseract").process_file_sync(file_path, **config.get_config_dict())
        result.document_type, result.document_type_confidence = classify_document_from_layout(layout_result, config)
    elif result.layout is not None and not result.layout.is_empty():
        result.document_type, result.document_type_confidence = classify_document_from_layout(result, config)
    else:
        result.document_type, result.document_type_confidence = classify_document(result, config)
    return result
