"""Assess OCR fidelity against text-layer ground truth.

This development utility compares Tesseract OCR output against PDFs that already contain searchable
text. It reports token-level accuracy for plain text, markdown fidelity, and layout preservation
metrics so we can continuously monitor regression risk while tuning the OCR stack.

Example usage:
    uv run python scripts/ocr_quality_report.py --limit 5
    uv run python scripts/ocr_quality_report.py --sample --limit 25 --output results/ocr_quality.json
"""

from __future__ import annotations

import argparse
import json
import math
import random
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

import pypdfium2 as pdfium
from PIL import Image  # noqa: TC002

from kreuzberg import extract_file_sync
from kreuzberg._ocr._tesseract import TesseractBackend
from kreuzberg._types import ExtractionConfig
from kreuzberg.exceptions import MissingDependencyError

if TYPE_CHECKING:
    from collections.abc import Iterable, Sequence

    from kreuzberg._types import TableExtractionConfig as TableExtractionConfigType
else:  # pragma: no cover - optional dependency for older releases
    try:
        from kreuzberg._types import TableExtractionConfig as TableExtractionConfigType
    except ImportError:  # pragma: no cover - older releases
        TableExtractionConfigType = None  # type: ignore[assignment]


DEFAULT_PDF_ROOT = Path("test_documents/technical_pdfs")
DEFAULT_GROUND_TRUTH_ROOT = Path("test_documents/ground_truth")


@dataclass(frozen=True)
class DocumentSpec:
    """Describe a PDF that should be evaluated."""

    doc_id: str
    pdf_path: Path
    markdown_reference: Path | None = None
    pages: tuple[int, ...] | None = None
    dpi: int = 300


@dataclass(frozen=True)
class EvaluationScores:
    """Precision / recall / F1 triple."""

    precision: float
    recall: float
    f1: float


@dataclass(frozen=True)
class MarkdownStructureStats:
    """Basic markdown layout counters."""

    headings: dict[int, int]
    bullet: int
    ordered: int
    table_rows: int
    code_fences: int
    lines: int


@dataclass(frozen=True)
class DocumentEvaluation:
    """Result bundle for a single document."""

    doc_id: str
    pdf_path: Path
    page_count: int | None
    text_score: EvaluationScores | None
    numeric_score: EvaluationScores | None
    layout_truth_lines: int | None
    layout_ocr_lines: int | None
    layout_delta: float | None
    markdown_score: EvaluationScores | None
    markdown_structure_delta: float | None
    markdown_reference_source: str | None
    markdown_structure_truth: MarkdownStructureStats | None
    markdown_structure_candidate: MarkdownStructureStats | None
    notes: tuple[str, ...]


DEFAULT_DOCS: tuple[DocumentSpec, ...] = (
    DocumentSpec(
        doc_id="searchable",
        pdf_path=Path("test_documents/pdfs/searchable.pdf"),
        pages=(0,),
    ),
    DocumentSpec(
        doc_id="embedded_tables",
        pdf_path=Path("test_documents/pdfs/embedded_images_tables.pdf"),
        pages=(0,),
    ),
    DocumentSpec(
        doc_id="code_formula",
        pdf_path=Path("test_documents/pdfs/code_and_formula.pdf"),
        pages=(0,),
    ),
)


def _list_documents_from_ground_truth(
    pdf_root: Path,
    ground_truth_root: Path,
) -> list[DocumentSpec]:
    specs: list[DocumentSpec] = []
    if not ground_truth_root.exists():
        return specs

    for md_path in sorted(ground_truth_root.rglob("*.md")):
        relative = md_path.relative_to(ground_truth_root)
        pdf_candidate = (pdf_root / relative).with_suffix(".pdf")
        if not pdf_candidate.exists():
            continue
        specs.append(
            DocumentSpec(
                doc_id=str(relative.with_suffix("")),
                pdf_path=pdf_candidate,
                markdown_reference=md_path,
                pages=None,
            )
        )
    return specs


def _filter_documents_by_name(
    documents: Sequence[DocumentSpec],
    selectors: Sequence[str],
) -> tuple[list[DocumentSpec], set[str]]:
    normalized: list[tuple[str, str]] = []
    matched: dict[str, bool] = {}
    for selector in selectors:
        if not selector:
            continue
        lowered = selector.lower()
        normalized.append((selector, lowered))
        matched.setdefault(lowered, False)

    if not normalized:
        return list(documents), set()

    filtered: list[DocumentSpec] = []
    for spec in documents:
        candidates = {
            spec.doc_id.lower(),
            spec.pdf_path.stem.lower(),
            spec.pdf_path.name.lower(),
            str(spec.pdf_path).lower(),
        }
        doc_matched = False
        for _, lowered in normalized:
            if matched[lowered]:
                if any(lowered in candidate for candidate in candidates):
                    doc_matched = True
                continue
            if any(lowered in candidate for candidate in candidates):
                matched[lowered] = True
                doc_matched = True
        if doc_matched:
            filtered.append(spec)

    unmatched = {original for original, lowered in normalized if not matched.get(lowered, False)}
    return filtered, unmatched


def _tokenize_plain_text(text: str) -> Counter[str]:
    normalized = text.lower().replace("\u2013", "-").replace("\u2014", "-")
    normalized = "".join(ch if (ch >= " " or ch in "\n\r\t") else "" for ch in normalized)
    translation_table = str.maketrans(dict.fromkeys("()[],.;:+`", " "))
    normalized = normalized.translate(translation_table)
    tokens = normalized.split()
    return Counter(tokens)


def _numeric_view(tokens: Counter[str]) -> Counter[str]:
    numeric_tokens: Counter[str] = Counter()
    for token, count in tokens.items():
        stripped = token.strip("()[]{}")
        if not any(ch.isdigit() for ch in stripped):
            continue
        if any(ch.isalpha() for ch in stripped):
            continue
        numeric_tokens[stripped] += count
    return numeric_tokens


def _token_scores(truth: Counter[str], candidate: Counter[str]) -> EvaluationScores:
    truth_total = sum(truth.values())
    candidate_total = sum(candidate.values())
    if truth_total == 0 and candidate_total == 0:
        return EvaluationScores(precision=1.0, recall=1.0, f1=1.0)
    overlap = sum(min(truth[token], candidate[token]) for token in truth)
    precision = overlap / candidate_total if candidate_total else 0.0
    recall = overlap / truth_total if truth_total else 0.0
    f1 = 0.0 if precision + recall == 0 else 2 * precision * recall / (precision + recall)
    return EvaluationScores(precision=precision, recall=recall, f1=f1)


def _normalize_markdown_tokens(markdown: str) -> Counter[str]:
    normalized = markdown.replace("|", " ")
    normalized = normalized.lower().replace("\u2013", "-").replace("\u2014", "-")
    normalized = "".join(ch if (ch >= " " or ch in "\n\r\t") else "" for ch in normalized)
    translation_table = str.maketrans(dict.fromkeys("()[],.;:+`", " "))
    normalized = normalized.translate(translation_table)
    return Counter(normalized.split())


def _markdown_structure_stats(markdown: str) -> MarkdownStructureStats:
    headings: Counter[int] = Counter()
    bullet = 0
    ordered = 0
    table_rows = 0
    code_fences = 0
    lines = 0

    for line in markdown.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        lines += 1

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
        headings=dict(headings),
        bullet=bullet,
        ordered=ordered,
        table_rows=table_rows,
        code_fences=code_fences,
        lines=lines,
    )


def _relative_delta(observed: int, expected: int) -> float:
    if expected == 0 and observed == 0:
        return 0.0
    if expected == 0:
        return 1.0
    delta = abs(observed - expected) / expected
    return min(delta, 1.0)


def _structure_delta(reference: MarkdownStructureStats, candidate: MarkdownStructureStats) -> float:
    heading_levels = set(reference.headings) | set(candidate.headings)
    heading_deltas = [
        _relative_delta(candidate.headings.get(level, 0), reference.headings.get(level, 0)) for level in heading_levels
    ]
    aggregate_deltas = [
        _relative_delta(candidate.bullet, reference.bullet),
        _relative_delta(candidate.ordered, reference.ordered),
        _relative_delta(candidate.table_rows, reference.table_rows),
        _relative_delta(candidate.code_fences, reference.code_fences),
        _relative_delta(candidate.lines, reference.lines),
    ]
    deltas = heading_deltas + aggregate_deltas
    return sum(deltas) / len(deltas) if deltas else 0.0


def _structure_stats_to_dict(stats: MarkdownStructureStats | None) -> dict[str, int] | None:
    if stats is None:
        return None
    return {
        "headings": dict(stats.headings),
        "bullet": stats.bullet,
        "ordered": stats.ordered,
        "table_rows": stats.table_rows,
        "code_fences": stats.code_fences,
        "lines": stats.lines,
    }


def _page_indices(
    pdf: pdfium.PdfDocument,
    pages: Sequence[int] | None,
    max_pages: int | None,
) -> tuple[int, ...]:
    indices = tuple(pages) if pages is not None else tuple(range(len(pdf)))
    if max_pages is not None:
        indices = indices[: max(0, max_pages)]
    return indices


def _load_pdf_text(
    pdf_path: Path,
    pages: Sequence[int] | None,
    max_pages: int | None,
) -> tuple[list[str], tuple[int, ...]]:
    pdf = pdfium.PdfDocument(str(pdf_path))
    try:
        indices = _page_indices(pdf, pages, max_pages)
        texts: list[str] = []
        for page_number in indices:
            page = pdf[page_number]
            try:
                text_page = page.get_textpage()
                try:
                    texts.append(text_page.get_text_bounded())
                finally:
                    text_page.close()
            finally:
                page.close()
        return texts, indices
    finally:
        pdf.close()


def _render_pages(pdf_path: Path, indices: Sequence[int], dpi: int) -> list[Image.Image]:
    pdf = pdfium.PdfDocument(str(pdf_path))
    try:
        images: list[Image.Image] = []
        scale = dpi / 72.0
        for page_number in indices:
            page = pdf[page_number]
            try:
                images.append(page.render(scale=scale).to_pil())
            finally:
                page.close()
        return images
    finally:
        pdf.close()


def _layout_lines(pages: Iterable[str]) -> int:
    return sum(1 for page in pages for line in page.splitlines() if line.strip())


def _load_reference_markdown(spec: DocumentSpec, mode: str) -> tuple[str | None, str | None]:
    if mode == "ground_truth" and spec.markdown_reference and spec.markdown_reference.exists():
        return spec.markdown_reference.read_text(encoding="utf-8"), "ground_truth"

    tables_config: TableExtractionConfigType | None = None
    if TableExtractionConfigType is not None:
        tables_config = TableExtractionConfigType(extract_from_ocr=True)

    try:
        result = extract_file_sync(
            str(spec.pdf_path),
            config=ExtractionConfig(ocr=None, tables=tables_config, use_cache=False),
        )
    except (MissingDependencyError, ModuleNotFoundError, RuntimeError, ValueError):
        return None, None

    if not result.content:
        return None, None

    if result.tables:
        table_markdown = "\n\n".join(table.get("text", "") for table in result.tables if table.get("text")).strip()
        if table_markdown:
            return table_markdown, "extracted"
    return result.content, "extracted"


def evaluate_document(
    spec: DocumentSpec,
    *,
    markdown_reference_mode: str,
    evaluate_markdown: bool,
    max_pages: int | None,
) -> DocumentEvaluation:
    """Evaluate OCR quality metrics for a single document specification."""
    notes: list[str] = []
    try:
        page_texts, indices = _load_pdf_text(spec.pdf_path, spec.pages, max_pages)
    except pdfium.PdfiumError as exc:  # pragma: no cover - depends on external data
        notes.append(f"pdf_load_error:{exc}")
        return DocumentEvaluation(
            doc_id=spec.doc_id,
            pdf_path=spec.pdf_path,
            page_count=None,
            text_score=None,
            numeric_score=None,
            layout_truth_lines=None,
            layout_ocr_lines=None,
            layout_delta=None,
            markdown_score=None,
            markdown_structure_delta=None,
            markdown_reference_source=None,
            markdown_structure_truth=None,
            markdown_structure_candidate=None,
            notes=tuple(notes),
        )

    backend = TesseractBackend()
    images = _render_pages(spec.pdf_path, indices, spec.dpi)

    ocr_text_pages: list[str] = []
    ocr_markdown_pages: list[str] = []
    for image in images:
        text_result = backend.process_image_sync(
            image,
            output_format="text",
            enable_table_detection=False,
        )
        ocr_text_pages.append(text_result.content)

        if evaluate_markdown:
            markdown_result = backend.process_image_sync(
                image,
                output_format="markdown",
                enable_table_detection=True,
            )
            ocr_markdown_pages.append(markdown_result.content)

    truth_tokens = _tokenize_plain_text("\n".join(page_texts))
    ocr_tokens = _tokenize_plain_text("\n".join(ocr_text_pages))
    text_score = _token_scores(truth_tokens, ocr_tokens)

    numeric_truth = _numeric_view(truth_tokens)
    numeric_score: EvaluationScores | None
    if sum(numeric_truth.values()):
        numeric_ocr = _numeric_view(ocr_tokens)
        numeric_score = _token_scores(numeric_truth, numeric_ocr)
    else:
        numeric_score = None

    truth_lines = _layout_lines(page_texts)
    ocr_lines = _layout_lines(ocr_text_pages)
    layout_delta = abs(ocr_lines - truth_lines) / truth_lines if truth_lines else 0.0

    reference_markdown: str | None = None
    markdown_source: str | None = None
    if evaluate_markdown:
        reference_markdown, markdown_source = _load_reference_markdown(spec, markdown_reference_mode)
    markdown_score: EvaluationScores | None = None
    markdown_structure_delta: float | None = None
    structure_reference: MarkdownStructureStats | None = None
    structure_candidate: MarkdownStructureStats | None = None
    if evaluate_markdown and reference_markdown:
        markdown_truth_tokens = _normalize_markdown_tokens(reference_markdown)
        markdown_ocr_tokens = _normalize_markdown_tokens("\n".join(ocr_markdown_pages))
        markdown_score = _token_scores(markdown_truth_tokens, markdown_ocr_tokens)

        structure_reference = _markdown_structure_stats(reference_markdown)
        structure_candidate = _markdown_structure_stats("\n".join(ocr_markdown_pages))
        markdown_structure_delta = _structure_delta(structure_reference, structure_candidate)
    elif evaluate_markdown:
        markdown_source = None
        notes.append("markdown_reference_missing")

    return DocumentEvaluation(
        doc_id=spec.doc_id,
        pdf_path=spec.pdf_path,
        page_count=len(indices),
        text_score=text_score,
        numeric_score=numeric_score,
        layout_truth_lines=truth_lines,
        layout_ocr_lines=ocr_lines,
        layout_delta=layout_delta,
        markdown_score=markdown_score,
        markdown_structure_delta=markdown_structure_delta,
        markdown_reference_source=markdown_source,
        markdown_structure_truth=structure_reference,
        markdown_structure_candidate=structure_candidate,
        notes=tuple(notes),
    )


def _mean(values: Sequence[float]) -> float | None:
    return sum(values) / len(values) if values else None


def _summary(results: Sequence[DocumentEvaluation]) -> dict[str, float | int | None]:
    text_f1 = [res.text_score.f1 for res in results if res.text_score is not None]
    numeric_f1 = [res.numeric_score.f1 for res in results if res.numeric_score is not None]
    markdown_f1 = [res.markdown_score.f1 for res in results if res.markdown_score is not None]
    layout_deltas = [res.layout_delta for res in results if res.layout_delta is not None]
    structure_deltas = [res.markdown_structure_delta for res in results if res.markdown_structure_delta is not None]

    return {
        "documents_evaluated": len(results),
        "text_f1_avg": _mean(text_f1),
        "text_f1_min": min(text_f1) if text_f1 else None,
        "text_f1_max": max(text_f1) if text_f1 else None,
        "numeric_f1_avg": _mean(numeric_f1),
        "markdown_f1_avg": _mean(markdown_f1),
        "layout_delta_avg": _mean(layout_deltas),
        "layout_delta_max": max(layout_deltas) if layout_deltas else None,
        "markdown_structure_delta_avg": _mean(structure_deltas),
        "markdown_structure_delta_max": max(structure_deltas) if structure_deltas else None,
    }


def _worst_results(results: Sequence[DocumentEvaluation], count: int = 5) -> list[dict[str, float | str]]:
    scored = [res for res in results if res.text_score is not None]
    scored.sort(key=lambda res: res.text_score.f1 if res.text_score else math.inf)
    return [
        {
            "doc_id": res.doc_id,
            "pdf_path": str(res.pdf_path),
            "text_f1": res.text_score.f1 if res.text_score else None,
            "layout_delta": res.layout_delta,
            "markdown_f1": res.markdown_score.f1 if res.markdown_score else None,
            "markdown_structure_delta": res.markdown_structure_delta,
        }
        for res in scored[:count]
    ]


def _evaluation_to_dict(result: DocumentEvaluation) -> dict[str, object]:
    return {
        "doc_id": result.doc_id,
        "pdf_path": str(result.pdf_path),
        "page_count": result.page_count,
        "text_score": (
            {
                "precision": result.text_score.precision,
                "recall": result.text_score.recall,
                "f1": result.text_score.f1,
            }
            if result.text_score
            else None
        ),
        "numeric_score": (
            {
                "precision": result.numeric_score.precision,
                "recall": result.numeric_score.recall,
                "f1": result.numeric_score.f1,
            }
            if result.numeric_score
            else None
        ),
        "layout_truth_lines": result.layout_truth_lines,
        "layout_ocr_lines": result.layout_ocr_lines,
        "layout_delta": result.layout_delta,
        "markdown_score": (
            {
                "precision": result.markdown_score.precision,
                "recall": result.markdown_score.recall,
                "f1": result.markdown_score.f1,
            }
            if result.markdown_score
            else None
        ),
        "markdown_structure_delta": result.markdown_structure_delta,
        "markdown_reference_source": result.markdown_reference_source,
        "markdown_structure_truth": _structure_stats_to_dict(result.markdown_structure_truth),
        "markdown_structure_candidate": _structure_stats_to_dict(result.markdown_structure_candidate),
        "notes": list(result.notes),
    }


def _build_document_list(
    args: argparse.Namespace,
) -> list[DocumentSpec]:
    documents: list[DocumentSpec] = []

    use_ground_truth = args.use_ground_truth or bool(args.doc)
    if use_ground_truth:
        documents = _list_documents_from_ground_truth(args.pdf_root, args.ground_truth_root)
        if not documents and args.use_ground_truth:
            pass

    if not documents:
        documents = list(DEFAULT_DOCS)

    if args.doc:
        documents, unmatched = _filter_documents_by_name(documents, args.doc)
        for _selector in sorted(unmatched):
            pass
        if not documents:
            raise SystemExit("[error] No documents matched any provided --doc selector.")
    apply_sample = bool(args.sample and not args.doc)
    if args.sample and args.doc:
        pass

    if args.shard_count is not None:
        total_docs = len(documents)
        if total_docs == 0:
            return []
        documents = [doc for index, doc in enumerate(documents) if index % args.shard_count == args.shard_index]

    if apply_sample and args.limit and len(documents) > args.limit:
        random.seed(args.random_seed)
        documents = random.sample(documents, args.limit)
    elif args.limit is not None and args.limit >= 0:
        documents = documents[: args.limit]

    return documents


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments for the OCR quality report script."""
    parser = argparse.ArgumentParser(description="Evaluate OCR quality using Tesseract.")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("ocr_quality.json"),
        help="Destination JSON file for the detailed report.",
    )
    parser.add_argument(
        "--pdf-root",
        type=Path,
        default=DEFAULT_PDF_ROOT,
        help="Root directory containing source PDFs (used with --use-ground-truth).",
    )
    parser.add_argument(
        "--ground-truth-root",
        type=Path,
        default=DEFAULT_GROUND_TRUTH_ROOT,
        help="Directory containing markdown ground truth files.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Restrict evaluation to the first N documents.",
    )
    parser.add_argument(
        "--doc",
        action="append",
        dest="doc",
        help=(
            "Restrict evaluation to documents whose ID, PDF filename, or path contains the "
            "provided substring (case-insensitive). Repeatable."
        ),
    )
    parser.add_argument(
        "--sample",
        action="store_true",
        help="Randomly sample documents instead of taking the first N.",
    )
    parser.add_argument(
        "--random-seed",
        type=int,
        default=None,
        help="Random seed used with --sample.",
    )
    parser.add_argument(
        "--use-ground-truth",
        action="store_true",
        help="Evaluate all PDFs that have matching markdown files under --ground-truth-root.",
    )
    parser.add_argument(
        "--markdown-reference",
        choices=("ground_truth", "extract"),
        default="ground_truth",
        help="Source for reference markdown when computing markdown fidelity.",
    )
    parser.add_argument(
        "--skip-markdown",
        action="store_true",
        help="Disable markdown evaluation (faster, omits markdown metrics).",
    )
    parser.add_argument(
        "--max-pages",
        type=int,
        default=None,
        help="Only evaluate the first N pages of each document (disables markdown metrics).",
    )
    parser.add_argument(
        "--shard-index",
        type=int,
        default=None,
        help="Zero-based shard index when splitting the document set across multiple runs.",
    )
    parser.add_argument(
        "--shard-count",
        type=int,
        default=None,
        help="Total number of shards when splitting the document set across multiple runs.",
    )
    return parser.parse_args()


def main() -> None:
    """Entry point for running the OCR quality report."""
    args = parse_args()

    if (args.shard_index is None) ^ (args.shard_count is None):
        raise SystemExit("--shard-index and --shard-count must be provided together.")
    if args.shard_count is not None:
        if args.shard_count <= 0:
            raise SystemExit("--shard-count must be greater than zero.")
        if args.shard_index < 0 or args.shard_index >= args.shard_count:
            raise SystemExit("--shard-index must be between 0 and shard-count - 1.")

    documents = _build_document_list(args)
    if not documents:
        return

    evaluate_markdown = not args.skip_markdown
    if args.max_pages is not None:
        evaluate_markdown = False
    elif args.skip_markdown:
        pass

    if args.max_pages is not None:
        pass
    if not evaluate_markdown:
        pass

    results: list[DocumentEvaluation] = []
    for spec in documents:
        eval_result = evaluate_document(
            spec,
            markdown_reference_mode=args.markdown_reference,
            evaluate_markdown=evaluate_markdown,
            max_pages=args.max_pages,
        )
        results.append(eval_result)

    summary = _summary(results)
    worst = _worst_results(results)

    payload = {
        "config": {
            "use_ground_truth": args.use_ground_truth,
            "pdf_root": str(args.pdf_root),
            "ground_truth_root": str(args.ground_truth_root),
            "limit": args.limit,
            "sample": args.sample,
            "random_seed": args.random_seed,
            "markdown_reference": args.markdown_reference,
            "skip_markdown": args.skip_markdown,
            "max_pages": args.max_pages,
            "markdown_evaluated": evaluate_markdown,
            "shard_index": args.shard_index,
            "shard_count": args.shard_count,
        },
        "summary": summary,
        "worst_offenders": worst,
        "results": [_evaluation_to_dict(res) for res in results],
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2))

    for _key, _value in summary.items():
        pass
    if worst:
        for entry in worst:
            f"{entry['text_f1']:.3f}" if entry["text_f1"] is not None else "n/a"
            (f"{entry['layout_delta']:.3f}" if entry["layout_delta"] is not None else "n/a")
            (f"{entry['markdown_f1']:.3f}" if entry["markdown_f1"] is not None else "n/a")
            (f"{entry['markdown_structure_delta']:.3f}" if entry["markdown_structure_delta"] is not None else "n/a")


if __name__ == "__main__":
    main()
