//! Tables in legacy `.ppt` decks: a group shape whose cells are rebuilt into a grid from
//! their anchors, rather than read as loose lines in stream order.

use super::super::*;
use super::text_records::{container, record_header, text_chars_atom};

fn atom(rec_ver_instance: u16, rec_type: u16, body: &[u8]) -> Vec<u8> {
    let mut buf = record_header(rec_ver_instance, rec_type, body.len() as u32);
    buf.extend_from_slice(body);
    buf
}

/// The group's own shape: an `FSPGR`, an `FSP`, and an `OPT` setting `property`.
fn group_shape(property: u16) -> Vec<u8> {
    let mut opt = property.to_le_bytes().to_vec();
    opt.extend_from_slice(&0u32.to_le_bytes());
    let mut body = atom(0x0001, 0xF009, &[0; 16]);
    body.extend(atom(0x0002, MSOFBT_FSP, &[0; 8]));
    body.extend(atom(0x0013, 0xF00B, &opt));
    container(MSOFBT_SP_CONTAINER, &body)
}

/// A shape of `shape_type` at `[left, top, right, bottom]`, with `text` in a client text box.
fn shape(shape_type: u16, rect: [i32; 4], text: Option<&str>) -> Vec<u8> {
    let anchor: Vec<u8> = rect.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut body = atom((shape_type << 4) | 0x2, MSOFBT_FSP, &[0; 8]);
    body.extend(atom(0x0000, MSOFBT_CHILD_ANCHOR, &anchor));
    if let Some(text) = text {
        body.extend(container(0xF00D, &text_chars_atom(text)));
    }
    container(MSOFBT_SP_CONTAINER, &body)
}

const TEXT_BOX: u16 = 202;

fn cell(rect: [i32; 4], text: &str) -> Vec<u8> {
    shape(TEXT_BOX, rect, Some(text))
}

/// The content of a group (what `read_table` is given): its own shape, then `shapes`.
fn group(property: u16, shapes: &[Vec<u8>]) -> Vec<u8> {
    let mut body = group_shape(property);
    for s in shapes {
        body.extend_from_slice(s);
    }
    body
}

fn table(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect()
}

#[test]
fn should_rebuild_a_table_stored_bottom_right_first() {
    // Two columns with an empty spacer column between them, and a border line; stored
    // bottom-right first, as PowerPoint can write it.
    let body = group(
        MSO_PROPERTY_TABLE_PROPERTIES,
        &[
            cell([110, 20, 200, 30], "253"),
            cell([100, 20, 110, 30], ""),
            cell([0, 20, 100, 30], "2"),
            cell([110, 10, 200, 20], "27,039"),
            cell([100, 10, 110, 20], ""),
            cell([0, 10, 100, 20], "1"),
            shape(MSOSPT_LINE, [0, 10, 200, 11], None),
            cell([110, 0, 200, 10], "Cases"),
            cell([100, 0, 110, 10], ""),
            cell([0, 0, 100, 10], "Day"),
        ],
    );
    assert_eq!(
        read_table(&body, 0, body.len()),
        Some(table(&[&["Day", "Cases"], &["1", "27,039"], &["2", "253"]]))
    );
}

#[test]
fn should_write_a_merged_cell_once_at_its_top_left_position() {
    let body = group(
        MSO_PROPERTY_TABLE_PROPERTIES,
        &[
            cell([0, 0, 200, 10], "Quarter\rtotals"),
            cell([0, 10, 100, 30], "Lab"),
            cell([100, 10, 200, 20], "a"),
            cell([100, 20, 200, 30], "b"),
        ],
    );
    assert_eq!(
        read_table(&body, 0, body.len()),
        Some(table(&[&["Quarter totals", ""], &["Lab", "a"], &["", "b"]]))
    );
}

#[test]
fn should_leave_a_group_that_is_not_a_table_and_read_an_empty_table_as_empty() {
    let shapes = [cell([0, 0, 100, 10], "x")];
    let not_a_table = group(MSO_PROPERTY_PIB, &shapes);
    assert_eq!(read_table(&not_a_table, 0, not_a_table.len()), None);

    let empty = group(MSO_PROPERTY_TABLE_PROPERTIES, &[cell([0, 0, 100, 10], "")]);
    assert_eq!(read_table(&empty, 0, empty.len()), Some(Vec::new()));
}

#[test]
fn should_keep_table_cells_out_of_the_slide_text() {
    let table_group = container(
        MSOFBT_SPGR_CONTAINER,
        &group(
            MSO_PROPERTY_TABLE_PROPERTIES,
            &[cell([0, 10, 100, 20], "1"), cell([0, 0, 100, 10], "Day")],
        ),
    );
    let mut drawing = container(0xF00D, &text_chars_atom("Distribution of cases"));
    drawing = container(MSOFBT_SP_CONTAINER, &drawing);
    drawing.extend(table_group);
    let slide = container(
        RT_SLIDE,
        &container(0x040C, &container(0xF002, &container(MSOFBT_SPGR_CONTAINER, &drawing))),
    );

    let mut warnings = Vec::new();
    let (slides, _, _) = extract_texts_from_records(&slide, false, None, &mut warnings).expect("records parse");
    assert_eq!(slides.len(), 1);
    assert_eq!(slides[0].text, "Distribution of cases");
    assert_eq!(slides[0].tables, vec![table(&[&["Day"], &["1"]])]);
}
