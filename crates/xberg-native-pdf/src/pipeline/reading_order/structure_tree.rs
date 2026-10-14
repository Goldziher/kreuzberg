//! Structure tree reading order strategy.
//!
//! Uses PDF Tagged structure tree (ISO 32000-1:2008 Section 14.7) to determine
//! the correct reading order for text spans based on their MCID values.

use crate::error::Result;
use crate::layout::TextSpan;
use crate::pipeline::{OrderedTextSpan, ReadingOrderInfo};

use super::{ArticleThreadStrategy, ReadingOrderContext, ReadingOrderStrategy, XYCutStrategy};

const MIN_COLUMN_SIDE_LINES: usize = 4;
const COLUMN_LINE_PAIR_HEIGHT_RATIO: f32 = 1.25;
const BASELINE_DEDUP_TOLERANCE: f32 = 2.0;

/// Structure tree-based reading order strategy.
///
/// This is the PDF-spec-compliant approach for Tagged PDFs (ISO 32000-1:2008
/// Section 14.7). It uses the structure tree's pre-order traversal to determine
/// the logical reading order of marked content.
///
/// For spans without MCIDs or when no structure tree is available, it falls
/// back to the XYCutStrategy (recursive spatial partitioning).
pub struct StructureTreeStrategy {
    /// Fallback for spans without MCIDs and for MCID orderings that
    /// fail the column-respecting sanity check.
    fallback: XYCutStrategy,
}

impl StructureTreeStrategy {
    /// Construct a new strategy with a default XY-cut fallback.
    pub fn new() -> Self {
        Self {
            fallback: XYCutStrategy::new(),
        }
    }

    /// Order spans when the structure tree cannot be trusted for this page.
    ///
    /// Prefers article-thread order (`/Threads`, §12.4.3) when the canonical
    /// helper supplied bead rectangles for the page; otherwise uses the
    /// geometric XY-cut fallback. With no bead rects this is exactly the prior
    /// behaviour (fails closed).
    fn fallback_order(&self, spans: Vec<TextSpan>, context: &ReadingOrderContext) -> Result<Vec<OrderedTextSpan>> {
        if context.bead_rects.as_ref().is_some_and(|b| !b.is_empty()) {
            return ArticleThreadStrategy::new().apply(spans, context);
        }
        self.fallback.apply(spans, context)
    }
}

fn is_rtl_dominant(spans: &[TextSpan]) -> bool {
    let rtl_characters = spans
        .iter()
        .flat_map(|span| span.text.chars())
        .filter(|character| character.is_alphabetic() && crate::text::is_rtl_text(*character as u32))
        .count();
    let ltr_characters = spans
        .iter()
        .flat_map(|span| span.text.chars())
        .filter(|character| character.is_alphabetic() && !crate::text::is_rtl_text(*character as u32))
        .count();
    rtl_characters > ltr_characters
}

/// X position of the widest gap in `xs_sorted`, or `None` when it is too narrow to be a column gutter.
fn column_gutter_position(xs_sorted: &[f32], x_min: f32, x_extent: f32) -> Option<f32> {
    let mut largest_gap = 0.0_f32;
    let mut largest_gap_at = x_min;
    for w in xs_sorted.windows(2) {
        let gap = w[1] - w[0];
        if gap > largest_gap {
            largest_gap = gap;
            largest_gap_at = (w[0] + w[1]) * 0.5;
        }
    }
    // The gap must be a substantial fraction of the page width to be
    // considered a column gutter (not just inter-word whitespace). ~keep
    if largest_gap < x_extent * 0.1 || largest_gap < 30.0 {
        return None;
    }
    Some(largest_gap_at)
}

fn distinct_baselines(ordered_spans: &[&TextSpan], columns: &[u8], column: u8) -> Vec<f32> {
    let mut baselines: Vec<f32> = ordered_spans
        .iter()
        .zip(columns)
        .filter(|(_, candidate)| **candidate == column)
        .map(|(span, _)| span.bbox.y)
        .collect();
    baselines.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    baselines.into_iter().fold(Vec::<f32>::new(), |mut lines, baseline| {
        if lines
            .last()
            .is_none_or(|last| (baseline - last).abs() >= BASELINE_DEDUP_TOLERANCE)
        {
            lines.push(baseline);
        }
        lines
    })
}

fn column_lines_are_paired(ordered_spans: &[&TextSpan], left_lines: &[f32], right_lines: &[f32]) -> bool {
    // Both sides of a real column gutter recur through the same vertical
    // region. A compact table beside single-column prose can create two X
    // clusters and many spans, but not four paired text lines. ~keep
    let mut heights: Vec<f32> = ordered_spans.iter().map(|span| span.bbox.height).collect();
    heights.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let line_pair_tolerance = heights[heights.len() / 2].max(1.0) * COLUMN_LINE_PAIR_HEIGHT_RATIO;
    let paired_left_lines = left_lines
        .iter()
        .filter(|left| {
            right_lines
                .iter()
                .any(|right| (*left - right).abs() <= line_pair_tolerance)
        })
        .count();
    let paired_right_lines = right_lines
        .iter()
        .filter(|right| {
            left_lines
                .iter()
                .any(|left| (*right - left).abs() <= line_pair_tolerance)
        })
        .count();
    paired_left_lines >= MIN_COLUMN_SIDE_LINES && paired_right_lines >= MIN_COLUMN_SIDE_LINES
}

/// Detect whether applying `mcid_order` to `spans` would produce a
/// horizontal zigzag pattern — a reliable signal that MCIDs were
/// assigned in content-stream order rather than visual reading order
/// on a multi-column page.
///
/// Heuristic:
/// 1. Project span X-centers onto a 1-D histogram to detect 2+ columns
///    (requires a gap wider than the column width estimate).
/// 2. Walk the MCID-ordered sequence of spans and count how many times
///    it crosses between different column clusters.
/// 3. If crossings exceed `2 * (num_columns - 1)` the order is zigzagging
///    (a column-respecting order crosses columns only at bottom-of-one /
///    top-of-next transitions).
fn mcid_order_zigzags_columns(spans: &[TextSpan], mcid_order: &[u32]) -> bool {
    let rtl_dominant = is_rtl_dominant(spans);
    let mcid_to_idx: std::collections::HashMap<u32, usize> = spans
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.mcid.map(|m| (m, i)))
        .collect();
    let ordered_spans: Vec<&TextSpan> = mcid_order
        .iter()
        .filter_map(|m| mcid_to_idx.get(m))
        .map(|&i| &spans[i])
        .collect();
    let ordered_x: Vec<f32> = ordered_spans
        .iter()
        .map(|span| {
            if rtl_dominant {
                span.bbox.x + span.bbox.width
            } else {
                span.bbox.x
            }
        })
        .collect();
    if ordered_x.len() < 10 {
        return false;
    }

    // 1-D k-means-lite: detect if there are 2+ clusters of X positions
    // separated by a meaningful gap. ~keep
    let mut xs_sorted: Vec<f32> = ordered_x.clone();
    xs_sorted.sort_by(|a, b| crate::utils::safe_float_cmp(*a, *b));
    let x_min = xs_sorted[0];
    let x_max = xs_sorted[xs_sorted.len() - 1];
    let x_extent = x_max - x_min;
    if x_extent < 50.0 {
        return false;
    }

    let Some(largest_gap_at) = column_gutter_position(&xs_sorted, x_min, x_extent) else {
        return false;
    };

    let columns: Vec<u8> = ordered_x
        .iter()
        .map(|&x| if x < largest_gap_at { 0 } else { 1 })
        .collect();
    let left_support = columns.iter().filter(|&&column| column == 0).count();
    let right_support = columns.len() - left_support;
    if left_support < MIN_COLUMN_SIDE_LINES || right_support < MIN_COLUMN_SIDE_LINES {
        return false;
    }
    let left_lines = distinct_baselines(&ordered_spans, &columns, 0);
    let right_lines = distinct_baselines(&ordered_spans, &columns, 1);
    if left_lines.len() < MIN_COLUMN_SIDE_LINES || right_lines.len() < MIN_COLUMN_SIDE_LINES {
        return false;
    }
    if !column_lines_are_paired(&ordered_spans, &left_lines, &right_lines) {
        return false;
    }

    let crossings = columns.windows(2).filter(|w| w[0] != w[1]).count();
    // For proper column reading order: left-column finished, then a
    // SINGLE crossing to right-column. More than 3 crossings means
    // the order is interleaving columns rather than respecting them. ~keep
    crossings > 3
}

impl Default for StructureTreeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadingOrderStrategy for StructureTreeStrategy {
    fn apply(&self, spans: Vec<TextSpan>, context: &ReadingOrderContext) -> Result<Vec<OrderedTextSpan>> {
        // If structure tree has suspect content, fall back to geometric ordering
        // Per ISO 32000-1:2008 Section 14.7.1, suspects=true means the structure
        // tree may contain errors or unreliable content. ~keep
        if context.suspects {
            tracing::debug!("structure tree marked as suspect, falling back to geometric ordering");
            return self.fallback_order(spans, context);
        }

        // If no structure tree or MCID order, fall back (article threads first,
        // then geometric) — see `fallback_order`. ~keep
        let mcid_order = match &context.mcid_order {
            Some(order) if !order.is_empty() => order,
            _ => return self.fallback_order(spans, context),
        };

        // Trust-check: if the MCID ordering would zigzag horizontally
        // across a clear two-column layout, the structure tree is
        // untrustworthy for reading order (common in PDFs where the
        // authoring tool assigned MCIDs in content-stream order without
        // respecting column visual order). Fall back to geometric. ~keep
        if mcid_order_zigzags_columns(&spans, mcid_order) {
            tracing::debug!("MCID order zigzags across columns, falling back to geometric ordering");
            return self.fallback_order(spans, context);
        }

        let mcid_to_order: std::collections::HashMap<u32, usize> = mcid_order
            .iter()
            .enumerate()
            .map(|(order, &mcid)| (mcid, order))
            .collect();

        let mut with_mcid: Vec<(TextSpan, usize)> = Vec::new();
        let mut without_mcid: Vec<TextSpan> = Vec::new();

        for span in spans {
            if let Some(mcid) = span.mcid {
                if let Some(&order) = mcid_to_order.get(&mcid) {
                    with_mcid.push((span, order));
                } else {
                    without_mcid.push(span);
                }
            } else {
                without_mcid.push(span);
            }
        }

        with_mcid.sort_by_key(|(_, order)| *order);

        let mut result = Vec::new();
        let mut reading_order = 0;

        for (span, _) in with_mcid {
            result.push(OrderedTextSpan::with_info(
                span,
                reading_order,
                ReadingOrderInfo::structure_tree(),
            ));
            reading_order += 1;
        }

        if !without_mcid.is_empty() {
            let untagged_ordered = self.fallback.apply(without_mcid, context)?;
            for mut ordered_span in untagged_ordered {
                ordered_span.reading_order = reading_order;
                ordered_span.order_info = ReadingOrderInfo::fallback();
                result.push(ordered_span);
                reading_order += 1;
            }
        }

        Ok(result)
    }

    fn name(&self) -> &'static str {
        "StructureTreeStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight};

    fn make_span(text: &str, x: f32, y: f32, mcid: Option<u32>) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: Rect::new(x, y, 50.0, 12.0),
            font_name: "Test".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color::black(),
            mcid,
            mcid_scope: None,
            sequence: 0,
            offset_semantic: false,
            split_boundary_before: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    #[test]
    fn test_structure_tree_ordering() {
        let spans = vec![
            make_span("Third", 0.0, 100.0, Some(2)),
            make_span("First", 0.0, 50.0, Some(0)),
            make_span("Second", 0.0, 75.0, Some(1)),
        ];

        let strategy = StructureTreeStrategy::new();
        let context = ReadingOrderContext::new().with_mcid_order(vec![0, 1, 2]);
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "First");
        assert_eq!(ordered[1].span.text, "Second");
        assert_eq!(ordered[2].span.text, "Third");
    }

    #[test]
    fn test_fallback_for_untagged() {
        let spans = vec![
            make_span("Tagged", 0.0, 100.0, Some(0)),
            make_span("Untagged", 0.0, 50.0, None),
        ];

        let strategy = StructureTreeStrategy::new();
        let context = ReadingOrderContext::new().with_mcid_order(vec![0]);
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "Tagged");
        assert_eq!(ordered[1].span.text, "Untagged");
    }

    #[test]
    fn test_no_structure_tree_fallback() {
        let spans = vec![make_span("Bottom", 0.0, 50.0, None), make_span("Top", 0.0, 100.0, None)];

        let strategy = StructureTreeStrategy::new();
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        assert_eq!(ordered[0].span.text, "Top");
        assert_eq!(ordered[1].span.text, "Bottom");
    }

    #[test]
    fn test_geometric_fallback_multi_column() {
        let spans = vec![
            // Left column - multiple words with small gaps
            // (each make_span emits a 50pt-wide span; first three
            // share Y=100 and stride 5pt apart so the left column's
            // total X extent is 0..60pt + 50pt span width = 0..110pt,
            // which clears the MIN_RESULT_WIDTH_PT = 60pt floor that
            // find_horizontal_split applies to reject sliver columns). ~keep
            make_span("Left Top Word1", 0.0, 100.0, None),
            make_span("Left Top Word2", 5.0, 100.0, None),
            make_span("Left Top Word3", 10.0, 100.0, None),
            make_span("Left Bottom", 0.0, 50.0, None),
            // Right column (gap >> word gaps). Two spans at x=200
            // give a right-column extent of 200..250 = 50pt — below
            // the 60pt floor. Add a second word so the right column
            // has extent 200..305 = 105pt, well above the floor. ~keep
            make_span("Right Top", 200.0, 100.0, None),
            make_span("Right Top2", 255.0, 100.0, None),
            make_span("Right Bottom", 200.0, 50.0, None),
            make_span("Right Bottom2", 255.0, 50.0, None),
        ];

        let mut strategy = StructureTreeStrategy::new();
        strategy.fallback = XYCutStrategy::new().with_prefer_horizontal(true);
        let context = ReadingOrderContext::new();
        let ordered = strategy.apply(spans, &context).unwrap();

        let left_indices: Vec<_> = ordered
            .iter()
            .enumerate()
            .filter(|(_, s)| s.span.text.starts_with("Left"))
            .map(|(i, _)| i)
            .collect();
        let right_indices: Vec<_> = ordered
            .iter()
            .enumerate()
            .filter(|(_, s)| s.span.text.starts_with("Right"))
            .map(|(i, _)| i)
            .collect();

        assert!(
            left_indices.iter().all(|&l| right_indices.iter().all(|&r| l < r)),
            "Left column should be processed before right column"
        );
    }

    #[test]
    fn test_suspects_fallback_to_geometric() {
        let spans = vec![
            make_span("StructOrder2", 0.0, 100.0, Some(1)),
            make_span("StructOrder1", 0.0, 50.0, Some(0)),
        ];

        let strategy = StructureTreeStrategy::new();

        let context = ReadingOrderContext::new()
            .with_mcid_order(vec![0, 1])
            .with_suspects(false);
        let ordered = strategy.apply(spans.clone(), &context).unwrap();
        assert_eq!(ordered[0].span.text, "StructOrder1");
        assert_eq!(ordered[1].span.text, "StructOrder2");

        let context = ReadingOrderContext::new()
            .with_mcid_order(vec![0, 1])
            .with_suspects(true);
        let ordered = strategy.apply(spans, &context).unwrap();
        assert_eq!(ordered[0].span.text, "StructOrder2");
        assert_eq!(ordered[1].span.text, "StructOrder1");
    }

    #[test]
    fn rtl_ragged_lines_do_not_trigger_ltr_column_zigzag_rejection() {
        let spans: Vec<_> = (0..12)
            .map(|mcid| {
                let x = 100.0 + (mcid % 4) as f32 * 80.0;
                let mut span = make_span("עברית", x, 700.0 - mcid as f32 * 12.0, Some(mcid));
                span.bbox.width = 550.0 - x;
                span
            })
            .collect();
        let order: Vec<u32> = (0..12).collect();

        assert!(!mcid_order_zigzags_columns(&spans, &order));
    }

    #[test]
    fn rtl_interleaved_columns_still_trigger_zigzag_rejection() {
        let spans: Vec<_> = (0..12)
            .map(|mcid| {
                let right_edge = if mcid % 2 == 0 { 280.0 } else { 560.0 };
                let mut span = make_span("עברית", right_edge - 120.0, 700.0 - mcid as f32 * 12.0, Some(mcid));
                span.bbox.width = 120.0;
                span
            })
            .collect();
        let order: Vec<u32> = (0..12).collect();

        assert!(mcid_order_zigzags_columns(&spans, &order));
    }

    #[test]
    fn rtl_large_type_columns_allow_font_relative_baseline_misalignment() {
        let spans: Vec<_> = (0..12)
            .map(|mcid| {
                let column = mcid % 2;
                let row = mcid / 2;
                let right_edge = if column == 0 { 280.0 } else { 560.0 };
                let y = 700.0 - row as f32 * 48.0 - column as f32 * 22.0;
                let mut span = make_span("עברית", right_edge - 120.0, y, Some(mcid));
                span.bbox.width = 120.0;
                span.bbox.height = 20.0;
                span
            })
            .collect();
        let order: Vec<u32> = (0..12).collect();

        assert!(mcid_order_zigzags_columns(&spans, &order));
    }

    #[test]
    fn ltr_interleaved_columns_still_trigger_zigzag_rejection() {
        let spans: Vec<_> = (0..12)
            .map(|mcid| {
                let x = if mcid % 2 == 0 { 100.0 } else { 500.0 };
                make_span("English", x, 700.0 - mcid as f32 * 12.0, Some(mcid))
            })
            .collect();
        let order: Vec<u32> = (0..12).collect();

        assert!(mcid_order_zigzags_columns(&spans, &order));
    }

    #[test]
    fn incidental_rtl_text_does_not_disable_ltr_column_rejection() {
        let mut spans: Vec<_> = (0..12)
            .map(|mcid| {
                let x = if mcid % 2 == 0 { 100.0 } else { 500.0 };
                make_span("English prose", x, 700.0 - mcid as f32 * 12.0, Some(mcid))
            })
            .collect();
        spans[0].text.push_str(" עברית");
        let order: Vec<u32> = (0..12).collect();

        assert!(mcid_order_zigzags_columns(&spans, &order));
    }
}
