"""Generate OCR quality metrics against text-layer PDFs.

Usage:
    uv run python scripts/ocr_quality_report.py --output v4-quality.json
"""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pypdfium2 as pdfium
from PIL import Image  # noqa: TC002

from kreuzberg import extract_file_sync
from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    import collections.abc as collections_abc

    from kreuzberg._types import TableExtractionConfig as TableExtractionConfigClass
else:  # pragma: no cover - optional dependency for older releases
    try:
        from kreuzberg._types import TableExtractionConfig as TableExtractionConfigClass
    except ImportError:  # pragma: no cover - older releases
        TableExtractionConfigClass = None  # type: ignore[assignment]


@dataclass(frozen=True)
class QualityDocument:
    """Metadata describing a document used for OCR quality evaluation."""

    id: str
    pdf_path: Path
    pages: tuple[int, ...] = (0,)
    dpi: int = 300


QUALITY_DATASET = (
    QualityDocument("searchable", Path("test_documents/pdfs/searchable.pdf")),
    QualityDocument("embedded_tables", Path("test_documents/pdfs/embedded_images_tables.pdf")),
    QualityDocument("code_formula", Path("test_documents/pdfs/code_and_formula.pdf")),
)


def _render_pdf_to_images(document: QualityDocument) -> list[Image.Image]:
    pdf = pdfium.PdfDocument(str(document.pdf_path))
    scale = document.dpi / 72.0
    return [pdf[page_idx].render(scale=scale).to_pil() for page_idx in document.pages]


def _load_text_layer(document: QualityDocument) -> str:
    pdf = pdfium.PdfDocument(str(document.pdf_path))
    texts: list[str] = []
    for page_idx in document.pages:
        text_page = pdf[page_idx].get_textpage()
        texts.append(text_page.get_text_bounded())
    return "\n".join(texts)


def _tokenize(text: str) -> list[str]:
    normalized = text.lower().replace("\u2013", "-").replace("\u2014", "-")
    normalized = "".join(ch if (ch >= " " or ch in "\n\r\t") else "" for ch in normalized)
    translation_table = str.maketrans(dict.fromkeys("()[],.;:+`", " "))
    normalized = normalized.translate(translation_table)
    return normalized.split()


def _numeric_tokens(tokens: collections_abc.Iterable[str]) -> list[str]:
    numeric_tokens: list[str] = []
    for token in tokens:
        stripped = token.strip("()[]{}")
        if not any(ch.isdigit() for ch in stripped):
            continue
        if any(ch.isalpha() for ch in stripped):
            continue
        numeric_tokens.append(stripped)
    return numeric_tokens


def _compute_f1(truth_tokens: list[str], ocr_tokens: list[str]) -> dict[str, float]:
    truth_set = set(truth_tokens)
    ocr_set = set(ocr_tokens)
    overlap = truth_set & ocr_set

    precision = len(overlap) / max(1, len(ocr_set))
    recall = len(overlap) / max(1, len(truth_set))
    f1 = 0.0 if precision + recall == 0 else 2 * precision * recall / (precision + recall)
    return {"f1": f1, "precision": precision, "recall": recall}


def _extract_reference_markdown(document: QualityDocument) -> str | None:
    tables_config: Any | None = None
    if TableExtractionConfigClass is not None:
        tables_config = TableExtractionConfigClass(extract_from_ocr=True)
    try:
        result = extract_file_sync(
            str(document.pdf_path),
            config=ExtractionConfig(ocr=None, tables=tables_config, use_cache=False),
        )
    except (MissingDependencyError, ModuleNotFoundError, RuntimeError, ValueError):
        return None
    if not result.content:
        return None
    token_count = len(result.content.split())
    if token_count < 10:
        return None
    if result.tables:
        table_markdown = "\n\n".join(table.get("text", "") for table in result.tables if table.get("text")).strip()
        if table_markdown:
            return table_markdown

    return result.content


def _normalize_markdown(text: str) -> list[str]:
    normalized = text.replace("|", " ")
    normalized = normalized.lower().replace("\u2013", "-").replace("\u2014", "-")
    normalized = "".join(ch if (ch >= " " or ch in "\n\r\t") else "" for ch in normalized)
    translation_table = str.maketrans(dict.fromkeys("()[],.;:+`", " "))
    normalized = normalized.translate(translation_table)
    return normalized.split()


def evaluate_document(document: QualityDocument) -> dict[str, object]:
    """Compute OCR quality metrics for a single document."""
    text_layer = _load_text_layer(document)
    truth_tokens = _tokenize(text_layer)
    truth_numeric_tokens = _numeric_tokens(truth_tokens)

    images = _render_pdf_to_images(document)
    backend = TesseractBackend()

    ocr_text_blocks = [
        backend.process_image_sync(image, output_format="text", enable_table_detection=False).content
        for image in images
    ]
    ocr_text = "\n".join(ocr_text_blocks)
    ocr_tokens = _tokenize(ocr_text)
    ocr_numeric_tokens = _numeric_tokens(ocr_tokens)

    document_data = asdict(document)
    document_data["pdf_path"] = str(document_data["pdf_path"])

    metrics: dict[str, Any] = {
        "document": document_data,
        "text": _compute_f1(truth_tokens, ocr_tokens),
        "numeric": _compute_f1(truth_numeric_tokens, ocr_numeric_tokens),
    }

    reference_markdown = _extract_reference_markdown(document)
    if reference_markdown:
        ocr_markdown_blocks = [
            backend.process_image_sync(image, output_format="markdown", enable_table_detection=True).content
            for image in images
        ]
        ocr_markdown = "\n".join(ocr_markdown_blocks)
        markdown_metrics = _compute_f1(
            _normalize_markdown(reference_markdown),
            _normalize_markdown(ocr_markdown),
        )
        markdown_metrics["reference_length"] = len(reference_markdown.splitlines())
        markdown_metrics["ocr_length"] = len(ocr_markdown.splitlines())
        metrics["markdown"] = markdown_metrics
    else:
        metrics["markdown"] = None

    return metrics


def main() -> None:
    """Entry-point for the `ocr_quality_report.py` script."""
    parser = argparse.ArgumentParser(description="Generate OCR quality metrics.")
    parser.add_argument("--output", type=Path, default=Path("ocr_quality.json"), help="Path for JSON results.")
    args = parser.parse_args()

    results = [evaluate_document(doc) for doc in QUALITY_DATASET]
    args.output.write_text(json.dumps({"results": results}, indent=2))


if __name__ == "__main__":
    main()
