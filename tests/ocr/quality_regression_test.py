from __future__ import annotations

import collections.abc as collections_abc
import os
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

import pypdfium2 as pdfium
import pytest
from PIL import Image  # noqa: TC002

from kreuzberg import extract_file_sync
from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import ExtractionConfig, TableExtractionConfig
from kreuzberg.exceptions import MissingDependencyError


@dataclass(frozen=True)
class QualityDocument:
    """Metadata describing a quality evaluation document."""

    id: str
    pdf_path: Path
    pages: tuple[int, ...] = (0,)
    dpi: int = 300
    min_f1: float = 0.9
    max_layout_delta: float = 0.25
    min_numeric_f1: float | None = None
    evaluate_markdown: bool = True
    min_markdown_f1: float | None = None
    max_markdown_structure_delta: float | None = None


@dataclass(frozen=True)
class MarkdownStructureStats:
    """Basic structural statistics extracted from markdown text."""

    headings: Counter[int]
    bullet: int
    ordered: int
    table_rows: int
    code_fences: int
    lines: int


QUALITY_DATASET: tuple[QualityDocument, ...] = (
    QualityDocument(
        id="searchable",
        pdf_path=Path("test_documents/pdfs/searchable.pdf"),
        pages=(0,),
        min_f1=0.98,
        max_layout_delta=0.1,
        min_markdown_f1=0.95,
        max_markdown_structure_delta=None,
    ),
    QualityDocument(
        id="embedded_tables",
        pdf_path=Path("test_documents/pdfs/embedded_images_tables.pdf"),
        pages=(0,),
        min_f1=0.82,
        min_numeric_f1=0.75,
        max_layout_delta=0.35,
        min_markdown_f1=0.75,
        max_markdown_structure_delta=None,
    ),
    QualityDocument(
        id="code_formula",
        pdf_path=Path("test_documents/pdfs/code_and_formula.pdf"),
        pages=(0,),
        min_f1=0.88,
        max_layout_delta=0.25,
        min_markdown_f1=0.78,
        max_markdown_structure_delta=None,
    ),
)


pytestmark = pytest.mark.skipif(
    not os.getenv("RUN_OCR_QUALITY"),
    reason="Set RUN_OCR_QUALITY=1 to enable OCR quality regression benchmarks (slow)",
)


def _load_pdf_text(pdf_path: Path, pages: collections_abc.Iterable[int]) -> str:
    pdf = pdfium.PdfDocument(str(pdf_path))
    page_texts: list[str] = []
    for page_number in pages:
        page = pdf[page_number]
        text_page = page.get_textpage()
        page_texts.append(text_page.get_text_bounded())
    return "\n".join(page_texts)


def _render_pdf_to_images(
    pdf_path: Path,
    pages: collections_abc.Iterable[int],
    *,
    dpi: int,
) -> list[Image.Image]:
    pdf = pdfium.PdfDocument(str(pdf_path))
    images: list[Image.Image] = []
    scale = dpi / 72.0
    for page_number in pages:
        page = pdf[page_number]
        images.append(page.render(scale=scale).to_pil())
    return images


def _normalize_tokens(text: str) -> list[str]:
    normalized = text.lower().replace("\u2013", "-").replace("\u2014", "-")
    normalized = "".join(ch if (ch >= " " or ch in "\n\r\t") else "" for ch in normalized)
    translation_table = str.maketrans(dict.fromkeys("()[],.;:+`", " "))
    normalized = normalized.translate(translation_table)
    return normalized.split()


def _compute_f1(truth_tokens: list[str], ocr_tokens: list[str]) -> tuple[float, float, float]:
    truth_set = set(truth_tokens)
    ocr_set = set(ocr_tokens)
    overlap = truth_set & ocr_set

    precision = len(overlap) / max(1, len(ocr_set))
    recall = len(overlap) / max(1, len(truth_set))
    if precision + recall == 0:
        return 0.0, precision, recall
    f1 = 2 * precision * recall / (precision + recall)
    return f1, precision, recall


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


def _extract_reference_markdown(document: QualityDocument) -> str | None:
    try:
        result = extract_file_sync(
            str(document.pdf_path),
            config=ExtractionConfig(
                ocr=None,
                tables=TableExtractionConfig(extract_from_ocr=True),
                use_cache=False,
            ),
        )
    except MissingDependencyError:
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


def _markdown_structure_stats(markdown: str) -> MarkdownStructureStats:
    headings: Counter[int] = Counter()
    bullet = 0
    ordered = 0
    table_rows = 0
    code_fences = 0
    non_empty_lines = 0

    for line in markdown.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        non_empty_lines += 1

        if stripped.startswith("#"):
            level = len(stripped) - len(stripped.lstrip("#"))
            headings[level] += 1

        if stripped[:1] in {"-", "*", "+"} and (len(stripped) == 1 or stripped[1].isspace()):
            bullet += 1

        if stripped[:1].isdigit() and "." in stripped[:3]:
            ordered += 1

        if stripped.startswith("|") and stripped.count("|") >= 2:
            table_rows += 1

        if stripped.startswith("```"):
            code_fences += 1

    return MarkdownStructureStats(
        headings=headings,
        bullet=bullet,
        ordered=ordered,
        table_rows=table_rows,
        code_fences=code_fences,
        lines=non_empty_lines,
    )


def _relative_delta(observed: int, expected: int) -> float:
    base = max(expected, 1)
    return abs(observed - expected) / base


def _markdown_structure_delta(reference: MarkdownStructureStats, candidate: MarkdownStructureStats) -> float:
    ref_headings = reference.headings
    cand_headings = candidate.headings
    heading_levels = set(ref_headings.keys()) | set(cand_headings.keys())

    heading_deltas = [_relative_delta(cand_headings[level], ref_headings[level]) for level in heading_levels]
    aggregate_deltas = [
        _relative_delta(candidate.bullet, reference.bullet),
        _relative_delta(candidate.ordered, reference.ordered),
        _relative_delta(candidate.table_rows, reference.table_rows),
        _relative_delta(candidate.code_fences, reference.code_fences),
        _relative_delta(candidate.lines, reference.lines),
    ]
    deltas = [*heading_deltas, *aggregate_deltas]
    return max(deltas) if deltas else 0.0


def _normalize_markdown_tokens(markdown: str) -> list[str]:
    normalized = markdown.replace("|", " ")
    normalized = normalized.lower().replace("\u2013", "-").replace("\u2014", "-")
    normalized = "".join(ch if (ch >= " " or ch in "\n\r\t") else "" for ch in normalized)
    translation_table = str.maketrans(dict.fromkeys("()[],.;:+`", " "))
    normalized = normalized.translate(translation_table)
    return normalized.split()


def _layout_delta(truth_text: str, ocr_text: str) -> float:
    truth_lines = [line for line in truth_text.splitlines() if line.strip()]
    ocr_lines = [line for line in ocr_text.splitlines() if line.strip()]
    if not truth_lines:
        return 0.0
    return abs(len(ocr_lines) - len(truth_lines)) / len(truth_lines)


@pytest.mark.parametrize("document", QUALITY_DATASET, ids=lambda doc: doc.id)
def test_ocr_quality_against_text_layer(document: QualityDocument, capsys: pytest.CaptureFixture[str]) -> None:
    """Validate OCR fidelity by comparing against the pristine PDF text layer."""

    ground_truth_text = _load_pdf_text(document.pdf_path, document.pages)

    images = _render_pdf_to_images(document.pdf_path, document.pages, dpi=document.dpi)
    backend = TesseractBackend()
    ocr_results: list[str] = []
    for image in images:
        result = backend.process_image_sync(image, output_format="text", enable_table_detection=False)
        ocr_results.append(result.content)
    ocr_text = "\n".join(ocr_results)

    truth_tokens = _normalize_tokens(ground_truth_text)
    ocr_tokens = _normalize_tokens(ocr_text)
    f1, precision, recall = _compute_f1(truth_tokens, ocr_tokens)

    layout_delta = _layout_delta(ground_truth_text, ocr_text)

    numeric_f1 = None
    if document.min_numeric_f1 is not None:
        truth_numeric = _numeric_tokens(truth_tokens)
        ocr_numeric = _numeric_tokens(ocr_tokens)
        numeric_f1, _, _ = _compute_f1(truth_numeric, ocr_numeric)

    # Emit a compact summary for manual inspection when benchmarks run.
    numeric_repr = f"{numeric_f1:.3f}" if numeric_f1 is not None else "n/a"
    print(  # noqa: T201 - provide helpful benchmark context
        f"[{document.id}] f1={f1:.3f} precision={precision:.3f} recall={recall:.3f} "
        f"numeric_f1={numeric_repr} layout_delta={layout_delta:.3f}"
    )
    capsys.readouterr()

    assert f1 >= document.min_f1, (
        f"OCR F1 score {f1:.3f} below threshold {document.min_f1:.3f} for document '{document.id}'"
    )
    assert layout_delta <= document.max_layout_delta, (
        f"Layout delta {layout_delta:.3f} exceeds allowed {document.max_layout_delta:.3f} for document '{document.id}'"
    )
    if document.min_numeric_f1 is not None:
        assert numeric_f1 is not None
        assert numeric_f1 >= document.min_numeric_f1, (
            f"Numeric token F1 score {numeric_f1:.3f} below threshold "
            f"{document.min_numeric_f1:.3f} for document '{document.id}'"
        )

    if document.evaluate_markdown and document.min_markdown_f1 is not None:
        reference_markdown = _extract_reference_markdown(document)
        if reference_markdown is None:
            pytest.skip(
                "Table extraction dependencies (torch + transformers) are required for markdown quality checks. "
                "Install with 'pip install \"kreuzberg[vision-tables]\"' or disable markdown evaluation."
            )

        ocr_markdown_fragments: list[str] = []
        for image in images:
            markdown_result = backend.process_image_sync(
                image,
                output_format="markdown",
                enable_table_detection=True,
            )
            ocr_markdown_fragments.append(markdown_result.content)
        ocr_markdown = "\n".join(ocr_markdown_fragments)

        markdown_tokens_truth = _normalize_markdown_tokens(reference_markdown)
        markdown_tokens_ocr = _normalize_markdown_tokens(ocr_markdown)
        markdown_f1, markdown_precision, markdown_recall = _compute_f1(
            markdown_tokens_truth,
            markdown_tokens_ocr,
        )

        structure_delta = _markdown_structure_delta(
            _markdown_structure_stats(reference_markdown),
            _markdown_structure_stats(ocr_markdown),
        )

        print(  # noqa: T201 - provide helpful benchmark context
            f"[{document.id}] markdown_f1={markdown_f1:.3f} markdown_precision={markdown_precision:.3f} "
            f"markdown_recall={markdown_recall:.3f} markdown_structure_delta={structure_delta:.3f}"
        )
        capsys.readouterr()

        assert markdown_f1 >= document.min_markdown_f1, (
            f"Markdown token F1 score {markdown_f1:.3f} below threshold {document.min_markdown_f1:.3f} "
            f"for document '{document.id}'"
        )
        if document.max_markdown_structure_delta is not None:
            assert structure_delta <= document.max_markdown_structure_delta, (
                f"Markdown structural delta {structure_delta:.3f} exceeds allowed "
                f"{document.max_markdown_structure_delta:.3f} for document '{document.id}'"
            )
