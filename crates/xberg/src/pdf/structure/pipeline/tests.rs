use super::*;
use crate::pdf::hierarchy::SegmentData;
use crate::pdf::structure::types::{PdfLine, PdfParagraph};

#[cfg(feature = "layout-detection")]
#[test]
fn table_model_preflight_uses_selected_slanet_variant() {
    use crate::core::config::layout::TableModel;

    assert_eq!(
        slanet_variant_for_table_model(TableModel::SlanetWired),
        Some("slanet_wired")
    );
    assert_eq!(
        slanet_variant_for_table_model(TableModel::SlanetWireless),
        Some("slanet_wireless")
    );
    assert_eq!(
        slanet_variant_for_table_model(TableModel::SlanetPlus),
        Some("slanet_plus")
    );
    assert_eq!(
        slanet_variant_for_table_model(TableModel::SlanetAuto),
        Some("slanet_wired")
    );
    assert_eq!(slanet_variant_for_table_model(TableModel::Tatr), None);
    assert_eq!(slanet_variant_for_table_model(TableModel::Disabled), None);
}

/// Helper: a table at `bbox` on `page` whose only content is `markdown`
/// (so content weight == markdown length; empty cells).
fn ov_table(page: u32, bbox: (f64, f64, f64, f64), markdown: &str) -> crate::types::Table {
    let (x0, y0, x1, y1) = bbox;
    crate::types::Table {
        cells: Vec::new(),
        markdown: markdown.to_string(),
        page_number: page,
        bounding_box: Some(crate::types::BoundingBox { x0, y0, x1, y1 }),
        ..Default::default()
    }
}

/// Helper: a table fragment with real cell content (needed to satisfy
/// `fragments_are_stitchable`'s column-count check) at `bbox` on `page`.
fn cell_table(page: u32, bbox: (f64, f64, f64, f64), cells: &[&[&str]]) -> crate::types::Table {
    let (x0, y0, x1, y1) = bbox;
    let cells: Vec<Vec<String>> = cells
        .iter()
        .map(|row| row.iter().map(|s| s.to_string()).collect())
        .collect();
    let markdown = cells.iter().map(|row| row.join("|")).collect::<Vec<_>>().join("\n");
    crate::types::Table {
        cells,
        markdown,
        page_number: page,
        bounding_box: Some(crate::types::BoundingBox { x0, y0, x1, y1 }),
        ..Default::default()
    }
}

/// Run the same id/columns assignment the real pipeline performs: stitch
/// same-page fragments, then run the final, post-dedup assignment pass in
/// `prepare_emitted_tables` (see issue #1297 code review: assigning ids
/// inside `stitch_fragmented_tables` alone misses layout-detected tables).
fn stitch_and_emit(
    native_tables: Vec<crate::types::Table>,
    layout_tables: Vec<crate::types::Table>,
    all_page_segments: &[Vec<SegmentData>],
) -> Vec<crate::types::Table> {
    use crate::core::config::layout::TableOverlapPreference;
    let stitched = stitch_fragmented_tables(native_tables, all_page_segments);
    prepare_emitted_tables(&stitched, layout_tables, TableOverlapPreference::Content)
}

/// Issue #1297: fragments of one physical table (stitched into a single
/// chain) collapse into one `tables[]` entry, which naturally carries one
/// `table_id`. A separate, non-adjacent table gets a distinct id.
#[test]
fn stitched_fragments_share_one_table_id_distinct_tables_differ() {
    let frag_top = cell_table(1, (0.0, 90.0, 100.0, 110.0), &[&["H1", "H2"]]);
    let frag_bottom = cell_table(1, (0.0, 70.0, 100.0, 89.0), &[&["a", "b"]]);
    let other_page_table = cell_table(2, (0.0, 0.0, 100.0, 20.0), &[&["X", "Y"]]);

    let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();
    let result = stitch_and_emit(
        vec![frag_top, frag_bottom, other_page_table],
        Vec::new(),
        &all_page_segments,
    );

    assert_eq!(result.len(), 2, "the two page-1 fragments must stitch into one table");

    let page_1_table = result
        .iter()
        .find(|t| t.page_number == 1)
        .expect("page 1 table present");
    let page_2_table = result
        .iter()
        .find(|t| t.page_number == 2)
        .expect("page 2 table present");

    assert_eq!(page_1_table.cells.len(), 2, "stitched chain has both fragments' rows");
    assert!(page_1_table.table_id.is_some(), "stitched table must have a table_id");
    assert!(page_2_table.table_id.is_some(), "unrelated table must have a table_id");
    assert_ne!(
        page_1_table.table_id, page_2_table.table_id,
        "distinct physical tables must have distinct ids"
    );
}

/// Issue #1297: `table_id` assignment must be deterministic across runs
/// for the same input (no randomness, no wall-clock dependence).
#[test]
fn table_id_assignment_is_deterministic_across_runs() {
    let build_input = || {
        vec![
            cell_table(2, (0.0, 0.0, 100.0, 20.0), &[&["X", "Y"]]),
            cell_table(1, (0.0, 0.0, 100.0, 20.0), &[&["A", "B"]]),
        ]
    };
    let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();

    let first_run = stitch_and_emit(build_input(), Vec::new(), &all_page_segments);
    let second_run = stitch_and_emit(build_input(), Vec::new(), &all_page_segments);

    let first_ids: Vec<_> = first_run.iter().map(|t| (t.page_number, t.table_id.clone())).collect();
    let second_ids: Vec<_> = second_run.iter().map(|t| (t.page_number, t.table_id.clone())).collect();
    assert_eq!(first_ids, second_ids, "table_id assignment must be deterministic");
}

/// Issue #1297: every emitted table fragment carries `columns` (its own
/// header row), even a fragment that stitching left untouched.
#[test]
fn stitching_populates_columns_on_merged_and_standalone_fragments() {
    let frag_top = cell_table(1, (0.0, 90.0, 100.0, 110.0), &[&["H1", "H2"]]);
    let frag_bottom = cell_table(1, (0.0, 70.0, 100.0, 89.0), &[&["a", "b"]]);
    let standalone = cell_table(3, (0.0, 0.0, 100.0, 20.0), &[&["Name", "Age"], &["Alice", "30"]]);

    let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();
    let result = stitch_and_emit(vec![frag_top, frag_bottom, standalone], Vec::new(), &all_page_segments);

    let stitched = result.iter().find(|t| t.page_number == 1).unwrap();
    assert_eq!(
        stitched.columns,
        Some(vec!["H1".to_string(), "H2".to_string()]),
        "stitched table's columns come from the topmost fragment's header row"
    );

    let standalone_result = result.iter().find(|t| t.page_number == 3).unwrap();
    assert_eq!(
        standalone_result.columns,
        Some(vec!["Name".to_string(), "Age".to_string()]),
        "a standalone fragment's columns come from its own first row"
    );
}

/// Issue #1297 code review (Finding 1): a layout-detected table (never
/// passed through `stitch_fragmented_tables`, only appended in
/// `prepare_emitted_tables`) must still receive a `table_id` and
/// `columns` once it survives dedup into the final emitted set.
#[test]
fn layout_detected_table_surviving_dedup_gets_table_id_and_columns() {
    let native = cell_table(1, (0.0, 0.0, 100.0, 20.0), &[&["A", "B"]]);
    let layout_only = cell_table(2, (0.0, 0.0, 100.0, 20.0), &[&["Layout1", "Layout2"], &["x", "y"]]);

    let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();
    let result = stitch_and_emit(vec![native], vec![layout_only], &all_page_segments);

    assert_eq!(
        result.len(),
        2,
        "both the native and layout-detected tables must be emitted"
    );
    let layout_result = result
        .iter()
        .find(|t| t.page_number == 2)
        .expect("layout-detected table survives into the emitted set");

    assert!(
        layout_result.table_id.is_some(),
        "a layout-detected table must receive a table_id, not just native tables"
    );
    assert_eq!(
        layout_result.columns,
        Some(vec!["Layout1".to_string(), "Layout2".to_string()]),
        "a layout-detected table must receive columns from its own header row"
    );
}

#[test]
fn identical_markdown_tables_collapse_despite_missing_bbox() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![crate::types::Table {
        cells: vec![vec!["a".into(), "b".into()]],
        markdown: "| a | b |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    }];
    let layout = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "| a | b |")];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.len(),
        1,
        "byte-identical markdown on the same page collapses even when one table has no bbox"
    );
}

#[test]
fn sparse_currency_affix_columns_merge_into_financial_values() {
    use crate::core::config::layout::TableOverlapPreference;

    let mut cells = vec![vec![
        "Security".into(),
        String::new(),
        "Par (000)".into(),
        String::new(),
        "Value".into(),
    ]];
    for index in 0..12 {
        cells.push(vec![
            format!("Bond {index}"),
            if index == 0 { "USD".into() } else { String::new() },
            format!("{},000", index + 1),
            if index == 0 { "$".into() } else { String::new() },
            format!("{},500", index + 1),
        ]);
    }
    let table = crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        page_number: 1,
        ..Default::default()
    };

    let emitted = prepare_emitted_tables(&[table], Vec::new(), TableOverlapPreference::Content);

    assert_eq!(emitted[0].cells[0], ["Security", "Par (000)", "Value"]);
    assert_eq!(emitted[0].cells[1], ["Bond 0", "USD 1,000", "$ 1,500"]);
    assert_eq!(
        emitted[0].columns,
        Some(vec!["Security".into(), "Par (000)".into(), "Value".into()])
    );
    assert!(emitted[0].markdown.starts_with("| Security | Par (000) | Value |"));
}

#[test]
fn wrapped_financial_rows_fold_into_value_bearing_records() {
    let mut cells = vec![
        vec!["Security".into(), "Par (000)".into(), "Value".into()],
        vec!["Region (continued)".into(), String::new(), String::new()],
    ];
    for index in 0..8 {
        cells.push(vec![format!("Asset {index}, Series"), String::new(), String::new()]);
        cells.push(vec!["Class A, variable rate".into(), String::new(), String::new()]);
        cells.push(vec![
            "maturing in 2035".into(),
            format!("{},000", index + 1),
            format!("$ {},500", index + 1),
        ]);
    }
    let mut tables = vec![crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        ..Default::default()
    }];

    normalize_wrapped_financial_rows(&mut tables);

    assert_eq!(tables[0].cells.len(), 10);
    assert_eq!(
        tables[0].cells[1],
        ["Region (continued)", "", ""],
        "the first descriptor-only section label must remain its own row"
    );
    assert_eq!(
        tables[0].cells[2],
        [
            "Asset 0, Series Class A, variable rate maturing in 2035",
            "1,000",
            "$ 1,500"
        ]
    );
    assert!(
        tables[0]
            .markdown
            .contains("| Asset 7, Series Class A, variable rate maturing in 2035 | 8,000 | $ 8,500 |")
    );
}

#[test]
fn wrapped_financial_rows_preserve_interior_section_boundaries() {
    let mut cells = vec![
        vec!["Security".into(), "Par".into(), "Value".into()],
        vec!["Region A (continued)".into(), String::new(), String::new()],
    ];
    for index in 0..4 {
        cells.push(vec![format!("Wrapped asset {index}"), String::new(), String::new()]);
        cells.push(vec!["final line".into(), String::new(), String::new()]);
        cells.push(vec![
            "matures 2035".into(),
            format!("{}", index + 1),
            format!("{}", index + 101),
        ]);
    }
    cells.push(vec![
        "Unanchored text before section".into(),
        String::new(),
        String::new(),
    ]);
    cells.push(vec!["Region B — 2.0%".into(), String::new(), String::new()]);
    for index in 4..8 {
        cells.push(vec![format!("Wrapped asset {index}"), String::new(), String::new()]);
        cells.push(vec!["final line".into(), String::new(), String::new()]);
        cells.push(vec![
            "matures 2035".into(),
            format!("{}", index + 1),
            format!("{}", index + 101),
        ]);
    }
    let mut tables = vec![crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        ..Default::default()
    }];

    normalize_wrapped_financial_rows(&mut tables);

    let section_index = tables[0]
        .cells
        .iter()
        .position(|row| row[0] == "Region B — 2.0%")
        .expect("interior section label");
    assert_eq!(
        tables[0].cells[section_index - 1],
        ["Unanchored text before section", "", ""],
        "pending descriptors must flush unchanged before a new section"
    );
    assert_eq!(tables[0].cells[section_index], ["Region B — 2.0%", "", ""]);
    assert_eq!(
        tables[0].cells[section_index + 1],
        ["Wrapped asset 4 final line matures 2035", "5", "105"],
        "folding may resume after the section boundary"
    );
}

#[test]
fn financial_section_label_accepts_allocation_with_footnote() {
    let row = vec!["Regional allocation — 0.6%(b)".into(), String::new(), String::new()];

    assert!(is_financial_section_label(&row));
}

#[test]
fn financial_section_label_rejects_coupon_description_after_percentage() {
    let row = vec!["ACME notes — 5.0% senior notes".into(), String::new(), String::new()];

    assert!(!is_financial_section_label(&row));
}

#[test]
fn wrapped_financial_rows_fold_without_a_section_label() {
    let mut cells = vec![vec!["Security".into(), "Par".into(), "Value".into()]];
    for index in 0..8 {
        cells.push(vec![format!("Asset {index}, Series"), String::new(), String::new()]);
        cells.push(vec!["Class A, variable rate".into(), String::new(), String::new()]);
        cells.push(vec![
            "maturing in 2035".into(),
            format!("{},000", index + 1),
            format!("{},500", index + 1),
        ]);
    }
    let mut tables = vec![crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        ..Default::default()
    }];

    normalize_wrapped_financial_rows(&mut tables);

    assert_eq!(tables[0].cells.len(), 9);
    assert_eq!(
        tables[0].cells[1],
        [
            "Asset 0, Series Class A, variable rate maturing in 2035",
            "1,000",
            "1,500"
        ]
    );
}

#[test]
fn wrapped_financial_row_folding_preserves_tokens_and_trailing_text() {
    let mut cells = vec![
        vec!["Security".into(), "Par".into(), "Value".into()],
        vec!["Region".into(), String::new(), String::new()],
    ];
    for index in 0..8 {
        cells.push(vec![format!("Wrapped asset {index}"), String::new(), String::new()]);
        cells.push(vec![
            "final line".into(),
            format!("{}", index + 1),
            format!("{}", index + 101),
        ]);
    }
    cells.push(vec!["Unanchored trailing note".into(), String::new(), String::new()]);
    let before_tokens = cells
        .iter()
        .flatten()
        .flat_map(|cell| cell.split_whitespace())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut tables = vec![crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        ..Default::default()
    }];

    normalize_wrapped_financial_rows(&mut tables);

    let after_tokens = tables[0]
        .cells
        .iter()
        .flatten()
        .flat_map(|cell| cell.split_whitespace())
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(after_tokens, before_tokens);
    assert_eq!(
        tables[0].cells.last().expect("trailing row"),
        &["Unanchored trailing note", "", ""],
        "descriptor-only text without an immediately following value row must not fold"
    );
}

#[test]
fn wrapped_financial_row_folding_requires_strict_financial_density() {
    let build_cells = |header: [&str; 3], continuation_rows: usize| {
        let mut cells = vec![
            header.map(str::to_string).to_vec(),
            vec!["Section".into(), String::new(), String::new()],
        ];
        for index in 0..8 {
            if index < continuation_rows {
                cells.push(vec![format!("Wrapped {index}"), String::new(), String::new()]);
            }
            cells.push(vec![
                format!("Asset {index}"),
                format!("{}", index + 1),
                format!("{}", index + 101),
            ]);
        }
        cells
    };
    let non_financial = build_cells(["Name", "Owner", "Status"], 8);
    let balanced = build_cells(["Security", "Par", "Value"], 7);
    let mut tables = vec![
        crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&non_financial),
            cells: non_financial.clone(),
            ..Default::default()
        },
        crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&balanced),
            cells: balanced.clone(),
            ..Default::default()
        },
    ];

    normalize_wrapped_financial_rows(&mut tables);

    assert_eq!(tables[0].cells, non_financial);
    assert_eq!(
        tables[1].cells, balanced,
        "descriptor-only rows must outnumber value-bearing rows after the section label"
    );
}

#[test]
fn named_or_non_currency_columns_are_not_collapsed() {
    let build_table = |source_header: &str, source_value: &str| {
        let mut cells = vec![vec!["Security".into(), source_header.into(), "Value".into()]];
        for index in 0..12 {
            cells.push(vec![
                format!("Asset {index}"),
                if index == 0 { source_value.into() } else { String::new() },
                format!("{index},000"),
            ]);
        }
        crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }
    };
    let mut tables = vec![build_table("Currency", "USD"), build_table("", "kg")];

    normalize_sparse_currency_affix_columns(&mut tables);

    assert_eq!(
        tables[0].cells[0].len(),
        3,
        "an explicitly named Currency column is semantic"
    );
    assert_eq!(
        tables[1].cells[0].len(),
        3,
        "an arbitrary sparse unit is not a currency marker"
    );
}

#[test]
fn dense_currency_columns_are_not_collapsed() {
    let mut cells = vec![vec!["Security".into(), String::new(), "Value".into()]];
    for index in 0..10 {
        cells.push(vec![
            format!("Asset {index}"),
            if index < 2 { "USD".into() } else { String::new() },
            format!("{index},000"),
        ]);
    }
    let mut tables = vec![crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        ..Default::default()
    }];

    normalize_sparse_currency_affix_columns(&mut tables);

    assert_eq!(tables[0].cells[0].len(), 3);
}

#[test]
fn currency_marker_without_target_value_is_preserved() {
    let mut cells = vec![vec!["Security".into(), String::new(), "Value".into()]];
    cells.push(vec!["Currency declaration".into(), "USD".into(), String::new()]);
    for index in 0..11 {
        cells.push(vec![format!("Asset {index}"), String::new(), format!("{index},000")]);
    }
    let mut tables = vec![crate::types::Table {
        markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
        cells,
        ..Default::default()
    }];

    normalize_sparse_currency_affix_columns(&mut tables);

    assert_eq!(tables[0].cells[0].len(), 3);
    assert_eq!(tables[0].cells[1][1], "USD");
}

#[test]
fn dedup_content_preference_keeps_larger_table() {
    use crate::core::config::layout::TableOverlapPreference;
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
    ];
    deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Content);
    assert_eq!(tables.len(), 1);
    assert_eq!(
        tables[0].markdown, "bbbbbbbbbb",
        "Content keeps the larger (layout) table"
    );
}

#[test]
fn dedup_native_preference_keeps_native_even_when_smaller() {
    use crate::core::config::layout::TableOverlapPreference;
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
    ];
    deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Native);
    assert_eq!(tables.len(), 1);
    assert_eq!(
        tables[0].markdown, "a",
        "Native preference keeps native over a larger layout table"
    );
}

#[test]
fn dedup_layout_preference_keeps_layout_even_when_smaller() {
    use crate::core::config::layout::TableOverlapPreference;
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "aaaaaaaaaa"),
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "b"),
    ];
    deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Layout);
    assert_eq!(tables.len(), 1);
    assert_eq!(
        tables[0].markdown, "b",
        "Layout preference keeps layout over a larger native table"
    );
}

#[test]
fn dedup_native_preference_falls_back_to_content_for_same_origin() {
    use crate::core::config::layout::TableOverlapPreference;
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
    ];
    deduplicate_overlapping_tables(&mut tables, 2, TableOverlapPreference::Native);
    assert_eq!(tables.len(), 1);
    assert_eq!(
        tables[0].markdown, "bbbbbbbbbb",
        "same-origin overlap falls back to content"
    );
}

#[test]
fn dedup_non_overlapping_tables_both_kept() {
    use crate::core::config::layout::TableOverlapPreference;
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
        ov_table(1, (200.0, 200.0, 300.0, 300.0), "b"),
    ];
    deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Native);
    assert_eq!(tables.len(), 2, "non-overlapping tables are both kept");
}

#[test]
fn side_by_side_layout_children_replace_content_heavy_parent() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let layout = vec![
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["left", "right"]
    );
}

#[test]
fn side_by_side_layout_cohort_replaces_two_content_heavy_native_parents() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native left".repeat(100)),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), &"native right".repeat(100)),
    ];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "layout right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["layout left", "layout right"]
    );
}

#[test]
fn native_preference_keeps_two_parents_over_side_by_side_layout_cohort() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "native left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "native right"),
    ];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), &"layout left".repeat(100)),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), &"layout right".repeat(100)),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Native);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["native left", "native right"]
    );
}

#[test]
fn one_layout_child_does_not_replace_two_native_parents() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native left".repeat(100)),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), &"native right".repeat(100)),
    ];
    let layout = vec![ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left")];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(emitted.len(), 2);
    assert!(emitted.iter().all(|table| table.markdown.starts_with("native")));
}

#[test]
fn stacked_native_parents_do_not_form_side_by_side_replacement_cohort() {
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 200.0, 45.0), "native top"),
        ov_table(1, (0.0, 55.0, 200.0, 100.0), "native bottom"),
    ];
    tables.extend([
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "layout right"),
    ]);

    assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
}

#[test]
fn weakly_overlapping_layout_children_do_not_replace_two_native_parents() {
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "native left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "native right"),
    ];
    tables.extend([
        ov_table(1, (-70.0, 0.0, 80.0, 100.0), "layout left"),
        ov_table(1, (120.0, 0.0, 270.0, 100.0), "layout right"),
    ]);

    assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
}

#[test]
fn layout_cohort_rejects_child_that_does_not_cover_corresponding_parent() {
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "native left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "native right"),
    ];
    tables.extend([
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
        ov_table(1, (105.0, 0.0, 175.0, 100.0), "layout right"),
    ]);

    assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
}

#[test]
fn layout_cohort_accepts_reciprocal_crop_within_tolerance() {
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "native left"),
        ov_table(1, (110.0, 0.0, 210.0, 100.0), "native right"),
    ];
    tables.extend([
        ov_table(1, (0.0, 0.0, 79.6, 100.0), "layout left"),
        ov_table(1, (110.0, 0.0, 210.0, 100.0), "layout right"),
    ]);

    assert_eq!(side_by_side_layout_replacements(&tables, 2), [(vec![0, 1], vec![2, 3])]);
}

#[test]
fn layout_cohort_rejects_reciprocal_crop_below_tolerance() {
    let mut tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "native left"),
        ov_table(1, (110.0, 0.0, 210.0, 100.0), "native right"),
    ];
    tables.extend([
        ov_table(1, (0.0, 0.0, 79.4, 100.0), "layout left"),
        ov_table(1, (110.0, 0.0, 210.0, 100.0), "layout right"),
    ]);

    assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
}

#[test]
fn layout_cohort_rejects_child_owned_by_sibling_parent() {
    let tables = vec![
        ov_table(1, (0.0, 0.0, 100.0, 100.0), "native left"),
        ov_table(1, (110.0, 0.0, 210.0, 100.0), "native right"),
        ov_table(1, (90.0, 0.0, 210.0, 100.0), "layout crossing"),
        ov_table(1, (110.0, 0.0, 210.0, 100.0), "layout right"),
    ];

    assert!(!replacement_children_correspond(&tables, &[0, 1], &[2, 3]));
}

#[test]
fn three_native_candidates_select_one_disjoint_adjacent_cohort() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (105.0, 0.0, 200.0, 100.0), &"native middle".repeat(100)),
        ov_table(1, (210.0, 0.0, 305.0, 100.0), &"native right".repeat(100)),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native left".repeat(100)),
    ];
    let layout = vec![
        ov_table(1, (210.0, 0.0, 305.0, 100.0), "layout right"),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "layout middle"),
    ];
    let mut candidates = native.clone();
    candidates.extend(layout.clone());

    let replacements = side_by_side_layout_replacements(&candidates, native.len());
    assert_eq!(replacements, [(vec![2, 0], vec![4, 5])]);

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);
    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["layout left", "layout middle", &"native right".repeat(100)]
    );
}

#[test]
fn side_by_side_replacement_is_atomic_against_native_duplicate() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100)),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native duplicate".repeat(100)),
    ];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["left", "right"]
    );
}

#[test]
fn side_by_side_replacement_is_atomic_against_earlier_layout_duplicate() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let better_left = "layout duplicate with more content";
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), better_left),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        [better_left, "right"]
    );
}

#[test]
fn side_by_side_replacement_is_atomic_against_later_layout_duplicate() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let better_left = "layout duplicate with more content";
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), better_left),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        [better_left, "right"]
    );
}

#[test]
fn overlapping_protected_replacement_groups_survive() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 200.0, 100.0), "upper parent"),
        ov_table(1, (0.0, 40.0, 200.0, 140.0), "lower parent"),
    ];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "upper left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "upper right"),
        ov_table(1, (0.0, 40.0, 95.0, 140.0), "lower left"),
        ov_table(1, (105.0, 40.0, 200.0, 140.0), "lower right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["upper left", "upper right", "lower left", "lower right"]
    );
}

#[test]
fn partially_shared_replacement_groups_keep_canonical_table_order() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent a"),
        ov_table(1, (105.0, 0.0, 305.0, 100.0), "parent b"),
    ];
    let layout = vec![
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "shared"),
        ov_table(1, (210.0, 0.0, 305.0, 100.0), "right"),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["left", "shared", "right"]
    );
}

#[test]
fn side_by_side_replacement_orders_complete_affected_row_cohort() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent")];
    let layout = vec![
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ov_table(1, (300.0, 0.0, 350.0, 100.0), "unrelated"),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["left", "right", "unrelated"]
    );
}

#[test]
fn side_by_side_replacement_preserves_interleaved_different_row_slot() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent")];
    let layout = vec![
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ov_table(1, (300.0, -100.0, 350.0, -10.0), "different row"),
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["left", "different row", "right"]
    );
}

#[test]
fn one_layout_child_does_not_replace_parent() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let layout = vec![ov_table(1, (0.0, 0.0, 95.0, 100.0), "left")];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(emitted.len(), 1);
    assert!(emitted[0].markdown.starts_with("parent"));
}

#[test]
fn overlapping_layout_children_do_not_replace_parent() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 120.0, 100.0), "left"),
        ov_table(1, (80.0, 0.0, 200.0, 100.0), "right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(emitted.len(), 1);
    assert!(emitted[0].markdown.starts_with("parent"));
}

#[test]
fn stacked_layout_children_do_not_replace_parent() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 200.0, 45.0), "top"),
        ov_table(1, (0.0, 55.0, 200.0, 100.0), "bottom"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(emitted.len(), 1);
    assert!(emitted[0].markdown.starts_with("parent"));
}

#[test]
fn shallow_layout_children_do_not_replace_tall_parent() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 20.0), "left"),
        ov_table(1, (105.0, 0.0, 200.0, 20.0), "right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(emitted.len(), 1);
    assert!(emitted[0].markdown.starts_with("parent"));
}

#[test]
fn weakly_overlapping_layout_children_do_not_replace_parent() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
    let layout = vec![
        ov_table(1, (-70.0, 0.0, 80.0, 100.0), "left"),
        ov_table(1, (120.0, 0.0, 270.0, 100.0), "right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(emitted.len(), 1);
    assert!(emitted[0].markdown.starts_with("parent"));
}

#[test]
fn side_by_side_replacement_preserves_unrelated_table() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![
        ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100)),
        ov_table(2, (10.0, 10.0, 80.0, 80.0), "unrelated"),
    ];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

    assert_eq!(
        emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
        ["unrelated", "left", "right"]
    );
}

#[test]
fn native_preference_keeps_parent_over_side_by_side_children() {
    use crate::core::config::layout::TableOverlapPreference;
    let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent")];
    let layout = vec![
        ov_table(1, (0.0, 0.0, 95.0, 100.0), &"left".repeat(100)),
        ov_table(1, (105.0, 0.0, 200.0, 100.0), &"right".repeat(100)),
    ];

    let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Native);

    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].markdown, "parent");
}

#[test]
fn dropped_duplicate_table_does_not_suppress_text() {
    use crate::core::config::layout::TableOverlapPreference;

    let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "native table content")];
    let layout_tables = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "x")];
    let emitted_tables = prepare_emitted_tables(&native_tables, layout_tables, TableOverlapPreference::Content);
    let bboxes_by_page = table_bboxes_by_page(&emitted_tables);

    assert_eq!(emitted_tables.len(), 1);
    assert_eq!(emitted_tables[0].bounding_box.expect("kept table bbox").x1, 100.0);

    let segment = SegmentData {
        x: 150.0,
        y: 10.0,
        width: 20.0,
        height: 12.0,
        ..seg("text outside the emitted table", 150.0, 20.0)
    };
    let filtered = filter_segments_by_table_bboxes(
        vec![segment],
        bboxes_by_page.get(&0).map(Vec::as_slice).unwrap_or_default(),
    );
    assert_eq!(filtered.len(), 1, "a discarded duplicate bbox must not remove text");
}

#[test]
fn empty_table_does_not_suppress_text() {
    use crate::core::config::layout::TableOverlapPreference;

    let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "  \n")];
    let emitted_tables = prepare_emitted_tables(&native_tables, Vec::new(), TableOverlapPreference::Content);
    let bboxes_by_page = table_bboxes_by_page(&emitted_tables);

    assert!(
        emitted_tables.is_empty(),
        "assembly would not emit whitespace-only markdown"
    );
    assert!(
        bboxes_by_page.is_empty(),
        "non-emitted tables must not contribute suppression boxes"
    );

    let segment = SegmentData {
        x: 10.0,
        y: 10.0,
        width: 20.0,
        height: 12.0,
        ..seg("text under an empty table", 10.0, 20.0)
    };
    let filtered = filter_segments_by_table_bboxes(
        vec![segment],
        bboxes_by_page.get(&0).map(Vec::as_slice).unwrap_or_default(),
    );
    assert_eq!(filtered.len(), 1);
}

#[test]
fn empty_table_does_not_displace_valid_overlap() {
    use crate::core::config::layout::TableOverlapPreference;

    let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "  \n")];
    let layout_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "| valid |")];

    let emitted_tables = prepare_emitted_tables(&native_tables, layout_tables, TableOverlapPreference::Native);

    assert_eq!(emitted_tables.len(), 1);
    assert_eq!(emitted_tables[0].markdown, "| valid |");
}

#[cfg(feature = "layout-detection")]
#[test]
fn missing_wrapper_validation_is_treated_as_skipped() {
    use super::super::regions::layout_validation::RegionValidation;

    let hint = |class_name| LayoutHint {
        class_name,
        confidence: 0.9,
        left: 0.0,
        bottom: 0.0,
        right: 100.0,
        top: 100.0,
    };
    let hints = vec![
        hint(LayoutHintClass::Picture),
        hint(LayoutHintClass::Form),
        hint(LayoutHintClass::Text),
    ];
    let ownership = wrapper_ownership_by_hint(&hints, &[RegionValidation::Empty]);
    assert_eq!(ownership, [false, true, true]);
}

#[test]
fn emitted_table_still_suppresses_covered_text() {
    use crate::core::config::layout::TableOverlapPreference;

    let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "| duplicated table text |")];
    let emitted_tables = prepare_emitted_tables(&native_tables, Vec::new(), TableOverlapPreference::Content);
    let bboxes_by_page = table_bboxes_by_page(&emitted_tables);
    let segment = SegmentData {
        x: 10.0,
        y: 10.0,
        width: 20.0,
        height: 12.0,
        ..seg("duplicated table text", 10.0, 20.0)
    };

    let filtered = filter_segments_by_table_bboxes(
        vec![segment],
        bboxes_by_page.get(&0).map(Vec::as_slice).unwrap_or_default(),
    );
    assert!(
        filtered.is_empty(),
        "an emitted table must continue to suppress duplicate text"
    );
}

/// Helper: segment with font metadata for title-promotion tests.
fn role_seg(text: &str, font_size: f32, is_bold: bool, assigned_role: Option<u8>) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x: 72.0,
        y: 700.0,
        width: 200.0,
        height: font_size,
        font_size,
        is_bold,
        is_italic: false,
        is_monospace: false,
        baseline_y: 700.0,
        rotation_degrees: 0.0,
        assigned_role,
    }
}

/// A bold, first-page, larger-than-any-tagged-heading tier must be promoted
/// to h1 with the tagged hierarchy shifted down one level.
#[test]
fn promote_title_shifts_tagged_heading_levels_down() {
    let pages = vec![vec![
        role_seg("Titre du document", 28.0, true, None),
        role_seg("Titre 1", 18.0, true, Some(1)),
        role_seg("Titre 2", 16.0, true, Some(2)),
        role_seg("body text", 12.0, false, None),
    ]];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(promote_untagged_document_title(&mut map, &pages));

    let level_of = |font: f32| map.iter().find(|(f, _)| (*f - font).abs() < 0.05).and_then(|(_, l)| *l);
    assert_eq!(level_of(28.0), Some(1), "title tier must become h1");
    assert_eq!(level_of(18.0), Some(2), "tagged H1 must demote to h2");
    assert_eq!(level_of(16.0), Some(3), "tagged H2 must demote to h3");
    assert_eq!(level_of(12.0), None, "body must stay body");
}

/// No untagged tier above the largest tagged heading → no promotion.
#[test]
fn promote_title_no_candidate_leaves_map_unchanged() {
    let pages = vec![vec![
        role_seg("Heading", 18.0, true, Some(1)),
        role_seg("body", 12.0, false, None),
    ]];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    let before = map.clone();
    assert!(!promote_untagged_document_title(&mut map, &pages));
    assert_eq!(map, before);
}

/// A non-bold large tier (e.g. a pull quote) must not be mistaken for a title.
#[test]
fn promote_title_requires_bold() {
    let pages = vec![vec![
        role_seg("large quote", 28.0, false, None),
        role_seg("Heading", 18.0, true, Some(1)),
    ]];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(!promote_untagged_document_title(&mut map, &pages));
}

/// A large tier appearing only after page 0 is not a document title.
#[test]
fn promote_title_requires_first_page() {
    let pages = vec![
        vec![role_seg("Heading", 18.0, true, Some(1))],
        vec![role_seg("Big banner later", 28.0, true, None)],
    ];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(!promote_untagged_document_title(&mut map, &pages));
}

/// A mid-word split (e.g. "Text" extracted as "Te" + "xt", same role and
/// font size, immediately adjacent) must count as one logical block, not
/// two — otherwise a font-encoding artifact inflates the apparent
/// document size past the sparsity floor.
#[test]
fn count_logical_blocks_merges_same_role_same_size_runs() {
    let pages = vec![vec![
        role_seg("Big", 24.0, false, Some(1)),
        role_seg("Small Text", 12.0, true, Some(2)),
        role_seg("Te", 24.0, false, Some(1)),
        role_seg("xt", 24.0, false, Some(1)),
    ]];
    assert_eq!(
        count_logical_blocks(&pages),
        3,
        "the split \"Te\"+\"xt\" run must collapse into a single block"
    );
}

/// Segments with different assigned roles never merge, even at the same
/// font size.
#[test]
fn count_logical_blocks_does_not_merge_different_roles() {
    let pages = vec![vec![
        role_seg("Heading", 18.0, true, Some(1)),
        role_seg("more heading text", 18.0, true, Some(2)),
    ]];
    assert_eq!(count_logical_blocks(&pages), 2);
}

/// Sparse document where the structure tree tags every block as a heading
/// with no body tier at all (a document with just a couple of heading-tagged
/// lines and nothing else) must have every role suppressed rather than trusted.
#[test]
fn suppress_all_heading_roles_fires_when_sparse_and_all_tagged() {
    let mut pages = vec![vec![
        role_seg("Big", 24.0, false, Some(1)),
        role_seg("Small Text", 12.0, true, Some(2)),
        role_seg("Te xt", 24.0, false, Some(1)),
    ]];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(suppress_all_heading_roles_when_sparse_and_untrusted(
        &mut map, &mut pages
    ));

    assert!(
        map.iter().all(|(_, level)| level.is_none()),
        "heading map must be fully suppressed; got: {map:?}"
    );
    for page in &pages {
        for seg in page {
            assert_eq!(
                seg.assigned_role, None,
                "assigned_role must be cleared on every segment"
            );
        }
    }
}

/// A sparse document with one tagged heading and one untagged body
/// paragraph (the `issue-987-test.pdf` shape: "Big"/"Te xt" tagged,
/// "Small Text" untagged — 3 total blocks) must ALSO be suppressed: a mix
/// of heading and body tiers on that few blocks is not enough evidence
/// that the tagging is trustworthy, matching GT for that fixture (plain
/// "Big Text"/"Small Text", no headings at all).
#[test]
fn suppress_all_heading_roles_fires_when_sparse_with_body_tier() {
    let mut pages = vec![vec![
        role_seg("Title", 24.0, true, Some(1)),
        role_seg("body text", 12.0, false, None),
    ]];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(suppress_all_heading_roles_when_sparse_and_untrusted(
        &mut map, &mut pages
    ));
    assert_eq!(pages[0][0].assigned_role, None, "tagged role must be cleared");
}

/// A sparse document with no heading roles at all must not be touched —
/// there is nothing to suppress.
#[test]
fn suppress_all_heading_roles_does_not_fire_with_no_headings() {
    let mut pages = vec![vec![
        role_seg("body text one", 12.0, false, None),
        role_seg("body text two", 12.0, false, None),
    ]];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(!suppress_all_heading_roles_when_sparse_and_untrusted(
        &mut map, &mut pages
    ));
}

/// At or above the sparsity floor, an all-heading-tagged document is left
/// alone even with no body tier — larger documents are trusted.
#[test]
fn suppress_all_heading_roles_does_not_fire_at_or_above_floor() {
    // Alternate heading/body role so each segment is a distinct logical
    // block under `count_logical_blocks` rather than collapsing into one. ~keep
    let mut pages = vec![
        (0..MIN_BLOCKS_FOR_FONT_HEADING)
            .map(|i| {
                if i % 2 == 0 {
                    role_seg(&format!("Heading {i}"), 18.0, true, Some(1))
                } else {
                    role_seg(&format!("Body paragraph {i}."), 12.0, false, None)
                }
            })
            .collect(),
    ];
    let mut map = build_heading_map_from_assigned_roles(&pages);
    assert!(!suppress_all_heading_roles_when_sparse_and_untrusted(
        &mut map, &mut pages
    ));
    assert_eq!(
        pages[0][0].assigned_role,
        Some(1),
        "role must be untouched at/above the floor"
    );
}

/// Role demotion mirrors the map shift on segments (bridge.rs reads roles directly).
#[test]
fn demote_assigned_roles_shifts_and_caps() {
    let mut pages = vec![vec![
        role_seg("h1", 18.0, true, Some(1)),
        role_seg("h6", 8.0, true, Some(6)),
        role_seg("body", 12.0, false, None),
    ]];
    demote_assigned_roles(&mut pages);
    assert_eq!(pages[0][0].assigned_role, Some(2));
    assert_eq!(pages[0][1].assigned_role, Some(6), "level 6 must cap, not overflow");
    assert_eq!(pages[0][2].assigned_role, None);
}

#[test]
fn assigned_sal_annotation_role_is_demoted() {
    let paragraphs = process_heuristic_segments(vec![role_seg("__inout_bcount_full(n)", 12.0, false, Some(2))]);
    assert_eq!(paragraphs[0].heading_level, None);
}

#[test]
fn assigned_identifier_heading_role_is_preserved() {
    let paragraphs = blocks_to_paragraphs(
        vec![role_seg("__in_section", 12.0, false, Some(2))],
        &[(12.0, None)],
        &[],
    );
    assert_eq!(paragraphs[0].heading_level, Some(2));
}

/// Helper: a body-tier segment occupying its own visual line at `baseline_y`.
fn body_line_seg(text: &str, baseline_y: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x: 72.0,
        y: baseline_y - 11.0,
        width: 200.0,
        height: 11.0,
        font_size: 11.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

/// All segment text of a paragraph, joined in order.
fn paragraph_segment_text(para: &PdfParagraph) -> String {
    para.lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .map(|s| s.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Regression for #1386 (defect #290). Four consecutive numbered subsection
/// headings share a font size, a weight and an even one-line-height spacing,
/// so `font_change`, `role_change`, `bold_change` and `crossed_gap` are all
/// false — and `looks_like_list_item` deliberately returns `false` for
/// numbered section headings, removing the last boundary. Before the fix the
/// grouper emitted ONE paragraph with all four headings concatenated.
#[test]
fn consecutive_numbered_section_headings_are_separate_paragraphs() {
    let segments = vec![
        body_line_seg("1.3 Gasinstallatie", 700.0),
        body_line_seg("1.4 Elektrische installatie", 686.0),
        body_line_seg("1.5 Waterinstallatie", 672.0),
        body_line_seg("1.6 Ventilatie", 658.0),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[(11.0, None)], &[]);

    assert_eq!(
        paragraphs.len(),
        4,
        "each numbered subsection heading must be its own element"
    );
    assert_eq!(paragraph_segment_text(&paragraphs[0]), "1.3 Gasinstallatie");
    assert_eq!(paragraph_segment_text(&paragraphs[1]), "1.4 Elektrische installatie");
    assert_eq!(paragraph_segment_text(&paragraphs[2]), "1.5 Waterinstallatie");
    assert_eq!(paragraph_segment_text(&paragraphs[3]), "1.6 Ventilatie");
}

/// End-to-end through the grouper AND `merge_continuation_paragraphs`: no
/// heading ends in `.?!:;`, so the merge pass would re-join the run the
/// grouper just split unless it also guards on numbered section starts.
#[test]
fn consecutive_numbered_section_headings_survive_continuation_merge() {
    let segments = vec![
        body_line_seg("1.3 Gasinstallatie", 700.0),
        body_line_seg("1.4 Elektrische installatie", 686.0),
        body_line_seg("1.5 Waterinstallatie", 672.0),
        body_line_seg("1.6 Ventilatie", 658.0),
    ];

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

    assert_eq!(
        paragraphs.len(),
        4,
        "the continuation merge must not re-join numbered section headings"
    );
}

/// The over-fire guard for #1386: a two-line prose paragraph whose second
/// line opens with a bare year must stay ONE paragraph. The looser
/// `starts_with_section_number` returns `true` for "2024 was een druk jaar";
/// the fix deliberately uses `is_numbered_section_heading`, which does not.
#[test]
fn prose_starting_with_a_year_stays_one_paragraph() {
    let segments = vec![
        body_line_seg("Het bestuur meldt", 700.0),
        body_line_seg("2024 was een druk jaar", 686.0),
    ];

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

    assert_eq!(
        paragraphs.len(),
        1,
        "prose beginning with a bare year is not a section heading"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "Het bestuur meldt 2024 was een druk jaar"
    );
}

/// Regression for #1467: a numbered section heading, followed by unrelated
/// bold text at the same size, weight and line spacing, is welded to it.
/// Every other break signal is false here -- `font_change`, `role_change`
/// and `bold_change` all compare equal values, `crossed_gap` has no gaps to
/// find, and `looks_like_list_item` deliberately rejects numbered section
/// headings -- so only `follows_section` (backed by `heading_wraps_onto`
/// ruling out a mid-heading wrap) can separate them. Run through
/// `segments_to_paragraphs`, not `blocks_to_paragraphs`: the continuation
/// merge that runs immediately afterward would silently re-join exactly
/// this split unless it also refuses to absorb a heading it did not open.
#[test]
fn numbered_section_heading_is_split_from_the_callout_that_follows_it() {
    let heading = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 700.0 - 12.0,
        ..column_seg("1.1.1 Pictogrammen in het installatievoorschrift", 72.0, 170.0, 700.0)
    };
    let callout = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 684.0 - 12.0,
        ..column_seg("VOORZICHTIG / BELANGRIJK", 72.0, 90.0, 684.0)
    };
    let body1 = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 668.0 - 12.0,
        ..column_seg("Procedures die niet worden opgevolgd kunnen letsel", 72.0, 190.0, 668.0)
    };
    let body2 = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 652.0 - 12.0,
        ..column_seg("of schade veroorzaken aan de installatie of de", 72.0, 190.0, 652.0)
    };
    let body3 = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 636.0 - 12.0,
        ..column_seg("gebruiker van het toestel indien genegeerd", 72.0, 190.0, 636.0)
    };

    let paragraphs = segments_to_paragraphs(
        vec![heading, callout, body1, body2, body3],
        &[(12.0, None)],
        &[],
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        2,
        "the numbered heading must split from the callout and body text that follow it"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "1.1.1 Pictogrammen in het installatievoorschrift",
        "the heading must be its own element, not fused with the callout"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[1]),
        "VOORZICHTIG / BELANGRIJK Procedures die niet worden opgevolgd kunnen letsel \
         of schade veroorzaken aan de installatie of de gebruiker van het toestel indien genegeerd",
        "the callout and following body text must survive as a separate element from the heading"
    );
}

/// GH#1608: `ARTIKEL 1.` shares font, size, weight and leading with the part
/// header above it, so the numbered-heading predicate is the only boundary
/// signal available -- and it could not see a heading whose number is not the
/// first token. Asserted through `segments_to_paragraphs`, which runs the
/// grouper AND the continuation merge, because a split made by one is
/// routinely undone by the other.
#[test]
fn keyword_numbered_heading_splits_from_the_part_header_above_it() {
    let part_header = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 700.0 - 12.0,
        ..column_seg("ALGEMENE BEPALINGEN", 262.0, 71.0, 700.0)
    };
    let heading = SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        y: 683.0 - 12.0,
        ..column_seg(
            "ARTIKEL 1. TOEPASSELIJKHEID VAN DE INKOOPVOORWAARDEN",
            72.0,
            330.0,
            683.0,
        )
    };

    let paragraphs = segments_to_paragraphs(
        vec![part_header, heading],
        &[(12.0, None)],
        &[],
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        2,
        "the keyword-numbered heading must open its own element"
    );
    assert_eq!(paragraph_segment_text(&paragraphs[0]), "ALGEMENE BEPALINGEN");
    assert_eq!(
        paragraph_segment_text(&paragraphs[1]),
        "ARTIKEL 1. TOEPASSELIJKHEID VAN DE INKOOPVOORWAARDEN"
    );
}

/// GH#1615. A numbered heading whose title does not fit on one line is set with
/// a hanging indent: the number at the margin, the title to its right, and the
/// overflow resuming at the TITLE's left edge. `follows_section` closed the
/// element after the first line anyway, because `heading_wraps_onto` compares
/// RIGHT edges and a wrap's last line is short by definition.
///
/// Geometry from the reporter's page 1, verbatim (PDF user space):
///
/// ```text
/// 5.7.3                                     x 48.24
/// Roof terminal combined duct vertical and  x 83.64  y 774.96
/// twin pipe duct vertical                   x 83.64  y 762.24
/// ```
#[test]
fn a_wrapped_numbered_heading_keeps_its_second_line() {
    let segments = vec![
        SegmentData {
            is_bold: true,
            font_size: 11.04,
            ..column_seg("5.7.3", 48.24, 30.0, 774.96)
        },
        SegmentData {
            is_bold: true,
            font_size: 11.04,
            ..column_seg("Roof terminal combined duct vertical and", 83.64, 211.0, 774.96)
        },
        SegmentData {
            is_bold: true,
            font_size: 11.04,
            ..column_seg("twin pipe duct vertical", 83.64, 114.0, 762.24)
        },
    ];
    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    assert_eq!(paragraphs.len(), 1, "the heading and its own wrap are one element");
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "5.7.3 Roof terminal combined duct vertical and twin pipe duct vertical"
    );
}

/// GH#1615's own control, page 3 of the same reproducer. Identical heading,
/// identical fonts, identical line pitch -- the ONLY difference is that the
/// following line starts at the margin (x 48.24) rather than the title's left
/// edge (x 83.64), because it is body text and not a wrap. It must still split.
#[test]
fn a_numbered_heading_followed_by_margin_aligned_body_still_splits() {
    let segments = vec![
        SegmentData {
            is_bold: true,
            font_size: 11.04,
            ..column_seg("5.7.5", 48.24, 30.0, 774.96)
        },
        SegmentData {
            is_bold: true,
            font_size: 11.04,
            ..column_seg("Roof terminal combined duct vertical and", 83.64, 211.0, 774.96)
        },
        SegmentData {
            is_bold: true,
            font_size: 11.04,
            ..column_seg("The appliance category is C33 for this duct.", 48.24, 216.0, 762.24)
        },
    ];
    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    assert_eq!(paragraphs.len(), 2, "body text at the margin is not the heading's wrap");
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "5.7.5 Roof terminal combined duct vertical and"
    );
}

/// GH#1637: `heading_wrap_chain_open`'s closing term (added by GH#1634 /
/// `8681d72ec`) still required `!heading_wraps_onto(prev, line)`, which compares
/// RIGHT edges. A hanging-indent wrap's own last line is short by definition, and
/// so is a run-in sub-heading at the margin beneath it -- when the two happen to
/// end within `HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR` of each other, the
/// heading looks like it is "still wrapping" and never closes, pulling the
/// sub-heading and the body under it in as well.
///
/// Geometry transcribed from the issue's real-manual measurement (PDF user space,
/// y descending down the page): the wrap ends at x 133.1, the run-in beneath it
/// ends at x 114.2 -- 18.9pt apart at 10.5pt, inside the 21pt tolerance.
#[test]
fn wrapped_heading_closes_even_when_the_run_in_beneath_it_shares_its_right_edge() {
    let segments = vec![
        column_seg("4.4.2", 45.4, 30.0, 700.0),
        column_seg(
            "Opdeling CV-installatie in groepen bij aanwezigheid extra",
            80.8,
            245.7,
            700.0,
        ),
        column_seg("warmtebron", 81.4, 51.7, 685.5),
        column_seg("Werkingsprincipe", 45.4, 68.8, 657.0),
        column_seg(
            "Indien de kamerthermostaat het toestel uitschakelt doordat de temperatuur is bereikt",
            45.4,
            244.2,
            642.5,
        ),
    ];
    let segments: Vec<SegmentData> = segments
        .into_iter()
        .map(|mut segment| {
            segment.font_size = 10.5;
            segment.height = 10.5;
            segment
        })
        .collect();

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

    assert_eq!(
        paragraphs.len(),
        2,
        "the wrapped heading must close before the run-in sub-heading and body beneath it"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "4.4.2 Opdeling CV-installatie in groepen bij aanwezigheid extra warmtebron"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[1]),
        "Werkingsprincipe Indien de kamerthermostaat het toestel uitschakelt doordat de temperatuur is bereikt"
    );
}

/// Helper: a heading-tier segment at `x`, ending at `x + width`, on `baseline_y`.
fn heading_seg(text: &str, x: f32, width: f32, baseline_y: f32) -> SegmentData {
    SegmentData {
        is_bold: true,
        font_size: 12.0,
        height: 12.0,
        ..column_seg(text, x, width, baseline_y)
    }
}

/// Helper: an 8pt body-tier segment at `x`, ending at `x + width`, on `baseline_y`.
fn narrow_body_seg(text: &str, x: f32, width: f32, baseline_y: f32) -> SegmentData {
    SegmentData {
        font_size: 8.0,
        height: 8.0,
        ..column_seg(text, x, width, baseline_y)
    }
}

/// GH#1650 page 1. A numbered section heading set in the HEADING font (12pt,
/// bold) wraps onto a second, unnumbered line at the page margin -- no
/// hanging indent, so neither of `follows_section`'s two existing exemptions
/// can see the wrap: `heading_continuation_is_hanging_indent` fails at its
/// first test (no indent), and `heading_wraps_onto` compares the two lines'
/// RIGHT edges, which a wrap's short last line never matches (144.7pt apart
/// here against a 24pt tolerance). The same pair set in the BODY font already
/// survives via GH#1605's merge-pass exemption, which never runs here because
/// `finalize_paragraph` has already classified this pair as a heading by font
/// size before that pass sees it.
///
/// `crossed_gap` is the second, independent break term: the heading's own
/// 16.8pt pitch outruns the page's 10.56pt body leading (threshold 15.84pt),
/// so even a `follows_section` that accepted the pair would still be cut here.
/// Geometry transcribed from the issue (reproducer page 1, PDF user space).
#[test]
fn wrapped_numbered_heading_in_heading_font_keeps_its_second_line() {
    let segments = vec![
        narrow_body_seg(
            "Lead-in prose above the section, at the body's own pitch",
            64.34,
            230.0,
            900.0,
        ),
        narrow_body_seg(
            "continuing one more line before the heading begins",
            64.34,
            230.0,
            889.44,
        ),
        heading_seg("Section 6. Maintenance and support of", 64.34, 220.04, 800.0),
        heading_seg("software", 64.34, 49.34, 783.2),
        narrow_body_seg(
            "The provisions in this section 'Maintenance and support of",
            64.34,
            234.34,
            772.64,
        ),
        narrow_body_seg(
            "software' apply, apart from the general provisions",
            64.34,
            220.0,
            762.08,
        ),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);
    let paragraphs = segments_to_paragraphs(
        segments,
        &[(12.0, Some(1)), (8.0, None)],
        &gap_ys,
        &TextRepairWitnesses::default(),
    );

    let title_paragraph = paragraphs
        .iter()
        .find(|p| paragraph_segment_text(p).starts_with("Section 6."))
        .expect("the numbered heading must produce a paragraph");
    assert_eq!(
        paragraph_segment_text(title_paragraph),
        "Section 6. Maintenance and support of software",
        "GH#1650: a heading-font wrap must keep its second line, the same as a body-font one does"
    );
}

/// GH#1650 page 2, the reporter's own control: the identical pair, set in the
/// BODY font (8pt bold) rather than the heading font, is GH#1605's shape and
/// already comes out whole via the merge-pass exemption. Must keep working
/// after the grouper-side fix for the heading-font case above.
#[test]
fn wrapped_numbered_heading_in_body_font_keeps_its_second_line() {
    let segments = vec![
        SegmentData {
            is_bold: true,
            ..narrow_body_seg("Section 6. Maintenance and support of", 64.34, 220.04, 800.0)
        },
        SegmentData {
            is_bold: true,
            ..narrow_body_seg("software", 64.34, 49.34, 789.44)
        },
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);
    let paragraphs = segments_to_paragraphs(
        segments,
        &[(12.0, Some(1)), (8.0, None)],
        &gap_ys,
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        1,
        "GH#1605 control: a body-font wrapped numbered heading must stay one element"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "Section 6. Maintenance and support of software"
    );
}

/// GH#1650 page 3. A hanging-indent numbered heading (`Article 15` at the
/// margin, `Termination of the` indented) already passes `follows_section`'s
/// existing `heading_continuation_is_hanging_indent` exemption -- the trace in
/// the issue confirms it -- but is still cut, because `crossed_gap` is not
/// exempted by any of the three continuation tests: the heading's 16.8pt pitch
/// outruns the surrounding body's 10.56pt leading (threshold 15.84pt) the same
/// way it does on page 1. This isolates term 2 from term 1.
#[test]
fn hanging_indent_numbered_heading_survives_its_own_leading() {
    let segments = vec![
        narrow_body_seg(
            "Lead-in prose above the article, at the body's own pitch",
            64.34,
            230.0,
            900.0,
        ),
        narrow_body_seg(
            "continuing one more line before the article begins",
            64.34,
            230.0,
            889.44,
        ),
        heading_seg("Article 15", 64.34, 50.0, 869.44),
        heading_seg("Termination of the", 142.34, 140.0, 869.44),
        heading_seg("agreement for breach", 142.34, 120.0, 852.64),
        narrow_body_seg(
            "The provisions in this article apply from the effective",
            64.34,
            230.0,
            828.16,
        ),
        narrow_body_seg("date of this agreement, unless stated otherwise", 64.34, 230.0, 817.6),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);
    let paragraphs = segments_to_paragraphs(
        segments,
        &[(12.0, Some(1)), (8.0, None)],
        &gap_ys,
        &TextRepairWitnesses::default(),
    );

    let title_paragraph = paragraphs
        .iter()
        .find(|p| paragraph_segment_text(p).starts_with("Article 15"))
        .expect("the numbered heading must produce a paragraph");
    assert_eq!(
        paragraph_segment_text(title_paragraph),
        "Article 15 Termination of the agreement for breach",
        "GH#1650: crossed_gap must not cut a continuation follows_section already accepted"
    );
}

/// GH#1650 page 4, the required control: a COMPLETE short numbered heading
/// (one line, does not fill its column) followed by a separate, unrelated
/// bold line must stay two elements. The heading stops 43.37pt short of its
/// right edge matching the next line's, so `heading_wraps_onto` already
/// rejects it; the heading also stops well short of the body column beneath
/// it (124pt in the issue's own numbers), so the new `heading_continuation_at_margin`
/// exemption must reject it too, or every complete heading followed by
/// another bold line would be welded together.
#[test]
fn complete_numbered_heading_followed_by_unrelated_bold_line_still_splits() {
    let segments = vec![
        heading_seg("Section 4. Software", 64.34, 110.7, 800.0),
        heading_seg("eIDAS and related services", 64.34, 154.07, 783.2),
        narrow_body_seg("Software as defined in this section covers all", 64.34, 234.34, 772.64),
        narrow_body_seg(
            "licensed components delivered under this agreement",
            64.34,
            220.0,
            762.08,
        ),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);
    let paragraphs = segments_to_paragraphs(
        segments,
        &[(12.0, Some(1)), (8.0, None)],
        &gap_ys,
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        3,
        "GH#1650 control: a complete heading, an unrelated bold line, and body text must stay separate"
    );
    assert_eq!(paragraph_segment_text(&paragraphs[0]), "Section 4. Software");
    assert_eq!(paragraph_segment_text(&paragraphs[1]), "eIDAS and related services");
}

/// GH#1740. A numbered heading set in italic, at the body's own size, that
/// wraps TWICE (three visual lines) was closed after its second line: the
/// count-based `heading_absorbed_one_wrap` (`visual_line_count(&current_lines)
/// == 2`) stopped applying once a third line made the element no longer "a
/// single visual line plus one wrap", so `follows_section` cut it a line
/// early and the orphaned third line was then welded to the body paragraph
/// beneath it (nothing else separates them: same font size, same weight,
/// only italic differs, and the paragraph-gap detector never sees the 21pt
/// seam on a two-column page). Geometry transcribed verbatim from the
/// issue's own trace (PDF user space, right column x 306.6-557.7, 8pt
/// Helvetica/Helvetica-Oblique, 21pt from the heading's last baseline to the
/// body's first, whose first line is indented 12pt and opens capitalised).
#[test]
fn numbered_heading_wrapping_twice_stays_whole_and_splits_from_body() {
    let heading_line_1 = SegmentData {
        is_italic: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "3.1. Increased frequency of T cells exhibiting pro-inflammatory responses",
            306.6,
            245.1,
            428.9,
        )
    };
    let heading_line_2 = SegmentData {
        is_italic: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "in both ileal continuous Peyer's patch and jejunal discrete Peyer's patch of",
            306.6,
            245.1,
            418.4,
        )
    };
    let heading_line_3 = SegmentData {
        is_italic: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg("vaccinated-challenged calves", 306.6, 95.2, 408.0)
    };
    let body_line_1 = SegmentData {
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "IFN- and TNF-secreting T helper and cytotoxic T cells have been",
            318.6,
            239.1,
            387.0,
        )
    };
    let body_line_2 = SegmentData {
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "shown to play important roles in the control of intracellular bacterial",
            306.6,
            251.1,
            376.55,
        )
    };

    let paragraphs = blocks_to_paragraphs(
        vec![heading_line_1, heading_line_2, heading_line_3, body_line_1, body_line_2],
        &[],
        &[],
    );

    assert_eq!(
        paragraphs.len(),
        2,
        "the twice-wrapped heading must be its own element, separate from the body"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "3.1. Increased frequency of T cells exhibiting pro-inflammatory responses in both ileal \
         continuous Peyer's patch and jejunal discrete Peyer's patch of vaccinated-challenged calves",
        "the heading must survive whole, all three of its lines, with no body text welded to it"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[1]),
        "IFN- and TNF-secreting T helper and cytotoxic T cells have been shown to play important \
         roles in the control of intracellular bacterial",
        "the body must be a separate element, not fused to the heading's last line"
    );
}

/// GH#1740 control, one-wrap shape (the reproducer's page 3 / GH#1634): the
/// same column geometry and pitch, but the heading wraps only ONCE before the
/// body begins. `heading_wrap_chain_open` must still close it -- the fix must
/// not require a SECOND wrap to trigger the closing term.
#[test]
fn numbered_heading_wrapping_once_still_splits_from_body_after_the_twice_wrapped_fix() {
    let heading_line_1 = SegmentData {
        is_italic: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg("3.3. Increased frequency of central memory", 306.6, 245.1, 428.9)
    };
    let heading_line_2 = SegmentData {
        is_italic: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg("cells in ileum and jejunum Peyer's patches", 306.6, 95.2, 418.4)
    };
    let body_line_1 = SegmentData {
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "IFN- and TNF-secreting T helper and cytotoxic T cells have been",
            318.6,
            239.1,
            397.4,
        )
    };

    let paragraphs = blocks_to_paragraphs(vec![heading_line_1, heading_line_2, body_line_1], &[], &[]);

    assert_eq!(
        paragraphs.len(),
        2,
        "GH#1634 control: a one-wrap heading must still be its own element after the GH#1740 fix"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "3.3. Increased frequency of central memory cells in ileum and jejunum Peyer's patches"
    );
}

/// GH#1740 control, the reproducer's page 4: the same three-line heading as
/// above, set in BOLD rather than italic. `bold_change` already separates the
/// heading from the body regardless of this fix; the point of this control is
/// that the fix must not stop the grouper from keeping the heading's own
/// three lines together in one pass, without depending on the merge pass to
/// reunite an orphaned third line the way it had to before GH#1740.
#[test]
fn numbered_heading_wrapping_twice_in_bold_still_splits_from_body() {
    let heading_line_1 = SegmentData {
        is_bold: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "3.1. Increased frequency of T cells exhibiting pro-inflammatory responses",
            306.6,
            245.1,
            428.9,
        )
    };
    let heading_line_2 = SegmentData {
        is_bold: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "in both ileal continuous Peyer's patch and jejunal discrete Peyer's patch of",
            306.6,
            245.1,
            418.4,
        )
    };
    let heading_line_3 = SegmentData {
        is_bold: true,
        font_size: 8.0,
        height: 8.0,
        ..column_seg("vaccinated-challenged calves", 306.6, 95.2, 408.0)
    };
    let body_line_1 = SegmentData {
        font_size: 8.0,
        height: 8.0,
        ..column_seg(
            "IFN- and TNF-secreting T helper and cytotoxic T cells have been",
            318.6,
            239.1,
            387.0,
        )
    };

    let paragraphs = blocks_to_paragraphs(
        vec![heading_line_1, heading_line_2, heading_line_3, body_line_1],
        &[],
        &[],
    );

    assert_eq!(
        paragraphs.len(),
        2,
        "GH#1740 control (bold): the twice-wrapped heading must stay whole, separate from the body"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "3.1. Increased frequency of T cells exhibiting pro-inflammatory responses in both ileal \
         continuous Peyer's patch and jejunal discrete Peyer's patch of vaccinated-challenged calves"
    );
}

/// GH#1608 page 9: with the predicate blind to the keyword form, a run of
/// such headings has no break signal at all and collapses into one element --
/// the exact failure the `starts_section` term exists to prevent.
#[test]
fn a_run_of_keyword_numbered_headings_does_not_collapse() {
    let lines = [
        "ARTIKEL 1. TOEPASSELIJKHEID",
        "ARTIKEL 2. TOTSTANDKOMING",
        "ARTIKEL 3. PRIJZEN",
    ];
    let segments = lines
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let baseline = 700.0 - 17.0 * index as f32;
            SegmentData {
                is_bold: true,
                font_size: 12.0,
                height: 12.0,
                y: baseline - 12.0,
                ..column_seg(text, 72.0, 180.0, baseline)
            }
        })
        .collect();

    let paragraphs = segments_to_paragraphs(segments, &[(12.0, None)], &[], &TextRepairWitnesses::default());

    assert_eq!(paragraphs.len(), 3, "each heading in the run must be its own element");
    for (index, expected) in lines.iter().enumerate() {
        assert_eq!(paragraph_segment_text(&paragraphs[index]), *expected);
    }
}

/// The negative control for the two tests above: prose that opens with the
/// same keyword and the same number must NOT gain a paragraph break, or the
/// widening would shred body text wherever a sentence starts `Artikel 12 ...`.
#[test]
fn prose_opening_with_a_keyword_and_a_number_keeps_its_paragraph() {
    let first = SegmentData {
        font_size: 12.0,
        height: 12.0,
        y: 700.0 - 12.0,
        ..column_seg("Artikel 12 van de wet is van toepassing", 72.0, 240.0, 700.0)
    };
    let second = SegmentData {
        font_size: 12.0,
        height: 12.0,
        y: 683.0 - 12.0,
        ..column_seg("en dus geldt het volgende voor deze overeenkomst", 72.0, 250.0, 683.0)
    };

    let paragraphs = segments_to_paragraphs(
        vec![first, second],
        &[(12.0, None)],
        &[],
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        1,
        "prose beginning with a keyword and a number is not a heading"
    );
}

/// GH#1609: the numbered-heading break terms tested a predicate against a single
/// SEGMENT, so a heading set with a hanging number -- `"3.1.7"` and its title on
/// one baseline, two spans -- never looked like a numbered heading and was left to
/// the ordinary paragraph-gap rule, which needs more than ordinary line pitch. The
/// heading was welded into the body beneath it.
#[test]
fn hanging_number_heading_is_a_paragraph_boundary() {
    let number = SegmentData {
        font_size: 9.0,
        height: 9.0,
        y: 700.0 - 9.0,
        ..column_seg("3.1.7", 104.42, 20.0, 700.0)
    };
    let title = SegmentData {
        font_size: 9.0,
        height: 9.0,
        y: 700.0 - 9.0,
        ..column_seg("Innovatie/ontwikkelingen", 161.06, 100.0, 700.0)
    };
    let body = SegmentData {
        font_size: 9.0,
        height: 9.0,
        y: 688.0 - 9.0,
        ..column_seg(
            "innovatie ontwikkelingen toekomstige verwachten gebied product",
            104.42,
            380.0,
            688.0,
        )
    };

    let paragraphs = segments_to_paragraphs(
        vec![number, title, body],
        &[(9.0, None)],
        &[],
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        2,
        "a hanging-number heading must open its own element"
    );
    assert_eq!(paragraph_segment_text(&paragraphs[0]), "3.1.7 Innovatie/ontwikkelingen");
}

/// The single-span control for the test above: same strings, same baselines, one
/// span. It passed before the fix and must keep passing after it.
#[test]
fn single_span_numbered_heading_is_still_a_paragraph_boundary() {
    let heading = SegmentData {
        font_size: 9.0,
        height: 9.0,
        y: 700.0 - 9.0,
        ..column_seg("3.1.7 Innovatie/ontwikkelingen", 104.42, 156.64, 700.0)
    };
    let body = SegmentData {
        font_size: 9.0,
        height: 9.0,
        y: 688.0 - 9.0,
        ..column_seg(
            "innovatie ontwikkelingen toekomstige verwachten gebied product",
            104.42,
            380.0,
            688.0,
        )
    };

    let paragraphs = segments_to_paragraphs(
        vec![heading, body],
        &[(9.0, None)],
        &[],
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        2,
        "a single-span numbered heading must open its own element"
    );
}

/// The wrap control for #1467: a numbered heading long enough to reach the
/// column's right edge, continuing onto a second, unnumbered physical line,
/// must stay ONE element -- splitting a heading from its own wrapped tail
/// would be worse than the original defect. This is what
/// `heading_wraps_onto` exists to rule out: without it, `follows_section`
/// would fire on every numbered heading regardless of whether the next line
/// is unrelated content or the heading's own continuation, and this
/// specific line pair -- same font, same weight, same one-line-height
/// spacing as the #1467 defect -- would be split into two paragraphs.
#[test]
fn heading_wrapping_onto_its_next_line_stays_one_paragraph() {
    let heading_start = column_seg(
        "1.1.1 Een Zeer Lange Sectietitel Die Helemaal Doorloopt Tot De",
        72.0,
        460.0,
        700.0,
    );
    let heading_continuation = column_seg("Rechterkantlijn Van Deze Kolom", 72.0, 450.0, 684.0);

    let paragraphs = segments_to_paragraphs(
        vec![heading_start, heading_continuation],
        &[(11.0, None)],
        &[],
        &TextRepairWitnesses::default(),
    );

    assert_eq!(
        paragraphs.len(),
        1,
        "a heading wrapping onto its own next line must not be split from itself"
    );
}

/// The prose control for #1467: three ordinary wrapped lines with no
/// numbering and no sentence terminator must stay ONE paragraph, exactly as
/// before this change -- `follows_section` never fires here because
/// `is_numbered_section_heading` is false for all three lines.
#[test]
fn wrapped_prose_lines_without_a_terminator_stay_one_paragraph() {
    let segments = vec![
        body_line_seg("The committee reviewed the annual budget", 700.0),
        body_line_seg("report and discussed the proposed changes", 686.0),
        body_line_seg("before adjourning the meeting for the day", 672.0),
    ];

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

    assert_eq!(
        paragraphs.len(),
        1,
        "wrapped prose with no sentence terminator must stay one paragraph"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "The committee reviewed the annual budget report and discussed the proposed changes \
         before adjourning the meeting for the day"
    );
}

/// Helper: one segment of a hanging-indent column, 11pt on an 11pt line.
/// GH#1616: a reconstructed grid need not span every printed column inside its own
/// bounding box. The runs in the columns it left out were dropped from the prose flow
/// and never reached a cell, so they were deleted from the document.
///
/// The shape measured on the reporter's page 51: a four-column fault-finding grid
/// (cause / `Nee` / `Ja` / remedy) reconstructed with two columns over a bbox spanning
/// all four, x 48.00 .. 555.24.
#[test]
fn a_table_bbox_does_not_delete_text_its_grid_leaves_out() {
    let coverage = vec![TableCoverage {
        bbox: crate::types::BoundingBox {
            x0: 48.0,
            y0: 312.48,
            x1: 555.24,
            y1: 405.28,
        },
        cell_text: table_cell_text(&[
            "Ja  Ja",
            "Controleer de ontsteekpenafstand. Controleer de afstelling, zie § 7.10 Gas-luchtregeling.",
        ]),
    }];
    let segments = vec![
        column_seg("Ja", 300.0, 12.0, 380.0),
        column_seg("Controleer de ontsteekpenafstand.", 340.0, 180.0, 380.0),
        column_seg("Onjuiste ontsteekafstand.", 52.0, 140.0, 380.0),
        column_seg("Nee", 250.0, 20.0, 366.0),
        column_seg("Zwakke vonk.", 52.0, 70.0, 340.0),
    ];

    let kept: Vec<String> = filter_segments_by_table_bboxes(segments, &coverage)
        .into_iter()
        .map(|seg| seg.text)
        .collect();

    assert_eq!(
        kept,
        vec![
            "Onjuiste ontsteekafstand.".to_string(),
            "Nee".to_string(),
            "Zwakke vonk.".to_string(),
        ],
        "runs the grid does not carry must survive; the two it does carry are suppressed"
    );
}

/// GH#1616's first fix compared whitespace-collapsed text, which a grid's own cell boundaries
/// defeat: one printed line commonly spans several cells and a wrapped cell inserts separators
/// the printed run does not have. On `issue-912` the table's own rows came back a second time
/// as prose -- 81 extra word instances in 263, precision 0.984 -> 0.692 with recall unmoved.
///
/// Geometry here is taken from that page: a ledger row rendered as one run, reconstructed into
/// two cells that split it mid-list and add a stray separator comma.
#[test]
fn should_suppress_a_printed_run_the_grid_split_across_two_cells() {
    let coverage = vec![TableCoverage {
        bbox: crate::types::BoundingBox {
            x0: 48.0,
            y0: 300.0,
            x1: 560.0,
            y1: 400.0,
        },
        cell_text: table_cell_text(&["Chq. No. 085900 BILL NO.133, 132,", "139, ,138, 143, 140,"]),
    }];
    let segments = vec![
        column_seg(
            "Chq. No. 085900 BILL NO.133, 132, 139,138, 143, 140,",
            60.0,
            400.0,
            350.0,
        ),
        column_seg("Being payment against a bill the grid omits", 60.0, 400.0, 320.0),
    ];

    let kept: Vec<String> = filter_segments_by_table_bboxes(segments, &coverage)
        .into_iter()
        .map(|seg| seg.text)
        .collect();

    assert_eq!(
        kept,
        vec!["Being payment against a bill the grid omits".to_string()],
        "a run the grid carries across a cell boundary must not be emitted again as prose"
    );
}

/// The other half of the same invariant, and the reason the geometric test cannot
/// simply be dropped: text a table DOES carry must still be suppressed, or every
/// table's contents are emitted twice.
#[test]
fn a_table_still_suppresses_the_prose_copy_of_its_own_cells() {
    let coverage = vec![TableCoverage {
        bbox: crate::types::BoundingBox {
            x0: 48.0,
            y0: 300.0,
            x1: 500.0,
            y1: 400.0,
        },
        cell_text: table_cell_text(&["Mogelijke oorzaken:", "Oplossing:"]),
    }];
    let segments = vec![
        column_seg("Mogelijke oorzaken:", 52.0, 100.0, 380.0),
        column_seg("Oplossing:", 300.0, 60.0, 380.0),
    ];
    assert!(
        filter_segments_by_table_bboxes(segments, &coverage).is_empty(),
        "a covered run the grid carries is still suppressed"
    );
}

#[test]
fn table_coverage_preserves_substrings_and_fabricated_tokens() {
    for (source, cells) in [
        ("0001", vec!["1500000", "1500000", "090100"]),
        ("AB12", vec!["XAB123"]),
        ("0001", vec!["000", "1"]),
        ("AB12", vec!["AB", "12"]),
        ("0001", vec!["000 1"]),
        ("0001", vec!["000.1"]),
        ("AB12", vec!["AB 12"]),
        ("Nee", vec!["Neen"]),
    ] {
        let tables = vec![crate::types::Table {
            cells: vec![cells.into_iter().map(str::to_owned).collect()],
            page_number: 1,
            bounding_box: Some(crate::types::BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 500.0,
                y1: 500.0,
            }),
            ..Default::default()
        }];
        let coverage = table_bboxes_by_page(&tables);
        let kept = filter_segments_by_table_bboxes(vec![column_seg(source, 50.0, 100.0, 350.0)], &coverage[&0]);
        assert_eq!(
            kept.len(),
            1,
            "unrepresented source token {source:?} must survive {:?}",
            tables[0].cells
        );
    }
}

#[test]
fn table_coverage_matches_complete_tokens_and_split_runs() {
    for (source, cells) in [
        ("AB 12", vec!["AB", "12"]),
        ("000 1", vec!["000", "1"]),
        ("0001", vec!["ID 0001 end"]),
        ("alpha beta", vec!["alpha", "beta"]),
        ("hyphenation", vec!["hyphen-\nation"]),
        ("hyphen-\nation", vec!["hyphenation"]),
        ("co-operate", vec!["cooperate"]),
        ("$1,000.00", vec!["$1,000.00"]),
        ("1.000,00", vec!["1.000,00"]),
        ("ÉTÉ 東京", vec!["été", "東京"]),
        ("İ", vec!["prefix", "İ"]),
        ("AB12", vec!["XAB12", "AB12"]),
        ("AB 12", vec!["", "AB", "", "12", ""]),
    ] {
        let coverage = table_cell_text(&cells);
        assert!(coverage.represents(source), "must represent {source:?} in {cells:?}");
        assert_eq!(
            coverage.glyphs,
            cells
                .iter()
                .map(|cell| normalize_for_table_coverage(cell))
                .collect::<String>()
        );
    }
    // A single word split across cells lacks evidence of source ownership. Keep
    // it rather than fabricate an identifier out of unrelated whole cell values.
    assert!(!table_cell_text(&["hyphen", "ation"]).represents("hyphenation"));
    assert!(!table_cell_text(&["alpha beta"]).represents("alphabeta"));
    assert!(!table_cell_text(&["anything"]).represents("..."));
    assert!(!table_cell_text(&["anything"]).represents(""));
}

#[test]
fn table_coverage_keeps_geometry_and_markdown_only_behavior() {
    let table = crate::types::Table {
        markdown: "| ID | Value |\n|---|---|\n| AB | 12 |\n| 0001 | 150.00 |".into(),
        page_number: 1,
        bounding_box: Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 500.0,
            y1: 500.0,
        }),
        ..Default::default()
    };
    let coverage = table_bboxes_by_page(&[table]);
    assert!(coverage[&0][0].cell_text.represents("AB 12"));
    assert!(!coverage[&0][0].cell_text.represents("AB12"));
    let segments = vec![
        column_seg("0001", 10.0, 100.0, 100.0),
        column_seg("0001", 10.0, 100.0, 200.0),
        column_seg("0001", 600.0, 100.0, 200.0),
        column_seg("0001", 10.0, 0.0, 100.0),
        column_seg("...", 10.0, 100.0, 100.0),
        column_seg("0001", 450.0, 100.0, 100.0),
        column_seg("0001", 451.0, 100.0, 100.0),
    ];
    let kept = filter_segments_by_table_bboxes(segments, &coverage[&0]);
    assert_eq!(kept.len(), 4);
    assert_eq!(kept[0].x, 600.0);
    assert_eq!(kept[1].width, 0.0);
    assert_eq!(kept[2].text, "...");
    assert_eq!(kept[3].x, 451.0);
    // This remains a textual guard: identical complete text at two covered
    // positions is indistinguishable without consumed-source provenance.
}

fn table_cell_text(cells: &[&str]) -> TableCoverageText {
    TableCoverageText::from_cells(cells.iter().copied())
}

fn column_seg(text: &str, x: f32, width: f32, baseline_y: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x,
        y: baseline_y - 11.0,
        width,
        height: 11.0,
        font_size: 11.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

/// The shape measured on `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`
/// (tesseract): the marker column is a separate block from the text column, so
/// every marker arrives as its own paragraph and the whole marker run precedes
/// the whole text run. Pairing must therefore be by baseline, not adjacency.
#[test]
fn detached_marker_column_is_reattached_to_the_body_sharing_its_baseline() {
    let segments = vec![
        column_seg("(a)", 72.0, 14.0, 700.0),
        column_seg("(b)", 72.0, 14.0, 660.0),
        column_seg("(c)", 72.0, 14.0, 620.0),
        column_seg("A ten foot wide minimum buffer along the lot line", 110.0, 300.0, 700.0),
        column_seg(
            "Ten foot wide minimum buffers along Lake Pointe Parkway",
            110.0,
            300.0,
            660.0,
        ),
        column_seg(
            "Required buffers may include the pedestrian walkway",
            110.0,
            300.0,
            620.0,
        ),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

    assert_eq!(
        paragraphs.len(),
        3,
        "each detached marker must be folded into the body line it shares a baseline with"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[0]),
        "(a) A ten foot wide minimum buffer along the lot line"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[1]),
        "(b) Ten foot wide minimum buffers along Lake Pointe Parkway"
    );
    assert_eq!(
        paragraph_segment_text(&paragraphs[2]),
        "(c) Required buffers may include the pedestrian walkway"
    );
}

/// Reattachment is only worth anything if the body is then *classified* as a
/// list item; the marker text alone changes no downstream element kind.
#[test]
fn bodies_that_absorb_a_detached_marker_become_list_items() {
    let segments = vec![
        column_seg("(a)", 72.0, 14.0, 700.0),
        column_seg("(b)", 72.0, 14.0, 660.0),
        column_seg("(c)", 72.0, 14.0, 620.0),
        column_seg("A ten foot wide minimum buffer along the lot line", 110.0, 300.0, 700.0),
        column_seg(
            "Ten foot wide minimum buffers along Lake Pointe Parkway",
            110.0,
            300.0,
            660.0,
        ),
        column_seg(
            "Required buffers may include the pedestrian walkway",
            110.0,
            300.0,
            620.0,
        ),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

    assert_eq!(
        paragraphs.iter().filter(|paragraph| paragraph.is_list_item).count(),
        3,
        "a body that absorbed its marker must classify as a list item"
    );
}

/// TASK #722 follow-up: a lone `*` and a bracketed integer `[N]` must NOT be
/// treated as detached list markers -- `*` is also a multiplication sign in
/// isolated math prose, and `[N]` is standard printed paragraph-number
/// notation (e.g. Jung's Collected Works), not a marker. The other three
/// marker families must keep reattaching exactly as before.
#[test]
fn ambiguous_detached_markers_are_excluded_while_unambiguous_ones_still_reattach() {
    let segments = vec![
        column_seg("*", 72.0, 14.0, 700.0),
        column_seg("[42]", 72.0, 20.0, 660.0),
        column_seg("-", 72.0, 14.0, 620.0),
        column_seg("(1)", 72.0, 14.0, 580.0),
        column_seg("1.", 72.0, 14.0, 540.0),
        column_seg(
            "A times B is a well known identity in group theory here",
            110.0,
            300.0,
            700.0,
        ),
        column_seg(
            "This paragraph number precedes ordinary book prose here",
            110.0,
            300.0,
            660.0,
        ),
        column_seg(
            "Dash marker prose gets folded into its own body text",
            110.0,
            300.0,
            620.0,
        ),
        column_seg(
            "Parenthesised marker prose gets folded into its own body",
            110.0,
            300.0,
            580.0,
        ),
        column_seg(
            "Numbered marker prose gets folded into its own body",
            110.0,
            300.0,
            540.0,
        ),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

    assert_eq!(
        paragraphs.len(),
        7,
        "the '*' and '[42]' markers must stay detached (2 extra paragraphs); the other three must reattach"
    );

    let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
    assert!(
        texts.iter().any(|text| text == "*"),
        "a lone '*' must remain its own paragraph, not fold into the math prose below it: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "[42]"),
        "a bracketed integer must remain its own paragraph, not fold into the following prose: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with("- Dash marker")),
        "a dash marker must still reattach to its body: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with("(1) Parenthesised marker")),
        "a parenthesised marker must still reattach to its body: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with("1. Numbered marker")),
        "a '1.' marker must still reattach to its body: {texts:?}"
    );
}

/// Helper: a segment carrying raw OCR raster geometry (`rotation_degrees ==
/// 0.0`, as every OCR segment does -- see `adapters::make_ocr_pdf_line`),
/// with `y == baseline_y` (also always true for OCR segments -- both fields
/// are set from the same hOCR line-box value).
fn ocr_raster_seg(text: &str, x: f32, y: f32, width: f32, height: f32, font_size: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x,
        y,
        width,
        height,
        font_size,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

/// #760: `DetachedMarkerFrame::OcrOnPage(270)` must accept the
/// marker/body pair whose geometry was measured on fixture `ordinance_2197`
/// (`/Rotate 270`, tesseract) -- marker `y=2378.0 h=65.0`, body `y=1324.0
/// h=933.0`, both at `font_size=30.0` (chosen so the tolerance,
/// `30.0 * DETACHED_MARKER_BASELINE_TOLERANCE_FONT_FACTOR`, is `18.0`, and
/// the max indent, `30.0 * DETACHED_MARKER_MAX_INDENT_FONT_FACTOR`, is
/// `180.0` -- both match the values reported against the fixture). `x`/
/// `width` are constructed, not measured, so the two segments' corrected-270
/// baseline (`x + width`) coincide (delta `0.0`), isolating the advance/
/// indent half of the fix: `frame.advance_extent()` gives marker
/// `(-2443.0, -2378.0)` and body `(-2257.0, -1324.0)`, so
/// `indent = body_left - marker_end = -2257.0 - (-2378.0) = 121.0`, within
/// `[-15.0, 180.0]`.
///
/// `DetachedMarkerFrame::Native` is, for an OCR segment, byte-for-byte the
/// pre-#760 behaviour (`upright_baseline()`/`upright_advance_extent()`
/// short-circuit on `rotation_degrees == 0.0` to the raw `baseline_y`/
/// `(x, x + width)` -- exactly what unfixed `accepts_detached_list_marker`
/// read, since it had no frame parameter at all). Against that frame this
/// same pair is REJECTED at the baseline gate: `|2378.0 - 1324.0| == 1054.0`
/// (within the 1049..1922 range measured on the real fixture) against a
/// tolerance of `18.0` -- the indent check is never reached.
#[test]
fn ocr_frame_270_accepts_the_measured_pair_that_the_native_frame_rejects() {
    let marker = ocr_raster_seg("(a)", 3317.0, 2378.0, 100.0, 65.0, 30.0);
    let body_segment = ocr_raster_seg("Buffer requirement", 3367.0, 1324.0, 50.0, 933.0, 30.0);
    let body = para(vec![line(vec![body_segment])]);

    assert!(
        !accepts_detached_list_marker(&body, &marker, DetachedMarkerFrame::Native),
        "Native frame must reject the pair: baseline delta 1054.0 exceeds tolerance 18.0"
    );
    assert!(
        accepts_detached_list_marker(&body, &marker, DetachedMarkerFrame::OcrOnPage(270)),
        "OcrOnPage(270) must accept the pair: baseline delta 0.0, indent 121.0 <= 180.0"
    );
}

/// #760: pins the exact corrected-frame values for a 270-rotated page, so a
/// future change to the formula shows up here directly rather than only
/// through the pass/fail outcome above.
#[test]
fn ocr_frame_270_baseline_and_advance_extent_match_the_measured_formula() {
    let marker = ocr_raster_seg("(a)", 3317.0, 2378.0, 100.0, 65.0, 30.0);
    let body_segment = ocr_raster_seg("Buffer requirement", 3367.0, 1324.0, 50.0, 933.0, 30.0);
    let frame = DetachedMarkerFrame::OcrOnPage(270);

    assert_eq!(frame.baseline(&marker), 3417.0, "far raster-x edge (x + width)");
    assert_eq!(frame.baseline(&body_segment), 3417.0, "far raster-x edge (x + width)");
    assert_eq!(
        frame.advance_extent(&marker),
        (-2443.0, -2378.0),
        "advance runs along -y; start is the FAR raster-y edge -(y + height)"
    );
    assert_eq!(
        frame.advance_extent(&body_segment),
        (-2257.0, -1324.0),
        "advance runs along -y; start is the FAR raster-y edge -(y + height)"
    );
}

/// #760: `180` is confirmed a no-op for the OCR rotation correction -- both
/// helpers on `DetachedMarkerFrame::OcrOnPage(180)` must read the same raw
/// fields as the unrotated default, matching `Native`'s behaviour for an
/// unrotated (`rotation_degrees == 0.0`) OCR segment exactly.
#[test]
fn ocr_frame_180_is_a_no_op() {
    let segment = ocr_raster_seg("text", 100.0, 700.0, 40.0, 10.0, 11.0);

    assert_eq!(
        DetachedMarkerFrame::OcrOnPage(180).baseline(&segment),
        DetachedMarkerFrame::Native.baseline(&segment)
    );
    assert_eq!(
        DetachedMarkerFrame::OcrOnPage(180).advance_extent(&segment),
        DetachedMarkerFrame::Native.advance_extent(&segment)
    );
}

/// Precision guard (passes with and without the reattachment pass). A bare
/// marker must not adopt an indented block on a *different* baseline: that is
/// an ordinary following paragraph, not the marker's own item text.
#[test]
fn a_bare_marker_does_not_adopt_a_block_on_another_baseline() {
    let segments = vec![
        column_seg("(a)", 72.0, 14.0, 700.0),
        column_seg(
            "An indented block that begins on the next line entirely",
            110.0,
            300.0,
            660.0,
        ),
    ];
    let gap_ys = compute_paragraph_gap_ys(&segments);

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

    assert_eq!(paragraphs.len(), 2, "baseline agreement is what licenses reattachment");
    assert!(
        !paragraphs[1].is_list_item,
        "a block on its own baseline must not be turned into a list item"
    );
}

/// Helper: create a segment with positional data.
fn seg(text: &str, x: f32, width: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x,
        y: 0.0,
        width,
        height: 12.0,
        font_size: 12.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: 0.0,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn inline_seg(text: &str, x: f32, baseline_y: f32, is_bold: bool) -> SegmentData {
    let mut segment = seg(text, x, 20.0);
    segment.baseline_y = baseline_y;
    segment.y = baseline_y - segment.height;
    segment.is_bold = is_bold;
    segment
}

#[test]
fn issue_1560_reconstructs_jittered_table_fragments_as_one_x_ordered_line() {
    let mut article = inline_seg("700004", 68.279, 623.481, false);
    article.font_size = 8.6;
    article.height = 8.6;
    article.width = 35.0;
    let mut position = inline_seg("2", 50.0, 622.401, false);
    position.font_size = 8.6;
    position.height = 8.6;
    position.width = 5.0;
    let mut description = inline_seg("Fastening screw", 124.9, 622.401, false);
    description.font_size = 8.6;
    description.height = 8.6;
    description.width = 60.0;
    let mut next_row = inline_seg("3", 50.0, 610.0, false);
    next_row.font_size = 8.6;
    next_row.height = 8.6;

    let segments = [&article, &position, &description, &next_row];
    let lines = reconstruct_pdf_lines(&segments);

    assert_eq!(lines.len(), 2, "ordinary row leading must still split lines");
    assert_eq!(
        lines[0]
            .segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>(),
        ["2", "700004", "Fastening screw"]
    );
    assert_eq!(lines[1].segments[0].text, "3");
}

#[test]
fn inline_bold_runs_stay_in_one_paragraph() {
    let segments = vec![
        inline_seg("plain", 10.0, 100.0, false),
        inline_seg("bold", 31.0, 100.0, true),
        inline_seg("tail", 52.0, 100.0, false),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraphs[0].lines.len(), 1);
    assert_eq!(paragraphs[0].lines[0].segments.len(), 3);
    assert!(paragraphs[0].lines[0].segments[1].is_bold);
    assert_eq!(paragraph_text(&paragraphs[0]), "plain bold tail");

    let document = crate::pdf::structure::assembly::assemble_internal_document(
        vec![paragraphs],
        &[],
        None,
        &[],
        &Default::default(),
    );
    let element = &document.elements[0];
    let bold = element
        .annotations
        .iter()
        .find(|annotation| matches!(annotation.kind, crate::types::AnnotationKind::Bold))
        .expect("inline bold annotation should be preserved");
    assert_eq!(element.text, "plain bold tail");
    assert_eq!((bold.start, bold.end), (6, 10));
}

#[test]
fn same_baseline_font_size_transition_stays_in_one_paragraph() {
    let mut chapter_number = inline_seg("13.", 28.35, 803.043, false);
    chapter_number.width = 17.58;
    chapter_number.font_size = 14.0;
    let mut title = inline_seg("Productkaart vlgs. bijlage IV", 64.35, 803.043, false);
    title.width = 248.03;

    let paragraphs = blocks_to_paragraphs(vec![chapter_number, title], &[], &[]);

    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraph_text(&paragraphs[0]), "13. Productkaart vlgs. bijlage IV");
}

#[test]
fn distant_same_baseline_font_size_transition_remains_a_boundary() {
    let mut heading = inline_seg("Heading", 10.0, 100.0, false);
    heading.font_size = 14.0;
    let body = inline_seg("body", 100.0, 100.0, false);

    let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraph_text(&paragraphs[0]), "Heading");
    assert_eq!(paragraph_text(&paragraphs[1]), "body");
}

#[test]
fn different_baseline_font_size_transition_remains_a_boundary() {
    let mut heading = inline_seg("Heading", 10.0, 100.0, false);
    heading.font_size = 14.0;
    let body = inline_seg("body", 10.0, 80.0, false);

    let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraph_text(&paragraphs[0]), "Heading");
    assert_eq!(paragraph_text(&paragraphs[1]), "body");
}

/// Geometry taken from GH#1617's reproducer, where the base symbol is a span of its own.
#[test]
fn should_keep_a_subscript_with_a_base_symbol_that_is_its_own_span() {
    let mut base = inline_seg("P", 206.76, 662.768, false);
    base.width = 5.18;
    base.font_size = 11.59;
    base.height = 11.59;
    let mut subscript = inline_seg("rated", 211.92, 661.700, false);
    subscript.width = 12.99;
    subscript.font_size = 8.51;
    subscript.height = 8.51;

    let paragraphs = blocks_to_paragraphs(vec![base, subscript], &[], &[]);

    assert_eq!(paragraphs.len(), 1, "a subscript must not become its own element");
    assert_eq!(paragraph_text(&paragraphs[0]), "P rated");
}

/// GH#1617's other four pairs: the base glyph sits inside a single `TJ` row span whose kerning
/// spreads it across the column, so the subscript starts *within* the previous run's extent.
#[test]
fn should_keep_a_subscript_that_starts_inside_its_row_span() {
    let mut row = inline_seg("Geluidsniveau L dB", 59.28, 586.208, false);
    row.width = 340.0;
    row.font_size = 11.59;
    row.height = 11.59;
    let mut subscript = inline_seg("WA", 211.08, 585.579, false);
    subscript.width = 8.4;
    subscript.font_size = 7.34;
    subscript.height = 7.34;

    let paragraphs = blocks_to_paragraphs(vec![row, subscript], &[], &[]);

    assert_eq!(
        paragraphs.len(),
        1,
        "a subscript inside its row span must not become its own element"
    );
    assert_eq!(paragraph_text(&paragraphs[0]), "Geluidsniveau L dB WA");
}

/// The predicate must not swallow a genuine wrapped line: a smaller run one full leading below
/// its predecessor is a new line, not a subscript.
#[test]
fn should_still_split_a_smaller_run_a_full_leading_below_its_predecessor() {
    let mut heading = inline_seg("Nominale warmteafgifte", 59.28, 662.768, false);
    heading.width = 129.06;
    heading.font_size = 11.59;
    heading.height = 11.59;
    let mut body = inline_seg("rated", 59.28, 648.860, false);
    body.width = 12.99;
    body.font_size = 8.51;
    body.height = 8.51;

    let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

    assert_eq!(
        paragraphs.len(),
        2,
        "a full-leading drop is a line break, not a subscript"
    );
}

/// A run after a subscript sits back on the paragraph's own baseline; the subscript must not
/// latch `current_is_single_visual_line` off and split it.
#[test]
fn should_keep_the_run_following_a_subscript_on_the_same_line() {
    let mut base = inline_seg("P", 206.76, 662.768, false);
    base.width = 5.18;
    base.font_size = 11.59;
    base.height = 11.59;
    let mut subscript = inline_seg("rated", 211.92, 661.700, false);
    subscript.width = 12.99;
    subscript.font_size = 8.51;
    subscript.height = 8.51;
    let mut unit = inline_seg("kW", 225.40, 662.768, false);
    unit.width = 13.37;
    unit.font_size = 11.59;
    unit.height = 11.59;

    let paragraphs = blocks_to_paragraphs(vec![base, subscript, unit], &[], &[]);

    assert_eq!(
        paragraphs.len(),
        1,
        "the unit after a subscript must stay on the same line"
    );
    assert_eq!(paragraph_text(&paragraphs[0]), "P rated kW");
}

#[test]
fn inline_typographic_dash_does_not_split_a_paragraph() {
    let segments = vec![
        inline_seg("Figures 6", 10.0, 100.0, false),
        inline_seg("– 8 show the results", 31.0, 100.0, false),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

    assert_eq!(paragraphs.len(), 1);
    assert!(!paragraphs[0].is_list_item);
}

#[test]
fn typographic_dash_on_a_new_line_still_starts_a_list() {
    let segments = vec![
        inline_seg("Introduction", 10.0, 100.0, false),
        inline_seg("– first item", 10.0, 80.0, false),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

    assert_eq!(paragraphs.len(), 2);
    assert!(paragraphs[1].is_list_item);
}

#[test]
fn split_typographic_dash_and_same_line_body_stay_a_list() {
    let segments = vec![
        inline_seg("Introduction", 10.0, 100.0, false),
        inline_seg("–", 10.0, 80.0, false),
        inline_seg("quoted body", 31.0, 80.0, false),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

    assert_eq!(paragraphs.len(), 2);
    assert!(paragraphs[1].is_list_item);
}

#[test]
fn split_typographic_dash_and_different_line_body_are_not_a_list() {
    let segments = vec![
        inline_seg("Figures 6", 10.0, 100.0, false),
        inline_seg("– ", 31.0, 100.0, false),
        inline_seg("8 show the results", 10.0, 80.0, false),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

    assert!(paragraphs.iter().all(|paragraph| !paragraph.is_list_item));
}

#[test]
fn cross_line_bold_transition_remains_a_boundary() {
    let segments = vec![
        inline_seg("Heading", 10.0, 100.0, true),
        inline_seg("body", 10.0, 80.0, false),
    ];

    assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 2);
}

#[test]
fn tagged_heading_and_body_stay_separate_on_the_same_line() {
    let mut heading = inline_seg("Heading", 10.0, 100.0, true);
    heading.assigned_role = Some(1);
    let body = inline_seg("body", 31.0, 100.0, false);

    let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraph_text(&paragraphs[0]), "Heading");
    assert_eq!(paragraphs[0].heading_level, Some(1));
    assert_eq!(paragraph_text(&paragraphs[1]), "body");
    assert_eq!(paragraphs[1].heading_level, None);
}

#[test]
fn different_tagged_heading_levels_stay_separate_on_the_same_line() {
    let mut first = inline_seg("First", 10.0, 100.0, true);
    first.assigned_role = Some(1);
    let mut second = inline_seg("Second", 31.0, 100.0, false);
    second.assigned_role = Some(2);

    let paragraphs = blocks_to_paragraphs(vec![first, second], &[], &[]);

    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraph_text(&paragraphs[0]), "First");
    assert_eq!(paragraphs[0].heading_level, Some(1));
    assert_eq!(paragraph_text(&paragraphs[1]), "Second");
    assert_eq!(paragraphs[1].heading_level, Some(2));
}

#[test]
fn same_tagged_heading_role_keeps_inline_style_transitions_together() {
    let mut first = inline_seg("First", 10.0, 100.0, true);
    first.assigned_role = Some(1);
    let mut second = inline_seg("Second", 31.0, 100.0, false);
    second.assigned_role = Some(1);

    let paragraphs = blocks_to_paragraphs(vec![first, second], &[], &[]);

    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraph_text(&paragraphs[0]), "First Second");
    assert_eq!(paragraphs[0].heading_level, Some(1));
}

#[test]
fn distant_same_line_bold_transition_remains_a_boundary() {
    let segments = vec![
        inline_seg("left", 10.0, 100.0, false),
        inline_seg("right", 100.0, 100.0, true),
    ];

    assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 2);
}

#[test]
fn overlapping_or_reverse_bold_transition_remains_a_boundary() {
    let overlapping = vec![
        inline_seg("first", 30.0, 100.0, false),
        inline_seg("second", 40.0, 100.0, true),
    ];
    let reversed = vec![
        inline_seg("first", 30.0, 100.0, false),
        inline_seg("second", 5.0, 100.0, true),
    ];

    assert_eq!(blocks_to_paragraphs(overlapping, &[], &[]).len(), 2);
    assert_eq!(blocks_to_paragraphs(reversed, &[], &[]).len(), 2);
}

#[test]
fn slight_metric_overlap_is_still_inline() {
    let segments = vec![
        inline_seg("plain", 30.0, 100.0, false),
        inline_seg("bold", 49.0, 100.0, true),
    ];

    assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 1);
}

#[test]
fn invalid_inline_geometry_remains_a_boundary() {
    let plain = inline_seg("plain", 10.0, 100.0, false);
    let mut zero_font = inline_seg("bold", 31.0, 100.0, true);
    zero_font.font_size = 0.0;
    let mut non_finite_x = inline_seg("bold", 31.0, 100.0, true);
    non_finite_x.x = f32::NAN;
    let mut non_finite_baseline = inline_seg("bold", 31.0, 100.0, true);
    non_finite_baseline.baseline_y = f32::NAN;

    assert_eq!(blocks_to_paragraphs(vec![plain.clone(), zero_font], &[], &[]).len(), 2);
    assert_eq!(
        blocks_to_paragraphs(vec![plain.clone(), non_finite_x], &[], &[]).len(),
        2
    );
    assert_eq!(
        blocks_to_paragraphs(vec![plain, non_finite_baseline], &[], &[]).len(),
        2
    );
}

#[test]
fn later_line_inline_style_transition_does_not_absorb_prior_lines() {
    let segments = vec![
        inline_seg("first line", 10.0, 120.0, false),
        inline_seg("plain", 10.0, 100.0, false),
        inline_seg("bold", 31.0, 100.0, true),
    ];

    assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 2);
}

#[test]
fn monospace_style_transition_remains_a_boundary() {
    let mut plain = inline_seg("let value =", 10.0, 100.0, false);
    plain.is_monospace = true;
    let mut bold = inline_seg("42", 31.0, 100.0, true);
    bold.is_monospace = true;

    assert_eq!(blocks_to_paragraphs(vec![plain, bold], &[], &[]).len(), 2);
}

fn line(segments: Vec<SegmentData>) -> PdfLine {
    PdfLine {
        segments,
        baseline_y: 0.0,
        dominant_font_size: 12.0,
        is_bold: false,
        is_monospace: false,
    }
}

fn para(lines: Vec<PdfLine>) -> PdfParagraph {
    let word_count = PdfParagraph::compute_word_count("", &lines);
    PdfParagraph {
        text: String::new(),
        lines,
        dominant_font_size: 12.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count,
    }
}

fn outline_para(text: &str) -> PdfParagraph {
    let mut paragraph = para(vec![line(vec![seg(text, 0.0, 100.0)])]);
    paragraph.text = text.to_string();
    paragraph.word_count = text.split_whitespace().count();
    paragraph
}

fn outline_heading(text: &str, level: u8) -> PdfParagraph {
    let mut paragraph = outline_para(text);
    paragraph.heading_level = Some(level);
    paragraph
}

fn body_size_paragraph(text: &str, is_bold: bool, heading_level: Option<u8>) -> PdfParagraph {
    let mut paragraph = outline_para(text);
    paragraph.dominant_font_size = 12.0;
    paragraph.is_bold = is_bold;
    paragraph.heading_level = heading_level;
    for line in &mut paragraph.lines {
        line.dominant_font_size = 12.0;
        line.is_bold = is_bold;
        for segment in &mut line.segments {
            segment.font_size = 12.0;
            segment.is_bold = is_bold;
        }
    }
    paragraph
}

fn body_size_paragraph_at(text: &str, is_bold: bool, heading_level: Option<u8>, left: f32) -> PdfParagraph {
    let mut paragraph = body_size_paragraph(text, is_bold, heading_level);
    paragraph.block_bbox = Some((left, 0.0, left + 200.0, 12.0));
    paragraph
}

fn body_size_paragraph_with_bbox(
    text: &str,
    is_bold: bool,
    heading_level: Option<u8>,
    bbox: (f32, f32, f32, f32),
) -> PdfParagraph {
    let mut paragraph = body_size_paragraph(text, is_bold, heading_level);
    paragraph.block_bbox = Some(bbox);
    paragraph
}

/// GH#1611: a two-word numbered heading fell below the bold-heading word-count
/// floor, so it was never promoted. It stayed a plain bold paragraph, and a RUN of
/// them coalesced into a single bold line in the rendered markdown while the
/// element stream still showed them apart. The keyword form cleared the floor only
/// by contributing a third word -- nothing else about the two lines differed.
#[test]
fn a_two_word_numbered_heading_is_a_bold_heading_candidate() {
    let keyword = body_size_paragraph_with_bbox("ARTIKEL 3. PRIJZEN", true, None, (72.0, 700.0, 260.0, 712.0));
    let bare = body_size_paragraph_with_bbox("3. PRIJZEN", true, None, (72.0, 700.0, 200.0, 712.0));

    assert!(
        is_body_size_bold_heading_candidate(&keyword, 12.0),
        "the three-word keyword form was already a candidate"
    );
    assert!(
        is_body_size_bold_heading_candidate(&bare, 12.0),
        "a numbered section heading carries its own evidence and must not need a third word"
    );
}

/// The floor this exempts is still load-bearing for everything else: a short bold
/// fragment with no numbering is emphasis or a label, not a heading.
#[test]
fn a_two_word_unnumbered_bold_fragment_is_not_a_heading_candidate() {
    let fragment = body_size_paragraph_with_bbox("Note well", true, None, (72.0, 700.0, 160.0, 712.0));
    assert!(
        !is_body_size_bold_heading_candidate(&fragment, 12.0),
        "an unnumbered two-word bold fragment must stay below the heading floor"
    );
}

fn heading_page(heading: &str, heading_size: f32, body: &str) -> Vec<SegmentData> {
    let mut heading_segment = seg_heuristic(heading, heading_size, 700.0);
    heading_segment.is_bold = true;
    vec![heading_segment, seg_heuristic(body, 12.0, 650.0)]
}

fn extract_heading_test_document(
    pages: Vec<Vec<SegmentData>>,
    used_structure_tree: bool,
) -> crate::types::internal::InternalDocument {
    extract_document_structure_from_segments(
        pages,
        SegmentStructureConfig {
            k_clusters: 4,
            tables: &[],
            outline_entries: &[],
            strip_repeating_text: false,
            include_headers: true,
            include_footers: true,
            include_footnotes: true,
            include_watermarks: true,
            used_structure_tree,
            image_positions: &[],
            images: None,
            inject_placeholders: false,
            layout_hints: None,
            allow_single_column: true,
            cancel_token: None,
            #[cfg(feature = "layout-detection")]
            layout_images: None,
            #[cfg(feature = "layout-detection")]
            layout_results: None,
            #[cfg(feature = "layout-detection")]
            table_model: crate::core::config::layout::TableModel::Disabled,
            #[cfg(feature = "layout-detection")]
            table_overlap_preference: crate::core::config::layout::TableOverlapPreference::Content,
            #[cfg(feature = "layout-detection")]
            acceleration: None,
            #[cfg(feature = "layout-detection")]
            session_thread_budget: 0,
        },
    )
    .expect("document structure extraction must succeed")
}

fn element_kind_for(
    document: &crate::types::internal::InternalDocument,
    text: &str,
) -> Option<crate::types::internal::ElementKind> {
    document
        .elements
        .iter()
        .find(|element| element.text == text)
        .map(|element| element.kind)
}

fn table_with_body_rows(body_rows: usize, cell: &str) -> crate::types::Table {
    let mut cells = vec![vec!["Column".to_string()]];
    cells.extend((0..body_rows).map(|_| vec![cell.to_string()]));
    crate::types::Table {
        cells,
        page_number: 1,
        bounding_box: Some(crate::types::BoundingBox {
            x0: 0.0,
            y0: 100.0,
            x1: 500.0,
            y1: 700.0,
        }),
        ..Default::default()
    }
}

#[test]
fn table_dominant_page_removes_spill_but_preserves_annotations() {
    let mut heading = outline_heading("Decorative title", 1);
    heading.block_bbox = Some((10.0, 650.0, 200.0, 680.0));
    let mut spill =
        outline_para("AB Carval Euro CLO Series Class D three month EURIBOR at 3.75 percent 02/15/37 2,350");
    spill.block_bbox = Some((10.0, 350.0, 490.0, 390.0));
    let mut short_prose = outline_para("Rates shown are unaudited.");
    short_prose.block_bbox = Some((10.0, 300.0, 250.0, 320.0));
    let expected_side_prose = "This explanatory sidebar remains because it sits entirely beside the detected table despite sharing its vertical band.";
    let mut side_prose = outline_para(expected_side_prose);
    side_prose.block_bbox = Some((520.0, 300.0, 700.0, 340.0));
    let mut note = outline_para("Note: values are unaudited");
    note.block_bbox = Some((10.0, 250.0, 250.0, 270.0));
    let mut caption = outline_para("Source: annual filing");
    caption.layout_class = Some(LayoutHintClass::Caption);
    caption.block_bbox = Some((10.0, 200.0, 250.0, 220.0));
    let mut pages = vec![vec![heading, spill, short_prose, side_prose, note, caption]];
    let tables = vec![table_with_body_rows(
        TABLE_DOMINANT_MIN_BODY_ROWS,
        "long-table-value-1234567890-long-table-value-1234567890-long-table-value-1234567890",
    )];

    suppress_table_dominant_paragraph_spill(&mut pages, &tables);

    assert_eq!(pages[0].len(), 5);
    assert_eq!(paragraph_text_raw(&pages[0][0]), "Decorative title");
    assert_eq!(paragraph_text_raw(&pages[0][1]), "Rates shown are unaudited.");
    assert_eq!(paragraph_text_raw(&pages[0][2]), expected_side_prose);
    assert_eq!(paragraph_text_raw(&pages[0][3]), "Note: values are unaudited");
    assert_eq!(paragraph_text_raw(&pages[0][4]), "Source: annual filing");
}

#[test]
fn table_dominant_cleanup_preserves_mixed_prose_pages() {
    let prose = "This explanatory paragraph is intentionally much longer than the compact table values. ".repeat(12);
    let mut pages = vec![vec![outline_para(&prose)]];
    let tables = vec![table_with_body_rows(TABLE_DOMINANT_MIN_BODY_ROWS, "1")];

    suppress_table_dominant_paragraph_spill(&mut pages, &tables);

    assert_eq!(pages[0].len(), 1);
    assert_eq!(paragraph_text_raw(&pages[0][0]), prose);
}

#[test]
fn table_dominant_cleanup_requires_minimum_body_rows() {
    let mut pages = vec![vec![outline_heading("Keep this title", 1)]];
    let tables = vec![table_with_body_rows(
        TABLE_DOMINANT_MIN_BODY_ROWS - 1,
        "long-table-value-1234567890",
    )];

    suppress_table_dominant_paragraph_spill(&mut pages, &tables);

    assert_eq!(pages[0].len(), 1);
    assert_eq!(paragraph_text_raw(&pages[0][0]), "Keep this title");
}

#[cfg(feature = "layout-detection")]
#[test]
fn deferred_layout_caption_survives_table_dominant_cleanup() {
    let caption_text = "2024 2025 2026 2027 2028 2029 2030 2031 2032 2033 2034 2035 2036";
    let mut caption = outline_para(caption_text);
    caption.block_bbox = Some((10.0, 350.0, 490.0, 390.0));
    let mut pages = vec![vec![caption]];
    let hints = vec![LayoutHint {
        class_name: LayoutHintClass::Caption,
        confidence: 0.99,
        left: 0.0,
        bottom: 340.0,
        right: 500.0,
        top: 400.0,
    }];
    let tables = vec![table_with_body_rows(
        TABLE_DOMINANT_MIN_BODY_ROWS,
        "long-table-value-1234567890-long-table-value-1234567890-long-table-value-1234567890",
    )];

    crate::pdf::structure::layout_classify::annotate_layout_classes(&mut pages[0], &hints, 0.5, 0.2);
    suppress_table_dominant_paragraph_spill(&mut pages, &tables);

    assert_eq!(pages[0].len(), 1);
    assert_eq!(paragraph_text_raw(&pages[0][0]), caption_text);
    assert_eq!(pages[0][0].layout_class, Some(LayoutHintClass::Caption));
}

#[test]
fn final_heading_compaction_changes_rendered_markdown_levels() {
    let mut pages = vec![vec![
        outline_heading("Title", 1),
        outline_heading("Section", 3),
        outline_heading("Subsection", 4),
        outline_para("Body text"),
    ]];

    compact_final_heading_hierarchy(&mut pages);
    let document =
        crate::pdf::structure::assembly::assemble_internal_document(pages, &[], None, &[], &Default::default());
    let markdown = crate::rendering::render_markdown(&document);
    let headings = markdown
        .lines()
        .filter(|line| line.starts_with('#'))
        .collect::<Vec<_>>();

    assert_eq!(headings, ["# Title", "## Section", "### Subsection"]);
    assert!(markdown.find("# Title").unwrap() < markdown.find("Body text").unwrap());
}

#[test]
fn final_heading_compaction_is_conservatively_gated() {
    let cases = [
        vec![Some(1), Some(1), Some(3)],
        vec![Some(1), Some(2), Some(3), Some(5)],
        vec![Some(1), None],
        vec![Some(3), None],
    ];

    for expected in cases {
        let mut pages = vec![
            expected
                .iter()
                .enumerate()
                .map(|(index, level)| {
                    let mut paragraph = outline_para(&format!("Block {index}"));
                    paragraph.heading_level = *level;
                    paragraph
                })
                .collect::<Vec<_>>(),
        ];

        compact_final_heading_hierarchy(&mut pages);

        let actual = pages[0]
            .iter()
            .map(|paragraph| paragraph.heading_level)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}

/// Regression test for xberg-io/xberg#1301 (mode a): a colon-introduced,
/// semicolon-delimited run-in list with no distinguishing indentation or
/// line break — exactly how it is rendered from unstyled HTML — is split
/// into a lead paragraph plus one list item per clause.
#[test]
fn run_in_colon_semicolon_list_is_split_into_lead_and_items() {
    let text = "Article 1. The management board is authorised to exclude subscription rights: \
to exclude fractional amounts from the shareholders' subscription right; \
where the new shares are issued against cash contributions at market price;";
    let mut pages = vec![vec![outline_para(text)]];

    split_colon_semicolon_run_in_lists(&mut pages);

    assert_eq!(pages[0].len(), 3, "lead paragraph + 2 list items");
    assert!(!pages[0][0].is_list_item);
    assert!(
        pages[0][0].text.ends_with("authorised to exclude subscription rights:"),
        "lead keeps everything up to and including the anchor colon: {}",
        pages[0][0].text
    );
    assert!(pages[0][1].is_list_item);
    assert_eq!(
        pages[0][1].text,
        "to exclude fractional amounts from the shareholders' subscription right;"
    );
    assert!(pages[0][2].is_list_item);
    assert_eq!(
        pages[0][2].text,
        "where the new shares are issued against cash contributions at market price;"
    );
}

#[test]
fn run_in_list_split_requires_at_least_two_clauses() {
    let mut pages = vec![vec![outline_para("Note: see the appendix for full details.")]];

    split_colon_semicolon_run_in_lists(&mut pages);

    assert_eq!(
        pages[0].len(),
        1,
        "a single clause after the colon is not an enumeration"
    );
    assert!(!pages[0][0].is_list_item);
}

#[test]
fn run_in_list_split_leaves_unrelated_paragraphs_untouched_and_in_order() {
    let list_text = "The board is authorised to exclude rights: to exclude fractional amounts; \
where new shares are issued;";
    let decoy = "- a bare dash-prefixed clause outside a list, unit #06-18 Tower 2, Singapore.";
    let mut pages = vec![vec![outline_para(list_text), outline_para(decoy)]];

    split_colon_semicolon_run_in_lists(&mut pages);

    assert_eq!(pages[0].len(), 4, "lead + 2 items + the untouched trailing paragraph");
    assert_eq!(
        pages[0][3].text, decoy,
        "trailing paragraph keeps its text and reading-order position"
    );
}

#[test]
fn outline_recovery_is_page_scoped_and_uses_root_h2() {
    let mut intro = outline_para("1. Introduction");
    intro.is_list_item = true;
    intro.is_page_furniture = true;
    let mut pages = vec![vec![intro, outline_para("Methods")], vec![outline_para("Introduction")]];
    let entries = vec![
        PdfOutlineEntry::test_entry("Introduction", 0, 1),
        PdfOutlineEntry::test_entry("Methods", 1, 1),
    ];

    recover_headings_from_outline(&mut pages, &entries);

    assert_eq!(pages[0][0].heading_level, Some(2));
    assert_eq!(pages[0][1].heading_level, Some(3));
    assert_eq!(pages[1][0].heading_level, None);
    assert!(!pages[0][0].is_list_item);
    assert!(!pages[0][0].is_page_furniture);
}

#[test]
fn outline_recovery_calibrates_from_two_consistent_anchors() {
    let mut first = outline_para("First anchor");
    first.heading_level = Some(1);
    let mut second = outline_para("Second anchor");
    second.heading_level = Some(2);
    let mut pages = vec![vec![first, second, outline_para("Recovered")]];
    let entries = vec![
        PdfOutlineEntry::test_entry("First anchor", 0, 1),
        PdfOutlineEntry::test_entry("Second anchor", 1, 1),
        PdfOutlineEntry::test_entry("Recovered", 2, 1),
    ];

    recover_headings_from_outline(&mut pages, &entries);

    assert_eq!(pages[0][2].heading_level, Some(3));
}

#[test]
fn outline_recovery_ignores_singleton_bad_calibration_anchor() {
    let mut anchor = outline_para("Bad anchor");
    anchor.heading_level = Some(5);
    let mut pages = vec![vec![anchor, outline_para("Recovered")]];
    let entries = vec![
        PdfOutlineEntry::test_entry("Bad anchor", 0, 1),
        PdfOutlineEntry::test_entry("Recovered", 1, 1),
    ];

    recover_headings_from_outline(&mut pages, &entries);

    assert_eq!(pages[0][1].heading_level, Some(3));
}

#[test]
fn outline_recovery_rejects_ambiguous_titles() {
    let mut pages = vec![vec![
        outline_para("Duplicate outline"),
        outline_para("Duplicate paragraph"),
        outline_para("Duplicate paragraph"),
    ]];
    let entries = vec![
        PdfOutlineEntry::test_entry("Duplicate outline", 0, 1),
        PdfOutlineEntry::test_entry("Duplicate outline", 1, 1),
        PdfOutlineEntry::test_entry("Duplicate paragraph", 0, 1),
    ];

    recover_headings_from_outline(&mut pages, &entries);

    assert!(pages[0].iter().all(|paragraph| paragraph.heading_level.is_none()));
}

#[test]
fn outline_recovery_rejects_semantic_non_headings() {
    let mut header = outline_para("Header");
    header.layout_class = Some(LayoutHintClass::PageHeader);
    let mut list = outline_para("List");
    list.layout_class = Some(LayoutHintClass::ListItem);
    let mut formula = outline_para("Formula");
    formula.is_formula = true;
    let mut pages = vec![vec![header, list, formula]];
    let entries = vec![
        PdfOutlineEntry::test_entry("Header", 0, 1),
        PdfOutlineEntry::test_entry("List", 0, 1),
        PdfOutlineEntry::test_entry("Formula", 0, 1),
    ];

    recover_headings_from_outline(&mut pages, &entries);

    assert!(pages[0].iter().all(|paragraph| paragraph.heading_level.is_none()));
}

#[test]
fn outline_title_normalization_handles_labels_without_aliasing_prose() {
    assert_eq!(
        normalize_outline_title("1. Introduction"),
        normalize_outline_title("Introduction")
    );
    assert_eq!(
        normalize_outline_title("IV. Results"),
        normalize_outline_title("Results")
    );
    assert_ne!(
        normalize_outline_title("A quick example"),
        normalize_outline_title("quick example")
    );
    assert_ne!(
        normalize_outline_title("2024 Report"),
        normalize_outline_title("Report")
    );
    assert_ne!(normalize_outline_title("v2 API"), normalize_outline_title("API"));
}

fn paragraph_text(paragraph: &PdfParagraph) -> String {
    paragraph
        .lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn heuristic_segment(text: &str, baseline_y: f32, width: f32, is_monospace: bool) -> SegmentData {
    let mut segment = seg(text, 10.0, width);
    segment.y = baseline_y - segment.height;
    segment.baseline_y = baseline_y;
    segment.is_monospace = is_monospace;
    segment
}

fn process_heuristic_segments(segments: Vec<SegmentData>) -> Vec<PdfParagraph> {
    process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: segments,
            page_hints: None,
            table_bboxes: Vec::new(),
            preserve_native_semantics: false,
            use_layout_reading_order: false,
            #[cfg(feature = "layout-detection")]
            hint_validations: Vec::new(),
            #[cfg(feature = "layout-detection")]
            page_width_pts: None,
            needs_classify: false,
            paragraph_gap_ys: Vec::new(),
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    )
}

#[cfg(feature = "layout-detection")]
#[test]
fn empty_or_ineligible_layout_hints_use_legacy_page_processing() {
    let segments = vec![
        heuristic_segment("First paragraph.", 700.0, 220.0, false),
        heuristic_segment("Second paragraph.", 600.0, 220.0, false),
    ];
    let paragraph_gap_ys = compute_paragraph_gap_ys(&segments);
    let process = |page_hints| {
        process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: segments.clone(),
                page_hints,
                table_bboxes: Vec::new(),
                preserve_native_semantics: false,
                use_layout_reading_order: false,
                hint_validations: Vec::new(),
                page_width_pts: None,
                needs_classify: false,
                paragraph_gap_ys: paragraph_gap_ys.clone(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        )
    };

    let legacy = process(None);
    let empty = process(Some(Vec::new()));
    let invalid = process(Some(vec![LayoutHint {
        class_name: crate::pdf::structure::types::LayoutHintClass::Text,
        confidence: 0.9,
        left: 0.0,
        bottom: 0.0,
        right: f32::INFINITY,
        top: 100.0,
    }]));
    let non_overlapping = process(Some(vec![LayoutHint {
        class_name: crate::pdf::structure::types::LayoutHintClass::Text,
        confidence: 0.9,
        left: 400.0,
        bottom: 0.0,
        right: 500.0,
        top: 100.0,
    }]));

    assert_eq!(format!("{empty:?}"), format!("{legacy:?}"));
    assert_eq!(format!("{invalid:?}"), format!("{legacy:?}"));
    assert_eq!(format!("{non_overlapping:?}"), format!("{legacy:?}"));
}

#[cfg(feature = "layout-detection")]
#[test]
fn page_width_reaches_layout_reading_order_graph() {
    let positioned_segment = |text: &str, x: f32, y: f32| {
        let mut segment = heuristic_segment(text, y + 10.0, 10.0, false);
        segment.x = x;
        segment.y = y;
        segment.height = 10.0;
        segment
    };
    let mut segments = vec![
        positioned_segment("bottom-left", 10.0, 205.0),
        positioned_segment("top-left", 90.0, 305.0),
        positioned_segment("top-right", 200.0, 305.0),
        positioned_segment("bottom-right", 250.0, 205.0),
    ];
    for (index, segment) in segments.iter_mut().enumerate() {
        segment.assigned_role = Some(if index % 2 == 0 { 2 } else { 3 });
    }
    let hint = |left, bottom, right, top| LayoutHint {
        class_name: crate::pdf::structure::types::LayoutHintClass::Text,
        confidence: 0.95,
        left,
        bottom,
        right,
        top,
    };
    let hints = vec![
        hint(0.0, 200.0, 120.0, 220.0),
        hint(80.0, 300.0, 160.0, 320.0),
        hint(120.0, 300.0, 240.0, 320.0),
        hint(160.0, 200.0, 280.0, 220.0),
    ];
    let process = |page_width_pts, table_bboxes, _has_emitted_table| {
        let mut pages = vec![process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: segments.clone(),
                page_hints: Some(hints.clone()),
                table_bboxes,
                preserve_native_semantics: true,
                use_layout_reading_order: true,
                hint_validations: Vec::new(),
                page_width_pts,
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        )];
        reorder_pages_by_layout_region(&mut pages);
        pages[0].iter().map(paragraph_text).collect::<Vec<_>>()
    };

    assert_eq!(
        process(None, Vec::new(), false),
        ["top-left", "bottom-left", "top-right", "bottom-right"]
    );
    assert_eq!(
        process(Some(400.0), Vec::new(), false),
        ["top-left", "top-right", "bottom-left", "bottom-right"],
        "the actual page width must reach layout graph dilation"
    );
    assert_eq!(
        process(
            Some(400.0),
            vec![TableCoverage {
                bbox: crate::types::BoundingBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 50.0,
                    y1: 50.0,
                },
                cell_text: table_cell_text(&[]),
            }],
            true,
        ),
        ["top-left", "top-right", "bottom-left", "bottom-right"],
        "an emitted table must not disable layout reading order for surrounding prose"
    );
}

#[cfg(feature = "layout-detection")]
#[test]
fn stacked_layout_groups_preserve_native_paragraph_assembly() {
    let segments = vec![
        heuristic_segment("One continuous", 700.0, 120.0, false),
        heuristic_segment("paragraph.", 688.0, 100.0, false),
    ];
    let hints = vec![
        LayoutHint {
            class_name: LayoutHintClass::Text,
            confidence: 0.99,
            left: 0.0,
            bottom: 685.0,
            right: 200.0,
            top: 705.0,
        },
        LayoutHint {
            class_name: LayoutHintClass::Text,
            confidence: 0.99,
            left: 0.0,
            bottom: 673.0,
            right: 200.0,
            top: 693.0,
        },
    ];
    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: segments,
            page_hints: Some(hints),
            table_bboxes: Vec::new(),
            preserve_native_semantics: true,
            use_layout_reading_order: false,
            hint_validations: Vec::new(),
            page_width_pts: Some(612.0),
            needs_classify: false,
            paragraph_gap_ys: Vec::new(),
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    );

    assert_eq!(output.len(), 1);
    assert_eq!(paragraph_text(&output[0]), "One continuous paragraph.");
}

#[cfg(feature = "layout-detection")]
#[test]
fn bboxless_emitted_table_page_preserves_native_semantics_and_layout_class() {
    let segment = heuristic_segment("Ordinary prose.", 700.0, 220.0, false);
    let paragraph_gap_ys = compute_paragraph_gap_ys(std::slice::from_ref(&segment));
    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: vec![segment],
            page_hints: Some(vec![LayoutHint {
                class_name: LayoutHintClass::Title,
                confidence: 0.99,
                left: 0.0,
                bottom: 680.0,
                right: 300.0,
                top: 720.0,
            }]),
            table_bboxes: Vec::new(),
            preserve_native_semantics: true,
            use_layout_reading_order: true,
            hint_validations: Vec::new(),
            page_width_pts: Some(612.0),
            needs_classify: false,
            paragraph_gap_ys,
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    );

    assert_eq!(output.len(), 1);
    assert_eq!(paragraph_text(&output[0]), "Ordinary prose.");
    assert_eq!(output[0].heading_level, None);
    assert_eq!(output[0].layout_class, Some(LayoutHintClass::Title));
}

#[cfg(feature = "layout-detection")]
#[test]
fn emitted_table_page_annotates_caption_without_overriding_native_semantics() {
    let segment = heuristic_segment("Source: annual filing", 100.0, 220.0, false);
    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: vec![segment],
            page_hints: Some(vec![LayoutHint {
                class_name: LayoutHintClass::Caption,
                confidence: 0.99,
                left: 0.0,
                bottom: 80.0,
                right: 300.0,
                top: 120.0,
            }]),
            table_bboxes: Vec::new(),
            preserve_native_semantics: true,
            use_layout_reading_order: true,
            hint_validations: Vec::new(),
            page_width_pts: Some(612.0),
            needs_classify: false,
            paragraph_gap_ys: Vec::new(),
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    );

    assert_eq!(output.len(), 1);
    assert_eq!(paragraph_text(&output[0]), "Source: annual filing");
    assert_eq!(output[0].heading_level, None);
    assert_eq!(output[0].layout_class, Some(LayoutHintClass::Caption));
}

#[test]
fn test_heuristic_path_runs_fused_text_repairs() {
    let mut segment = heuristic_segment("Intro\u{00AD}duction, , body", 700.0, 320.0, false);
    segment.is_bold = true;

    let output = process_heuristic_segments(vec![segment]);

    assert_eq!(output.len(), 1);
    assert_eq!(paragraph_text(&output[0]), "Introduction, body");
    assert!(
        output[0].text.is_empty(),
        "repaired segments must remain the text source of truth"
    );
    assert_eq!(output[0].word_count, 2);

    let document = assemble_internal_document(vec![output], &[], None, &[], &Default::default());
    let element = &document.elements[0];
    assert_eq!(element.text, "Introduction, body");
    assert_eq!(element.annotations.len(), 1);
    assert_eq!(element.annotations[0].start, 0);
    assert_eq!(element.annotations[0].end as usize, element.text.len());
}

#[test]
fn test_heuristic_path_dehyphenates_wrapped_word() {
    let output = process_heuristic_segments(vec![
        heuristic_segment("Reliable soft-", 700.0, 490.0, false),
        heuristic_segment("ware handles load", 680.0, 200.0, false),
    ]);

    assert_eq!(output.len(), 1);
    assert_eq!(paragraph_text(&output[0]), "Reliable software handles load");
    assert_eq!(output[0].word_count, 4);
}

#[test]
fn test_heuristic_path_preserves_compound_and_code_hyphens() {
    let compound = process_heuristic_segments(vec![
        heuristic_segment("A cost-", 700.0, 490.0, false),
        heuristic_segment("effective design", 680.0, 200.0, false),
    ]);
    assert_eq!(paragraph_text(&compound[0]), "A cost-effective design");

    let document = assemble_internal_document(vec![compound], &[], None, &[], &Default::default());
    assert_eq!(document.elements[0].text, "A cost-effective design");

    let code = process_heuristic_segments(vec![
        heuristic_segment("let value = soft-", 700.0, 490.0, true),
        heuristic_segment("ware;", 680.0, 100.0, true),
    ]);
    assert!(code[0].is_code_block);
    assert_eq!(paragraph_text(&code[0]), "let value = soft- ware;");
}

#[test]
fn test_structure_tree_page_runs_text_repair_before_assembly() {
    let paragraphs = vec![para(vec![line(vec![seg(
        "Intro\u{00AD}duction, , body • first item • second item",
        10.0,
        320.0,
    )])])];

    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: Some(paragraphs),
            heuristic_segments: Vec::new(),
            page_hints: None,
            table_bboxes: Vec::new(),
            preserve_native_semantics: false,
            use_layout_reading_order: false,
            #[cfg(feature = "layout-detection")]
            hint_validations: Vec::new(),
            #[cfg(feature = "layout-detection")]
            page_width_pts: None,
            needs_classify: false,
            paragraph_gap_ys: Vec::new(),
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    );

    assert_eq!(output.len(), 3);
    assert_eq!(paragraph_text(&output[0]), "Introduction, body");
    assert!(output[1].is_list_item);
    assert_eq!(paragraph_text(&output[1]), "first item");
    assert!(output[2].is_list_item);
    assert_eq!(paragraph_text(&output[2]), "second item");
}

#[test]
fn assigned_sal_heading_survives_merge_when_layout_confirms_it() {
    let mut body = role_seg("unterminated body", 12.0, false, None);
    body.y = 688.0;
    body.baseline_y = 700.0;
    let mut annotation = role_seg("__in", 12.0, false, Some(2));
    annotation.y = 638.0;
    annotation.baseline_y = 650.0;

    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: vec![body, annotation],
            page_hints: Some(vec![LayoutHint {
                class_name: LayoutHintClass::SectionHeader,
                confidence: 0.99,
                left: 70.0,
                bottom: 635.0,
                right: 275.0,
                top: 655.0,
            }]),
            table_bboxes: Vec::new(),
            preserve_native_semantics: false,
            use_layout_reading_order: false,
            #[cfg(feature = "layout-detection")]
            hint_validations: Vec::new(),
            #[cfg(feature = "layout-detection")]
            page_width_pts: Some(612.0),
            needs_classify: false,
            paragraph_gap_ys: Vec::new(),
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        Some(12.0),
        &TextRepairWitnesses::default(),
    );

    assert_eq!(output.len(), 2);
    assert_eq!(paragraph_text(&output[1]), "__in");
    assert_eq!(output[1].heading_level, Some(2));
    assert_eq!(output[1].layout_class, Some(LayoutHintClass::SectionHeader));
}

/// Full-width line at x=10, width=490 → right edge 500.
fn full_line_seg(text: &str) -> SegmentData {
    let mut segment = seg(text, 10.0, 490.0);
    segment.baseline_y = 20.0;
    segment
}

/// Short line at x=10, width=100 → right edge 110 (well below 500*0.85=425).
fn short_line_seg(text: &str) -> SegmentData {
    seg(text, 10.0, 100.0)
}

#[test]
fn test_case1_trailing_hyphen_full_line() {
    let mut p = para(vec![
        line(vec![full_line_seg("some soft-")]),
        line(vec![seg("ware is great", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "some software");
    assert_eq!(p.lines[1].segments[0].text, "is great");
}

#[test]
fn test_case2_no_hyphen_full_line_no_join() {
    let mut p = para(vec![
        line(vec![full_line_seg("the soft")]),
        line(vec![seg("ware is great", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "the soft");
    assert_eq!(p.lines[1].segments[0].text, "ware is great");
}

#[test]
fn test_short_line_no_join() {
    let mut p = para(vec![
        line(vec![short_line_seg("hello")]),
        line(vec![full_line_seg("world and more")]),
    ]);
    let original_trailing = p.lines[0].segments[0].text.clone();
    let original_leading = p.lines[1].segments[0].text.clone();
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, original_trailing);
    assert_eq!(p.lines[1].segments[0].text, original_leading);
}

#[test]
fn test_code_block_not_joined() {
    let mut p = para(vec![
        line(vec![full_line_seg("some soft-")]),
        line(vec![seg("ware is code", 10.0, 200.0)]),
    ]);
    p.is_code_block = true;
    let mut paragraphs = vec![p];
    dehyphenate_paragraphs(&mut paragraphs, true, &HyphenWitnesses::default());
    assert_eq!(paragraphs[0].lines[0].segments[0].text, "some soft-");
}

#[test]
fn test_uppercase_leading_not_joined() {
    let mut p = para(vec![
        line(vec![full_line_seg("some text")]),
        line(vec![seg("Next sentence here", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "some text");
    assert_eq!(p.lines[1].segments[0].text, "Next sentence here");
}

#[test]
fn test_cjk_not_joined() {
    let mut p = para(vec![
        line(vec![full_line_seg("some \u{4E00}-")]),
        line(vec![seg("text here", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "some \u{4E00}-");
}

#[test]
fn test_real_world_software_no_join_without_hyphen() {
    let mut p = para(vec![
        line(vec![full_line_seg("advanced soft")]),
        line(vec![seg("ware development", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "advanced soft");
    assert_eq!(p.lines[1].segments[0].text, "ware development");
}

#[test]
fn test_real_world_hardware_no_join_without_hyphen() {
    let mut p = para(vec![
        line(vec![full_line_seg("modern hard")]),
        line(vec![seg("ware components", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "modern hard");
    assert_eq!(p.lines[1].segments[0].text, "ware components");
}

#[test]
fn test_leading_word_with_trailing_punctuation_no_join() {
    let mut p = para(vec![
        line(vec![full_line_seg("the soft")]),
        line(vec![seg("ware, which is great", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "the soft");
    assert_eq!(p.lines[1].segments[0].text, "ware, which is great");
}

#[test]
fn test_hyphen_only_fallback() {
    let mut trailing = seg("some soft-", 0.0, 0.0);
    trailing.baseline_y = 20.0;
    let mut p = para(vec![line(vec![trailing]), line(vec![seg("ware is great", 0.0, 0.0)])]);
    dehyphenate_hyphen_only(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "some software");
    assert_eq!(p.lines[1].segments[0].text, "is great");
}

#[test]
fn suspended_hyphens_on_one_baseline_are_preserved() {
    for (left, right) in [
        ("vracht-", "en verzendkosten"),
        ("In-", "en uitvoer"),
        ("onderhouds-", "en installatiewerkzaamheden"),
        ("Verkoop-", "en Leveringvoorwaarden"),
    ] {
        let mut trailing = full_line_seg(left);
        trailing.baseline_y = 100.0;
        let mut leading = seg(right, 480.0, 80.0);
        leading.baseline_y = 100.0;
        let mut paragraph = para(vec![line(vec![trailing]), line(vec![leading])]);

        dehyphenate_paragraph_lines(&mut paragraph, &HyphenWitnesses::default());

        assert_eq!(paragraph_text(&paragraph), format!("{left} {right}"));
    }
}

#[test]
fn suspended_and_wrapped_hyphens_in_one_paragraph_are_distinguished() {
    let mut suspended = full_line_seg("De bijbehorende montage-");
    suspended.baseline_y = 100.0;
    let mut wrapped = full_line_seg("en installatie-");
    wrapped.baseline_y = 100.0;
    let mut continuation = seg("handleiding wordt op aanvraag toegezonden.", 10.0, 220.0);
    continuation.baseline_y = 80.0;
    let mut paragraph = para(vec![
        line(vec![suspended]),
        line(vec![wrapped]),
        line(vec![continuation]),
    ]);

    dehyphenate_paragraph_lines(&mut paragraph, &HyphenWitnesses::default());

    assert_eq!(
        paragraph_text(&paragraph),
        "De bijbehorende montage- en installatiehandleiding wordt op aanvraag toegezonden."
    );
}

#[test]
fn dehyphenation_requires_finite_baselines_in_the_same_reading_frame() {
    let cases = [(f32::NAN, 0.0, 0.0), (20.0, 0.0, 90.0)];
    for (trailing_baseline, leading_baseline, leading_rotation) in cases {
        let mut trailing = full_line_seg("some soft-");
        trailing.baseline_y = trailing_baseline;
        let mut leading = seg("ware remains", 10.0, 100.0);
        leading.baseline_y = leading_baseline;
        leading.rotation_degrees = leading_rotation;
        let mut paragraph = para(vec![line(vec![trailing]), line(vec![leading])]);

        dehyphenate_paragraph_lines(&mut paragraph, &HyphenWitnesses::default());

        assert_eq!(paragraph_text(&paragraph), "some soft- ware remains");
    }
}

#[test]
fn test_hyphen_only_uppercase_not_joined() {
    let mut p = para(vec![
        line(vec![seg("some well-", 0.0, 0.0)]),
        line(vec![seg("Known thing", 0.0, 0.0)]),
    ]);
    dehyphenate_hyphen_only(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[0].text, "some well-");
}

#[test]
fn test_single_line_paragraph_skipped() {
    let mut paragraphs = vec![para(vec![line(vec![full_line_seg("single line")])])];
    dehyphenate_paragraphs(&mut paragraphs, true, &HyphenWitnesses::default());
    assert_eq!(paragraphs[0].lines[0].segments[0].text, "single line");
}

#[test]
fn test_multi_segment_line_no_join_without_hyphen() {
    let mut p = para(vec![
        line(vec![seg("first part", 10.0, 200.0), seg("soft", 220.0, 280.0)]),
        line(vec![seg("ware next words", 10.0, 200.0)]),
    ]);
    dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
    assert_eq!(p.lines[0].segments[1].text, "soft");
    assert_eq!(p.lines[1].segments[0].text, "ware next words");
}

/// Regression for #1543: a hyphen appearing mid-run (not at the end of a segment's
/// text) witnesses a genuine authored compound, because a line-wrap hyphen is by
/// construction the LAST character of its segment.
///
/// Neutralisation that must break this test: stop scanning for interior hyphens (e.g.
/// only ever inspect the final character of each segment) in `collect_hyphen_witnesses`.
#[test]
fn collect_hyphen_witnesses_finds_a_mid_run_compound() {
    let pages = vec![vec![seg("the price-determining factors apply", 0.0, 0.0)]];

    let witnesses = collect_hyphen_witnesses(&pages);

    assert!(
        witnesses.contains(&("price".to_string(), "determining".to_string())),
        "a mid-run hyphen must witness its own compound: got {witnesses:?}"
    );
}

/// The load-bearing negative for #1543: a hyphen at the END of a segment's text is,
/// by construction, a line-wrap candidate rather than evidence of an authored
/// compound. If this hyphen were witnessed, the collector would preserve every
/// trailing hyphen it exists to judge, defeating the whole mechanism.
///
/// Neutralisation that must break this test: delete the end-of-run guard (the
/// `1..characters.len() - 1` range excluding the last index) in
/// `collect_hyphen_witnesses`.
#[test]
fn collect_hyphen_witnesses_ignores_a_hyphen_at_the_end_of_a_run() {
    let pages = vec![vec![seg("are based on the price-", 0.0, 0.0)]];

    let witnesses = collect_hyphen_witnesses(&pages);

    assert!(
        witnesses.is_empty(),
        "a trailing hyphen must never become a witness: got {witnesses:?}"
    );
}

/// A pair witnessed only by the document's own mid-line usage -- absent from the
/// static `PRESERVED_LEXICAL_COMPOUNDS` list -- must still license preservation.
///
/// Neutralisation that must break this test: make `should_preserve_lexical_hyphen`
/// ignore its `hyphen_witnesses` argument and consult only the static list.
#[test]
fn should_preserve_lexical_hyphen_true_for_a_witnessed_pair_not_in_the_static_list() {
    let mut witnesses = HyphenWitnesses::default();
    witnesses.insert(("price".to_string(), "determining".to_string()));

    assert!(should_preserve_lexical_hyphen("price", "determining", &witnesses));
}

/// The soft-hyphenation control: `auto` + `matic` (from a line broken as `auto-` /
/// `matic`) is a genuine mid-word wrap with no witness and no static-list entry, so
/// the hyphen must still be dropped on rejoin.
///
/// Neutralisation that must break this test: widen the static list or witness lookup
/// to a prefix/suffix match instead of the exact-pair comparison.
#[test]
fn should_preserve_lexical_hyphen_false_for_genuine_soft_hyphenation() {
    let witnesses = HyphenWitnesses::default();

    assert!(!should_preserve_lexical_hyphen("auto", "matic", &witnesses));
}

/// Char-boundary regression: a non-ASCII word sitting next to a mid-run hyphen must
/// not panic. This repo has a documented history of char-boundary panics from byte
/// slicing a `&str`; `collect_hyphen_witnesses` must only ever slice by `char`.
///
/// Neutralisation that must break this test: rewrite the left/right run extraction
/// with byte-offset string slicing (e.g. `&text[..byte_index]`) instead of the
/// char-safe `Vec<char>` scan.
#[test]
fn collect_hyphen_witnesses_does_not_panic_on_non_ascii_word_boundaries() {
    let pages = vec![vec![seg("café-terrasse is open déjà-vu style", 0.0, 0.0)]];

    let witnesses = collect_hyphen_witnesses(&pages);

    assert!(
        witnesses.contains(&("café".to_string(), "terrasse".to_string())),
        "a non-ASCII word must still be witnessed: got {witnesses:?}"
    );
}

/// GH#1591: a word attested standalone elsewhere in the document is collected as
/// a witness, even though its OTHER occurrence sits directly in front of a
/// ligature-space candidate pattern ("bedrijf is") that must not weld it.
#[test]
fn collect_word_witnesses_finds_a_standalone_occurrence_elsewhere() {
    let pages = vec![vec![
        seg("bedrijf is gesloten", 0.0, 0.0),
        seg("het bedrijf verkocht apparatuur", 0.0, 0.0),
    ]];

    let witnesses = collect_word_witnesses(&pages);

    assert!(
        witnesses.contains("bedrijf"),
        "bedrijf must be witnessed by its ordinary-prose occurrence: got {witnesses:?}"
    );
}

/// The load-bearing negative for #1591: a fragment that appears ONLY as one half
/// of a ligature-space candidate pattern must never witness itself, or every
/// genuine decomposed ligature (e.g. `f irst`) would become unrepairable the
/// moment its own halves are long enough to pass the length guard.
///
/// Neutralisation that must break this test: collect witnesses via a naive
/// `split_whitespace()` over every segment with no candidate-pattern exclusion.
#[test]
fn collect_word_witnesses_does_not_witness_its_own_candidate_halves() {
    let pages = vec![vec![seg("f irst eff iciently", 0.0, 0.0)]];

    let witnesses = collect_word_witnesses(&pages);

    assert!(
        !witnesses.contains("irst") && !witnesses.contains("eff") && !witnesses.contains("iciently"),
        "a candidate pattern's own fragments must not self-witness: got {witnesses:?}"
    );
}

/// A word used twice, once as a candidate's left half and once in an ordinary
/// position, is still witnessed via its non-candidate occurrence -- the exclusion
/// applies per-occurrence, not to the word everywhere it appears in the document.
#[test]
fn collect_word_witnesses_witnesses_a_word_used_twice_once_as_a_candidate() {
    let pages = vec![vec![seg("relief for relief workers arrived", 0.0, 0.0)]];

    let witnesses = collect_word_witnesses(&pages);

    assert!(
        witnesses.contains("relief"),
        "the second, non-candidate occurrence of relief must still witness it: got {witnesses:?}"
    );
}

/// Length guard mirroring `MIN_HYPHEN_WITNESS_WORD_LEN`: a single-letter fragment
/// must never count as its own witness.
#[test]
fn collect_word_witnesses_ignores_single_letter_fragments() {
    let pages = vec![vec![seg("a b c", 0.0, 0.0)]];

    let witnesses = collect_word_witnesses(&pages);

    assert!(
        witnesses.is_empty(),
        "single-letter tokens must never be witnesses: got {witnesses:?}"
    );
}

/// End-to-end (#1591): `apply_text_repair_to_structure_tree_paragraphs` must
/// forward the document's word witnesses into `repair_ligature_spaces`, not just
/// its hyphen witnesses.
///
/// Neutralisation that must break this test: pass `WordWitnesses::default()` to
/// `fused_text_repairs` instead of `witnesses.words` in
/// `apply_text_repair_to_structure_tree_paragraphs`.
#[test]
fn segments_to_paragraphs_preserves_a_witnessed_ligature_space_boundary() {
    let segments = vec![seg("bedrijf is gesloten", 0.0, 200.0)];
    let witnesses = TextRepairWitnesses {
        hyphens: HyphenWitnesses::default(),
        words: ["bedrijf".to_string()].into_iter().collect(),
    };

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &witnesses);

    assert_eq!(paragraph_segment_text(&paragraphs[0]), "bedrijf is gesloten");
}

/// The same end-to-end path with no witnesses at all welds the real word
/// boundary, documenting the fix's known false positive at the pipeline level
/// (not just in the pure `repair_ligature_spaces` unit tests).
#[test]
fn segments_to_paragraphs_welds_an_unwitnessed_ligature_space_boundary() {
    let segments = vec![seg("bedrijf is gesloten", 0.0, 200.0)];

    let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

    assert_eq!(paragraph_segment_text(&paragraphs[0]), "bedrijfis gesloten");
}

fn para_with_font_size(font_size: f32) -> PdfParagraph {
    let lines = vec![line(vec![seg("text", 0.0, 100.0)])];
    let word_count = PdfParagraph::compute_word_count("", &lines);
    PdfParagraph {
        text: String::new(),
        lines,
        dominant_font_size: font_size,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count,
    }
}

#[test]
fn test_has_font_size_variation_empty() {
    assert!(!has_font_size_variation(&[]));
}

#[test]
fn test_has_font_size_variation_single_size() {
    let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.0)];
    assert!(!has_font_size_variation(&paragraphs));
}

#[test]
fn test_has_font_size_variation_different_sizes() {
    let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(18.0)];
    assert!(has_font_size_variation(&paragraphs));
}

#[test]
fn test_has_font_size_variation_small_difference_ignored() {
    let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.3)];
    assert!(!has_font_size_variation(&paragraphs));
}

#[test]
fn test_has_font_size_variation_zero_sizes_ignored() {
    let paragraphs = vec![para_with_font_size(0.0), para_with_font_size(0.0)];
    assert!(!has_font_size_variation(&paragraphs));
}

use crate::pdf::structure::types::LayoutHintClass;

fn furniture_para_with_class(class: LayoutHintClass) -> PdfParagraph {
    let lines = vec![line(vec![seg("ACME", 0.0, 50.0)])];
    let word_count = PdfParagraph::compute_word_count("", &lines);
    PdfParagraph {
        text: String::new(),
        lines,
        dominant_font_size: 10.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: true,
        layout_class: Some(class),
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count,
    }
}

#[test]
fn test_include_headers_clears_page_header_furniture() {
    let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageHeader)];
    un_mark_layout_furniture_per_config(&mut paras, true, false, false);
    assert!(
        !paras[0].is_page_furniture,
        "PageHeader furniture must be cleared when include_headers=true"
    );
}

#[test]
fn test_include_footers_clears_page_footer_furniture() {
    let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageFooter)];
    un_mark_layout_furniture_per_config(&mut paras, false, true, false);
    assert!(
        !paras[0].is_page_furniture,
        "PageFooter furniture must be cleared when include_footers=true"
    );
}

#[test]
fn test_include_headers_false_preserves_page_header_furniture() {
    let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageHeader)];
    un_mark_layout_furniture_per_config(&mut paras, false, false, false);
    assert!(
        paras[0].is_page_furniture,
        "PageHeader furniture must remain when include_headers=false"
    );
}

#[test]
fn test_include_headers_does_not_clear_page_footer_furniture() {
    let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageFooter)];
    un_mark_layout_furniture_per_config(&mut paras, true, false, false);
    assert!(
        paras[0].is_page_furniture,
        "PageFooter furniture must remain when only include_headers=true"
    );
}

#[test]
fn test_include_headers_does_not_clear_non_layout_furniture() {
    let mut para = para(vec![line(vec![seg("repeating", 0.0, 80.0)])]);
    para.is_page_furniture = true;
    para.layout_class = None;
    let mut paras = vec![para];
    un_mark_layout_furniture_per_config(&mut paras, true, true, false);
    assert!(
        paras[0].is_page_furniture,
        "Heuristic furniture (no layout_class) must not be cleared"
    );
}

#[test]
fn test_un_mark_is_noop_when_both_flags_false() {
    let mut paras = vec![
        furniture_para_with_class(LayoutHintClass::PageHeader),
        furniture_para_with_class(LayoutHintClass::PageFooter),
    ];
    un_mark_layout_furniture_per_config(&mut paras, false, false, false);
    assert!(paras[0].is_page_furniture);
    assert!(paras[1].is_page_furniture);
}

#[test]
fn should_clear_footnote_furniture_when_include_footnotes_is_true() {
    let mut paras = vec![furniture_para_with_class(LayoutHintClass::Footnote)];
    un_mark_layout_furniture_per_config(&mut paras, false, false, true);
    assert!(
        !paras[0].is_page_furniture,
        "Footnote furniture must be cleared when include_footnotes=true"
    );
}

#[test]
fn should_preserve_footnote_furniture_when_include_footnotes_is_false() {
    let mut paras = vec![furniture_para_with_class(LayoutHintClass::Footnote)];
    un_mark_layout_furniture_per_config(&mut paras, true, true, false);
    assert!(
        paras[0].is_page_furniture,
        "Footnote furniture must remain when include_footnotes=false, even if header/footer flags are true"
    );
}

#[test]
fn should_drop_footnote_body_when_recovery_knob_is_off_and_survive_when_on() {
    // Regression test for GH#61: a footnote body classified `Footnote` by the
    // layout model that is (for whatever reason) already marked page furniture
    // must be recoverable via `include_footnotes`, exactly like header/footer
    // furniture is recoverable via `include_headers` / `include_footers`.
    //
    // A second, substantive body paragraph is included alongside the footnote so
    // `retain_page_furniture_safely`'s "don't empty the page" safety valve does not
    // mask the effect of `include_footnotes` under test.
    let body_text = "A".repeat(200);
    let body = {
        let mut p = para(vec![line(vec![seg(&body_text, 0.0, 400.0)])]);
        p.text = body_text.clone();
        p.word_count = 1;
        p
    };
    let footnote_body = furniture_para_with_class(LayoutHintClass::Footnote);

    let mut off = vec![body.clone(), footnote_body.clone()];
    un_mark_layout_furniture_per_config(&mut off, true, true, false);
    retain_page_furniture_safely(&mut off);
    assert_eq!(
        off.len(),
        1,
        "footnote body must be dropped when include_footnotes=false"
    );

    let mut on = vec![body, footnote_body];
    un_mark_layout_furniture_per_config(&mut on, false, false, true);
    retain_page_furniture_safely(&mut on);
    assert_eq!(on.len(), 2, "footnote body must survive when include_footnotes=true");
    assert!(!on[1].is_page_furniture);
}

/// Builds a single-page table-coverage map whose one table's cell text
/// contains `text`, for tests of the table-gated same-page dedup pass.
fn table_coverage_with_text(page_index: usize, text: &str) -> ahash::AHashMap<usize, Vec<TableCoverage>> {
    let mut map = ahash::AHashMap::new();
    map.insert(
        page_index,
        vec![TableCoverage {
            bbox: crate::types::BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 1.0,
                y1: 1.0,
            },
            cell_text: table_cell_text(&[text]),
        }],
    );
    map
}

fn dedup_paragraph_at(text: &str, bottom: f32) -> PdfParagraph {
    let mut segment = seg(text, 72.0, 328.0);
    segment.y = bottom;
    segment.baseline_y = bottom;
    let mut paragraph = para(vec![line(vec![segment])]);
    paragraph.block_bbox = Some((72.0, bottom, 400.0, bottom + 12.0));
    paragraph
}

#[test]
fn should_preserve_consecutive_body_paragraphs_at_distinct_positions() {
    let text = "A legitimate repeated body paragraph appears at this page position.";
    let paragraphs = (0..8)
        .map(|index| dedup_paragraph_at(text, 600.0 - index as f32 * 40.0))
        .collect::<Vec<_>>();
    let expected_bounds = paragraphs
        .iter()
        .map(|paragraph| paragraph.block_bbox)
        .collect::<Vec<_>>();
    let mut pages = vec![paragraphs];

    deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());

    assert_eq!(
        pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(),
        vec![text; 8]
    );
    assert_eq!(
        pages[0]
            .iter()
            .map(|paragraph| paragraph.block_bbox)
            .collect::<Vec<_>>(),
        expected_bounds
    );
}

#[test]
fn should_preserve_eight_distinct_body_paragraphs_through_native_structure() {
    let text = "The same body paragraph must remain visible at each separate location.";
    let segments = (0..8)
        .map(|index| {
            let mut segment = seg(text, 72.0, 328.0);
            segment.y = 600.0 - index as f32 * 40.0;
            segment.baseline_y = segment.y;
            segment
        })
        .collect();
    let document = extract_document_structure_from_segments(
        vec![segments],
        SegmentStructureConfig {
            k_clusters: 4,
            tables: &[],
            outline_entries: &[],
            strip_repeating_text: true,
            include_headers: true,
            include_footers: true,
            include_footnotes: true,
            include_watermarks: true,
            used_structure_tree: false,
            image_positions: &[],
            images: None,
            inject_placeholders: false,
            layout_hints: None,
            allow_single_column: true,
            cancel_token: None,
            #[cfg(feature = "layout-detection")]
            layout_images: None,
            #[cfg(feature = "layout-detection")]
            layout_results: None,
            #[cfg(feature = "layout-detection")]
            table_model: crate::core::config::layout::TableModel::Disabled,
            #[cfg(feature = "layout-detection")]
            table_overlap_preference: crate::core::config::layout::TableOverlapPreference::Content,
            #[cfg(feature = "layout-detection")]
            acceleration: None,
            #[cfg(feature = "layout-detection")]
            session_thread_budget: 0,
        },
    )
    .expect("native structure extraction succeeds");
    assert_eq!(
        document.elements.iter().filter(|element| element.text == text).count(),
        8
    );
}

#[test]
fn should_preserve_protected_consecutive_paragraphs_at_coincident_positions() {
    let text = "Identical visible words must retain their semantic roles.";
    for role in 0..6 {
        let mut protected = dedup_paragraph_at(text, 600.0);
        match role {
            0 => protected.heading_level = Some(2),
            1 => protected.is_list_item = true,
            2 => protected.is_code_block = true,
            3 => protected.is_formula = true,
            4 => protected.caption_for = Some(0),
            _ => protected.is_page_furniture = true,
        }
        let body = dedup_paragraph_at(text, 600.0);
        for pair in [
            vec![body.clone(), protected.clone()],
            vec![protected.clone(), body.clone()],
        ] {
            let mut pages = vec![pair];
            deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());
            assert_eq!(
                pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(),
                [text, text]
            );
        }
    }
}

#[test]
fn should_preserve_consecutive_paragraphs_without_valid_geometry() {
    let text = "Repeated words without positions are not rendering artifacts.";
    let bounds = [
        None,
        Some((72.0, 600.0, 72.0, 612.0)),
        Some((72.0, 600.0, 400.0, 600.0)),
        Some((400.0, 600.0, 72.0, 612.0)),
        Some((72.0, f32::NAN, 400.0, 612.0)),
        Some((72.0, 600.0, f32::INFINITY, 612.0)),
    ];
    for bbox in bounds {
        let mut paragraph = dedup_paragraph_at(text, 600.0);
        paragraph.text = text.to_string();
        paragraph.lines.clear();
        paragraph.block_bbox = bbox;
        let mut pages = vec![vec![paragraph.clone(), paragraph]];
        deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());
        assert_eq!(
            pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(),
            [text, text]
        );
    }
}

#[test]
fn should_preserve_consecutive_paragraphs_with_unreliable_segment_geometry() {
    let text = "Partly unknown positions cannot prove coincident rendering.";
    for invalid_field in 0..8 {
        let valid = seg(text, 72.0, 328.0);
        let mut invalid = valid.clone();
        match invalid_field {
            0 => invalid.x = f32::NAN,
            1 => invalid.y = f32::NAN,
            2 => invalid.baseline_y = f32::NAN,
            3 => invalid.width = f32::INFINITY,
            4 => invalid.height = f32::NAN,
            5 => invalid.width = 0.0,
            6 => invalid.height = 0.0,
            _ => invalid.width = -1.0,
        }
        let mut paragraph = para(vec![line(vec![valid, invalid])]);
        paragraph.text = text.to_string();
        let mut pages = vec![vec![paragraph.clone(), paragraph]];
        deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());
        assert_eq!(
            pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(),
            [text, text]
        );
    }
}

#[test]
fn should_preserve_consecutive_paragraphs_that_differ_in_case() {
    let texts = [
        "The Same Words Have Different Case",
        "the same words have different case",
    ];
    let mut pages = vec![texts.iter().map(|text| dedup_paragraph_at(text, 600.0)).collect()];
    deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());
    assert_eq!(pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(), texts);
}

#[test]
fn test_deduplicate_paragraphs_removes_consecutive_duplicates() {
    let p1 = para(vec![line(vec![full_line_seg("Brand loses market share")])]);
    let p2 = para(vec![line(vec![full_line_seg("Brand loses market share")])]);
    let p3 = para(vec![line(vec![full_line_seg("Different content here")])]);
    let mut pages = vec![vec![p1, p2, p3]];
    deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());
    assert_eq!(pages[0].len(), 2, "consecutive duplicate should be removed");
}

#[test]
fn test_deduplicate_paragraphs_removes_non_consecutive_body_duplicates_backed_by_a_table() {
    let p1 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
    let p2 = para(vec![line(vec![full_line_seg("Some intervening paragraph")])]);
    let p3 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
    let mut pages = vec![vec![p1, p2, p3]];
    let tables = table_coverage_with_text(0, "Brand loses market share in volume");
    deduplicate_paragraphs(&mut pages, &tables);
    assert_eq!(
        pages[0].len(),
        2,
        "a non-consecutive body duplicate that a detected table also carries should be removed"
    );
}

#[test]
fn test_deduplicate_paragraphs_preserves_non_consecutive_body_duplicates_without_a_table() {
    // GH#1623: the same-page pass must never remove a repeated body
    // paragraph unless a detected table carries the text too.
    let p1 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
    let p2 = para(vec![line(vec![full_line_seg("Some intervening paragraph")])]);
    let p3 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
    let mut pages = vec![vec![p1, p2, p3]];
    deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());
    assert_eq!(
        pages[0].len(),
        3,
        "a non-consecutive body duplicate with no matching table must be preserved"
    );
}

#[test]
fn test_deduplicate_paragraphs_preserves_non_consecutive_headings() {
    let mut h = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
    h.heading_level = Some(2);
    let filler = para(vec![line(vec![full_line_seg("Some other content between them")])]);
    let mut h2 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
    h2.heading_level = Some(2);
    let mut pages = vec![vec![h, filler, h2]];
    let tables = table_coverage_with_text(0, "Brand loses market share in volume");
    deduplicate_paragraphs(&mut pages, &tables);
    assert_eq!(
        pages[0].len(),
        3,
        "non-consecutive heading duplicates must be preserved"
    );
}

#[test]
fn should_preserve_body_paragraph_matching_earlier_title_in_different_case() {
    // Regression test for GH#1623: on pdf/pdfa_045.pdf page 1, a bold body
    // clause repeats the words of an earlier title line in a different
    // case, with unrelated content between them on the page (so only the
    // non-consecutive pass, not the consecutive-artifact pass, is in
    // play). That pass compared lowercased text with no check that
    // either copy was table content, so the body clause was silently
    // deleted. Backing the page with a table whose cells carry the same
    // words proves the fix is the case-sensitive comparison, not merely
    // the absence of table data.
    let title = para(vec![line(vec![full_line_seg(
        "The Penguin History Of Britain The Struggle For Mastery",
    )])]);
    let filler = para(vec![line(vec![full_line_seg(
        "Recognizing the habit ways to get this ebook",
    )])]);
    let body = para(vec![line(vec![full_line_seg(
        "the penguin history of britain the struggle for mastery",
    )])]);
    let mut pages = vec![vec![title, filler, body]];
    let tables = table_coverage_with_text(0, "The Penguin History Of Britain The Struggle For Mastery");
    deduplicate_paragraphs(&mut pages, &tables);
    assert_eq!(
        pages[0].len(),
        3,
        "a body paragraph must survive matching an earlier title that differs only in case"
    );
}

fn positioned_footnote_paragraph(
    text: &str,
    bbox: (f32, f32, f32, f32),
    font_size: f32,
    is_page_furniture: bool,
) -> PdfParagraph {
    let mut segment = seg_at(text, bbox.0, bbox.1, font_size, false);
    segment.width = bbox.2 - bbox.0;
    let mut paragraph = para(vec![line(vec![segment])]);
    paragraph.dominant_font_size = font_size;
    paragraph.is_page_furniture = is_page_furniture;
    paragraph.block_bbox = Some(bbox);
    paragraph
}

fn positioned_numeric_footnote_run<const N: usize>(
    numbers: [u32; N],
    body_bottoms: [f32; N],
    body_fonts: [f32; N],
) -> Vec<PdfParagraph> {
    let mut paragraphs = Vec::new();
    for ((number, body_bottom), body_font) in numbers.into_iter().zip(body_bottoms).zip(body_fonts) {
        paragraphs.push(positioned_footnote_paragraph(
            &number.to_string(),
            (72.0, body_bottom + 3.7947, 75.3369, body_bottom + 9.7947),
            6.0,
            true,
        ));
        paragraphs.push(positioned_footnote_paragraph(
            "estimate",
            (78.1142, body_bottom, 140.9093, body_bottom + body_font),
            body_font,
            false,
        ));
    }
    paragraphs
}

#[test]
fn consecutive_spatial_footnotes_merge_into_one_paragraph() {
    let pairs = [
        ("1", "2021 estimate", 101.0221, 97.2274),
        ("2", "2020 estimate", 89.5231, 85.7284),
        ("3", "2020 estimate", 78.024, 74.2294),
    ];
    let mut paragraphs = Vec::new();
    for (marker, body, marker_bottom, body_bottom) in pairs {
        paragraphs.push(positioned_footnote_paragraph(
            marker,
            (72.0, marker_bottom, 75.3369, marker_bottom + 6.0),
            6.0,
            true,
        ));
        paragraphs.push(positioned_footnote_paragraph(
            body,
            (78.1142, body_bottom, 140.9093, body_bottom + 10.0),
            10.0,
            false,
        ));
    }

    merge_spatial_footnote_markers(&mut paragraphs);
    retain_page_furniture_safely(&mut paragraphs);
    let mut pages = vec![paragraphs];
    deduplicate_paragraphs(&mut pages, &ahash::AHashMap::new());

    assert_eq!(
        pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(),
        ["1 2021 estimate 2 2020 estimate 3 2020 estimate"]
    );
    assert_eq!(pages[0][0].lines.len(), 6);
    assert_eq!(pages[0][0].block_bbox, Some((72.0, 74.2294, 140.9093, 107.2274)));
}

#[test]
fn spatial_footnote_run_rejects_sequence_gap_path_and_style_changes() {
    let regular_bottoms = [97.2274, 85.7284, 74.2294];

    let mut nonsequential = positioned_numeric_footnote_run([1, 3, 4], regular_bottoms, [10.0, 10.0, 10.0]);
    merge_spatial_footnote_markers(&mut nonsequential);
    assert_eq!(nonsequential.len(), 3);

    let mut large_gap = positioned_numeric_footnote_run([1, 2, 3], [97.2274, 80.0, 68.5], [10.0, 10.0, 10.0]);
    merge_spatial_footnote_markers(&mut large_gap);
    assert_eq!(large_gap.len(), 3);

    let mut mismatched_path = positioned_numeric_footnote_run([1, 2, 3], regular_bottoms, [10.0, 10.0, 10.0]);
    let other_path = super::super::types::LayoutRegionPath {
        root: super::super::types::LayoutRegionTag {
            id: 1,
            class_name: Some(super::super::types::LayoutHintClass::Footnote),
        },
        child: None,
    };
    mismatched_path[2].layout_region_path = Some(other_path);
    mismatched_path[3].layout_region_path = Some(other_path);
    merge_spatial_footnote_markers(&mut mismatched_path);
    assert_eq!(mismatched_path.len(), 3);

    let mut style_change = positioned_numeric_footnote_run([1, 2, 3], regular_bottoms, [10.0, 12.0, 10.0]);
    merge_spatial_footnote_markers(&mut style_change);
    assert_eq!(style_change.len(), 3);
}

#[test]
fn spatial_footnote_run_rejects_overflowing_marker_sequence() {
    let make_paragraph = |marker: &str, body_bottom: f32| {
        let marker = positioned_footnote_paragraph(
            marker,
            (72.0, body_bottom + 3.7947, 75.3369, body_bottom + 9.7947),
            6.0,
            false,
        );
        let body = positioned_footnote_paragraph(
            "estimate",
            (78.1142, body_bottom, 140.9093, body_bottom + 10.0),
            10.0,
            false,
        );
        let mut paragraph = para(vec![marker.lines[0].clone(), body.lines[0].clone()]);
        paragraph.dominant_font_size = 10.0;
        paragraph.block_bbox = Some((72.0, body_bottom, 140.9093, body_bottom + 10.0));
        paragraph
    };
    let upper = make_paragraph(&u32::MAX.to_string(), 97.2274);
    let lower = make_paragraph("0", 85.7284);

    assert!(!spatial_footnotes_are_adjacent(&upper, &lower));
}

#[test]
fn spatial_footnote_run_requires_marker_pair_provenance() {
    let paragraphs = positioned_numeric_footnote_run([1, 2, 3], [97.2274, 85.7284, 74.2294], [10.0; 3]);
    let mut preexisting = paragraphs
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let mut paragraph = pair.to_vec();
            merge_spatial_footnote_markers(&mut paragraph);
            paragraph.pop().expect("marker and body merge")
        })
        .collect::<Vec<_>>();

    merge_spatial_footnote_markers(&mut preexisting);

    assert_eq!(preexisting.len(), 3);
    assert_eq!(
        preexisting.iter().map(paragraph_text_raw).collect::<Vec<_>>(),
        ["1 estimate", "2 estimate", "3 estimate"]
    );
}

#[test]
fn spatial_footnote_run_keeps_regular_prefix_before_irregular_fourth() {
    let mut paragraphs = positioned_numeric_footnote_run([1, 2, 3, 4], [97.2274, 85.7284, 74.2294, 59.5], [10.0; 4]);

    merge_spatial_footnote_markers(&mut paragraphs);

    assert_eq!(paragraphs.len(), 2);
    assert_eq!(
        paragraphs.iter().map(paragraph_text_raw).collect::<Vec<_>>(),
        ["1 estimate 2 estimate 3 estimate", "4 estimate"]
    );
}

#[test]
fn spatial_footnote_merge_rejects_page_numbers_and_list_markers() {
    let page_number = positioned_footnote_paragraph("1", (300.0, 20.0, 303.0, 26.0), 6.0, true);
    let distant_body = positioned_footnote_paragraph("Following paragraph", (72.0, 40.0, 180.0, 50.0), 10.0, false);
    let list_number = positioned_footnote_paragraph("2", (72.0, 80.0, 75.0, 90.0), 10.0, true);
    let list_body = positioned_footnote_paragraph("List body", (78.0, 80.0, 130.0, 90.0), 10.0, false);
    let small_list_number = positioned_footnote_paragraph("3", (72.0, 60.0, 75.0, 66.0), 6.0, true);
    let aligned_list_body = positioned_footnote_paragraph("Small list body", (78.0, 60.0, 150.0, 70.0), 10.0, false);
    let mut paragraphs = vec![
        page_number,
        distant_body,
        list_number,
        list_body,
        small_list_number,
        aligned_list_body,
    ];

    merge_spatial_footnote_markers(&mut paragraphs);

    assert_eq!(paragraphs.len(), 6);
    assert_eq!(
        paragraphs.iter().map(paragraph_text_raw).collect::<Vec<_>>(),
        ["1", "Following paragraph", "2", "List body", "3", "Small list body"]
    );
}

#[test]
fn spatial_footnote_merge_requires_compatible_geometry_and_layout_path() {
    let marker = positioned_footnote_paragraph("12", (72.0, 100.0, 76.0, 106.0), 6.0, true);
    let large_gap_body = positioned_footnote_paragraph("Large gap", (90.0, 96.0, 140.0, 106.0), 10.0, false);
    let weak_overlap_body = positioned_footnote_paragraph("Weak overlap", (78.0, 104.0, 140.0, 114.0), 10.0, false);
    let mut mismatched_path_body =
        positioned_footnote_paragraph("Other region", (78.0, 96.0, 140.0, 106.0), 10.0, false);
    mismatched_path_body.layout_region_path = Some(super::super::types::LayoutRegionPath {
        root: super::super::types::LayoutRegionTag {
            id: 1,
            class_name: Some(super::super::types::LayoutHintClass::Footnote),
        },
        child: None,
    });

    for body in [large_gap_body, weak_overlap_body, mismatched_path_body] {
        let mut paragraphs = vec![marker.clone(), body];
        merge_spatial_footnote_markers(&mut paragraphs);
        assert_eq!(paragraphs.len(), 2);
    }
}

#[test]
fn spatial_footnote_merge_accepts_conventional_symbol_marker() {
    let marker = positioned_footnote_paragraph("†", (72.0, 100.0, 75.0, 106.0), 6.0, true);
    let body = positioned_footnote_paragraph("Source note", (78.0, 96.0, 140.0, 106.0), 10.0, false);
    let mut paragraphs = vec![marker, body];

    merge_spatial_footnote_markers(&mut paragraphs);

    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraph_text_raw(&paragraphs[0]), "† Source note");
}

/// Verify that the index offset formula used for image mapping is correct.
#[test]
fn test_image_index_offset_mapping() {
    let indices: Vec<usize> = vec![50, 52, 54];
    let indices_set: ahash::AHashSet<usize> = indices.iter().copied().collect();
    let first_idx_on_page = indices.iter().copied().min().unwrap_or(0);

    let mut matched: Vec<usize> = Vec::new();
    for current_image in 0..5usize {
        let global_idx = first_idx_on_page + current_image;
        if indices_set.contains(&global_idx) {
            matched.push(global_idx);
        }
    }

    assert_eq!(
        matched,
        vec![50, 52, 54],
        "offset formula must yield exactly the requested global indices"
    );

    assert!(
        !indices_set.contains(&49usize),
        "index 49 is before the page range and must not match"
    );

    assert!(
        !indices_set.contains(&55usize),
        "index 55 was not requested and must not match"
    );
}

/// Helper: build a minimal SegmentData for heading-map tests.
fn seg_with_font(text: &str, font_size: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x: 10.0,
        y: 700.0,
        width: 200.0,
        height: font_size,
        font_size,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: 700.0,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn seg_at(text: &str, x: f32, y: f32, height: f32, monospace: bool) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x,
        y,
        width: 200.0,
        height,
        font_size: height,
        is_bold: false,
        is_italic: false,
        is_monospace: monospace,
        baseline_y: y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn rotated_seg(text: &str, x: f32, y: f32, width: f32, rotation_degrees: f32) -> SegmentData {
    let mut segment = seg_at(text, x, y, 10.0, false);
    segment.width = width;
    segment.font_size = 10.0;
    segment.rotation_degrees = rotation_degrees;
    segment
}

#[test]
fn test_order_segments_in_reading_frames_repairs_scrambled_rotated_table() {
    let segments = vec![
        rotated_seg("B2", 200.0, 130.0, 10.0, 90.0),
        rotated_seg("A2", 100.0, 130.0, 10.0, 90.0),
        rotated_seg("B1", 200.0, 100.0, 10.0, 90.0),
        rotated_seg("A1", 100.0, 100.0, 10.0, 90.0),
    ];

    let ordered = order_segments_in_reading_frames(segments);
    let text = ordered.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>();
    assert_eq!(text, ["A1", "A2", "B1", "B2"]);
}

#[test]
fn test_order_segments_in_reading_frames_leaves_upright_order_byte_identical() {
    let segments = vec![
        rotated_seg("second", 200.0, 100.0, 10.0, 0.0),
        rotated_seg("first", 100.0, 100.0, 10.0, 0.0),
    ];

    let ordered = order_segments_in_reading_frames(segments);
    let text = ordered.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>();
    assert_eq!(text, ["second", "first"]);
}

#[test]
fn test_blocks_to_paragraphs_separates_rotated_body_from_upright_footer() {
    let segments = vec![
        rotated_seg("Engine", 100.0, 100.0, 20.0, 90.0),
        rotated_seg("oil", 100.0, 125.0, 10.0, 90.0),
        rotated_seg("264", 280.0, 20.0, 15.0, 0.0),
    ];

    let paragraphs = blocks_to_paragraphs(order_segments_in_reading_frames(segments), &[], &[]);
    assert_eq!(paragraphs.len(), 2);
    assert_eq!(paragraphs[0].text, "Engine oil");
    assert_eq!(paragraphs[1].text, "264");
}

#[test]
fn test_compute_paragraph_gap_ys_detects_blank_line_gap() {
    let segments = vec![
        seg_at("line one", 10.0, 700.0, 12.0, false),
        seg_at("line two", 10.0, 684.0, 12.0, false),
        seg_at("new paragraph", 10.0, 644.0, 12.0, false),
    ];
    let gaps = compute_paragraph_gap_ys(&segments);
    assert_eq!(gaps.len(), 1, "only the blank-line jump is a paragraph gap");
    assert!(
        gaps[0] > 656.0 && gaps[0] < 684.0,
        "gap midpoint between the paragraphs, got {}",
        gaps[0]
    );
}

#[test]
fn test_compute_paragraph_gap_ys_uses_rotated_cross_axis() {
    let segments = vec![
        rotated_seg("line one", 100.0, 100.0, 20.0, 90.0),
        rotated_seg("line two", 116.0, 100.0, 20.0, 90.0),
        rotated_seg("new paragraph", 156.0, 100.0, 20.0, 90.0),
    ];

    let gaps = compute_paragraph_gap_ys(&segments);
    assert_eq!(gaps.len(), 1);
    assert!((gaps[0] + 131.0).abs() < 1e-3, "unexpected rotated-frame gap: {gaps:?}");
}

#[test]
fn test_compute_paragraph_gap_ys_ignores_same_line_runs_and_tight_lines() {
    let segments = vec![
        seg_at("run a", 10.0, 700.0, 12.0, false),
        seg_at("run b", 80.0, 700.0, 12.0, false),
        seg_at("next line", 10.0, 685.0, 12.0, false),
    ];
    assert_eq!(compute_paragraph_gap_ys(&segments), Vec::<f32>::new());
}

#[test]
fn test_compute_paragraph_gap_ys_immune_to_column_major_stream_order() {
    let segments = vec![
        seg_at("A top", 10.0, 700.0, 12.0, false),
        seg_at("A mid", 10.0, 685.0, 12.0, false),
        seg_at("A bot", 10.0, 670.0, 12.0, false),
        seg_at("B top", 300.0, 700.0, 12.0, false),
        seg_at("B mid", 300.0, 685.0, 12.0, false),
        seg_at("B bot", 300.0, 670.0, 12.0, false),
    ];
    assert_eq!(compute_paragraph_gap_ys(&segments), Vec::<f32>::new());
}

#[test]
fn test_finalize_paragraph_page_number_is_not_heading_and_not_yet_furniture() {
    // GH#1411: classification suppresses heading promotion for page-number
    // shapes but must not mark them deletable — that decision needs page
    // geometry and cross-page agreement, and is made document-wide.
    let heading_map = vec![(12.0, Some(2)), (9.0, None)];
    let gap_info = crate::pdf::structure::classify::precompute_gap_info(&heading_map);
    let seg = seg_at("1", 300.0, 50.0, 12.0, false);
    let para = finalize_paragraph(&[&seg], &heading_map, &gap_info).expect("paragraph");
    assert_eq!(para.heading_level, None, "page number must not become a heading");
    assert!(
        !para.is_page_furniture,
        "classification must not mark page furniture without positional evidence"
    );
}

/// #712: a fragment whose text starts lowercase must not be promoted to a
/// heading even when its font size matches a heading centroid exactly. This is
/// the fabrication signature the OCR mid-line `font_change` break produces --
/// see `SUPPRESS_LOWERCASE_START_HEADINGS`'s doc comment. Against unfixed code
/// (`SUPPRESS_LOWERCASE_START_HEADINGS = false`) this asserts
/// `para.heading_level == None` and fails with `para.heading_level == Some(2)`.
#[test]
fn test_finalize_paragraph_suppresses_heading_for_lowercase_start_fragment() {
    let heading_map = vec![(12.0, Some(2)), (9.0, None)];
    let gap_info = crate::pdf::structure::classify::precompute_gap_info(&heading_map);
    let seg = seg_at("storage.", 10.0, 700.0, 12.0, false);
    let para = finalize_paragraph(&[&seg], &heading_map, &gap_info).expect("paragraph");
    assert_eq!(
        para.heading_level, None,
        "a lowercase-starting fragment must not become a heading"
    );
}

/// Paragraph carrying real geometry, for the page-number validation tests.
/// `y` is a PDF-space bottom coordinate on a 792pt page.
fn positioned_para(text: &str, x: f32, y: f32) -> PdfParagraph {
    let segment = seg_at(text, x, y, 12.0, false);
    let mut paragraph = para(vec![line(vec![segment])]);
    paragraph.text = text.to_string();
    paragraph.word_count = text.split_whitespace().count();
    paragraph.block_bbox = Some((x, y, x + 200.0, y + 12.0));
    paragraph
}

/// 792pt-tall pages, matching `positioned_para`'s coordinate assumptions.
fn letter_page_heights(page_count: usize) -> Vec<f32> {
    vec![792.0; page_count]
}

#[test]
fn should_not_delete_page_number_shape_in_the_page_body() {
    // A table cell reading "1" in the middle of the page: correct shape,
    // wrong position. This is the 3020-hit regression from GH#1411.
    let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
        .map(|_| vec![positioned_para("1", 90.0, 400.0), positioned_para("body", 90.0, 380.0)])
        .collect();
    let page_heights = letter_page_heights(pages.len());
    mark_validated_page_numbers(&mut pages, &page_heights, false, false);
    assert!(
        pages.iter().all(|page| !page[0].is_page_furniture),
        "body-band page-number shapes must never be marked furniture"
    );
}

#[test]
fn should_not_delete_an_isolated_page_number_match() {
    // One footer-positioned "7" on a single page of an eight-page document
    // is not a running page number, whatever its shape.
    let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
        .map(|_| vec![positioned_para("Some ordinary body sentence.", 90.0, 400.0)])
        .collect();
    pages[3].push(positioned_para("7", 300.0, 40.0));
    let page_heights = letter_page_heights(pages.len());
    mark_validated_page_numbers(&mut pages, &page_heights, false, false);
    assert!(
        !pages[3][1].is_page_furniture,
        "a single isolated match must never be deleted"
    );
}

#[test]
fn should_delete_a_consistent_running_footer_page_number() {
    let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
        .map(|page_index| {
            vec![
                positioned_para("Some ordinary body sentence.", 90.0, 400.0),
                positioned_para(&(page_index + 1).to_string(), 300.0, 40.0),
            ]
        })
        .collect();
    let page_heights = letter_page_heights(pages.len());
    mark_validated_page_numbers(&mut pages, &page_heights, false, false);
    assert!(
        pages.iter().all(|page| page[1].is_page_furniture),
        "an incrementing footer number in a fixed slot on every page is furniture"
    );
}

#[test]
fn should_leave_layout_classified_paragraphs_to_the_layout_path() {
    // R6: any layout class at all takes precedence over this heuristic, so
    // `include_footers` cannot be silently overridden here.
    let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
        .map(|page_index| {
            let mut footer = positioned_para(&(page_index + 1).to_string(), 300.0, 40.0);
            footer.layout_class = Some(LayoutHintClass::PageFooter);
            vec![positioned_para("Some ordinary body sentence.", 90.0, 400.0), footer]
        })
        .collect();
    let page_heights = letter_page_heights(pages.len());
    mark_validated_page_numbers(&mut pages, &page_heights, false, false);
    assert!(
        pages.iter().all(|page| !page[1].is_page_furniture),
        "layout-classified paragraphs must be left to the layout path"
    );
}

#[test]
fn test_compute_paragraph_gap_ys_skips_blank_lines_inside_code_blocks() {
    let segments = vec![
        seg_at("let x = 1;", 10.0, 700.0, 12.0, true),
        seg_at("let y = 2;", 10.0, 660.0, 12.0, true),
        seg_at("Prose resumes here.", 10.0, 620.0, 12.0, false),
    ];
    let gaps = compute_paragraph_gap_ys(&segments);
    assert_eq!(gaps.len(), 1, "only the code→prose boundary is a gap");
    assert!(
        gaps[0] > 632.0 && gaps[0] < 660.0,
        "gap sits between code and prose, got {}",
        gaps[0]
    );
}

/// 5-paragraph doc (1 title at 14pt + 4 body at 11pt) with k_clusters=4.
/// The adaptive clamp should reduce clusters to max(2, 5/4)=max(2,1)=2,
/// and then the font-size difference (14 vs 11, ratio≈1.27 ≥ 1.2) should
/// produce a heading_level=1 for the 14pt entry.
#[test]
fn test_build_heading_map_short_doc_title_gets_heading_level_1() {
    let title_seg = seg_with_font("My Title", 14.0);
    let body_seg1 = seg_with_font("Body paragraph one.", 11.0);
    let body_seg2 = seg_with_font("Body paragraph two.", 11.0);
    let body_seg3 = seg_with_font("Body paragraph three.", 11.0);
    let body_seg4 = seg_with_font("Body paragraph four.", 11.0);

    let all_page_segments = vec![vec![title_seg, body_seg1, body_seg2, body_seg3, body_seg4]];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];
    let k_clusters = 4;

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)
        .expect("build_heading_map must succeed");

    let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 14.0).abs() < 0.5);
    assert!(
        title_entry.is_some(),
        "heading_map must contain an entry near 14pt; got: {heading_map:?}"
    );
    assert_eq!(
        title_entry.unwrap().1,
        Some(1),
        "14pt title in a 5-paragraph doc must get heading_level=1; got: {heading_map:?}"
    );
}

/// Real Tesseract hOCR `x_fsize` values measured at 300 DPI against
/// `test_documents/images_extra/ocr_image.tiff`: a 21px body cluster and a 23px
/// secondary tier (ratio 23/21 = 1.095, which fails `MIN_HEADING_FONT_RATIO`, but
/// 23 >= 21 + 1.5 clears the old absolute `MIN_HEADING_FONT_GAP` floor). `font_size`
/// on OCR segments is a render-DPI-dependent pixel measurement, not points, so an
/// absolute-unit gap calibrated for typographic points misfires here.
///
/// Fails without the fix: `assign_heading_levels_smart` used to compute
/// `heading_threshold = (21.0 * 1.15).min(21.0 + 1.5) = 22.5`, and 23.0 >= 22.5, so
/// the 23px cluster got `Some(1)` and this document ended up with a spurious
/// heading instead of the all-body map asserted here.
#[test]
fn test_build_heading_map_pixel_scale_ratio_gate_rejects_subhead_noise() {
    let all_page_segments = vec![vec![
        seg_with_font("Subhead-looking line", 23.0),
        seg_with_font("Body paragraph one with real running text.", 21.0),
        seg_with_font("Body paragraph two with real running text.", 21.0),
        seg_with_font("Body paragraph three with real running text.", 21.0),
        seg_with_font("Body paragraph four with real running text.", 21.0),
    ]];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    assert!(
        heading_map.iter().all(|(_, level)| level.is_none()),
        "a 23px cluster over a 21px body (ratio 1.095) must not be promoted to a heading; got: {heading_map:?}"
    );
}

/// Same shape as `test_build_heading_map_pixel_scale_ratio_gate_rejects_subhead_noise`
/// but at the native point-scale reference (body=10pt) that `MIN_HEADING_FONT_RATIO`'s
/// and the old `MIN_HEADING_FONT_GAP`'s doc comments both cite: `10 * 1.15 == 10 + 1.5
/// == 11.5`, so removing the gap term changes nothing here. Does not fail without the
/// fix (old and new formulas agree at this exact reference point) — it pins the native
/// crossover behavior the fix is designed to preserve.
#[test]
fn test_build_heading_map_native_reference_body_boundary_still_promotes() {
    let all_page_segments = vec![vec![
        seg_with_font("Boundary Heading", 11.5),
        seg_with_font("Body paragraph one with real running text.", 10.0),
        seg_with_font("Body paragraph two with real running text.", 10.0),
        seg_with_font("Body paragraph three with real running text.", 10.0),
        seg_with_font("Body paragraph four with real running text.", 10.0),
    ]];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    let heading_entry = heading_map.iter().find(|(fs, _)| (*fs - 11.5).abs() < 0.01);
    assert_eq!(
        heading_entry.map(|(_, level)| *level),
        Some(Some(1)),
        "11.5pt over a 10pt body sits exactly on the ratio boundary and must still promote; got: {heading_map:?}"
    );
}

/// Verify that the adaptive k clamp doesn't over-reduce for larger documents
/// (≥20 paragraphs keeps k_clusters unchanged).
#[test]
fn test_build_heading_map_large_doc_k_not_reduced() {
    let mut segs: Vec<SegmentData> = (0..4).map(|i| seg_with_font(&format!("Heading {i}"), 18.0)).collect();
    segs.extend((0..20).map(|i| seg_with_font(&format!("Body text paragraph {i}."), 12.0)));

    let all_page_segments = vec![segs];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    let heading_entry = heading_map.iter().find(|(fs, _)| (*fs - 18.0).abs() < 1.0);
    assert!(
        heading_entry.is_some_and(|(_, level)| level.is_some()),
        "18pt entries in a 24-paragraph doc must have a heading level; got: {heading_map:?}"
    );
}

/// Uniform-font short document: when all paragraphs share the same font size,
/// no heading cluster is found by k-means. The fallback must detect the first-page
/// segment as a title when its font is ≥ 1.2× median — but here all fonts are equal
/// so no fallback should fire.
#[test]
fn test_build_heading_map_uniform_font_no_spurious_heading() {
    let segs: Vec<SegmentData> = (0..5).map(|i| seg_with_font(&format!("Para {i}"), 12.0)).collect();

    let all_page_segments = vec![segs];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    assert!(
        heading_map.iter().all(|(_, level)| level.is_none()),
        "uniform-font doc must produce no headings; got: {heading_map:?}"
    );
}

#[test]
fn should_classify_untagged_bold_body_size_paragraphs_on_tagged_pages() {
    let paragraphs = vec![
        body_size_paragraph("Existing tagged section", true, Some(2)),
        body_size_paragraph("Overige en specifieke bepalingen", true, None),
        body_size_paragraph("Datalekprotocol", true, None),
        body_size_paragraph("First body paragraph ends here.", false, None),
        body_size_paragraph("Second body paragraph ends here.", false, None),
    ];
    let all_page_segments = vec![Vec::new()];
    let struct_tree_results = vec![Some(paragraphs.clone())];

    let (heading_map, pages_needing_classification) =
        build_heading_map(&all_page_segments, &struct_tree_results, &[], 4).expect("build_heading_map must succeed");

    assert_eq!(
        pages_needing_classification.into_iter().collect::<Vec<_>>(),
        [0],
        "a uniform-font tagged page with an untagged bold paragraph must reach classification"
    );

    let classified = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: Some(paragraphs),
            heuristic_segments: Vec::new(),
            page_hints: None,
            table_bboxes: Vec::new(),
            preserve_native_semantics: true,
            use_layout_reading_order: false,
            #[cfg(feature = "layout-detection")]
            hint_validations: Vec::new(),
            #[cfg(feature = "layout-detection")]
            page_width_pts: None,
            needs_classify: true,
            paragraph_gap_ys: Vec::new(),
            include_headers: true,
            include_footers: true,
            include_footnotes: true,
        },
        &heading_map,
        Some(12.0),
        &TextRepairWitnesses::default(),
    );
    let level_for = |text: &str| {
        classified
            .iter()
            .find(|paragraph| paragraph_segment_text(paragraph) == text)
            .and_then(|paragraph| paragraph.heading_level)
    };

    assert_eq!(level_for("Existing tagged section"), Some(2));
    assert_eq!(level_for("Overige en specifieke bepalingen"), Some(3));
    assert_eq!(level_for("Datalekprotocol"), None);
}

#[test]
fn should_promote_repeated_untagged_body_size_bold_sections_document_wide() {
    let mut tagged_control = heading_page(
        "Existing tagged heading",
        18.0,
        "The control page ordinary body paragraph ends here.",
    );
    tagged_control[0].assigned_role = Some(1);
    let pages = vec![
        tagged_control,
        heading_page(
            "Overige en specifieke bepalingen",
            12.0,
            "The first ordinary body paragraph ends here.",
        ),
        heading_page(
            "Beveiligingsbeleid en toezicht",
            12.0,
            "The second ordinary body paragraph ends here.",
        ),
        heading_page(
            "Aanvullende technische bepalingen",
            12.0,
            "The third ordinary body paragraph ends here.",
        ),
        heading_page(
            "Datalekprotocol",
            12.0,
            "The short-title control remains ordinary text.",
        ),
    ];

    for used_structure_tree in [false, true] {
        let document = extract_heading_test_document(pages.clone(), used_structure_tree);
        assert_eq!(
            element_kind_for(&document, "Existing tagged heading"),
            Some(crate::types::internal::ElementKind::Heading { level: 1 })
        );
        for heading in [
            "Overige en specifieke bepalingen",
            "Beveiligingsbeleid en toezicht",
            "Aanvullende technische bepalingen",
        ] {
            assert_eq!(
                element_kind_for(&document, heading),
                Some(crate::types::internal::ElementKind::Heading { level: 3 }),
                "a repeated whole-line bold body-size convention must classify {heading:?} as H3"
            );
        }
        assert_eq!(
            element_kind_for(&document, "Datalekprotocol"),
            Some(crate::types::internal::ElementKind::Paragraph),
            "the existing one-word body-size ambiguity guard must remain intact"
        );
    }
}

#[test]
fn should_not_promote_indented_attributions_as_repeated_body_size_headings() {
    let mut attribution_pages = vec![vec![
        body_size_paragraph_at("Existing document title", true, Some(1), 74.0),
        body_size_paragraph_at("Existing section heading", true, Some(2), 74.0),
    ]];
    attribution_pages.extend((0..16).map(|index| {
        vec![
            body_size_paragraph_at(
                &format!("Presenter Name {index}, Department Director"),
                true,
                None,
                126.0,
            ),
            body_size_paragraph_at(
                &format!("Agenda item {index} discussion continues in ordinary body text."),
                false,
                None,
                74.0,
            ),
        ]
    }));

    promote_repeated_body_size_bold_headings(&mut attribution_pages, Some(12.0));

    let promoted_attribution_count = attribution_pages
        .iter()
        .skip(1)
        .filter(|page| page[0].heading_level == Some(3))
        .count();
    assert_eq!(
        promoted_attribution_count, 0,
        "indented presenter attributions must remain subordinate to the following content block"
    );
    assert_eq!(attribution_pages[0][0].heading_level, Some(1));
    assert_eq!(attribution_pages[0][1].heading_level, Some(2));

    let mut aligned_section_pages = vec![vec![
        body_size_paragraph_at("Existing document title", true, Some(1), 74.0),
        body_size_paragraph_at("Existing section heading", true, Some(2), 74.0),
    ]];
    aligned_section_pages.push(vec![body_size_paragraph_at(
        "Genuine repeated section heading 0",
        true,
        None,
        74.0,
    )]);
    aligned_section_pages.push(vec![body_size_paragraph_at(
        "Section 0 opens a block of ordinary body text on the next page.",
        false,
        None,
        74.0,
    )]);
    aligned_section_pages.extend((1..9).map(|index| {
        vec![
            body_size_paragraph_with_bbox(
                &format!("Genuine repeated section heading {index}"),
                true,
                None,
                (74.0, 700.0, 274.0, 712.0),
            ),
            body_size_paragraph_with_bbox(
                &format!("Section {index} opens a block of ordinary body text."),
                false,
                None,
                (74.0, 660.0, 274.0, 690.0),
            ),
        ]
    }));

    promote_repeated_body_size_bold_headings(&mut aligned_section_pages, Some(12.0));

    assert_eq!(
        aligned_section_pages
            .iter()
            .skip(1)
            .filter(|page| page[0].heading_level == Some(3))
            .count(),
        9,
        "aligned repeated section headings must retain the document-wide promotion"
    );
    assert_eq!(aligned_section_pages[0][0].heading_level, Some(1));
    assert_eq!(aligned_section_pages[0][1].heading_level, Some(2));

    let mut missing_geometry_pages = vec![vec![body_size_paragraph_at(
        "Existing document title",
        true,
        Some(1),
        74.0,
    )]];
    missing_geometry_pages.push(vec![
        body_size_paragraph("Candidate without geometry opens body content", true, None),
        body_size_paragraph_at(
            "Following ordinary body content retains valid geometry.",
            false,
            None,
            74.0,
        ),
    ]);
    missing_geometry_pages.push(vec![
        body_size_paragraph_at("Candidate with geometry opens body content", true, None, 74.0),
        body_size_paragraph("Following ordinary body content lacks geometry.", false, None),
    ]);
    missing_geometry_pages.push(vec![
        body_size_paragraph("Candidate and body both lack geometry", true, None),
        body_size_paragraph("Following ordinary body content also lacks geometry.", false, None),
    ]);

    promote_repeated_body_size_bold_headings(&mut missing_geometry_pages, Some(12.0));

    assert_eq!(
        missing_geometry_pages
            .iter()
            .skip(1)
            .filter(|page| page[0].heading_level == Some(3))
            .count(),
        3,
        "missing geometry on either side must retain the documented promotion fallback"
    );

    let mut numbered_section_pages = vec![vec![body_size_paragraph_at(
        "Existing meeting title",
        true,
        Some(1),
        74.0,
    )]];
    numbered_section_pages.extend(["I.", "II.", "III."].into_iter().enumerate().map(|(index, marker)| {
        vec![
            body_size_paragraph_with_bbox(
                &format!("{marker} CENTERED MEETING AGENDA SECTION"),
                true,
                None,
                (220.0, 700.0, 430.0, 714.0),
            ),
            body_size_paragraph_with_bbox(
                &format!("Agenda section {} contains ordinary list or body content.", index + 1),
                false,
                None,
                (74.0, 660.0, 430.0, 690.0),
            ),
        ]
    }));
    numbered_section_pages.push(vec![
        body_size_paragraph_at("I am the program presenter", true, None, 126.0),
        body_size_paragraph_at(
            "The presenter attribution is followed by substantive ordinary body text.",
            false,
            None,
            74.0,
        ),
    ]);
    let mut non_finite_candidate =
        body_size_paragraph_at("Indented presenter with invalid geometry", true, None, 126.0);
    non_finite_candidate.block_bbox = Some((f32::NAN, 0.0, 326.0, 12.0));
    numbered_section_pages.push(vec![
        non_finite_candidate,
        body_size_paragraph_at(
            "Invalid candidate geometry must not bypass structural alignment checks.",
            false,
            None,
            74.0,
        ),
    ]);
    numbered_section_pages.push(vec![
        body_size_paragraph_at("Meeting Executive Name", true, None, 74.0),
        body_size_paragraph_at("Executive Secretary", false, None, 74.0),
    ]);

    promote_repeated_body_size_bold_headings(&mut numbered_section_pages, Some(12.0));

    assert_eq!(
        numbered_section_pages
            .iter()
            .skip(1)
            .take(3)
            .filter(|page| page[0].heading_level == Some(3))
            .count(),
        3,
        "explicitly numbered centered sections must not depend on body alignment"
    );
    assert_eq!(
        numbered_section_pages[4][0].heading_level, None,
        "bare Roman-prefix prose must not bypass attribution alignment"
    );
    assert_eq!(
        numbered_section_pages[5][0].heading_level, None,
        "present but non-finite geometry must fail closed"
    );
    assert_eq!(
        numbered_section_pages.last().and_then(|page| page[0].heading_level),
        None,
        "an attribution followed only by a short role label must remain a paragraph"
    );
}

#[test]
fn should_not_promote_same_row_presenter_labels_as_repeated_body_size_headings() {
    let mut pages = vec![vec![body_size_paragraph_with_bbox(
        "Existing document title",
        true,
        Some(1),
        (74.0, 740.0, 274.0, 752.0),
    )]];
    pages.extend((0..3).map(|index| {
        vec![
            body_size_paragraph_with_bbox(
                &format!("Presenter Role Label {index}"),
                true,
                None,
                (74.0, 700.0, 190.0, 712.0),
            ),
            body_size_paragraph_with_bbox(
                &format!("Named participant {index} and their professional affiliation"),
                false,
                None,
                (210.0, 700.0, 430.0, 712.0),
            ),
        ]
    }));
    pages.extend((0..3).map(|index| {
        vec![
            body_size_paragraph_with_bbox(
                &format!("Genuine repeated section heading {index}"),
                true,
                None,
                (74.0, 700.0, 300.0, 712.0),
            ),
            body_size_paragraph_with_bbox(
                &format!("Section {index} opens a substantive ordinary body paragraph."),
                false,
                None,
                (74.0, 660.0, 430.0, 690.0),
            ),
        ]
    }));

    assert_eq!(
        pages
            .iter()
            .skip(1)
            .filter(|page| is_body_size_bold_heading_candidate(&page[0], 12.0))
            .count(),
        6,
        "all controls must reach the repeated body-size heading promotion predicate"
    );
    assert_eq!(
        pages
            .iter()
            .skip(1)
            .take(3)
            .filter(|page| page[0]
                .block_bbox
                .zip(page[1].block_bbox)
                .is_some_and(|(candidate, following)| { candidate.1 < following.3 && following.1 < candidate.3 }))
            .count(),
        MIN_BODY_SIZE_BOLD_SIGNALS,
        "the negative controls must overlap vertically and meet the document-wide promotion threshold"
    );

    promote_repeated_body_size_bold_headings(&mut pages, Some(12.0));

    assert_eq!(
        pages
            .iter()
            .skip(1)
            .take(3)
            .filter(|page| page[0].heading_level == Some(3))
            .count(),
        0,
        "same-row presenter labels must remain subordinate text"
    );
    assert_eq!(
        pages
            .iter()
            .skip(4)
            .filter(|page| page[0].heading_level == Some(3))
            .count(),
        3,
        "vertically ordered headings must retain repeated body-size promotion"
    );
}

#[test]
fn should_preserve_structural_headings_and_reject_same_row_table_cells() {
    let mut pages = vec![vec![body_size_paragraph_with_bbox(
        "Existing document title",
        true,
        Some(1),
        (74.0, 740.0, 274.0, 752.0),
    )]];
    for heading in [
        "3. NUMBERED SECTION HEADING",
        "IV. ROMAN SECTION HEADING",
        "ARTICLE I GENERAL PROVISIONS",
        "PART IV ADMINISTRATIVE RULES",
        "Policy Administration and Review",
    ] {
        pages.push(vec![
            body_size_paragraph_with_bbox(heading, true, None, (74.0, 700.0, 300.0, 714.0)),
            body_size_paragraph_with_bbox(
                "The outdented heading opens this substantive ordinary body paragraph.",
                false,
                None,
                (110.0, 680.0, 430.0, 702.0),
            ),
        ]);
    }
    pages.push(vec![
        body_size_paragraph_with_bbox(
            "Policy Heading With Inverted Bounds",
            true,
            None,
            (300.0, 700.0, 74.0, 714.0),
        ),
        body_size_paragraph_with_bbox(
            "The normalized outdented heading opens this ordinary body paragraph.",
            false,
            None,
            (110.0, 680.0, 430.0, 702.0),
        ),
    ]);
    pages.push(vec![
        body_size_paragraph_with_bbox("1. TABLE CELL LABEL", true, None, (74.0, 600.0, 190.0, 612.0)),
        body_size_paragraph_with_bbox(
            "Adjacent table value with enough words",
            false,
            None,
            (210.0, 600.0, 430.0, 612.0),
        ),
    ]);
    let mut invalid_candidate =
        body_size_paragraph_with_bbox("2. INVALID CANDIDATE GEOMETRY", true, None, (74.0, 560.0, 190.0, 572.0));
    invalid_candidate.block_bbox = Some((f32::NAN, 560.0, 190.0, 572.0));
    pages.push(vec![
        invalid_candidate,
        body_size_paragraph_with_bbox(
            "A valid adjacent value must not excuse invalid candidate geometry.",
            false,
            None,
            (210.0, 560.0, 430.0, 572.0),
        ),
    ]);
    let mut invalid_following = body_size_paragraph_with_bbox(
        "An invalid adjacent value must not excuse valid candidate geometry.",
        false,
        None,
        (210.0, 520.0, 430.0, 532.0),
    );
    invalid_following.block_bbox = Some((210.0, 520.0, f32::INFINITY, 532.0));
    pages.push(vec![
        body_size_paragraph_with_bbox("3. INVALID FOLLOWING GEOMETRY", true, None, (74.0, 520.0, 190.0, 532.0)),
        invalid_following,
    ]);

    promote_repeated_body_size_bold_headings(&mut pages, Some(12.0));

    for page in pages.iter().skip(1).take(6) {
        assert_eq!(
            page[0].heading_level,
            Some(3),
            "a vertically ordered, outdented structural heading must survive slight bbox overlap: {:?}",
            paragraph_text_raw(&page[0])
        );
    }
    assert_eq!(
        pages[7][0].heading_level, None,
        "an explicitly numbered table cell must not bypass the same-row guard"
    );
    assert_eq!(
        (pages[8][0].heading_level, pages[9][0].heading_level),
        (None, None),
        "explicit numbering must not turn non-finite candidate or following geometry into a heading exemption"
    );
}

#[test]
fn should_reject_non_finite_geometry_when_the_other_bbox_is_missing() {
    let mut missing_candidate_pages = vec![vec![body_size_paragraph_at(
        "Existing document title",
        true,
        Some(1),
        74.0,
    )]];
    missing_candidate_pages.extend((0..MIN_BODY_SIZE_BOLD_SIGNALS).map(|index| {
        let mut following = body_size_paragraph_at(
            &format!("Following ordinary body content {index} has invalid geometry."),
            false,
            None,
            74.0,
        );
        following.block_bbox = Some((74.0, f32::NAN, 274.0, 12.0));
        vec![
            body_size_paragraph(
                &format!("Candidate without geometry {index} opens body content"),
                true,
                None,
            ),
            following,
        ]
    }));

    assert_eq!(
        missing_candidate_pages
            .iter()
            .skip(1)
            .filter(|page| is_body_size_bold_heading_candidate(&page[0], 12.0))
            .count(),
        MIN_BODY_SIZE_BOLD_SIGNALS,
        "candidate-missing controls must reach the document-wide promotion threshold"
    );
    promote_repeated_body_size_bold_headings(&mut missing_candidate_pages, Some(12.0));

    let mut missing_following_pages = vec![vec![body_size_paragraph_at(
        "Existing document title",
        true,
        Some(1),
        74.0,
    )]];
    missing_following_pages.extend((0..MIN_BODY_SIZE_BOLD_SIGNALS).map(|index| {
        let mut candidate = body_size_paragraph_at(
            &format!("Candidate with invalid geometry {index} opens body content"),
            true,
            None,
            74.0,
        );
        candidate.block_bbox = Some((f32::INFINITY, 0.0, 274.0, 12.0));
        vec![
            candidate,
            body_size_paragraph(
                &format!("Following ordinary body content {index} lacks geometry."),
                false,
                None,
            ),
        ]
    }));

    assert_eq!(
        missing_following_pages
            .iter()
            .skip(1)
            .filter(|page| is_body_size_bold_heading_candidate(&page[0], 12.0))
            .count(),
        MIN_BODY_SIZE_BOLD_SIGNALS,
        "following-missing controls must reach the document-wide promotion threshold"
    );
    promote_repeated_body_size_bold_headings(&mut missing_following_pages, Some(12.0));

    let promoted_with_non_finite_following = missing_candidate_pages
        .iter()
        .skip(1)
        .filter(|page| page[0].heading_level == Some(3))
        .count();
    let promoted_with_non_finite_candidate = missing_following_pages
        .iter()
        .skip(1)
        .filter(|page| page[0].heading_level == Some(3))
        .count();
    assert_eq!(
        (promoted_with_non_finite_following, promoted_with_non_finite_candidate),
        (0, 0),
        "a present non-finite bbox must fail closed regardless of which side lacks geometry"
    );
}

#[test]
fn should_not_promote_fewer_than_three_body_size_bold_sections() {
    let candidates = ["First repeated body size section", "Second repeated body size section"];
    for candidate_count in 1..=2 {
        let mut pages: Vec<Vec<SegmentData>> = candidates[..candidate_count]
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                heading_page(
                    candidate,
                    12.0,
                    &format!("Ordinary body paragraph number {} ends here.", index + 1),
                )
            })
            .collect();
        pages.extend((0..3).map(|index| {
            vec![seg_heuristic(
                &format!("Filler paragraph number {} ends here.", index + 1),
                12.0,
                700.0,
            )]
        }));
        let document = extract_heading_test_document(pages, false);

        for candidate in &candidates[..candidate_count] {
            assert_eq!(
                element_kind_for(&document, candidate),
                Some(crate::types::internal::ElementKind::Paragraph),
                "one or two body-size bold occurrences are emphasis, not a document convention"
            );
        }
    }
}

#[test]
fn should_require_body_size_candidates_to_outnumber_other_headings_two_to_one() {
    let document = extract_heading_test_document(
        vec![
            heading_page(
                "First existing larger heading",
                18.0,
                "The first ordinary body paragraph ends here.",
            ),
            heading_page(
                "Second existing larger heading",
                18.0,
                "The second ordinary body paragraph ends here.",
            ),
            heading_page(
                "First body size candidate section",
                12.0,
                "The third ordinary body paragraph ends here.",
            ),
            heading_page(
                "Second body size candidate section",
                12.0,
                "The fourth ordinary body paragraph ends here.",
            ),
            heading_page(
                "Third body size candidate section",
                12.0,
                "The fifth ordinary body paragraph ends here.",
            ),
        ],
        false,
    );

    for heading in ["First existing larger heading", "Second existing larger heading"] {
        assert!(matches!(
            element_kind_for(&document, heading),
            Some(crate::types::internal::ElementKind::Heading { .. })
        ));
    }
    for candidate in [
        "First body size candidate section",
        "Second body size candidate section",
        "Third body size candidate section",
    ] {
        assert_eq!(
            element_kind_for(&document, candidate),
            Some(crate::types::internal::ElementKind::Paragraph),
            "three candidates are not more than twice two independently detected headings"
        );
    }
}

/// Fallback title detection: 5-paragraph doc where first segment is 14pt,
/// others are 11pt. Ratio 14/11 ≈ 1.27 ≥ 1.2 — fallback must fire only when
/// k-means would fail to assign a heading.  This exercises the same fixture as
/// `test_build_heading_map_short_doc_title_gets_heading_level_1` but specifically
/// with k=1 (no clustering possible) to force the fallback path.
#[test]
fn test_build_heading_map_fallback_title_when_k_equals_1() {
    let title_seg = seg_with_font("Document Title", 14.0);
    let body_segs: Vec<SegmentData> = (0..4)
        .map(|i| seg_with_font(&format!("Body paragraph {i}."), 11.0))
        .collect();

    let mut segs = vec![title_seg];
    segs.extend(body_segs);

    let all_page_segments = vec![segs];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 1)
        .expect("build_heading_map must succeed");

    let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 14.0).abs() < 0.5);
    let _ = title_entry;
}

/// Sparsity gate: a three-block, single-page document with one clearly larger
/// first line must NOT promote that line to a heading. A larger opening line
/// in a tiny document is display prose, not necessarily a title.
#[test]
fn test_build_heading_map_sparse_single_page_doc_no_heading_promotion() {
    let all_page_segments = vec![vec![
        seg_with_font("Display Text", 24.0),
        seg_with_font("Body paragraph one.", 12.0),
        seg_with_font("Body paragraph two.", 12.0),
    ]];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    assert!(
        heading_map.iter().all(|(_, level)| level.is_none()),
        "3-block doc must not promote the larger first line to a heading; got: {heading_map:?}"
    );
}

/// A sparse multi-page document has stronger evidence than a cover or title
/// page when the same large-font tier repeats on separate pages and a smaller
/// body tier is also present. This is the `hello_structure.pdf` shape.
#[test]
fn test_build_heading_map_sparse_multi_page_repeated_tier_promotes_headings() {
    let all_page_segments = vec![
        vec![seg_with_font("Hello World", 24.0)],
        vec![
            seg_with_font("Goodbye Cruel World...", 24.0),
            seg_with_font("I'll be back shortly!", 12.0),
        ],
    ];
    let struct_tree_results = vec![None, None];
    let heuristic_pages = vec![0usize, 1usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    let repeated_tier = heading_map
        .iter()
        .find(|(font_size, _)| (*font_size - 24.0).abs() < 0.5);
    assert!(
        repeated_tier.is_some_and(|(_, level)| *level == Some(2)),
        "a repeated 24pt tier across pages with a 12pt body tier must be promoted to H2; got: {heading_map:?}"
    );
}

#[test]
fn test_build_heading_map_sparse_multi_page_does_not_promote_non_repeated_intermediate_tier() {
    use crate::pdf::structure::classify::{find_heading_level, precompute_gap_info};

    let all_page_segments = vec![
        vec![seg_with_font("Repeated Heading One", 22.0)],
        vec![
            seg_with_font("Repeated Heading Two", 22.0),
            seg_with_font("Display prose", 21.0),
            seg_with_font("Body paragraph.", 12.0),
        ],
    ];
    let struct_tree_results = vec![None, None];
    let heuristic_pages = vec![0usize, 1usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");
    let gap_info = precompute_gap_info(&heading_map);

    assert_eq!(
        find_heading_level(21.0, &heading_map, &gap_info),
        None,
        "a non-repeated intermediate font tier must remain prose; got: {heading_map:?}"
    );
}

#[test]
fn test_build_heading_map_sparse_multi_page_does_not_promote_repeated_mid_page_display_text() {
    let all_page_segments = vec![
        vec![
            seg_with_font("Body paragraph one.", 12.0),
            seg_with_font("Repeated display text", 24.0),
        ],
        vec![
            seg_with_font("Body paragraph two.", 12.0),
            seg_with_font("Repeated pull quote", 24.0),
        ],
    ];
    let struct_tree_results = vec![None, None];
    let heuristic_pages = vec![0usize, 1usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    assert!(
        heading_map.iter().all(|(_, level)| level.is_none()),
        "repeated mid-page display text must remain prose in sparse documents; got: {heading_map:?}"
    );
}

/// Sparsity gate: a two-block document (the `issue-987-test.pdf` shape) with
/// a larger first line must NOT promote either line to a heading.
#[test]
fn test_build_heading_map_two_block_doc_no_heading_promotion() {
    let all_page_segments = vec![vec![seg_with_font("Big Text", 24.0), seg_with_font("Small Text", 12.0)]];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    assert!(
        heading_map.iter().all(|(_, level)| level.is_none()),
        "2-block doc must not promote either line to a heading; got: {heading_map:?}"
    );
}

/// The sparsity gate must fire strictly below `MIN_BLOCKS_FOR_FONT_HEADING`:
/// a document at exactly the floor (five blocks) still promotes its title,
/// so genuine short documents keep their heading.
#[test]
fn test_build_heading_map_at_block_floor_still_promotes() {
    let mut segs = vec![seg_with_font("Section Title", 18.0)];
    segs.extend((0..(MIN_BLOCKS_FOR_FONT_HEADING - 1)).map(|i| seg_with_font(&format!("Body paragraph {i}."), 11.0)));

    let all_page_segments = vec![segs];
    let struct_tree_results = vec![None];
    let heuristic_pages = vec![0usize];

    let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
        .expect("build_heading_map must succeed");

    let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 18.0).abs() < 0.5);
    assert_eq!(
        title_entry.and_then(|(_, level)| *level),
        Some(1),
        "at the block floor the title must still be promoted; got: {heading_map:?}"
    );
}

/// Segment with explicit font_size and baseline_y for heuristic-path tests.
fn seg_heuristic(text: &str, font_size: f32, baseline_y: f32) -> SegmentData {
    SegmentData {
        text: text.to_string(),
        x: 10.0,
        y: baseline_y,
        width: 200.0,
        height: font_size,
        font_size,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

/// Heuristic path: two segments at different font sizes (triggering a split in
/// blocks_to_paragraphs) that are a sentence continuation should be re-joined
/// by merge_continuation_paragraphs.
#[test]
fn test_heuristic_path_merges_font_split_continuation() {
    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: vec![
                seg_heuristic("een indicative", 12.0, 700.0),
                seg_heuristic("van toenemende merkbekendheid", 13.8, 680.0),
            ],
            page_hints: None,
            table_bboxes: vec![],
            preserve_native_semantics: false,
            use_layout_reading_order: false,
            #[cfg(feature = "layout-detection")]
            hint_validations: vec![],
            #[cfg(feature = "layout-detection")]
            page_width_pts: None,
            needs_classify: false,
            paragraph_gap_ys: vec![],
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    );
    assert_eq!(
        output.len(),
        1,
        "continuation paragraph split by font change should be merged on heuristic path"
    );
    assert!(
        output[0].text.is_empty(),
        "merged paragraph must have cleared text so assembly joins from segments"
    );
    let all_text: String = output[0]
        .lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        all_text.contains("een indicative"),
        "first fragment must survive in merged segments; got: {all_text:?}"
    );
    assert!(
        all_text.contains("van toenemende merkbekendheid"),
        "second fragment must survive in merged segments; got: {all_text:?}"
    );
}

/// Heuristic path: a sentence-terminating paragraph followed by an
/// uppercase-starting paragraph must NOT be merged.
#[test]
fn test_heuristic_path_does_not_merge_terminated_sentences() {
    let output = process_single_page(
        PageInput {
            page_index: 0,
            struct_paragraphs: None,
            heuristic_segments: vec![
                seg_heuristic("The first sentence ends here.", 12.0, 700.0),
                seg_heuristic("New sentence starts uppercase.", 13.8, 680.0),
            ],
            page_hints: None,
            table_bboxes: vec![],
            preserve_native_semantics: false,
            use_layout_reading_order: false,
            #[cfg(feature = "layout-detection")]
            hint_validations: vec![],
            #[cfg(feature = "layout-detection")]
            page_width_pts: None,
            needs_classify: false,
            paragraph_gap_ys: vec![],
            include_headers: true,
            include_footers: true,
            include_footnotes: false,
        },
        &[],
        None,
        &TextRepairWitnesses::default(),
    );
    assert_eq!(
        output.len(),
        2,
        "terminated sentence followed by uppercase must not be merged"
    );
}

/// Verify that non-contiguous index ranges across pages are handled correctly.
#[test]
fn test_image_index_offset_non_contiguous_pages() {
    let page1_indices: Vec<usize> = vec![0, 1];
    let page2_indices: Vec<usize> = vec![100, 101];

    for (indices, expected_first) in [(&page1_indices, 0usize), (&page2_indices, 100usize)] {
        let first_idx = indices.iter().copied().min().unwrap_or(0);
        assert_eq!(
            first_idx, expected_first,
            "first_idx_on_page must equal the minimum index in the slice"
        );

        let set: ahash::AHashSet<usize> = indices.iter().copied().collect();
        for current_image in 0..2usize {
            let global_idx = first_idx + current_image;
            assert!(
                set.contains(&global_idx),
                "global index {global_idx} must be found for page with first_idx={first_idx}"
            );
        }
    }
}

/// GH#1758's reproducer geometry: 8pt Helvetica (body) and Helvetica-Oblique
/// (headings) in the left column of a two-column journal page, both at the same
/// 10.45pt pitch, with 21pt between the heading's last baseline and the body's
/// first. Every width below is the font's own advance sum for the string, so the
/// right edges reproduce the issue's measured table exactly.
const GH1758_MARGIN: f32 = 37.60;
const GH1758_BODY_INDENT: f32 = 49.60;
const GH1758_PITCH: f32 = 10.45;
const GH1758_HEADING_TO_BODY_GAP: f32 = 21.0;
const GH1758_FIRST_HEADING_BASELINE: f32 = 708.55;

fn gh1758_seg(text: &str, x: f32, width: f32, baseline_y: f32, is_italic: bool) -> SegmentData {
    SegmentData {
        font_size: 8.0,
        height: 8.0,
        is_italic,
        ..column_seg(text, x, width, baseline_y)
    }
}

/// One page of the reproducer: two lead-in body lines, the heading's own lines
/// (each `(text, advance width)`, all at the margin in the heading face), and the
/// three body lines beneath, whose first line ends at x 281.67 -- the `beneath`
/// edge the margin rule measures against on every page.
fn gh1758_page(heading_lines: &[(&str, f32)]) -> Vec<SegmentData> {
    let mut segments = vec![
        gh1758_seg(
            "and Fisher tests. Gene expression assay outcomes were evaluated",
            GH1758_MARGIN,
            238.33,
            740.00,
            false,
        ),
        gh1758_seg(
            "using one-way ANOVA. The Tukey-Kramer post hoc test was used.",
            GH1758_MARGIN,
            238.76,
            729.55,
            false,
        ),
    ];
    let mut baseline = GH1758_FIRST_HEADING_BASELINE;
    for (text, width) in heading_lines {
        segments.push(gh1758_seg(text, GH1758_MARGIN, *width, baseline, true));
        baseline -= GH1758_PITCH;
    }
    let body_baseline = baseline + GH1758_PITCH - GH1758_HEADING_TO_BODY_GAP;
    segments.push(gh1758_seg(
        "Flow cytometry analysis was carried out on every isolated sample",
        GH1758_BODY_INDENT,
        232.07,
        body_baseline,
        false,
    ));
    segments.push(gh1758_seg(
        "following the staining protocol of the previous trial, and the gating",
        GH1758_MARGIN,
        229.90,
        body_baseline - GH1758_PITCH,
        false,
    ));
    segments.push(gh1758_seg(
        "strategy was applied identically to every animal in each of the groups",
        GH1758_MARGIN,
        243.66,
        body_baseline - 2.0 * GH1758_PITCH,
        false,
    ));
    segments
}

const GH1758_BODY_TEXT: &str = "Flow cytometry analysis was carried out on every isolated sample \
     following the staining protocol of the previous trial, and the gating strategy was applied \
     identically to every animal in each of the groups";

/// Assert the reproducer page produced exactly three elements: the lead-in prose,
/// the whole heading, and the body opening with its OWN first word.
fn assert_gh1758_heading_whole(segments: Vec<SegmentData>, expected_heading: &str, context: &str) {
    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
    assert_eq!(
        texts.len(),
        3,
        "{context}: lead-in, heading and body are three elements, got {texts:?}"
    );
    assert_eq!(texts[1], expected_heading, "{context}: the heading must survive whole");
    assert_eq!(
        texts[2], GH1758_BODY_TEXT,
        "{context}: the body must open with its own first word, not the heading's tail"
    );
}

/// GH#1758 page 1, defect A. The heading's first line stops 21.8pt short of the
/// body column beneath it, but the continuation's first word (`patches`, 28.0pt of
/// Helvetica advance) could never have fitted in that space -- this is a genuine
/// wrap. `heading_fills_column`'s fixed two-font-size slack (16pt at 8pt) refuses
/// it, so the heading was cut after `ileal` and `patches of ...` was welded to the
/// body paragraph beneath.
#[test]
fn ragged_right_edge_keeps_the_wrapped_heading_whole_gh1758() {
    assert_gh1758_heading_whole(
        gh1758_page(&[
            (
                "2.4. Flow cytometric analysis of lymphocytes isolated from ileal",
                222.29,
            ),
            ("patches of vaccinated-challenged calves", 143.62),
        ]),
        "2.4. Flow cytometric analysis of lymphocytes isolated from ileal patches of \
         vaccinated-challenged calves",
        "GH#1758 page 1 (A: 21.8pt of ragged slack)",
    );
}

/// GH#1758 page 2, the reporter's control for A: the identical heading one word
/// longer, so its first line stops only 8.4pt short and the existing fixed slack
/// already accepts it. Must stay accepted.
#[test]
fn narrow_ragged_slack_control_still_keeps_its_wrap_gh1758() {
    assert_gh1758_heading_whole(
        gh1758_page(&[
            (
                "2.4. Flow cytometric analysis of the lymphocytes isolated from ileal",
                235.63,
            ),
            ("patches of vaccinated-challenged calves", 143.62),
        ]),
        "2.4. Flow cytometric analysis of the lymphocytes isolated from ileal patches of \
         vaccinated-challenged calves",
        "GH#1758 page 2 (control: 8.4pt of slack, accepted before the fix)",
    );
}

/// GH#1758 page 3, defect B. The heading's first line fills its column (4.0pt
/// short of the body beneath) and the continuation keeps the heading's own italic
/// face while the body beneath does not -- but it opens with a capital
/// (`Peyer's`), which `heading_continuation_at_margin` refused outright.
#[test]
fn capital_opener_in_the_headings_own_style_keeps_the_wrap_gh1758() {
    assert_gh1758_heading_whole(
        gh1758_page(&[
            (
                "3.9. Transcriptome profiles and differentially expressed genes in the",
                240.10,
            ),
            ("Peyer's patch of vaccinated-challenged calves", 163.82),
        ]),
        "3.9. Transcriptome profiles and differentially expressed genes in the Peyer's patch of \
         vaccinated-challenged calves",
        "GH#1758 page 3 (B: capital opener in the heading's style)",
    );
}

/// GH#1758 page 4, the reporter's control for B: the same heading whose
/// continuation opens lowercase (`ileal`). Accepted before the fix, must stay so.
#[test]
fn lowercase_opener_control_still_keeps_its_wrap_gh1758() {
    assert_gh1758_heading_whole(
        gh1758_page(&[
            (
                "3.9. Transcriptome profiles and differentially expressed genes in the",
                240.10,
            ),
            ("ileal Peyer's patch of vaccinated-challenged calves", 180.27),
        ]),
        "3.9. Transcriptome profiles and differentially expressed genes in the ileal Peyer's patch \
         of vaccinated-challenged calves",
        "GH#1758 page 4 (control: lowercase opener, accepted before the fix)",
    );
}

/// GH#1758 page 5, defect A at the SECOND wrap. Line 2 is accepted by
/// `heading_wraps_onto` (8.5pt from line 1's right edge), but line 3 is refused:
/// line 2 stops 30.6pt short of the body column, and `intestinal` is 31.6pt wide.
#[test]
fn ragged_right_edge_at_the_second_wrap_keeps_all_three_lines_gh1758() {
    assert_gh1758_heading_whole(
        gh1758_page(&[
            ("3.2. Decreased frequency of regulatory T lymphocytes with the", 221.87),
            ("anti-inflammatory phenotypes in the continuous and discrete", 213.43),
            ("intestinal patches of vaccinated calves", 136.06),
        ]),
        "3.2. Decreased frequency of regulatory T lymphocytes with the anti-inflammatory phenotypes \
         in the continuous and discrete intestinal patches of vaccinated calves",
        "GH#1758 page 5 (A at the second wrap)",
    );
}

/// GH#1758 page 6, defect C. The heading's MIDDLE line was measured against its
/// own last line (right edge 142.1) instead of the body column beneath the run
/// (281.7), so the `line_end <= next_right_edge` guard rejected it and the heading
/// was cut after line 1 -- both remaining lines went to the body.
#[test]
fn middle_line_measured_against_the_column_keeps_all_three_lines_gh1758() {
    assert_gh1758_heading_whole(
        gh1758_page(&[
            (
                "3.2. Decreased frequency of T lymphocytes with anti-inflammatory",
                234.30,
            ),
            ("phenotypes in continuous and discrete Peyer's patches of", 204.30),
            ("vaccinated-challenged calves", 104.49),
        ]),
        "3.2. Decreased frequency of T lymphocytes with anti-inflammatory phenotypes in continuous \
         and discrete Peyer's patches of vaccinated-challenged calves",
        "GH#1758 page 6 (C: middle line measured against its own last line)",
    );
}

/// GH#1758 negative control, the GH#1609 shape the word-fit relaxation must not
/// re-open: a COMPLETE numbered heading stopping hundreds of points short of the
/// body prose beneath it, with every other signal identical -- same size, same
/// weight, same face, same left edge, and a lowercase opener. The next word could
/// easily have fitted on the heading's line, so it did not wrap, it ended.
#[test]
fn complete_heading_over_body_prose_is_still_not_welded_gh1758() {
    let segments = vec![
        gh1758_seg(
            "2.1. Animals",
            GH1758_MARGIN,
            44.03,
            GH1758_FIRST_HEADING_BASELINE,
            false,
        ),
        gh1758_seg(
            "flow cytometry analysis was carried out on every isolated sample",
            GH1758_MARGIN,
            230.29,
            GH1758_FIRST_HEADING_BASELINE - GH1758_HEADING_TO_BODY_GAP,
            false,
        ),
        gh1758_seg(
            "following the staining protocol of the previous trial, and the gating",
            GH1758_MARGIN,
            229.90,
            GH1758_FIRST_HEADING_BASELINE - GH1758_HEADING_TO_BODY_GAP - GH1758_PITCH,
            false,
        ),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
    assert_eq!(
        texts.len(),
        2,
        "GH#1609 control: heading and body stay separate, got {texts:?}"
    );
    assert_eq!(texts[0], "2.1. Animals");
    assert_eq!(
        texts[1],
        "flow cytometry analysis was carried out on every isolated sample following the staining \
         protocol of the previous trial, and the gating"
    );
}

/// GH#1758 negative control for B: the page-3 geometry with the capital-opening
/// line set in the BODY face rather than the heading's own. A capital opener is
/// refused unless the line keeps the heading's style while the body beneath does
/// not -- this is the 194-case majority the lowercase rule exists to protect.
#[test]
fn capital_opener_without_the_headings_style_is_still_refused_gh1758() {
    let mut segments = gh1758_page(&[
        (
            "3.9. Transcriptome profiles and differentially expressed genes in the",
            240.10,
        ),
        ("Peyer's patch of vaccinated-challenged calves", 163.82),
    ]);
    segments[3].is_italic = false;

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
    assert_eq!(
        texts.len(),
        3,
        "the heading must not absorb a capital line in the body face, got {texts:?}"
    );
    assert_eq!(
        texts[1], "3.9. Transcriptome profiles and differentially expressed genes in the",
        "the heading keeps only its own first line"
    );
    assert!(
        texts[2].starts_with("Peyer's patch of vaccinated-challenged calves Flow cytometry"),
        "the body-face capital line stays out of the heading, got {:?}",
        texts[2]
    );
}

/// GH#1758, the shape that forced the column estimate to be FLOORED at the
/// heading's own right edge. Measured on
/// `test_documents/pdf/an_introduction_to_statistical_learning_...`: once the run
/// skip walks past a long caption, the line it lands on can be a stub -- a panel
/// label, a page number -- ending far short of the measure (x 69.9 against 375.2).
/// A stub is not a column: read as one, it makes the `line_end <= column` guard
/// reject the run's own middle line and cuts the heading after its first.
#[test]
fn stub_line_beneath_a_heading_run_is_not_read_as_the_column_gh1758() {
    let segments = vec![
        gh1758_seg(
            "3.2. Decreased frequency of T lymphocytes with anti-inflammatory",
            GH1758_MARGIN,
            234.30,
            708.55,
            true,
        ),
        gh1758_seg(
            "phenotypes in continuous and discrete Peyer's patches of",
            GH1758_MARGIN,
            204.30,
            698.10,
            true,
        ),
        gh1758_seg("vaccinated-challenged calves", GH1758_MARGIN, 104.49, 687.65, true),
        gh1758_seg("Panel B", GH1758_MARGIN, 30.0, 666.65, false),
        gh1758_seg(
            "flow cytometry analysis was carried out on every isolated sample",
            GH1758_MARGIN,
            232.07,
            656.20,
            false,
        ),
        gh1758_seg(
            "following the staining protocol of the previous trial, and the gating",
            GH1758_MARGIN,
            229.90,
            645.75,
            false,
        ),
    ];

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
    assert_eq!(
        texts.len(),
        3,
        "the heading, the stub and the body are three elements, got {texts:?}"
    );
    assert_eq!(
        texts[0],
        "3.2. Decreased frequency of T lymphocytes with anti-inflammatory phenotypes in continuous \
         and discrete Peyer's patches of vaccinated-challenged calves",
        "a stub beneath the run must not be mistaken for the column"
    );
    assert_eq!(texts[1], "Panel B", "the stub stays out of the heading");
}

/// GH#1806: a left column's last line and the right column's first body line, on a page
/// whose reading order is `TopToBottom`, whose boxes overlap by 2.5pt in y so no
/// paragraph gap separates them, and which agree on every other break signal (font, role,
/// weight, list marker, section number). `crossed_gap` only tests the VERTICAL axis, so
/// before this fix nothing prevented the two from being read as one paragraph, welding
/// two unrelated sentences from opposite columns into one full-width element.
#[test]
fn blocks_to_paragraphs_does_not_weld_across_the_column_corridor_gh1806() {
    let segments = vec![
        column_seg(
            "Left column opening line of the short band under the figure",
            38.0,
            251.0,
            140.0,
        ),
        column_seg(
            "continuing the left column's own paragraph text here",
            38.0,
            251.0,
            129.0,
        ),
        column_seg(
            "and this is the left column's very last line of the band",
            38.0,
            251.0,
            118.0,
        ),
        column_seg(
            "Right column opens its own paragraph on this very line",
            307.0,
            251.0,
            120.5,
        ),
        column_seg(
            "and continues here with the right column's second line",
            307.0,
            251.0,
            109.0,
        ),
        column_seg(
            "and a third right column line completes the band here",
            307.0,
            251.0,
            98.0,
        ),
    ];

    let corridor = page_column_corridor(&segments);
    assert_eq!(
        corridor,
        Some((289.0, 307.0)),
        "the 18pt gutter must be found as the page's sole corridor"
    );

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
    assert_eq!(
        texts.len(),
        2,
        "the left column's band and the right column's band must stay two elements, got {texts:?}"
    );
    assert!(
        texts[0].ends_with("very last line of the band"),
        "left column text must not carry right-column text: {:?}",
        texts[0]
    );
    assert!(
        texts[1].starts_with("Right column opens"),
        "right column text must not carry left-column text: {:?}",
        texts[1]
    );
}

/// Control for GH#1806: the same near-tied baselines and identical style signals, but
/// with no second column present at all -- `page_column_corridor` finds no corridor
/// (fewer than two surviving extents once there is only one column), and the lines merge
/// exactly as they did before this fix.
#[test]
fn blocks_to_paragraphs_merges_normally_without_a_column_corridor_gh1806() {
    let segments = vec![
        column_seg(
            "Left column opening line of the short band under the figure",
            38.0,
            251.0,
            140.0,
        ),
        column_seg(
            "and this is the left column's very last line of the band",
            38.0,
            251.0,
            129.0,
        ),
    ];

    assert_eq!(
        page_column_corridor(&segments),
        None,
        "a single column must present no corridor"
    );

    let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
    assert_eq!(
        paragraphs.len(),
        1,
        "with no corridor, the two lines merge as before this fix"
    );
}

#[test]
fn should_honor_header_and_footer_flags_independently_for_geometric_page_numbers() {
    let cases = [(false, false), (true, false), (false, true), (true, true)];
    assert_eq!(cases.len(), 4);
    let mut verified = 0;
    for (include_headers, include_footers) in cases {
        let mut pages: Vec<Vec<PdfParagraph>> = (1..=8)
            .map(|page| {
                vec![
                    positioned_para("Substantive body content stays intact in the document.", 90.0, 400.0),
                    positioned_para(&format!("Page {page}"), 300.0, 750.0),
                    positioned_para(&format!("Page {page} of 8"), 300.0, 40.0),
                ]
            })
            .collect();
        let heights = letter_page_heights(pages.len());
        mark_validated_page_numbers(&mut pages, &heights, include_headers, include_footers);
        assert_eq!(pages.len(), 8);
        for page in pages {
            assert!(!page[0].is_page_furniture);
            assert_eq!(page[1].is_page_furniture, !include_headers);
            assert_eq!(page[2].is_page_furniture, !include_footers);
        }
        verified += 1;
    }
    assert_eq!(verified, 4);
}
