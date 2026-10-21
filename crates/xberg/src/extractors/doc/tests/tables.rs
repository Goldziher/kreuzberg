use super::*;

fn paragraph(content: &str, table: Option<DocTableMark>) -> DocParagraph {
    DocParagraph {
        content: content.to_string(),
        list: None,
        heading_level: None,
        table,
    }
}

const CELL: Option<DocTableMark> = Some(DocTableMark {
    cell_end: true,
    row_end: false,
});
const ROW: Option<DocTableMark> = Some(DocTableMark {
    cell_end: false,
    row_end: true,
});
const IN_CELL: Option<DocTableMark> = Some(DocTableMark {
    cell_end: false,
    row_end: false,
});

fn element_kinds(doc: &InternalDocument) -> Vec<&'static str> {
    doc.elements
        .iter()
        .map(|element| match element.kind {
            ElementKind::Table { .. } => "table",
            ElementKind::Paragraph => "paragraph",
            ElementKind::Heading { .. } => "heading",
            ElementKind::ListStart { .. } => "list-start",
            ElementKind::ListItem { .. } => "list-item",
            ElementKind::ListEnd => "list-end",
            _ => "other",
        })
        .collect()
}

#[test]
fn should_emit_consecutive_table_paragraphs_as_one_table() {
    let paragraphs = [
        paragraph("A long introduction paragraph.", None),
        paragraph("Region", CELL),
        paragraph("Q1", CELL),
        paragraph("", ROW),
        paragraph("North", CELL),
        paragraph("", CELL),
        paragraph("", ROW),
        paragraph("After the table, more prose follows.", None),
    ];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs);

    assert_eq!(element_kinds(&doc), ["paragraph", "table", "paragraph"]);
    assert_eq!(
        doc.tables[0].cells,
        [
            vec!["Region".to_string(), "Q1".to_string()],
            vec!["North".to_string(), String::new()]
        ]
    );
}

#[test]
fn should_join_the_paragraphs_of_a_cell_with_line_breaks() {
    let paragraphs = [
        paragraph("first", IN_CELL),
        paragraph("second", CELL),
        paragraph("", ROW),
    ];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs);

    assert_eq!(doc.tables[0].cells, [vec!["first\nsecond".to_string()]]);
}

#[test]
fn should_leave_out_a_table_with_no_text() {
    let paragraphs = [paragraph("", CELL), paragraph("", CELL), paragraph("", ROW)];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs);

    assert!(doc.elements.is_empty());
    assert!(doc.tables.is_empty());
}

#[test]
fn should_close_an_open_list_before_a_table() {
    let mut item = paragraph("An item", None);
    item.list = Some(crate::extraction::doc::DocListMembership {
        level: 0,
        ordered: false,
    });
    let paragraphs = [item, paragraph("cell", CELL), paragraph("", ROW)];
    let mut doc = InternalDocument::new("doc");

    push_paragraph_elements(&mut doc, &paragraphs);

    assert_eq!(element_kinds(&doc), ["list-start", "list-item", "list-end", "table"]);
}
