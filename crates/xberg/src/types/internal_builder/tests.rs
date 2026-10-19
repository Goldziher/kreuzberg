use super::*;
use bytes::Bytes;
use std::borrow::Cow;

/// A cell keeps its equation, and the equation still reaches the formula
/// list: a caller asking for the formulas of a document gets all of them.
/// A grid is padded before it reaches the builder, so a spanned or merged
/// cell arrives several times. Its equation is one equation.
#[test]
fn test_a_spanned_or_merged_cell_reports_its_equation_once() {
    let mut b = InternalDocumentBuilder::new("docx");
    let spanned = "$$E=mc^2$$".to_string();
    let cells = vec![
        vec![spanned.clone(), spanned.clone(), spanned.clone()],
        vec![spanned.clone(), spanned.clone(), spanned.clone()],
    ];
    b.push_table_from_cells(&cells, None, None);

    let doc = b.build();
    let latex: Vec<&str> = doc.recorded_formulas.iter().map(|f| f.latex.as_str()).collect();
    assert_eq!(latex, vec!["E=mc^2"], "one cell, one equation");
}

#[test]
fn test_table_cell_math_reaches_the_formula_list_and_stays_in_the_cell() {
    let mut b = InternalDocumentBuilder::new("docx");
    let cells = vec![
        vec!["Area".to_string(), "$$A=\\pi r^{2}$$".to_string()],
        vec!["Roots".to_string(), "$$x=\\frac{-b}{2a}$$".to_string()],
    ];
    b.push_table_from_cells(&cells, None, None);
    let doc = b.build();

    let latex: Vec<&str> = doc.recorded_formulas.iter().map(|f| f.latex.as_str()).collect();
    assert_eq!(latex, vec!["A=\\pi r^{2}", "x=\\frac{-b}{2a}"]);

    let table = doc.tables.first().expect("the table survives");
    assert!(
        table.cells[0][1].contains("A=\\pi r^{2}"),
        "the cell keeps its equation: {:?}",
        table.cells[0][1]
    );
}

/// A price is not mathematics, so a lone `$` must not open a formula.
#[test]
fn test_table_cell_currency_is_not_a_formula() {
    let mut b = InternalDocumentBuilder::new("docx");
    let cells = vec![vec!["Widget".to_string(), "$5 each, $7 boxed".to_string()]];
    b.push_table_from_cells(&cells, None, None);

    assert!(
        b.build().recorded_formulas.is_empty(),
        "currency must not become a formula"
    );
}

#[test]
fn test_empty_builder() {
    let b = InternalDocumentBuilder::new("test");
    let doc = b.build();
    assert!(doc.elements.is_empty());
    assert!(doc.relationships.is_empty());
    assert_eq!(doc.source_format, "test");
}

#[test]
fn test_push_heading() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_heading(1, "Introduction", Some(1), None);
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    let elem = &doc.elements[0];
    assert_eq!(elem.text, "Introduction");
    assert_eq!(elem.kind, ElementKind::Heading { level: 1 });
    assert_eq!(elem.depth, 0);
    assert_eq!(elem.anchor.as_deref(), Some("introduction"));
    assert_eq!(elem.page, Some(1));
}

#[test]
fn test_heading_depth_from_level() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_heading(1, "H1", None, None);
    b.push_heading(2, "H2", None, None);
    b.push_heading(3, "H3", None, None);
    let doc = b.build();

    assert_eq!(doc.elements[0].depth, 0);
    assert_eq!(doc.elements[1].depth, 1);
    assert_eq!(doc.elements[2].depth, 2);
}

#[test]
fn test_push_paragraph() {
    let mut b = InternalDocumentBuilder::new("html");
    b.push_paragraph("Hello world.", vec![], Some(1), None);
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].text, "Hello world.");
    assert_eq!(doc.elements[0].kind, ElementKind::Paragraph);
    assert_eq!(doc.elements[0].depth, 0);
}

#[test]
fn test_paragraph_with_annotations() {
    let mut b = InternalDocumentBuilder::new("html");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 5,
        kind: super::super::document_structure::AnnotationKind::Bold,
    }];
    b.push_paragraph("Hello world.", ann, None, None);
    let doc = b.build();

    assert_eq!(doc.elements[0].annotations.len(), 1);
    assert_eq!(doc.elements[0].annotations[0].start, 0);
    assert_eq!(doc.elements[0].annotations[0].end, 5);
}

#[test]
fn test_list_depth_management() {
    let mut b = InternalDocumentBuilder::new("markdown");
    assert_eq!(b.depth, 0);
    b.push_list(false);
    assert_eq!(b.depth, 1);
    b.push_list_item("Item 1", false, vec![], Some(1), None);
    b.push_list_item("Item 2", false, vec![], Some(1), None);
    b.end_list();
    assert_eq!(b.depth, 0);

    let doc = b.build();
    assert_eq!(doc.elements.len(), 4);
    assert_eq!(doc.elements[0].kind, ElementKind::ListStart { ordered: false });
    assert_eq!(doc.elements[1].kind, ElementKind::ListItem { ordered: false });
    assert_eq!(doc.elements[1].depth, 1);
    assert_eq!(doc.elements[2].depth, 1);
    assert_eq!(doc.elements[3].kind, ElementKind::ListEnd);
    assert_eq!(doc.elements[3].depth, 0);
}

#[test]
fn test_nested_lists() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_list(false);
    b.push_list_item("Outer", false, vec![], None, None);
    b.push_list(true);
    b.push_list_item("Inner", true, vec![], None, None);
    b.end_list();
    b.end_list();

    let doc = b.build();
    assert_eq!(doc.elements.len(), 6);
    assert_eq!(doc.elements[0].depth, 0);
    assert_eq!(doc.elements[1].depth, 1);
    assert_eq!(doc.elements[2].depth, 1);
    assert_eq!(doc.elements[3].depth, 2);
    assert_eq!(doc.elements[4].depth, 1);
    assert_eq!(doc.elements[5].depth, 0);
}

#[test]
fn test_push_table() {
    let mut b = InternalDocumentBuilder::new("html");
    let table = Table {
        cells: vec![vec!["A".to_string(), "B".to_string()]],
        markdown: "| A | B |".to_string(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };
    b.push_table(table, Some(1), None);
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Table { table_index: 0 });
    assert_eq!(doc.tables.len(), 1);
    assert_eq!(doc.tables[0].cells[0][0], "A");
}

#[test]
fn test_push_image() {
    let mut b = InternalDocumentBuilder::new("pdf");
    let image = ExtractedImage {
        data: Bytes::from_static(&[0xFF, 0xD8]),
        format: Cow::Borrowed("jpeg"),
        image_index: 0,
        page_number: Some(1),
        width: Some(100),
        height: Some(200),
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: Some("A photo".to_string()),
        ocr_result: None,
        bounding_box: None,
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    };
    b.push_image(Some("A photo"), image, Some(1), None);
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Image { image_index: 0 });
    assert_eq!(doc.elements[0].text, "A photo");
    assert_eq!(doc.images.len(), 1);
}

#[test]
fn test_push_code() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_code("fn main() {}", Some("rust"), Some(1), None);
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Code);
    assert_eq!(doc.elements[0].text, "fn main() {}");
    let attrs = doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("language").unwrap(), "rust");
}

#[test]
fn test_push_code_no_language() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_code("echo hello", None, None, None);
    let doc = b.build();

    assert_eq!(doc.elements[0].kind, ElementKind::Code);
    assert!(doc.elements[0].attributes.is_none());
}

#[test]
fn test_push_formula() {
    let mut b = InternalDocumentBuilder::new("latex");
    b.push_formula("E = mc^2", Some(1), None);
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Formula);
    assert_eq!(doc.elements[0].text, "E = mc^2");
}

#[test]
fn test_push_footnote_ref() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_footnote_ref("1", "fn1", Some(1));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::FootnoteRef);
    assert_eq!(doc.elements[0].text, "1");
    assert_eq!(doc.elements[0].anchor.as_deref(), Some("fn1"));
    assert_eq!(doc.relationships.len(), 1);
    assert_eq!(doc.relationships[0].source, 0);
    assert_eq!(doc.relationships[0].kind, RelationshipKind::FootnoteReference);
    match &doc.relationships[0].target {
        RelationshipTarget::Key(k) => assert_eq!(k, "fn1"),
        _ => panic!("Expected Key target"),
    }
}

#[test]
fn test_push_footnote_definition() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_footnote_definition("This is a footnote.", "fn1", Some(1));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::FootnoteDefinition);
    assert_eq!(doc.elements[0].text, "This is a footnote.");
    assert_eq!(doc.elements[0].anchor.as_deref(), Some("fn1"));
}

#[test]
fn test_push_citation() {
    let mut b = InternalDocumentBuilder::new("latex");
    b.push_citation("Smith et al. 2024", "smith2024", Some(3));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Citation);
    assert_eq!(doc.elements[0].text, "Smith et al. 2024");
    assert_eq!(doc.elements[0].anchor.as_deref(), Some("smith2024"));
}

#[test]
fn test_quote_start_end() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_quote_start();
    b.push_paragraph("Quoted text.", vec![], None, None);
    b.push_quote_end();
    let doc = b.build();

    assert_eq!(doc.elements.len(), 3);
    assert_eq!(doc.elements[0].kind, ElementKind::QuoteStart);
    assert_eq!(doc.elements[0].depth, 0);
    assert_eq!(doc.elements[1].kind, ElementKind::Paragraph);
    assert_eq!(doc.elements[1].depth, 1);
    assert_eq!(doc.elements[2].kind, ElementKind::QuoteEnd);
    assert_eq!(doc.elements[2].depth, 0);
}

#[test]
fn test_push_page_break() {
    let mut b = InternalDocumentBuilder::new("pdf");
    b.push_list(false);
    b.push_page_break();
    b.end_list();
    let doc = b.build();

    let pb = doc.elements.iter().find(|e| e.kind == ElementKind::PageBreak).unwrap();
    assert_eq!(pb.depth, 0);
}

#[test]
fn test_push_slide() {
    let mut b = InternalDocumentBuilder::new("pptx");
    b.push_slide(1, Some("Title Slide"), Some(1));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Slide { number: 1 });
    assert_eq!(doc.elements[0].text, "Title Slide");
    let attrs = doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("title").unwrap(), "Title Slide");
}

#[test]
fn test_push_slide_no_title() {
    let mut b = InternalDocumentBuilder::new("pptx");
    b.push_slide(2, None, None);
    let doc = b.build();

    assert_eq!(doc.elements[0].kind, ElementKind::Slide { number: 2 });
    assert_eq!(doc.elements[0].text, "");
    assert!(doc.elements[0].attributes.is_none());
}

#[test]
fn test_push_admonition() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_admonition("warning", Some("Be careful"), Some(1));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::Admonition);
    let attrs = doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("kind").unwrap(), "warning");
    assert_eq!(attrs.get("title").unwrap(), "Be careful");
}

#[test]
fn test_push_admonition_no_title() {
    let mut b = InternalDocumentBuilder::new("markdown");
    b.push_admonition("note", None, None);
    let doc = b.build();

    let attrs = doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("kind").unwrap(), "note");
    assert!(!attrs.contains_key("title"));
    assert_eq!(doc.elements[0].text, "note");
}

#[test]
fn test_push_raw_block() {
    let mut b = InternalDocumentBuilder::new("html");
    b.push_raw_block("html", "<div>raw</div>", Some(1));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::RawBlock);
    assert_eq!(doc.elements[0].text, "<div>raw</div>");
    let attrs = doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("format").unwrap(), "html");
}

#[test]
fn test_push_metadata_block() {
    let mut b = InternalDocumentBuilder::new("email");
    let entries = vec![
        ("From".to_string(), "alice@example.com".to_string()),
        ("To".to_string(), "bob@example.com".to_string()),
    ];
    b.push_metadata_block(&entries, Some(1));
    let doc = b.build();

    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].kind, ElementKind::MetadataBlock);
    let attrs = doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("From").unwrap(), "alice@example.com");
    assert_eq!(attrs.get("To").unwrap(), "bob@example.com");
    assert!(doc.elements[0].text.contains("From: alice@example.com"));
    assert!(doc.elements[0].text.contains("To: bob@example.com"));
}

#[test]
fn test_set_metadata() {
    let mut b = InternalDocumentBuilder::new("pdf");
    b.set_metadata(Metadata {
        title: Some("My Document".to_string()),
        ..Metadata::default()
    });
    let doc = b.build();

    assert_eq!(doc.metadata.title.as_deref(), Some("My Document"));
}

#[test]
fn test_source_format() {
    let mut b = InternalDocumentBuilder::new("pdf");
    b.source_format("docx");
    let doc = b.build();
    assert_eq!(doc.source_format, "docx");
}

#[test]
fn test_node_count_increments() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("A", vec![], None, None);
    b.push_paragraph("B", vec![], None, None);
    b.push_paragraph("C", vec![], None, None);
    assert_eq!(b.node_count, 3);
    let doc = b.build();
    assert_eq!(doc.elements.len(), 3);
}

#[test]
fn test_ids_are_deterministic() {
    let build = || {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(1, "Hello", Some(1), None);
        b.push_paragraph("World", vec![], Some(1), None);
        b.build()
    };
    let doc1 = build();
    let doc2 = build();
    assert_eq!(doc1.elements[0].id, doc2.elements[0].id);
    assert_eq!(doc1.elements[1].id, doc2.elements[1].id);
}

#[test]
fn test_ids_differ_by_index() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("Same text", vec![], Some(1), None);
    b.push_paragraph("Same text", vec![], Some(1), None);
    let doc = b.build();
    assert_ne!(doc.elements[0].id, doc.elements[1].id);
}

#[test]
fn test_bbox_preserved() {
    let mut b = InternalDocumentBuilder::new("pdf");
    let bbox = BoundingBox {
        x0: 10.0,
        y0: 20.0,
        x1: 100.0,
        y1: 50.0,
    };
    b.push_paragraph("text", vec![], Some(1), Some(bbox));
    let doc = b.build();
    let elem_bbox = doc.elements[0].bbox.unwrap();
    assert_eq!(elem_bbox.x0, 10.0);
    assert_eq!(elem_bbox.y1, 50.0);
}

#[test]
fn test_slugify_basic() {
    assert_eq!(slugify("Hello World"), "hello-world");
}

#[test]
fn test_slugify_special_chars() {
    assert_eq!(slugify("What's New?"), "what-s-new");
}

#[test]
fn test_slugify_consecutive_specials() {
    assert_eq!(slugify("A -- B"), "a-b");
}

#[test]
fn test_slugify_leading_trailing() {
    assert_eq!(slugify("  Hello  "), "hello");
}

#[test]
fn test_slugify_numbers() {
    assert_eq!(slugify("Section 3.1"), "section-3-1");
}

#[test]
fn test_slugify_unicode() {
    assert_eq!(slugify("Über Cool"), "über-cool");
}

#[test]
fn test_slugify_empty() {
    assert_eq!(slugify(""), "");
}

#[test]
fn test_full_document_construction() {
    let mut b = InternalDocumentBuilder::new("markdown");

    b.set_metadata(Metadata {
        title: Some("Test Document".to_string()),
        ..Metadata::default()
    });

    b.push_heading(1, "Introduction", Some(1), None);
    b.push_paragraph("Welcome to the document.", vec![], Some(1), None);

    b.push_heading(2, "Details", Some(1), None);
    b.push_paragraph("Some details here.", vec![], Some(1), None);

    b.push_list(false);
    b.push_list_item("First item", false, vec![], Some(1), None);
    b.push_list_item("Second item", false, vec![], Some(1), None);
    b.end_list();

    b.push_code("let x = 1;", Some("rust"), Some(2), None);
    b.push_formula("E = mc^2", Some(2), None);

    b.push_footnote_ref("1", "fn1", Some(2));
    b.push_footnote_definition("A footnote.", "fn1", Some(2));

    b.push_quote_start();
    b.push_paragraph("A famous quote.", vec![], Some(3), None);
    b.push_quote_end();

    b.push_page_break();

    let doc = b.build();

    assert_eq!(doc.metadata.title.as_deref(), Some("Test Document"));
    assert_eq!(doc.source_format, "markdown");
    assert_eq!(doc.elements.len(), 16);
    assert_eq!(doc.relationships.len(), 1);

    assert_eq!(doc.elements[0].anchor.as_deref(), Some("introduction"));
    assert_eq!(doc.elements[2].anchor.as_deref(), Some("details"));
}

fn stub_image(index: u32) -> ExtractedImage {
    ExtractedImage {
        data: Bytes::from_static(&[0xFF, 0xD8]),
        format: Cow::Borrowed("jpeg"),
        image_index: index,
        page_number: None,
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: None,
        ocr_result: None,
        bounding_box: None,
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    }
}

#[test]
fn test_append_document_remaps_indices() {
    let mut dst = InternalDocumentBuilder::new("jupyter");
    dst.push_image(Some("existing"), stub_image(0), None, None);

    let mut src_builder = InternalDocumentBuilder::new("markdown");
    src_builder.push_paragraph("prose", vec![], None, None);
    src_builder.push_image(Some("stored"), stub_image(0), None, None);
    src_builder.push_element(InternalElement::text(
        ElementKind::Image { image_index: u32::MAX },
        "remote.png",
        0,
    ));
    let src = src_builder.build();

    dst.append_document(src);
    let doc = dst.build();

    let image_indices: Vec<u32> = doc
        .elements
        .iter()
        .filter_map(|e| match e.kind {
            ElementKind::Image { image_index } => Some(image_index),
            _ => None,
        })
        .collect();
    assert_eq!(
        image_indices,
        vec![0, 1, u32::MAX],
        "existing=0, appended stored=1, sentinel unchanged"
    );
    assert!(
        doc.elements.iter().any(|e| e.text == "prose"),
        "prose element carried over"
    );
    assert_eq!(doc.images.len(), 2, "both stored images retained");
}
