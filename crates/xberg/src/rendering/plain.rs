//! Render an `InternalDocument` to plain text.
//!
//! Emits text only, with no formatting. Double newlines separate blocks.
//! Inline text annotations (bold/italic/links) are stripped. Tables are
//! rendered as space-separated columns. Document-level PDF annotations
//! (issue #63), when present, are appended as a trailing "Annotations:"
//! section.

use crate::types::annotations::PdfAnnotation;
use crate::types::document_structure::ContentLayer;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

use super::common::{
    annotation_display_text, annotation_type_label, get_admonition_kind, get_admonition_title, image_ocr_text,
    parse_metadata_entries, render_table_plain,
};

/// Render an `InternalDocument` to plain text.
pub(crate) fn render_plain(doc: &InternalDocument) -> String {
    let mut out = String::with_capacity(doc.elements.len() * 80);
    let mut last_heading_depth = None;
    for elem in &doc.elements {
        if elem.layer != ContentLayer::Body || elem.kind.is_container_start() || elem.kind.is_container_end() {
            continue;
        }
        render_element_plain(doc, elem, &mut out, &mut last_heading_depth);
    }
    append_definitions_plain(doc, ElementKind::FootnoteDefinition, &mut out);
    append_definitions_plain(doc, ElementKind::CommentDefinition, &mut out);
    if let Some(annotations) = doc.annotations.as_deref() {
        let block = render_annotations_plain(annotations);
        if !block.is_empty() {
            if !out.trim_end().is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&block);
        }
    }
    out.truncate(out.trim_end().len());
    out
}

fn render_element_plain(
    doc: &InternalDocument,
    elem: &InternalElement,
    out: &mut String,
    heading_depth: &mut Option<u16>,
) {
    match elem.kind {
        ElementKind::Title | ElementKind::Heading { .. } | ElementKind::Paragraph => {
            render_text_plain(elem, out, heading_depth)
        }
        ElementKind::ListItem { .. } => render_list_item_plain(elem, out),
        ElementKind::Code | ElementKind::RawBlock => append_verbatim_plain(&elem.text, out),
        ElementKind::Formula | ElementKind::DefinitionDescription => append_block_plain(&elem.text, out),
        ElementKind::Table { table_index } => render_plain_table(doc, elem, table_index, out),
        ElementKind::Image { image_index } => render_image_plain(doc, elem, image_index, out),
        ElementKind::Citation | ElementKind::Slide { .. } | ElementKind::OcrText { .. } => {
            if !elem.text.is_empty() {
                append_block_plain(&elem.text, out);
            }
        }
        ElementKind::DefinitionTerm => {
            out.push_str(&elem.text);
            out.push_str(": ");
        }
        ElementKind::Admonition => render_admonition_plain(elem, out),
        ElementKind::MetadataBlock => render_metadata_plain(&elem.text, out),
        ElementKind::PageBreak => out.push('\n'),
        ElementKind::FootnoteRef
        | ElementKind::FootnoteDefinition
        | ElementKind::CommentRef
        | ElementKind::CommentDefinition
        | ElementKind::ListStart { .. }
        | ElementKind::ListEnd
        | ElementKind::QuoteStart
        | ElementKind::QuoteEnd
        | ElementKind::GroupStart
        | ElementKind::GroupEnd => {}
    }
}

fn render_text_plain(elem: &InternalElement, out: &mut String, last_heading_depth: &mut Option<u16>) {
    if elem.text.is_empty() {
        return;
    }
    let is_heading = matches!(elem.kind, ElementKind::Heading { .. });
    if is_heading {
        if let Some(last_depth) = *last_heading_depth
            && (last_depth == 0 || last_depth == 1)
            && last_depth == elem.depth
            && !out.is_empty()
            && !out.ends_with("\n\n")
        {
            out.push('\n');
        }
        *last_heading_depth = Some(elem.depth);
    }
    if matches!(elem.kind, ElementKind::Paragraph) || (is_heading && elem.depth > 0) {
        out.push_str(&"  ".repeat(elem.depth as usize));
    }
    out.push_str(&elem.text);
    if is_heading {
        append_heading_attributes_plain(elem, out);
        out.push('\n');
    } else {
        out.push_str("\n\n");
    }
}

fn append_heading_attributes_plain(elem: &InternalElement, out: &mut String) {
    let Some(attrs) = elem.public_attributes() else { return };
    let mut filtered: Vec<_> = attrs
        .iter()
        .filter(|(key, value)| !key.starts_with("xmlns") && !value.is_empty())
        .collect();
    filtered.sort_by_key(|(key, _)| key.as_str());
    let formatted: Vec<String> = filtered
        .iter()
        .map(|(key, value)| format!("{}: {}", key, value))
        .collect();
    if !formatted.is_empty() {
        out.push_str(" (");
        out.push_str(&formatted.join(", "));
        out.push(')');
    }
}

fn render_list_item_plain(elem: &InternalElement, out: &mut String) {
    // Plain text omits generated list markers but retains literal source labels. ~keep
    if let Some(label) = elem.list_item_source_label().filter(|label| !label.is_empty()) {
        out.push_str(label);
        out.push(' ');
    }
    out.push_str(&elem.text);
    out.push('\n');
}

fn append_verbatim_plain(text: &str, out: &mut String) {
    out.push_str(text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
}

fn append_block_plain(text: &str, out: &mut String) {
    out.push_str(text);
    out.push_str("\n\n");
}

fn render_plain_table(doc: &InternalDocument, elem: &InternalElement, index: u32, out: &mut String) {
    let Some(table) = doc.tables.get(index as usize) else {
        return;
    };
    let text = if !elem.text.is_empty() {
        elem.text.clone()
    } else if !table.cells.is_empty() {
        render_table_plain(&table.cells)
    } else {
        table.markdown.clone()
    };
    if !text.trim().is_empty() {
        out.push_str(&text);
        out.push('\n');
    }
}

fn render_image_plain(doc: &InternalDocument, elem: &InternalElement, index: u32, out: &mut String) {
    if let Some(image) = doc.images.get(index as usize) {
        if let Some(description) = image.description.as_deref()
            && !description.is_empty()
        {
            out.push_str("[Image: ");
            out.push_str(description);
            out.push_str("]\n\n");
        }
        if let Some(text) = image_ocr_text(doc, elem, index) {
            append_block_plain(text, out);
        }
    } else if !elem.text.trim().is_empty() {
        // Unresolved archive image references still retain the source caption. ~keep
        out.push_str("[Image: ");
        out.push_str(elem.text.trim());
        out.push_str("]\n\n");
    }
}

fn render_admonition_plain(elem: &InternalElement, out: &mut String) {
    out.push_str(get_admonition_title(elem).unwrap_or_else(|| get_admonition_kind(elem)));
    out.push_str("\n\n");
    if !elem.text.is_empty() {
        append_block_plain(&elem.text, out);
    }
}

fn render_metadata_plain(text: &str, out: &mut String) {
    let entries = parse_metadata_entries(text);
    if !entries.is_empty() {
        for (key, value) in &entries {
            out.push_str(key);
            out.push_str(": ");
            out.push_str(value);
            out.push('\n');
        }
        out.push('\n');
    } else if !text.is_empty() {
        append_verbatim_plain(text, out);
    }
}

fn append_definitions_plain(doc: &InternalDocument, kind: ElementKind, out: &mut String) {
    let mut definitions = doc
        .elements
        .iter()
        .filter(|elem| elem.kind == kind && elem.layer == ContentLayer::Footnote)
        .peekable();
    if definitions.peek().is_none() {
        return;
    }
    out.push('\n');
    for elem in definitions {
        append_block_plain(&elem.text, out);
    }
}

/// Render the document-level PDF annotations (issue #63) as a plain-text
/// section: an "Annotations:" header followed by one line per annotation.
///
/// Returns an empty string when `annotations` is empty.
fn render_annotations_plain(annotations: &[PdfAnnotation]) -> String {
    if annotations.is_empty() {
        return String::new();
    }

    let mut out = String::from("Annotations:\n");
    for annotation in annotations {
        out.push_str(annotation_type_label(annotation.annotation_type));
        out.push_str(" (page ");
        out.push_str(&annotation.page_number.to_string());
        out.push(')');

        if let Some(author) = annotation.author.as_deref().filter(|s| !s.is_empty()) {
            out.push_str(" by ");
            out.push_str(author);
        }

        if let Some(text) = annotation_display_text(annotation) {
            out.push_str(": ");
            out.push_str(text);
        }

        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::document_structure::ContentLayer;
    use crate::types::internal_builder::InternalDocumentBuilder;

    #[test]
    fn test_render_plain_title() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_title("My Document", None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(out, "My Document");
    }

    #[test]
    fn test_render_plain_heading() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(2, "Section", None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(out, "  Section");
    }

    #[test]
    fn test_render_plain_paragraph() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Hello world.", vec![], None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(out, "Hello world.");
    }

    #[test]
    fn test_render_plain_list_items() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_list_item("Alpha", false, vec![], None, None);
        b.push_list_item("Beta", false, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Alpha\n"), "got: {}", out);
        assert!(out.contains("Beta"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_ordered_list() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(true);
        b.push_list_item("First", true, vec![], None, None);
        b.push_list_item("Second", true, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("First"), "got: {}", out);
        assert!(out.contains("Second"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_code_block() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_code("fn main() {}", Some("rust"), None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("fn main() {}"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_formula() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_formula("E = mc^2", None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("E = mc^2"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_table() {
        let mut b = InternalDocumentBuilder::new("test");
        let cells = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ];
        b.push_table_from_cells(&cells, None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Name Age"), "got: {}", out);
        assert!(out.contains("Alice 30"), "got: {}", out);
    }

    #[test]
    fn should_use_explicit_plain_text_for_table_when_present() {
        let mut builder = InternalDocumentBuilder::new("test");
        let cells = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
        ];
        let table_index = builder.push_table_from_cells(&cells, None, None);
        builder.set_text(table_index, "Name: Alice\nAge: 30");

        let output = render_plain(&builder.build());

        assert_eq!(output, "Name: Alice\nAge: 30");
    }

    #[test]
    fn test_render_plain_image() {
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
            description: Some("A nice photo".to_string()),
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
        b.push_image(Some("A nice photo"), image, None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("[Image: A nice photo]"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_image_has_its_ocr_text_once_after_the_description() {
        let mut b = InternalDocumentBuilder::new("test");
        let image = crate::types::ExtractedImage {
            description: Some("A nice photo".to_string()),
            ocr_result: Some(Box::new(crate::types::ExtractedDocument {
                content: "Crate 17 holds forty blue lanterns".to_string(),
                ..Default::default()
            })),
            ..Default::default()
        };
        b.push_image(Some("A nice photo"), image, None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(
            out.trim_end(),
            "[Image: A nice photo]\n\nCrate 17 holds forty blue lanterns"
        );
    }

    #[test]
    fn test_render_plain_image_with_empty_ocr_text_adds_no_empty_block() {
        let mut b = InternalDocumentBuilder::new("test");
        let image = crate::types::ExtractedImage {
            description: Some("A nice photo".to_string()),
            ocr_result: Some(Box::new(crate::types::ExtractedDocument {
                content: String::new(),
                ..Default::default()
            })),
            ..Default::default()
        };
        b.push_image(Some("A nice photo"), image, None, None);
        b.push_paragraph("After", vec![], None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(out, "[Image: A nice photo]\n\nAfter");
    }

    #[test]
    fn test_render_plain_page_break() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Before", vec![], None, None);
        b.push_page_break();
        b.push_paragraph("After", vec![], None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Before"), "got: {}", out);
        assert!(out.contains("After"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_slide() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_slide(1, Some("Slide Title"), None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Slide Title"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_definition_term_and_description() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_definition_term("Rust", None);
        b.push_definition_description("A systems language", None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Rust: "), "got: {}", out);
        assert!(out.contains("A systems language"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_admonition_with_title() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_admonition("warning", Some("Be careful"), None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Be careful"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_admonition_without_title() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_admonition("note", None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("note"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_raw_block() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_raw_block("tex", "\\LaTeX{}", None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("\\LaTeX{}"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_metadata_block() {
        let mut b = InternalDocumentBuilder::new("test");
        let entries = vec![("Author".to_string(), "Alice".to_string())];
        b.push_metadata_block(&entries, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Author: Alice"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_empty_document() {
        let b = InternalDocumentBuilder::new("test");
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(out, "");
    }

    #[test]
    fn test_render_plain_strips_annotations() {
        use crate::types::document_structure::{AnnotationKind, TextAnnotation};
        let mut b = InternalDocumentBuilder::new("test");
        let ann = vec![TextAnnotation {
            start: 0,
            end: 5,
            kind: AnnotationKind::Bold,
        }];
        b.push_paragraph("Hello world", ann, None, None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert_eq!(out, "Hello world");
    }

    #[test]
    fn render_plain_excludes_internal_heading_attributes() {
        use ahash::AHashMap;

        let mut b = InternalDocumentBuilder::new("test");
        let heading = b.push_heading(1, "Service agreement", None, None);
        b.set_attributes(
            heading,
            AHashMap::from_iter([
                ("xberg:internal:font-size-pt".to_string(), "18".to_string()),
                ("role".to_string(), "contract".to_string()),
            ]),
        );

        assert_eq!(render_plain(&b.build()), "Service agreement (role: contract)");
    }

    #[test]
    fn test_render_plain_blockquote_content() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_quote_start();
        b.push_paragraph("Quoted text.", vec![], None, None);
        b.push_quote_end();
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Quoted text."), "got: {}", out);
    }

    #[test]
    fn test_render_plain_nested_list() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_list_item("Outer", false, vec![], None, None);
        b.push_list(false);
        b.push_list_item("Inner", false, vec![], None, None);
        b.end_list();
        b.end_list();
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Outer"), "got: {}", out);
        assert!(out.contains("Inner"), "got: {}", out);
    }

    #[test]
    fn test_render_plain_footnote_definitions_at_end() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Main text", vec![], None, None);
        b.push_footnote_ref("1", "fn1", None);
        let def = b.push_footnote_definition("A note.", "fn1", None);
        b.set_layer(def, ContentLayer::Footnote);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Main text"), "got: {}", out);
        assert!(out.contains("A note."), "got: {}", out);
    }

    #[test]
    fn test_render_plain_citation() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_citation("Smith 2024", "smith2024", None);
        let doc = b.build();
        let out = render_plain(&doc);
        assert!(out.contains("Smith 2024"), "got: {}", out);
    }
}

#[cfg(test)]
#[path = "plain_lint_tests.rs"]
mod lint_tests;
