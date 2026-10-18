//! Paragraph-block assignment for recognized sceptre lines.

use super::*;

/// Attribute key `pdf::structure::adapters::hocr_block_id` reads to merge
/// consecutive OCR lines into one `PdfParagraph`. Duplicated as a literal
/// (rather than imported) because it is also defined locally in
/// `ocr::hocr_parser` behind `feature = "ocr"`, which a sceptre-only build
/// does not enable.
pub(super) const HOCR_BLOCK_ID_ATTRIBUTE: &str = "hocr_block_id";

/// Maximum vertical gap between two consecutive sceptre lines, expressed as a
/// multiple of the running median line height seen so far, for the lines to
/// be grouped into the same paragraph block. Mirrors
/// `MAX_BLOCK_VERTICAL_GAP_IN_LINE_HEIGHTS` in `ocr::conversion`.
const BLOCK_MAX_VERTICAL_GAP_IN_LINE_HEIGHTS: f64 = 0.6;
/// Minimum fraction of the narrower of two consecutive lines' horizontal
/// extent that must overlap for them to be considered the same paragraph
/// column. Mirrors `MIN_BLOCK_HORIZONTAL_OVERLAP_RATIO` in `ocr::conversion`.
const BLOCK_MIN_HORIZONTAL_OVERLAP_RATIO: f64 = 0.3;
/// Maximum left-edge offset between two consecutive lines, expressed as a
/// multiple of the running median line height, allowed as a fallback when the
/// lines don't x-overlap enough on their own. Mirrors
/// `MAX_BLOCK_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS` in `ocr::conversion`.
const BLOCK_MAX_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS: f64 = 0.5;
/// Maximum y-center delta between two consecutive elements, expressed as a
/// multiple of the running median line height, for them to be treated as
/// fragments of the same physical text line rather than two different lines.
///
/// Sceptre's `width_ths` detection parameter under-merges letter-spaced or
/// widely-kerned runs on some documents, splitting one physical line into
/// several `TextLine` entries (see `sceptre::config::DetectionConfig::width_ths`,
/// measured on this same 16-page ordinance: 339 detected lines, 85 of them
/// single words, at the current default). Verified against Sceptre's own
/// output for `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`: a line
/// split into 9 word fragments (e.g. "In" / "accordance" / "with" / "Section"
/// / "2-133," / "the" / "PD" / "must" / "be") has consecutive y-center deltas
/// of ~1-3px against line heights of ~32-45px (ratio < 0.1), while genuinely
/// distinct lines on the same page have deltas of at least ~38px (ratio ≥ 0.5
/// even where their AABBs happen to vertically overlap). Fragments have a
/// near-zero vertical gap already, so the vertical-gap check below would let
/// them through regardless, but their narrow, non-overlapping x-extents fail
/// the horizontal overlap/left-edge check meant for paragraph continuation --
/// which produced one spurious block per word. This constant lets same-line
/// fragments skip that horizontal test entirely, mirroring the `ycenter_ths`
/// test `sceptre::detect::group::combine_into_lines` already uses internally
/// to decide the same "is this the same physical line" question.
const LINE_YCENTER_THS: f64 = 0.5;

/// Minimum first-line indent, as a multiple of the running median line height,
/// for a line to start a new paragraph block even though the vertical-gap and
/// horizontal-overlap checks would otherwise continue the current one. Mirrors
/// `MIN_PARAGRAPH_INDENT_IN_LINE_HEIGHTS` in `ocr::conversion`, which was added
/// for PaddleOCR after a 16-page scanned document produced exactly 16
/// paragraphs -- one full-page block per page -- because its paragraph breaks
/// are marked by indentation alone, with uniform leading throughout.
///
/// Unlike the PaddleOCR copy this check must NOT fire for the word fragments of
/// a single physical line (see [`LINE_YCENTER_THS`]): those march rightward
/// across the line and so look arbitrarily "indented" against the block's left
/// margin. [`assign_sceptre_block_ids`] therefore consults it only for lines
/// that are not same-line fragments.
const PARAGRAPH_MIN_INDENT_IN_LINE_HEIGHTS: f64 = 0.75;

/// Group sceptre's per-line elements into paragraph blocks using their own
/// AABB geometry, returning a stable block id per input element in the same
/// order, so `pdf::structure::adapters::ocr_doc_to_paragraphs` can merge
/// consecutive sceptre lines into one `PdfParagraph` instead of treating every
/// line as its own paragraph.
///
/// This is a deliberate, intentionally-flagged duplicate of
/// `crate::ocr::conversion::assign_line_block_ids`, which already gives
/// PaddleOCR the same paragraph grouping it otherwise lacks (#631). That
/// function is `#[cfg(paddle_ocr)]` inside a module gated on `feature =
/// "ocr"`; sceptre-ocr builds (`sceptre-ocr-ort` / `sceptre-ocr-tract`) enable
/// neither, so it cannot be called from here without pulling `feature = "ocr"`
/// into every sceptre-ocr build. Reconcile by extracting a shared, ungated
/// helper once both call sites are stable, rather than letting these two
/// copies drift apart silently.
pub(super) fn assign_sceptre_block_ids(elements: &[OcrElement]) -> Vec<String> {
    let mut block_ids = Vec::with_capacity(elements.len());
    let mut heights_seen: Vec<f64> = Vec::with_capacity(elements.len());
    let mut block_index: u32 = 0;
    let mut previous: Option<BoundingBox> = None;
    // Leftmost edge established so far by the current block's lines, used to
    // detect an indented paragraph start (see `starts_indented_paragraph`).
    let mut block_left_margin: f64 = 0.0;

    for element in elements {
        let bounds = geometry_bounds(&element.geometry).unwrap_or_default();
        insert_sorted(&mut heights_seen, bounds.y1 - bounds.y0);
        let median_line_height = running_median(&heights_seen);

        let continues_block = previous.as_ref().is_some_and(|previous| {
            lines_share_block(previous, &bounds, median_line_height)
                && (shares_physical_line(previous, &bounds, median_line_height)
                    || !starts_indented_paragraph(bounds.x0, block_left_margin, median_line_height))
        });
        if continues_block {
            block_left_margin = block_left_margin.min(bounds.x0);
        } else {
            block_index += 1;
            block_left_margin = bounds.x0;
        }
        block_ids.push(format!("sceptre-block-{block_index}"));
        previous = Some(bounds);
    }

    block_ids
}

/// Whether `current` is a fragment of the same physical text line as
/// `previous`, judged by their y-centers (see [`LINE_YCENTER_THS`]).
fn shares_physical_line(previous: &BoundingBox, current: &BoundingBox, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }
    let previous_ycenter = (previous.y0 + previous.y1) / 2.0;
    let current_ycenter = (current.y0 + current.y1) / 2.0;
    (current_ycenter - previous_ycenter).abs() < LINE_YCENTER_THS * median_line_height
}

/// Whether `x0` sits far enough right of `block_left_margin` -- the leftmost
/// edge established by the current block's lines so far -- to read as a new
/// paragraph's indented first line, in units of `median_line_height`.
///
/// This is independent of (and can override) the vertical-gap/overlap checks in
/// [`lines_share_block`], because real body text often marks a paragraph break
/// with indentation alone and no extra vertical space (see
/// [`PARAGRAPH_MIN_INDENT_IN_LINE_HEIGHTS`]).
fn starts_indented_paragraph(x0: f64, block_left_margin: f64, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }
    x0 - block_left_margin > median_line_height * PARAGRAPH_MIN_INDENT_IN_LINE_HEIGHTS
}

fn lines_share_block(previous: &BoundingBox, current: &BoundingBox, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }

    if shares_physical_line(previous, current, median_line_height) {
        return true;
    }

    let vertical_gap = if current.y0 >= previous.y1 {
        current.y0 - previous.y1
    } else if previous.y0 >= current.y1 {
        previous.y0 - current.y1
    } else {
        0.0
    };
    if vertical_gap > median_line_height * BLOCK_MAX_VERTICAL_GAP_IN_LINE_HEIGHTS {
        return false;
    }

    let overlap = (previous.x1.min(current.x1) - previous.x0.max(current.x0)).max(0.0);
    let narrower_width = (previous.x1 - previous.x0).min(current.x1 - current.x0);
    let overlap_ratio = if narrower_width > 0.0 {
        overlap / narrower_width
    } else {
        0.0
    };
    let left_edge_offset = (previous.x0 - current.x0).abs();

    overlap_ratio >= BLOCK_MIN_HORIZONTAL_OVERLAP_RATIO
        || left_edge_offset <= median_line_height * BLOCK_MAX_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS
}

fn insert_sorted(sorted: &mut Vec<f64>, value: f64) {
    let index = sorted.partition_point(|existing| *existing < value);
    sorted.insert(index, value);
}

fn running_median(sorted: &[f64]) -> f64 {
    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }
    if len % 2 == 1 {
        sorted[len / 2]
    } else {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    }
}
