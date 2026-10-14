use super::*;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::OcrElementLevel;

fn line(text: &str, block: &str) -> InternalElement {
    let mut element = InternalElement::text(
        ElementKind::OcrText {
            level: OcrElementLevel::Line,
        },
        text,
        0,
    );
    element.attributes = Some(
        [(
            crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE.to_string(),
            block.to_string(),
        )]
        .into_iter()
        .collect(),
    );
    element
}

fn image_result(content: &str, elements: Vec<InternalElement>, table_count: usize) -> crate::types::ExtractedDocument {
    let mut document = InternalDocument::new("pdf");
    document.elements = elements;
    crate::types::ExtractedDocument {
        content: content.to_string(),
        tables: vec![crate::types::Table::default(); table_count],
        ocr_internal_document: Some(document),
        ..Default::default()
    }
}

#[test]
fn an_image_without_tables_adds_its_content_unchanged() {
    let result = image_result("TOTAL 60,000", vec![line("filtered away", "block_1")], 0);

    assert_eq!(xobject_text_outside_tables(&result), "TOTAL 60,000");
}

#[test]
fn an_image_whose_tables_carry_all_of_its_lines_adds_no_text() {
    let result = image_result("Product Price\nBanana 20", Vec::new(), 1);

    assert_eq!(xobject_text_outside_tables(&result), "");
}

#[test]
fn an_image_with_tables_adds_only_the_lines_its_tables_left() {
    let result = image_result(
        "Receipt\nSubtotal 60,000\nProduct Price\nBanana 20\nThank you",
        vec![
            line("Receipt", "block_1"),
            line("Subtotal 60,000", "block_1"),
            line("Thank you", "block_3"),
        ],
        1,
    );

    assert_eq!(
        xobject_text_outside_tables(&result),
        "Receipt\nSubtotal 60,000\n\nThank you"
    );
}
