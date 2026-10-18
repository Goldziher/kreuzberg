use super::*;

fn merge_control_table(label: &str, page: u32, bounding_box: Option<BoundingBox>) -> NativeTable {
    let cells = (0..2).map(|row| vec![format!("{label}{row}"), String::new()]).collect();
    let grid_cells = (0..2)
        .map(|row| crate::types::GridCell {
            content: format!("{label}{row}"),
            row,
            col: 0,
            row_span: 1,
            col_span: 2,
            is_header: row == 0,
            bbox: None,
            heading_level: None,
            style_name: None,
        })
        .collect();
    NativeTable {
        table: Table {
            cells,
            page_number: page,
            bounding_box,
            ..Default::default()
        },
        grid: Some(crate::types::TableGrid {
            rows: 2,
            cols: 2,
            cells: grid_cells,
        }),
    }
}

fn control_region() -> BoundingBox {
    BoundingBox {
        x0: 0.0,
        y0: 10.0,
        x1: 100.0,
        y1: 100.0,
    }
}

#[test]
fn should_retain_strict_table_and_paired_grid_for_exact_same_region() {
    let bbox = control_region();
    let mut strict = vec![merge_control_table("strict", 1, Some(bbox))];
    let expected_grid = strict[0].grid.clone();
    append_bordered_tables(&mut strict, vec![merge_control_table("bordered", 1, Some(bbox))]);
    assert_eq!(strict.len(), 1);
    assert_eq!(strict[0].table.cells[0][0], "strict0");
    assert_eq!(strict[0].grid, expected_grid);
}

#[test]
fn should_keep_disjoint_partially_overlapping_and_nested_regions_in_order() {
    let bbox = control_region();
    let regions = [
        BoundingBox {
            y0: 200.0,
            y1: 300.0,
            ..bbox
        },
        BoundingBox {
            x0: 50.0,
            x1: 150.0,
            ..bbox
        },
        BoundingBox {
            x0: 20.0,
            x1: 80.0,
            ..bbox
        },
        BoundingBox {
            x1: f64::from_bits(bbox.x1.to_bits() + 1),
            ..bbox
        },
    ];
    let mut strict = vec![merge_control_table("strict", 1, Some(bbox))];
    let bordered = regions
        .into_iter()
        .enumerate()
        .map(|(index, region)| merge_control_table(&format!("extra{index}"), 1, Some(region)))
        .collect();
    append_bordered_tables(&mut strict, bordered);
    assert_eq!(strict.len(), 5);
    assert_eq!(
        strict
            .iter()
            .map(|entry| entry.table.cells[0][0].as_str())
            .collect::<Vec<_>>(),
        vec!["strict0", "extra00", "extra10", "extra20", "extra30"]
    );
    for entry in strict {
        assert_eq!(
            entry.grid.expect("paired grid retained").cells[0].content,
            entry.table.cells[0][0]
        );
    }
}

#[test]
fn should_keep_identical_geometry_on_different_pages() {
    let bbox = control_region();
    let mut strict = vec![merge_control_table("strict", 1, Some(bbox))];
    append_bordered_tables(&mut strict, vec![merge_control_table("other", 2, Some(bbox))]);
    assert_eq!(
        strict.iter().map(|entry| entry.table.page_number).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn should_keep_missing_nonfinite_zero_area_and_inverted_region_candidates() {
    let bbox = control_region();
    let invalid = [
        None,
        Some(BoundingBox { x0: f64::NAN, ..bbox }),
        Some(BoundingBox {
            x1: f64::INFINITY,
            ..bbox
        }),
        Some(BoundingBox { x1: 0.0, ..bbox }),
        Some(BoundingBox { y1: 0.0, ..bbox }),
    ];
    let mut cases = 0;
    for region in invalid {
        let mut strict = vec![merge_control_table("strict", 1, region)];
        append_bordered_tables(&mut strict, vec![merge_control_table("bordered", 1, region)]);
        assert_eq!(strict.len(), 2);
        assert_eq!(strict[0].table.cells[0][0], "strict0");
        assert_eq!(strict[1].table.cells[0][0], "bordered0");
        cases += 1;
    }
    assert_eq!(cases, 5);
}

#[test]
fn should_treat_signed_zero_as_same_physical_coordinate() {
    let bbox = control_region();
    let mut strict = vec![merge_control_table("strict", 1, Some(bbox))];
    append_bordered_tables(
        &mut strict,
        vec![merge_control_table(
            "bordered",
            1,
            Some(BoundingBox { x0: -0.0, ..bbox }),
        )],
    );
    assert_eq!(strict.len(), 1);
}

#[test]
fn should_preserve_bordered_only_order_without_new_internal_deduplication() {
    let bbox = control_region();
    let mut strict = Vec::new();
    append_bordered_tables(
        &mut strict,
        vec![
            merge_control_table("first", 1, Some(bbox)),
            merge_control_table("second", 1, Some(bbox)),
        ],
    );
    assert_eq!(strict.len(), 2);
    assert_eq!(strict[0].table.cells[0][0], "first0");
    assert_eq!(strict[1].table.cells[0][0], "second0");
}
