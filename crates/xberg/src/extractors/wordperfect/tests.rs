use super::*;

#[test]
fn test_wordperfect_extractor_plugin_interface() {
    let extractor = WordPerfectExtractor::new();
    assert_eq!(extractor.name(), "wordperfect-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(extractor.priority(), 50);
    assert_eq!(
        extractor.supported_mime_types(),
        &["application/vnd.wordperfect", "application/wordperfect"]
    );
}

#[test]
fn test_wordperfect_extractor_initialize_shutdown() {
    let extractor = WordPerfectExtractor::new();
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

#[test]
fn test_wordperfect_extractor_default() {
    let extractor = WordPerfectExtractor;
    assert_eq!(extractor.name(), "wordperfect-extractor");
}

#[test]
fn test_build_document_simple_paragraph_with_bold() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::BoldStart,
            WpdEvent::Text("Hello".to_string()),
            WpdEvent::BoldEnd,
            WpdEvent::Space,
            WpdEvent::Text("world".to_string()),
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    assert_eq!(internal_doc.elements.len(), 1);
    let elem = &internal_doc.elements[0];
    assert_eq!(elem.text, "Hello world");
    assert_eq!(elem.kind, crate::types::internal::ElementKind::Paragraph);
    assert_eq!(elem.annotations.len(), 1);
    assert_eq!(elem.annotations[0].kind, AnnotationKind::Bold);
    assert_eq!(elem.annotations[0].start, 0);
    assert_eq!(elem.annotations[0].end, 5);
}

#[test]
fn test_build_document_heading() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::HeadingStart { level: 2 },
            WpdEvent::Text("Title".to_string()),
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    assert_eq!(internal_doc.elements.len(), 1);
    assert_eq!(
        internal_doc.elements[0].kind,
        crate::types::internal::ElementKind::Heading { level: 2 }
    );
    assert_eq!(internal_doc.elements[0].text, "Title");
}

#[test]
fn test_build_document_list() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::ListItemStart {
                ordered: true,
                level: 1,
                counter: 1,
            },
            WpdEvent::Text("First".to_string()),
            WpdEvent::ParagraphEnd,
            WpdEvent::ListItemEnd,
            WpdEvent::ListItemStart {
                ordered: true,
                level: 1,
                counter: 2,
            },
            WpdEvent::Text("Second".to_string()),
            WpdEvent::ParagraphEnd,
            WpdEvent::ListItemEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    use crate::types::internal::ElementKind;
    assert_eq!(internal_doc.elements[0].kind, ElementKind::ListStart { ordered: true });
    assert_eq!(internal_doc.elements[1].kind, ElementKind::ListItem { ordered: true });
    assert_eq!(internal_doc.elements[1].text, "First");
    assert_eq!(internal_doc.elements[2].kind, ElementKind::ListItem { ordered: true });
    assert_eq!(internal_doc.elements[2].text, "Second");
    assert_eq!(internal_doc.elements[3].kind, ElementKind::ListEnd);
}

#[test]
fn test_build_document_table() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::TableStart,
            WpdEvent::RowStart { header: true },
            WpdEvent::CellStart {
                column: 0,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("A".to_string()),
            WpdEvent::CellEnd,
            WpdEvent::CellStart {
                column: 1,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("B".to_string()),
            WpdEvent::CellEnd,
            WpdEvent::RowEnd,
            WpdEvent::RowStart { header: false },
            WpdEvent::CellStart {
                column: 0,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("1".to_string()),
            WpdEvent::CellEnd,
            WpdEvent::CellStart {
                column: 1,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("2".to_string()),
            WpdEvent::CellEnd,
            WpdEvent::RowEnd,
            WpdEvent::TableEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    use crate::types::internal::ElementKind;
    assert_eq!(internal_doc.elements.len(), 1);
    assert!(matches!(internal_doc.elements[0].kind, ElementKind::Table { .. }));
    assert_eq!(internal_doc.tables.len(), 1);
    assert_eq!(
        internal_doc.tables[0].cells,
        vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ]
    );
    let attrs = internal_doc.elements[0].attributes.as_ref().unwrap();
    assert_eq!(attrs.get("header_rows").unwrap(), "0");
}

#[test]
fn test_build_document_footnote() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::Text("See".to_string()),
            WpdEvent::NoteStart { endnote: false },
            WpdEvent::Text("A footnote body.".to_string()),
            WpdEvent::NoteEnd,
            WpdEvent::Text("here.".to_string()),
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    use crate::types::internal::ElementKind;

    let ref_elem = internal_doc
        .elements
        .iter()
        .find(|e| e.kind == ElementKind::FootnoteRef)
        .expect("expected a FootnoteRef element");
    assert_eq!(ref_elem.anchor.as_deref(), Some("fn1"));

    let def_elem = internal_doc
        .elements
        .iter()
        .find(|e| e.kind == ElementKind::FootnoteDefinition)
        .expect("expected a FootnoteDefinition element");
    assert_eq!(def_elem.text, "A footnote body.");
    assert_eq!(def_elem.anchor.as_deref(), Some("fn1"));

    let trailing = internal_doc
        .elements
        .iter()
        .find(|e| e.kind == ElementKind::Paragraph && e.text == "here.");
    assert!(trailing.is_some(), "text after NoteEnd should form its own paragraph");
}

const NOTE_BODY: &str = "The determination coefficient R2 equals 0.87; café.";

fn document_with_note(endnote: bool) -> InternalDocument {
    build_wordperfect_internal_document(&WpdDocument {
        events: vec![
            WpdEvent::Text("Before the note.".to_string()),
            WpdEvent::NoteStart { endnote },
            WpdEvent::Text(NOTE_BODY.to_string()),
            WpdEvent::NoteEnd,
            WpdEvent::Text("After the note.".to_string()),
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    })
}

fn assert_note_body_is_rendered_once(content: &str) {
    assert_eq!(
        content.matches(NOTE_BODY).count(),
        1,
        "the complete note body must occur exactly once: {content:?}"
    );
    assert_eq!(content.matches("Before the note.").count(), 1);
    assert_eq!(content.matches("After the note.").count(), 1);
}

fn assert_note_survives_final_rendering(output_format: crate::core::config::OutputFormat, endnote: bool) {
    let plain = matches!(output_format, crate::core::config::OutputFormat::Plain);
    let result = crate::extraction::derive::derive_extraction_result(document_with_note(endnote), false, output_format);
    let content = if plain {
        &result.content
    } else {
        result
            .formatted_content
            .as_deref()
            .expect("requested format must render")
    };
    assert_note_body_is_rendered_once(content);
}

#[test]
fn should_render_wordperfect_footnote_body_in_plain() {
    assert_note_survives_final_rendering(crate::core::config::OutputFormat::Plain, false);
}

#[test]
fn should_render_wordperfect_endnote_body_in_plain() {
    assert_note_survives_final_rendering(crate::core::config::OutputFormat::Plain, true);
}

#[test]
fn should_render_wordperfect_note_bodies_in_markdown() {
    for endnote in [false, true] {
        assert_note_survives_final_rendering(crate::core::config::OutputFormat::Markdown, endnote);
    }
}

#[test]
fn should_render_wordperfect_note_bodies_in_html() {
    for endnote in [false, true] {
        assert_note_survives_final_rendering(crate::core::config::OutputFormat::Html, endnote);
    }
}

#[test]
fn should_render_wordperfect_note_bodies_in_djot() {
    for endnote in [false, true] {
        assert_note_survives_final_rendering(crate::core::config::OutputFormat::Djot, endnote);
    }
}

#[test]
#[should_panic(expected = "the complete note body must occur exactly once")]
fn should_reject_final_plain_output_with_a_missing_note_body() {
    let mut document = document_with_note(false);
    document
        .elements
        .retain(|element| element.kind != crate::types::internal::ElementKind::FootnoteDefinition);
    let result =
        crate::extraction::derive::derive_extraction_result(document, false, crate::core::config::OutputFormat::Plain);
    assert_note_body_is_rendered_once(&result.content);
}

#[test]
fn test_build_document_header_and_footer_are_bracketed() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::HeaderStart,
            WpdEvent::Text("Running Header".to_string()),
            WpdEvent::HeaderEnd,
            WpdEvent::Text("Body text.".to_string()),
            WpdEvent::ParagraphEnd,
            WpdEvent::FooterStart,
            WpdEvent::Text("Running Footer".to_string()),
            WpdEvent::FooterEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);

    let header = internal_doc
        .elements
        .iter()
        .find(|e| e.text == "Running Header")
        .unwrap();
    assert_eq!(header.layer, ContentLayer::Header);

    let footer = internal_doc
        .elements
        .iter()
        .find(|e| e.text == "Running Footer")
        .unwrap();
    assert_eq!(footer.layer, ContentLayer::Footer);

    let body = internal_doc.elements.iter().find(|e| e.text == "Body text.").unwrap();
    assert_eq!(body.layer, ContentLayer::Body);
}

#[test]
fn test_build_document_aside_tagged_and_kept_separate() {
    let doc = WpdDocument {
        events: vec![
            WpdEvent::Text("Main text.".to_string()),
            WpdEvent::AsideStart {
                kind: "comment".to_string(),
            },
            WpdEvent::Text("A reviewer comment.".to_string()),
            WpdEvent::AsideEnd,
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);

    let aside = internal_doc
        .elements
        .iter()
        .find(|e| e.text == "A reviewer comment.")
        .expect("aside body pushed as its own element");
    assert_eq!(aside.attributes.as_ref().unwrap().get("aside_kind").unwrap(), "comment");

    let main = internal_doc
        .elements
        .iter()
        .find(|e| e.text == "Main text.")
        .expect("main text not spliced with aside content");
    assert_ne!(main.text, "Main text.A reviewer comment.");
}

#[test]
fn test_build_document_metadata() {
    let doc = WpdDocument {
        events: vec![],
        metadata: WpdMetadata {
            title: Some("My Doc".to_string()),
            author: Some("Jane Doe".to_string()),
            subject: Some("A subject".to_string()),
            keywords: Some("k1, k2".to_string()),
            raw: vec![],
        },
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    assert_eq!(internal_doc.metadata.title.as_deref(), Some("My Doc"));
    assert_eq!(internal_doc.metadata.authors, Some(vec!["Jane Doe".to_string()]));
    assert_eq!(internal_doc.metadata.subject.as_deref(), Some("A subject"));
    assert_eq!(internal_doc.metadata.keywords, Some(vec!["k1, k2".to_string()]));
}

#[test]
fn test_formatting_spanning_a_footnote_anchor_is_preserved() {
    // Regression: a bold run bracketing a footnote anchor must stay bold on
    // both the pre-anchor and post-anchor text (the note flush must not drop
    // the still-open span).
    let doc = WpdDocument {
        events: vec![
            WpdEvent::BoldStart,
            WpdEvent::Text("before".to_string()),
            WpdEvent::NoteStart { endnote: false },
            WpdEvent::Text("note body".to_string()),
            WpdEvent::NoteEnd,
            WpdEvent::Text("after".to_string()),
            WpdEvent::BoldEnd,
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    let before = internal_doc
        .elements
        .iter()
        .find(|e| e.text == "before")
        .expect("pre-anchor paragraph");
    assert_eq!(before.annotations.len(), 1);
    assert_eq!(before.annotations[0].kind, AnnotationKind::Bold);
    assert_eq!((before.annotations[0].start, before.annotations[0].end), (0, 6));

    let after = internal_doc
        .elements
        .iter()
        .find(|e| e.text == "after")
        .expect("post-anchor paragraph");
    assert_eq!(after.annotations.len(), 1, "formatting must continue after the note");
    assert_eq!(after.annotations[0].kind, AnnotationKind::Bold);
    assert_eq!((after.annotations[0].start, after.annotations[0].end), (0, 5));
}

#[test]
fn test_table_cell_inline_formatting_rendered_as_markdown() {
    // Regression: bold/italic/link inside a cell were dropped entirely.
    let doc = WpdDocument {
        events: vec![
            WpdEvent::TableStart,
            WpdEvent::RowStart { header: false },
            WpdEvent::CellStart {
                column: 0,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::BoldStart,
            WpdEvent::Text("Hi".to_string()),
            WpdEvent::BoldEnd,
            WpdEvent::CellEnd,
            WpdEvent::RowEnd,
            WpdEvent::TableEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    assert_eq!(internal_doc.tables.len(), 1);
    assert_eq!(internal_doc.tables[0].cells, vec![vec!["**Hi**".to_string()]]);
}

#[test]
fn test_whitespace_list_item_does_not_leak_into_next_element() {
    // Regression: a whitespace-only list item carrying a closed annotation
    // must not leave stale text/annotation on `main` that leaks forward.
    let doc = WpdDocument {
        events: vec![
            WpdEvent::ListItemStart {
                ordered: true,
                level: 1,
                counter: 1,
            },
            WpdEvent::BoldStart,
            WpdEvent::Text(" ".to_string()),
            WpdEvent::BoldEnd,
            WpdEvent::ListItemEnd,
            WpdEvent::Text("Hello".to_string()),
            WpdEvent::ParagraphEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    let hello = internal_doc
        .elements
        .iter()
        .find(|e| e.text.trim() == "Hello")
        .expect("Hello element");
    assert_eq!(hello.text, "Hello", "leading whitespace must not leak in");
    assert!(hello.annotations.is_empty(), "stray annotation must not leak in");
}

#[test]
fn test_nested_table_does_not_destroy_outer_table() {
    // Regression: an inner table nested in a cell clobbered the outer
    // TableBuilder; the outer table must survive and the inner text folds
    // into the enclosing cell.
    let doc = WpdDocument {
        events: vec![
            WpdEvent::TableStart,
            WpdEvent::RowStart { header: false },
            WpdEvent::CellStart {
                column: 0,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("outer".to_string()),
            WpdEvent::TableStart,
            WpdEvent::RowStart { header: false },
            WpdEvent::CellStart {
                column: 0,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("inner".to_string()),
            WpdEvent::CellEnd,
            WpdEvent::RowEnd,
            WpdEvent::TableEnd,
            WpdEvent::CellEnd,
            WpdEvent::RowEnd,
            WpdEvent::TableEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    assert_eq!(internal_doc.tables.len(), 1, "outer table must not be lost");
    assert!(internal_doc.tables[0].cells[0][0].contains("outer"));
}

#[test]
fn test_footnote_inside_table_cell_does_not_reorder_before_table() {
    // Regression: a note anchored in a cell pushed a standalone FootnoteRef
    // ahead of the deferred Table element. It must fold inline instead.
    use crate::types::internal::ElementKind;
    let doc = WpdDocument {
        events: vec![
            WpdEvent::TableStart,
            WpdEvent::RowStart { header: false },
            WpdEvent::CellStart {
                column: 0,
                col_span: 1,
                row_span: 1,
            },
            WpdEvent::Text("cell".to_string()),
            WpdEvent::NoteStart { endnote: false },
            WpdEvent::Text("note".to_string()),
            WpdEvent::NoteEnd,
            WpdEvent::CellEnd,
            WpdEvent::RowEnd,
            WpdEvent::TableEnd,
        ],
        metadata: WpdMetadata::default(),
    };

    let internal_doc = build_wordperfect_internal_document(&doc);
    assert_eq!(internal_doc.tables.len(), 1);
    assert!(
        !internal_doc.elements.iter().any(|e| e.kind == ElementKind::FootnoteRef),
        "nested note must not emit a standalone FootnoteRef"
    );
    assert!(
        internal_doc.tables[0].cells[0][0].contains("[1]"),
        "inline marker folded into cell"
    );
}

#[tokio::test]
async fn test_extract_content_empty_document_errors() {
    use crate::core::config::ExtractionConfig;

    let extractor = WordPerfectExtractor::new();
    let config = ExtractionConfig::default();
    // Not a valid WordPerfect document; extract_document should fail fast
    // rather than panicking on malformed/empty input.
    let result = extractor
        .extract_content(b"not a wordperfect document", "application/vnd.wordperfect", &config)
        .await;
    assert!(result.is_err());
}
