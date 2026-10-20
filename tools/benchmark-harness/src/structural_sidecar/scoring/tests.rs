use super::*;
use crate::markdown_quality::ORDER_SCORE_FLOOR;

/// A representative document exercising every scored dimension.
const SAMPLE: &str = "\
# Introduction

Opening paragraph with several words describing the overall context here.

## Background

Background prose paragraph explaining the prior work and the motivation clearly.

### Details

- First bullet point item
- Second bullet point item
    1. Nested ordered alpha
    2. Nested ordered beta

| Name | Age | City |
|------|-----|------|
| Alice | 30 | Berlin |
| Bob | 25 | Munich |

![architecture](arch.png)

Figure 1: The overall system architecture and its components.
";

fn baseline() -> StructuralSidecar {
    StructuralSidecar::from_markdown(SAMPLE)
}

fn sf1(pred: &StructuralSidecar, gt: &StructuralSidecar) -> f64 {
    score_structural(pred, gt).sf1
}

#[test]
fn f1_parts_from_splits_precision_and_recall() {
    assert_eq!(f1_parts_from(0.0, 0, 0), (1.0, 1.0, 1.0));
    assert_eq!(f1_parts_from(0.0, 3, 0), (0.0, 0.0, 0.0));
    let (f1, precision, recall) = f1_parts_from(1.0, 4, 1);
    assert!((precision - 0.25).abs() < 1e-9, "precision {precision}");
    assert!((recall - 1.0).abs() < 1e-9, "recall {recall}");
    assert!(precision < recall, "over-emission should read as low precision");
    assert!(f1 > 0.0 && f1 < 1.0);
}

#[test]
fn breakdown_reads_fabrication_as_low_precision() {
    let gt = StructuralSidecar::from_markdown("# Real\n\nBody paragraph text here.\n");
    let pred = StructuralSidecar::from_markdown(
        "# Real\n\n## Fabricated one\n\n## Fabricated two\n\n## Fabricated three\n\nBody paragraph text here.\n",
    );
    let score = score_structural(&pred, &gt);
    let (name, heading) = score.dimensions_pr()[1];
    assert_eq!(name, "heading");
    assert!(
        heading.precision < heading.recall,
        "fabricated headings should depress precision below recall: p{} r{}",
        heading.precision,
        heading.recall
    );
}

#[test]
fn breakdown_reads_omission_as_low_recall() {
    let gt =
        StructuralSidecar::from_markdown("# Real\n\n## Second\n\n## Third\n\n## Fourth\n\nBody paragraph text here.\n");
    let pred = StructuralSidecar::from_markdown("# Real\n\nBody paragraph text here.\n");
    let heading = score_structural(&pred, &gt).dimensions_pr()[1].1;
    assert!(
        heading.recall < heading.precision,
        "dropped headings should depress recall below precision: p{} r{}",
        heading.precision,
        heading.recall
    );
}

#[test]
fn grits_con_ignores_cell_position() {
    let cell = |row, col, text: &str| Cell {
        row,
        col,
        rowspan: 1,
        colspan: 1,
        is_header: false,
        text: text.to_string(),
    };
    let gt = TableNode {
        n_rows: 1,
        n_cols: 2,
        header_rows: 0,
        cells: vec![cell(0, 0, "alpha"), cell(0, 1, "beta")],
        spans_recoverable: false,
    };
    let pred = TableNode {
        n_rows: 2,
        n_cols: 1,
        header_rows: 0,
        cells: vec![cell(0, 0, "beta"), cell(1, 0, "alpha")],
        spans_recoverable: false,
    };
    let topology = grits(&pred, &gt);
    let content = grits_con(&pred, &gt);
    assert!(
        content > topology,
        "content F1 {content} should exceed topology F1 {topology} for correct-but-misplaced cells"
    );
    assert!(
        (content - 1.0).abs() < 1e-9,
        "identical content should score 1.0, got {content}"
    );
}

#[test]
fn table_content_dimension_is_reported_and_folded() {
    let score = score_markdown(SAMPLE, SAMPLE);
    let dims = score.dimensions();
    assert_eq!(dims.len(), 7);
    assert_eq!(dims[6].0, "table_content");
    assert!(
        (score.d6_table_content - 1.0).abs() < 1e-9,
        "identical doc ⇒ content F1 1.0"
    );
    assert!((score.sf1 - 1.0).abs() < 1e-9);
}

#[test]
fn folding_table_content_lifts_sf1_above_topology_alone() {
    // Content is fully recovered but two data cells are transposed: the
    // position-locked topology dimension (d3, GriTS-Top) is penalised while
    // the position-free content dimension (d6, GriTS-Con) stays 1.0. For a
    // table-only doc the reading-order fold is a no-op (<3 matched blocks),
    // so sf1 is the weighted mean of (d3, d6). If D6 were NOT folded, sf1
    // would equal d3; asserting sf1 > d3 proves the fold is live and would
    // fail if the D6 rollup term were removed. ~keep
    const GT: &str = "\
| H1 | H2 |
|----|----|
| a | b |
| c | d |
";
    const PRED_TRANSPOSED: &str = "\
| H1 | H2 |
|----|----|
| a | c |
| b | d |
";
    let score = score_markdown(PRED_TRANSPOSED, GT);
    assert!(
        score.d3_table < 1.0,
        "transposed cells must lower topology, got d3 {}",
        score.d3_table
    );
    assert!(
        (score.d6_table_content - 1.0).abs() < 1e-9,
        "content set is identical, d6 must be 1.0, got {}",
        score.d6_table_content
    );
    assert!(
        score.sf1 > score.d3_table + 1e-9,
        "folding d6 must lift sf1 above topology-only: sf1 {} vs d3 {}",
        score.sf1,
        score.d3_table
    );
}

#[test]
fn test_parser_extracts_all_dimensions() {
    let s = baseline();
    assert!(
        heading_infos(&s).len() >= 3,
        "expected >=3 headings, got {}",
        heading_infos(&s).len()
    );
    assert!(
        list_infos(&s).len() >= 4,
        "expected >=4 list items, got {}",
        list_infos(&s).len()
    );
    assert_eq!(tables(&s).len(), 1, "expected exactly one table");
    assert!(!edges(&s).is_empty(), "expected a caption binding edge");
    let max_depth = list_infos(&s).iter().map(|l| l.depth).max().unwrap();
    assert!(max_depth >= 1, "nested list depth not captured: {max_depth}");
    assert!(list_infos(&s).iter().any(|l| l.ordered), "ordered item not captured");
}

#[test]
fn test_math_fence_is_a_display_formula() {
    let s = StructuralSidecar::from_markdown("```math\nE = mc^2\n```\n");
    assert!(
        matches!(&s.nodes[..], [StructuralNode::Formula { display: true, text }] if text == "E = mc^2"),
        "{:?}",
        s.nodes
    );
}

#[test]
fn test_math_fence_in_a_list_item_is_a_display_formula() {
    let s = StructuralSidecar::from_markdown("- Energy\n\n  ```math\n  E = mc^2\n  ```\n");
    assert!(
        matches!(
            &s.nodes[..],
            [StructuralNode::Formula { display: true, text }, StructuralNode::ListItem { text: label, .. }]
                if text == "E = mc^2" && label == "Energy"
        ),
        "{:?}",
        s.nodes
    );
}

#[test]
fn test_code_math_is_inline_math() {
    let s = StructuralSidecar::from_markdown("$`x^2`$\n");
    assert!(
        matches!(&s.nodes[..], [StructuralNode::Paragraph { text }] if text == "x^2"),
        "{:?}",
        s.nodes
    );
}

#[test]
fn test_gfm_table_spans_not_recoverable() {
    let s = baseline();
    let t = tables(&s)[0];
    assert!(!t.spans_recoverable, "GFM pipe tables cannot express spans");
    assert!(t.cells.iter().all(|c| c.rowspan == 1 && c.colspan == 1));
    assert_eq!((t.n_rows, t.n_cols), (3, 3));
    let positions: HashSet<(usize, usize)> = t.cells.iter().map(|c| (c.row, c.col)).collect();
    assert_eq!(positions.len(), t.cells.len(), "table cell origins must be unique");
}

#[test]
fn test_identity_is_one() {
    let s = baseline();
    assert!(
        (sf1(&s, &s) - 1.0).abs() < 1e-9,
        "identity must be 1.0, got {}",
        sf1(&s, &s)
    );
}

#[test]
fn test_identity_via_json_roundtrip() {
    let s = baseline();
    let json = s.to_json().unwrap();
    let back: StructuralSidecar = serde_json::from_str(&json).unwrap();
    assert!((sf1(&back, &s) - 1.0).abs() < 1e-9);
}

fn assert_drops(perturbed: &StructuralSidecar, label: &str) {
    let gt = baseline();
    let score = sf1(perturbed, &gt);
    assert!(score < 1.0 - 1e-9, "{label} must score below identity, got {score}");
}

fn drop_first_heading(mut s: StructuralSidecar) -> StructuralSidecar {
    let idx = s
        .nodes
        .iter()
        .position(|n| matches!(n, StructuralNode::Heading { .. }))
        .unwrap();
    s.nodes.remove(idx);
    s.reading_order = (0..s.nodes.len()).collect();
    s
}

fn flatten_headings(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::Heading {
            level, path, parent, ..
        } = n
        {
            *level = 1;
            path.clear();
            *parent = None;
        }
    }
    s
}

fn unnest_list(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::ListItem { depth, parent_item, .. } = n {
            *depth = 0;
            *parent_item = None;
        }
    }
    s
}

fn flip_ordered(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::ListItem { ordered, .. } = n {
            *ordered = !*ordered;
        }
    }
    s
}

fn merge_table_row(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::Table(t) = n {
            let last_row = t.n_rows.saturating_sub(1);
            t.cells.retain(|c| c.row != last_row);
            t.n_rows = last_row;
        }
    }
    s
}

fn corrupt_rowspan(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::Table(t) = n
            && let Some(cell) = t.cells.first_mut()
        {
            cell.rowspan = 2;
            t.spans_recoverable = true;
        }
    }
    s
}

fn fabricate_table(mut s: StructuralSidecar) -> StructuralSidecar {
    s.nodes.push(StructuralNode::Table(TableNode {
        n_rows: 2,
        n_cols: 2,
        header_rows: 1,
        spans_recoverable: false,
        cells: vec![
            Cell {
                row: 0,
                col: 0,
                rowspan: 1,
                colspan: 1,
                is_header: true,
                text: "Q".into(),
            },
            Cell {
                row: 0,
                col: 1,
                rowspan: 1,
                colspan: 1,
                is_header: true,
                text: "R".into(),
            },
            Cell {
                row: 1,
                col: 0,
                rowspan: 1,
                colspan: 1,
                is_header: false,
                text: "99".into(),
            },
            Cell {
                row: 1,
                col: 1,
                rowspan: 1,
                colspan: 1,
                is_header: false,
                text: "88".into(),
            },
        ],
    }));
    s.reading_order = (0..s.nodes.len()).collect();
    s
}

fn transpose_table(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::Table(t) = n {
            for c in &mut t.cells {
                std::mem::swap(&mut c.row, &mut c.col);
            }
            std::mem::swap(&mut t.n_rows, &mut t.n_cols);
        }
    }
    s
}

fn unbind_caption(mut s: StructuralSidecar) -> StructuralSidecar {
    for n in &mut s.nodes {
        if let StructuralNode::Caption { binds_to, .. } = n {
            *binds_to = None;
        }
    }
    s
}

fn scramble_reading_order(mut s: StructuralSidecar) -> StructuralSidecar {
    s.reading_order.reverse();
    s
}

#[test]
fn test_drop_heading_drops() {
    assert_drops(&drop_first_heading(baseline()), "drop-heading");
}

#[test]
fn test_flatten_headings_drops() {
    assert_drops(&flatten_headings(baseline()), "flatten-headings");
}

#[test]
fn test_unnest_list_drops() {
    assert_drops(&unnest_list(baseline()), "un-nest-list");
}

#[test]
fn test_flip_ordered_drops() {
    assert_drops(&flip_ordered(baseline()), "flip-ordered");
}

#[test]
fn test_merge_table_row_drops() {
    assert_drops(&merge_table_row(baseline()), "merge-table-row");
}

#[test]
fn test_corrupt_rowspan_drops() {
    assert_drops(&corrupt_rowspan(baseline()), "corrupt-rowspan");
}

#[test]
fn test_fabricate_table_drops() {
    let gt = baseline();
    let fabricated = fabricate_table(baseline());
    let base = sf1(&gt, &gt);
    let after = sf1(&fabricated, &gt);
    assert!(
        after < base - 1e-6,
        "fabricated table must drop the score: base={base} after={after}"
    );
}

#[test]
fn test_transpose_table_drops() {
    assert_drops(&transpose_table(baseline()), "transpose-table");
}

#[test]
fn test_unbind_caption_drops() {
    assert_drops(&unbind_caption(baseline()), "unbind-caption");
}

#[test]
fn test_scramble_reading_order_is_raw_times_floor() {
    // Only reading order changes, so the content-structure base is untouched;
    // a fully reversed order collapses the LIS toward the ORDER_SCORE_FLOOR. ~keep
    let gt = baseline();
    let scrambled = scramble_reading_order(baseline());
    let ordered = sf1(&gt, &gt);
    let scrambled_score = sf1(&scrambled, &gt);
    assert!(
        scrambled_score < ordered,
        "scramble must lower the score: {scrambled_score} !< {ordered}"
    );
    // Lands at the floor (raw · 0.8), modulo the 1/n residue of the LIS. ~keep
    let floor = ordered * ORDER_SCORE_FLOOR;
    assert!(
        scrambled_score >= floor - 1e-9 && scrambled_score <= floor + 0.06,
        "scrambled ({scrambled_score}) should approximate raw·{ORDER_SCORE_FLOOR} = {floor}"
    );
}

#[test]
fn test_monotonic_degradation_chain() {
    let gt = baseline();
    let steps: Vec<StructuralSidecar> = {
        let s0 = baseline();
        let s1 = flatten_headings(s0.clone());
        let s2 = unnest_list(s1.clone());
        let s3 = flip_ordered(s2.clone());
        let s4 = merge_table_row(s3.clone());
        let s5 = unbind_caption(s4.clone());
        vec![s0, s1, s2, s3, s4, s5]
    };
    let scores: Vec<f64> = steps.iter().map(|s| sf1(s, &gt)).collect();
    for w in scores.windows(2) {
        assert!(w[1] <= w[0] + 1e-9, "monotonicity violated: {:?} then {:?}", w[0], w[1]);
    }
    assert!((scores[0] - 1.0).abs() < 1e-9, "chain must start at identity");
    assert!(scores.last().unwrap() < &scores[0], "chain must end below identity");
}

#[test]
fn test_fabricated_table_d3_is_zero_when_gt_has_none() {
    let gt_md = "# Title\n\nJust prose here, no tables at all in the ground truth.\n";
    let pred_md =
        "# Title\n\nJust prose here, no tables at all in the ground truth.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
    let gt = StructuralSidecar::from_markdown(gt_md);
    let pred = StructuralSidecar::from_markdown(pred_md);
    let score = score_structural(&pred, &gt);
    assert_eq!(score.d3_table, 0.0, "fabricated table must score D3=0");
    assert!(score.sf1 < score_structural(&gt, &gt).sf1, "fabrication must lower SF1");
}

#[test]
fn test_empty_docs_score_one() {
    let empty = StructuralSidecar::default();
    assert!((sf1(&empty, &empty) - 1.0).abs() < 1e-9);
}

// --- BUG 1: score_order must not match across node kinds. ---

#[test]
fn score_order_does_not_match_across_kinds() {
    // The table's flattened cell text ("Revenue Growth Q1 Q2") is textually
    // similar to the standalone paragraph ("Revenue growth was strong in Q1
    // and Q2"), and cross-kind matching would previously let `greedy_match`
    // a pairing no other dimension would ever form.
    const GT: &str = "\
Revenue growth was strong in Q1 and Q2.

Unrelated closing remarks about the fiscal year follow here.

| Revenue | Growth | Q1 | Q2 |
|---------|--------|----|----|
| Total | High | 10 | 20 |
";
    // Predicted document reproduces the same nodes but in a different order:
    // the table now comes first (before both paragraphs). If the order
    // dimension matched across kinds, the table's cell text could steal the
    // paragraph's slot in the greedy match and distort the LIS pairs.
    const PRED: &str = "\
| Revenue | Growth | Q1 | Q2 |
|---------|--------|----|----|
| Total | High | 10 | 20 |

Revenue growth was strong in Q1 and Q2.

Unrelated closing remarks about the fiscal year follow here.
";
    let gt = StructuralSidecar::from_markdown(GT);
    let pred = StructuralSidecar::from_markdown(PRED);

    let (_order_score, matched) = score_order(&pred, &gt);
    // Same-kind matching must produce exactly one pair per kind: one table
    // pair and two paragraph pairs (3 total) — never a cross-kind pairing
    // that would inflate or collapse the matched-pair count.
    assert_eq!(matched, 3, "expected 3 same-kind matches (1 table + 2 paragraphs)");

    // With the table reordered to the front, the paragraph order relative to
    // the table is now inverted vs GT; the kind-restricted LIS must detect
    // this instead of being masked or corrupted by a cross-kind table/paragraph
    let (order_score, _) = score_order(&pred, &gt);
    assert!(
        order_score < 1.0,
        "reordering the table ahead of the paragraphs must lower the order score, got {order_score}"
    );
}

// --- BUG 2: vacuous SF1 must not default to 1.0 when GT has ungraded content. ---

#[test]
fn vacuous_sf1_is_not_one_when_gt_has_ungraded_content() {
    // GT's caption has no preceding Image/Table/Figure, so the deterministic
    // nearest-preceding binder leaves it unbound (`binds_to: None`). An
    // unbound Caption/Footnote is not a Paragraph/Formula/Image/Figure (see
    // `paragraph_texts`), so before the BUG3 fix it was invisible to every
    // real, gradeable content. Pred captures none of it.
    const GT: &str = "Figure 1: The overall system architecture and its components.\n";
    let gt = StructuralSidecar::from_markdown(GT);
    let pred = StructuralSidecar::from_markdown("Something totally unrelated and wrong.\n");

    let score = score_structural(&pred, &gt);
    assert!(
        score.sf1 < 0.5,
        "GT has gradeable content pred entirely missed; sf1 must not be inflated, got {}",
        score.sf1
    );
}

#[test]
fn genuinely_empty_docs_still_score_one() {
    // Both pred and GT have zero nodes: a true vacuous match, distinct from
    // the "content exists but wasn't gated" case above.
    let empty = StructuralSidecar::default();
    assert!((sf1(&empty, &empty) - 1.0).abs() < 1e-9);
}

// --- BUG 3: an unbound GT caption/footnote must be a gradeable recall opportunity. ---

#[test]
fn unbound_gt_caption_lowers_edge_recall_instead_of_vanishing() {
    const GT: &str = "Figure 1: The overall system architecture and its components.\n";
    let gt = StructuralSidecar::from_markdown(GT);
    // GT's only caption is unbound (no preceding Image/Table/Figure), so
    // `edges(gt)` now includes it (empty target). Pred has no trace of it.
    assert!(
        matches!(&gt.nodes[0], StructuralNode::Caption { binds_to: None, .. }),
        "test setup expects an unbound GT caption"
    );

    let pred = StructuralSidecar::from_markdown("Something totally unrelated and wrong.\n");
    let score = score_structural(&pred, &gt);
    let edges_bd = score.dimensions_pr()[4];
    assert_eq!(edges_bd.0, "edges");
    assert!(
        edges_bd.1.recall < 1.0,
        "an unbound GT caption pred never produced must lower edge recall, got {}",
        edges_bd.1.recall
    );
    assert_eq!(
        score.d4_edges, 0.0,
        "unmatched unbound GT caption must score D4=0, not vacuous 1.0"
    );
}

#[test]
fn test_unbind_caption_drops_still_holds_with_edges_including_unbound() {
    // Regression: the existing `unbind_caption` perturbation (pred's caption
    // binding stripped, GT keeps its bound edge) must still drop the score
    // after BUG3 widens `edges()` to include unbound nodes.
    assert_drops(&unbind_caption(baseline()), "unbind-caption");
}
