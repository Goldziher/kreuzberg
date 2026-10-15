use super::render_plain;
use crate::types::document_structure::ContentLayer;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

#[test]
fn should_preserve_heading_separators_depth_and_sorted_public_attributes() {
    let mut attributed = InternalElement::text(ElementKind::Heading { level: 1 }, "C", 0);
    attributed.attributes = Some(ahash::AHashMap::from_iter([
        ("z".into(), "last".into()),
        ("a".into(), "first".into()),
        ("empty".into(), "".into()),
        ("xmlns:prefix".into(), "hidden".into()),
        ("xberg:internal:font-size-pt".into(), "18".into()),
    ]));
    let doc = InternalDocument {
        elements: vec![
            InternalElement::text(ElementKind::Heading { level: 1 }, "A", 0),
            InternalElement::text(ElementKind::Heading { level: 1 }, "B", 0),
            InternalElement::text(ElementKind::Paragraph, "Body", 0),
            attributed,
            InternalElement::text(ElementKind::Heading { level: 2 }, "D", 1),
            InternalElement::text(ElementKind::Heading { level: 3 }, "", 2),
            InternalElement::text(ElementKind::Heading { level: 2 }, "E", 1),
        ],
        ..Default::default()
    };
    assert_eq!(render_plain(&doc), "A\n\nB\nBody\n\nC (a: first, z: last)\n  D\n\n  E");
}

#[test]
fn should_filter_furniture_then_append_footnotes_comments_and_pdf_annotations_in_order() {
    let mut comment = InternalElement::text(ElementKind::CommentDefinition, "Comment", 0);
    comment.layer = ContentLayer::Footnote;
    let mut footnote = InternalElement::text(ElementKind::FootnoteDefinition, "Foot", 0);
    footnote.layer = ContentLayer::Footnote;
    let mut header = InternalElement::text(ElementKind::Paragraph, "Hidden", 0);
    header.layer = ContentLayer::Header;
    let annotation = serde_json::from_value(serde_json::json!({
        "annotation_type": "text", "content": "note", "page_number": 2, "author": "Writer"
    }))
    .expect("valid annotation fixture");
    let doc = InternalDocument {
        elements: vec![
            InternalElement::text(ElementKind::GroupStart, "Container", 0),
            InternalElement::text(ElementKind::Paragraph, "Body", 0),
            comment,
            header,
            footnote,
            InternalElement::text(ElementKind::CommentRef, "Ignored", 0),
            InternalElement::text(ElementKind::GroupEnd, "Container", 0),
        ],
        annotations: Some(vec![annotation]),
        ..Default::default()
    };
    assert_eq!(
        render_plain(&doc),
        "Body\n\n\nFoot\n\n\nComment\n\n\n\nAnnotations:\nText (page 2) by Writer: note"
    );
}

#[test]
fn should_preserve_table_precedence_image_ocr_and_unresolved_image_captions() {
    let doc = InternalDocument {
        elements: vec![
            InternalElement::text(ElementKind::Table { table_index: 0 }, "Explicit", 0),
            InternalElement::text(ElementKind::Table { table_index: 1 }, "", 0),
            InternalElement::text(ElementKind::Table { table_index: 2 }, "", 0),
            InternalElement::text(ElementKind::Table { table_index: 3 }, "Ignored", 0),
            InternalElement::text(ElementKind::Image { image_index: 0 }, "Ignored", 0),
            InternalElement::text(ElementKind::Image { image_index: 1 }, "  alt  ", 0),
            InternalElement::text(ElementKind::Image { image_index: 2 }, " \t ", 0),
        ],
        tables: vec![
            crate::types::Table {
                cells: vec![vec!["unused".into()]],
                ..Default::default()
            },
            crate::types::Table {
                cells: vec![vec!["A".into(), "B".into()]],
                ..Default::default()
            },
            crate::types::Table {
                markdown: "fallback".into(),
                ..Default::default()
            },
        ],
        images: vec![crate::types::ExtractedImage {
            description: Some("desc".into()),
            ocr_result: Some(Box::new(crate::types::ExtractedDocument {
                content: "OCR".into(),
                ..Default::default()
            })),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert_eq!(
        render_plain(&doc),
        "Explicit\nA B\n\nfallback\n[Image: desc]\n\nOCR\n\n[Image: alt]"
    );
}

#[test]
fn should_preserve_empty_block_newlines_and_verbatim_code_endings() {
    let doc = InternalDocument {
        elements: vec![
            InternalElement::text(ElementKind::Code, "a", 0),
            InternalElement::text(ElementKind::RawBlock, "raw\n", 0),
            InternalElement::text(ElementKind::DefinitionTerm, "Term", 0),
            InternalElement::text(ElementKind::DefinitionDescription, "", 0),
            InternalElement::text(ElementKind::Citation, "", 0),
            InternalElement::text(ElementKind::Slide { number: 1 }, "", 0),
            InternalElement::text(ElementKind::Formula, "", 0),
            InternalElement::text(ElementKind::Paragraph, "End", 1),
        ],
        ..Default::default()
    };
    assert_eq!(render_plain(&doc), "a\n\nraw\n\nTerm: \n\n\n\n  End");
}
