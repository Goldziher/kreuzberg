use super::*;
use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};
use crate::types::internal_builder::InternalDocumentBuilder;

#[test]
fn test_render_djot_title() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_title("My Document", None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert_eq!(out, "# My Document\n");
}

#[test]
fn test_render_djot_heading_levels() {
    for level in 1u8..=6 {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(level, "Heading", None, None);
        let doc = b.build();
        let out = render_djot(&doc);
        let hashes = "#".repeat(level as usize);
        assert!(
            out.starts_with(&format!("{} Heading", hashes)),
            "level {}: got {}",
            level,
            out
        );
    }
}

#[test]
fn test_render_djot_paragraph() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("Hello world.", vec![], None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert_eq!(out, "Hello world.\n");
}

#[test]
fn test_render_djot_unordered_list() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_list(false);
    b.push_list_item("Alpha", false, vec![], None, None);
    b.push_list_item("Beta", false, vec![], None, None);
    b.end_list();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("- Alpha\n"), "got: {}", out);
    assert!(out.contains("- Beta\n"), "got: {}", out);
}

#[test]
fn test_render_djot_ordered_list() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_list(true);
    b.push_list_item("First", true, vec![], None, None);
    b.push_list_item("Second", true, vec![], None, None);
    b.end_list();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("1. First\n"), "got: {}", out);
    assert!(out.contains("2. Second\n"), "got: {}", out);
}

/// #### FAILS against unfixed code
/// `InternalElement::set_list_item_source_label` does not exist on unfixed
/// code, so this does not compile without the fix. Once compiled, it proves
/// two things: a captured literal source label (here "B.") is written out
/// verbatim instead of the synthesized "1.", and a *sibling* item with no
/// label of its own still gets the ordinal it would have held either way
/// (2, not 1) -- `next_list_number` is called for every ordered item,
/// labeled or not, precisely so this doesn't happen.
#[cfg(feature = "pdf")]
#[test]
fn test_render_djot_ordered_list_with_a_literal_source_label() {
    use crate::types::internal::{ElementKind, InternalElement};

    let mut doc = InternalDocument::new("pdf");
    doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: true }, "", 0));
    let mut labeled = InternalElement::text(ElementKind::ListItem { ordered: true }, "General Provisions.", 1);
    labeled.set_list_item_source_label("B.");
    doc.push_element(labeled);
    doc.push_element(InternalElement::text(
        ElementKind::ListItem { ordered: true },
        "Second item",
        1,
    ));
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));

    let out = render_djot(&doc);
    assert!(out.contains("- B. General Provisions.\n"), "got: {out}");
    assert!(out.contains("2. Second item\n"), "got: {out}");
}

#[test]
fn test_render_djot_nested_list() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_list(false);
    b.push_list_item("Outer", false, vec![], None, None);
    b.push_list(false);
    b.push_list_item("Inner", false, vec![], None, None);
    b.end_list();
    b.end_list();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("- Outer\n"), "got: {}", out);
    assert!(out.contains("  - Inner\n"), "got: {}", out);
}

#[test]
fn test_render_djot_code_block_with_language() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_code("print('hi')", Some("python"), None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("```python\n"), "got: {}", out);
    assert!(out.contains("print('hi')"), "got: {}", out);
}

#[test]
fn test_render_djot_formula() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_formula("x^2 + y^2 = z^2", None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("$$\n"), "got: {}", out);
    assert!(out.contains("x^2 + y^2 = z^2"), "got: {}", out);
}

#[test]
fn test_render_djot_table() {
    let mut b = InternalDocumentBuilder::new("test");
    let cells = vec![
        vec!["Col1".to_string(), "Col2".to_string()],
        vec!["A".to_string(), "B".to_string()],
    ];
    b.push_table_from_cells(&cells, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("| Col1 | Col2 |"), "got: {}", out);
    assert!(out.contains("| A | B |"), "got: {}", out);
}

#[test]
fn test_render_djot_image() {
    let mut b = InternalDocumentBuilder::new("test");
    let image = crate::types::ExtractedImage {
        data: bytes::Bytes::new(),
        format: std::borrow::Cow::Borrowed("png"),
        image_index: 0,
        page_number: None,
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: Some("A diagram".to_string()),
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
    b.push_image(Some("A diagram"), image, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("![A diagram]()"), "got: {}", out);
}

#[test]
fn test_render_djot_page_break() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_page_break();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(!out.contains("---"), "PageBreak should not render as ---, got: {}", out);
}

#[test]
fn test_render_djot_slide() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_slide(1, Some("Slide Title"), None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("---"), "got: {}", out);
    assert!(out.contains("## Slide Title"), "got: {}", out);
}

#[test]
fn test_render_djot_definition_term_and_description() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_definition_term("Term", None);
    b.push_definition_description("The definition", None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("Term\n"), "got: {}", out);
    assert!(out.contains(": The definition"), "got: {}", out);
}

#[test]
fn test_render_djot_admonition_with_title() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_admonition("warning", Some("Careful!"), None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("::: warning"), "got: {}", out);
    assert!(out.contains("**Careful!**"), "got: {}", out);
    assert!(out.contains(":::"), "got: {}", out);
}

#[test]
fn test_render_djot_admonition_without_title() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_admonition("note", None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("::: note"), "got: {}", out);
}

#[test]
fn test_render_djot_raw_block() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_raw_block("tex", "\\LaTeX{}", None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("\\LaTeX{}"), "got: {}", out);
}

#[test]
fn test_render_djot_metadata_block() {
    let mut b = InternalDocumentBuilder::new("test");
    let entries = vec![("Author".to_string(), "Bob".to_string())];
    b.push_metadata_block(&entries, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("*Author*: Bob"), "got: {}", out);
}

#[test]
fn test_render_djot_empty_document() {
    let b = InternalDocumentBuilder::new("test");
    let doc = b.build();
    let out = render_djot(&doc);
    assert_eq!(out, "");
}

#[test]
fn test_render_djot_bold_annotation() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Bold,
    }];
    b.push_paragraph("Hello world", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("*Hello* world"), "got: {}", out);
}

#[test]
fn test_render_djot_italic_annotation() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Italic,
    }];
    b.push_paragraph("Hello world", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("_Hello_ world"), "got: {}", out);
}

#[test]
fn test_render_djot_code_annotation() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 4,
        kind: AnnotationKind::Code,
    }];
    b.push_paragraph("code rest", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("`code`"), "got: {}", out);
}

#[test]
fn test_render_djot_link_annotation() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 4,
        kind: AnnotationKind::Link {
            url: "https://example.com".to_string(),
            title: None,
        },
    }];
    b.push_paragraph("link text", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("[link](https://example.com)"), "got: {}", out);
}

#[test]
fn test_render_djot_strikethrough_annotation() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 3,
        kind: AnnotationKind::Strikethrough,
    }];
    b.push_paragraph("old new", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("{-old-}"), "got: {}", out);
}

#[test]
fn test_render_djot_underline_annotation() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 4,
        kind: AnnotationKind::Underline,
    }];
    b.push_paragraph("text rest", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("[text]{.underline}"), "got: {}", out);
}

#[test]
fn test_render_djot_multiple_non_overlapping_annotations() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![
        TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Bold,
        },
        TextAnnotation {
            start: 6,
            end: 11,
            kind: AnnotationKind::Italic,
        },
    ];
    b.push_paragraph("Hello world", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("*Hello*"), "got: {}", out);
    assert!(out.contains("_world_"), "got: {}", out);
}

#[test]
fn test_render_djot_overlapping_annotations_inner_skipped() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![
        TextAnnotation {
            start: 0,
            end: 11,
            kind: AnnotationKind::Bold,
        },
        TextAnnotation {
            start: 6,
            end: 11,
            kind: AnnotationKind::Italic,
        },
    ];
    b.push_paragraph("Hello world", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("*Hello world*"), "got: {}", out);
    assert!(!out.contains("_world_"), "overlapping should be skipped, got: {}", out);
}

#[test]
fn test_render_djot_blockquote() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_quote_start();
    b.push_paragraph("Quoted.", vec![], None, None);
    b.push_quote_end();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("> Quoted."), "got: {}", out);
}

#[test]
fn test_render_djot_nested_blockquote() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_quote_start();
    b.push_quote_start();
    b.push_paragraph("Deep.", vec![], None, None);
    b.push_quote_end();
    b.push_quote_end();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("> > Deep."), "got: {}", out);
}

#[test]
fn test_render_djot_list_inside_blockquote() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_quote_start();
    b.push_list(false);
    b.push_list_item("Item in quote", false, vec![], None, None);
    b.end_list();
    b.push_quote_end();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("> - Item in quote"), "got: {}", out);
}

#[test]
fn test_render_djot_footnote() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("See note", vec![], None, None);
    b.push_footnote_ref("1", "fn1", None);
    let def = b.push_footnote_definition("A note.", "fn1", None);
    b.set_layer(def, ContentLayer::Footnote);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("[^1]"), "got: {}", out);
    assert!(out.contains("[^1]: A note."), "got: {}", out);
}

#[test]
fn test_render_djot_footnote_ref_joins_the_block_it_follows() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_heading(1, "Title", None, None);
    b.push_footnote_ref("1", "fn1", None);
    b.push_paragraph("See note", vec![], None, None);
    b.push_footnote_ref("2", "fn2", None);
    b.push_footnote_ref("3", "fn3", None);
    b.push_quote_start();
    b.push_paragraph("Quoted", vec![], None, None);
    b.push_footnote_ref("4", "fn4", None);
    b.push_quote_end();
    b.push_table_from_cells(&[vec!["a".to_string(), "b".to_string()]], None, None);
    b.push_footnote_ref("5", "fn5", None);
    b.push_paragraph("Last", vec![], None, None);
    for n in 1..=5 {
        let def = b.push_footnote_definition("A note.", &format!("fn{n}"), None);
        b.set_layer(def, ContentLayer::Footnote);
    }
    let out = render_djot(&b.build());
    assert!(
        out.starts_with("# Title[^1]\n\nSee note[^2][^3]\n\n> Quoted[^4]\n"),
        "got: {}",
        out
    );
    // A table cannot take an inline marker, so the reference becomes its own paragraph.
    assert!(out.contains("|\n\n[^5]\n\nLast\n"), "got: {}", out);
}

#[test]
fn test_render_djot_multiple_footnotes() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_footnote_ref("a", "fn1", None);
    b.push_footnote_ref("b", "fn2", None);
    let d1 = b.push_footnote_definition("Note 1.", "fn1", None);
    let d2 = b.push_footnote_definition("Note 2.", "fn2", None);
    b.set_layer(d1, ContentLayer::Footnote);
    b.set_layer(d2, ContentLayer::Footnote);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("[^1]"), "got: {}", out);
    assert!(out.contains("[^2]"), "got: {}", out);
    assert!(out.contains("[^1]: Note 1."), "got: {}", out);
    assert!(out.contains("[^2]: Note 2."), "got: {}", out);
}

#[test]
fn test_render_djot_citation() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_citation("Doe 2023", "doe2023", None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("[^doe2023]: Doe 2023"), "got: {}", out);
}

#[test]
fn test_render_djot_normalizes_multiple_spaces() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("Hello   world  foo", vec![], None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("Hello world foo"),
        "multiple spaces should collapse, got: {}",
        out
    );
}

#[test]
fn test_render_djot_normalizes_newlines_to_spaces() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("Hello\nworld\nfoo", vec![], None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("Hello world foo"),
        "newlines should become spaces, got: {}",
        out
    );
}

#[test]
fn test_render_djot_strips_control_characters() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("Hello\x02world", vec![], None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("Helloworld"),
        "control chars should be stripped, got: {}",
        out
    );
    assert!(!out.contains('\x02'), "STX should be removed, got: {}", out);
}

#[test]
fn test_render_djot_normalizes_heading_text() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_heading(2, "Hello   world", None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("## Hello world"),
        "heading text should be normalized, got: {}",
        out
    );
}

#[test]
fn test_render_djot_normalizes_list_item_text() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_list(false);
    b.push_list_item("Hello   world", false, vec![], None, None);
    b.end_list();
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("- Hello world"),
        "list item text should be normalized, got: {}",
        out
    );
}

#[test]
fn test_render_djot_normalizes_annotated_text() {
    let mut b = InternalDocumentBuilder::new("test");
    let ann = vec![TextAnnotation {
        start: 0,
        end: 13,
        kind: AnnotationKind::Bold,
    }];
    b.push_paragraph("Hello   world rest", ann, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("*Hello world*"),
        "annotated text should be normalized, got: {}",
        out
    );
}

#[test]
fn test_render_djot_code_block_no_language() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_code("x = 1", None, None, None);
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        out.contains("```\n"),
        "no-lang code block should have bare fence, got: {}",
        out
    );
}

#[test]
fn test_djot_markdown_heading_parity() {
    use crate::rendering::render_markdown;

    let mut b = InternalDocumentBuilder::new("test");
    b.push_heading(1, "Title", None, None);
    b.push_heading(2, "Section", None, None);
    b.push_heading(3, "Subsection", None, None);
    let doc = b.build();

    let djot_out = render_djot(&doc);
    let md_out = render_markdown(&doc);

    assert!(djot_out.contains("# Title"), "djot heading 1, got: {}", djot_out);
    assert!(md_out.contains("# Title"), "md heading 1, got: {}", md_out);
    assert!(djot_out.contains("## Section"), "djot heading 2, got: {}", djot_out);
    assert!(md_out.contains("## Section"), "md heading 2, got: {}", md_out);
    assert!(djot_out.contains("### Subsection"), "djot heading 3, got: {}", djot_out);
    assert!(md_out.contains("### Subsection"), "md heading 3, got: {}", md_out);
}

#[test]
fn test_djot_markdown_table_content_parity() {
    use crate::rendering::render_markdown;

    let mut b = InternalDocumentBuilder::new("test");
    let cells = vec![
        vec!["Name".to_string(), "Value".to_string()],
        vec!["Alpha".to_string(), "100".to_string()],
        vec!["Beta".to_string(), "200".to_string()],
    ];
    b.push_table_from_cells(&cells, None, None);
    let doc = b.build();

    let djot_out = render_djot(&doc);
    let md_out = render_markdown(&doc);

    for cell in &["Name", "Value", "Alpha", "100", "Beta", "200"] {
        assert!(
            djot_out.contains(cell),
            "djot missing cell '{}', got: {}",
            cell,
            djot_out
        );
        assert!(md_out.contains(cell), "md missing cell '{}', got: {}", cell, md_out);
    }
}

#[test]
fn test_djot_markdown_block_count_parity() {
    use crate::rendering::render_markdown;

    let mut b = InternalDocumentBuilder::new("test");
    b.push_heading(1, "Title", None, None);
    b.push_paragraph("First paragraph.", vec![], None, None);
    b.push_paragraph("Second paragraph.", vec![], None, None);
    b.push_list(false);
    b.push_list_item("Item A", false, vec![], None, None);
    b.push_list_item("Item B", false, vec![], None, None);
    b.end_list();
    b.push_code("x = 1", Some("python"), None, None);
    let doc = b.build();

    let djot_out = render_djot(&doc);
    let md_out = render_markdown(&doc);

    let djot_blocks: Vec<&str> = djot_out.lines().filter(|l| !l.trim().is_empty()).collect();
    let md_blocks: Vec<&str> = md_out.lines().filter(|l| !l.trim().is_empty()).collect();

    assert!(
        djot_blocks.len() >= md_blocks.len() - 1,
        "djot block count ({}) should be close to markdown ({})\ndjot:\n{}\nmd:\n{}",
        djot_blocks.len(),
        md_blocks.len(),
        djot_out,
        md_out,
    );

    for text in &[
        "Title",
        "First paragraph",
        "Second paragraph",
        "Item A",
        "Item B",
        "x = 1",
    ] {
        assert!(djot_out.contains(text), "djot missing '{}', got: {}", text, djot_out);
        assert!(md_out.contains(text), "md missing '{}', got: {}", text, md_out);
    }
}

/// Regression #914: render_djot must not emit image markup for orphaned
/// `ElementKind::Image` elements — i.e. when `doc.images` is empty.
#[test]
fn test_image_out_of_bounds_index_dropped() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("Only text.", vec![], None, None);
    b.push_element(crate::types::internal::InternalElement::text(
        ElementKind::Image { image_index: 99 },
        "",
        0,
    ));
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(
        !out.contains("image_99"),
        "out-of-bounds image must be dropped; got: {}",
        out
    );
    assert!(out.contains("Only text."), "paragraph must still render; got: {}", out);
}

#[test]
fn test_image_out_of_bounds_index_with_desc_renders() {
    let mut b = InternalDocumentBuilder::new("test");
    b.push_element(crate::types::internal::InternalElement::text(
        ElementKind::Image { image_index: 99 },
        "Missing image",
        0,
    ));
    let doc = b.build();
    let out = render_djot(&doc);
    assert!(out.contains("![Missing image]()"), "got: {}", out);
}
#[test]
fn should_render_issue_2123_nested_math_as_native_djot() {
    let text = r"$\alpha  + `b`$";
    let annotations = vec![
        TextAnnotation {
            start: 0,
            end: text.len() as u32,
            kind: AnnotationKind::math(false),
        },
        TextAnnotation {
            start: 0,
            end: text.len() as u32,
            kind: AnnotationKind::Bold,
        },
        TextAnnotation {
            start: 0,
            end: text.len() as u32,
            kind: AnnotationKind::Link {
                url: "https://example.com".into(),
                title: None,
            },
        },
    ];
    let output = render_djot_annotated(text, &annotations);
    assert_eq!(output, r"[*$``\alpha  + `b` ``*](https://example.com)");
    let events: Vec<_> = jotdown::Parser::new(&output).collect();
    let literals: Vec<_> = events
        .iter()
        .filter_map(|event| {
            if let jotdown::Event::Str(value) = event {
                Some(value.as_ref())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(literals, [r"\alpha  + `b`"]);

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                jotdown::Event::Start(jotdown::Container::Math { display: false }, _)
            ))
            .count(),
        1
    );
}
