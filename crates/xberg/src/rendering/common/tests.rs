use super::*;
use crate::types::document_structure::{AnnotationKind, TextAnnotation};

#[test]
fn test_finalize_output_trims_and_adds_newline() {
    assert_eq!(finalize_output("Hello\n\n\n".to_string()), "Hello\n");
}

#[test]
fn test_finalize_output_empty_input() {
    assert_eq!(finalize_output("".to_string()), "");
}

#[test]
fn test_finalize_output_whitespace_only() {
    assert_eq!(finalize_output("   \n\n  ".to_string()), "");
}

#[test]
fn test_ensure_trailing_newline_adds_when_missing() {
    let mut s = "hello".to_string();
    ensure_trailing_newline(&mut s);
    assert_eq!(s, "hello\n");
}

#[test]
fn test_ensure_trailing_newline_no_double() {
    let mut s = "hello\n".to_string();
    ensure_trailing_newline(&mut s);
    assert_eq!(s, "hello\n");
}

#[test]
fn test_blockquote_prefix_depth_zero() {
    let result = apply_blockquote_prefix("hello\n", 0);
    assert_eq!(result.as_ref(), "hello\n");
    assert!(matches!(result, Cow::Borrowed(_)));
}

#[test]
fn test_blockquote_prefix_depth_one() {
    let result = apply_blockquote_prefix("hello\n", 1);
    assert_eq!(result.as_ref(), "> hello\n");
}

#[test]
fn test_blockquote_prefix_depth_two() {
    let result = apply_blockquote_prefix("hello\n", 2);
    assert_eq!(result.as_ref(), "> > hello\n");
}

#[test]
fn test_blockquote_prefix_multiline() {
    let result = apply_blockquote_prefix("line1\nline2\n", 1);
    assert_eq!(result.as_ref(), "> line1\n> line2\n");
}

#[test]
fn test_parse_metadata_entries_basic() {
    let entries = parse_metadata_entries("Author: Alice\nDate: 2024-01-01");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0], ("Author", "Alice"));
    assert_eq!(entries[1], ("Date", "2024-01-01"));
}

#[test]
fn test_parse_metadata_entries_empty() {
    let entries = parse_metadata_entries("");
    assert!(entries.is_empty());
}

#[test]
fn test_parse_metadata_entries_no_colon() {
    let entries = parse_metadata_entries("no colon here");
    assert!(entries.is_empty());
}

#[test]
fn test_parse_metadata_entries_empty_key() {
    let entries = parse_metadata_entries(": value");
    assert!(entries.is_empty());
}

#[test]
fn test_render_annotated_text_no_annotations() {
    let result = render_annotated_text("Hello", &[], |span, _| span.to_string());
    assert_eq!(result, "Hello");
}

#[test]
fn test_render_annotated_text_single_annotation() {
    let ann = vec![TextAnnotation {
        start: 0,
        end: 5,
        kind: AnnotationKind::Bold,
    }];
    let result = render_annotated_text("Hello world", &ann, |span, kind| match kind {
        AnnotationKind::Bold => format!("[B:{}]", span),
        _ => span.to_string(),
    });
    assert_eq!(result, "[B:Hello] world");
}

#[test]
fn test_render_annotated_text_multiple_non_overlapping() {
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
    let result = render_annotated_text("Hello world", &ann, |span, kind| match kind {
        AnnotationKind::Bold => format!("[B:{}]", span),
        AnnotationKind::Italic => format!("[I:{}]", span),
        _ => span.to_string(),
    });
    assert_eq!(result, "[B:Hello] [I:world]");
}

#[test]
fn test_render_annotated_text_overlapping_skips_inner() {
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
    let result = render_annotated_text("Hello world", &ann, |span, kind| match kind {
        AnnotationKind::Bold => format!("[B:{}]", span),
        AnnotationKind::Italic => format!("[I:{}]", span),
        _ => span.to_string(),
    });
    assert_eq!(result, "[B:Hello world]");
}

/// `TextAnnotation::start`/`end` are byte offsets that can be produced by any
/// extractor, not just the char-boundary-safe path in `pdf::structure::assembly`.
/// An annotation landing mid-codepoint made `&text[start..end]` panic with
/// "byte index 1 is not a char boundary".
///
/// The span here is deliberately NON-EMPTY after clamping. An empty one
/// (`start: 1, end: 1`) proves nothing: the accompanying `start >= end` skip
/// discards it before any slicing happens, so that case still passes with the
/// boundary clamp removed. `é` occupies bytes `0..2`, so `start: 1` cuts inside
/// it while `end: 3` is a real boundary -- reaching the slice and panicking
/// unless `start` is rounded up to 2.
#[test]
fn render_annotated_text_clamps_mid_codepoint_offset_instead_of_panicking() {
    let text = "éab";
    let ann = vec![TextAnnotation {
        start: 1,
        end: 3,
        kind: AnnotationKind::Bold,
    }];
    let result = render_annotated_text(text, &ann, |span, kind| match kind {
        AnnotationKind::Bold => format!("[B:{}]", span),
        _ => span.to_string(),
    });
    assert_eq!(
        result, "é[B:a]b",
        "start must round up to the char boundary at 2, bolding only the complete chars"
    );
}

/// The empty-after-clamping case, kept separately so each guard has its own test:
/// `start: 1, end: 1` inside `é` collapses to nothing and must be dropped.
#[test]
fn render_annotated_text_drops_an_annotation_that_clamps_to_empty() {
    let text = "é world";
    let ann = vec![TextAnnotation {
        start: 1,
        end: 1,
        kind: AnnotationKind::Bold,
    }];
    let result = render_annotated_text(text, &ann, |span, kind| match kind {
        AnnotationKind::Bold => format!("[B:{}]", span),
        _ => span.to_string(),
    });
    assert_eq!(result, text, "an annotation with no content must be dropped, not panic");
}

#[test]
fn test_render_state_blockquote_depth() {
    let mut state = RenderState::default();
    assert_eq!(state.blockquote_depth(), 0);
    state.push_container(NestingKind::BlockQuote, 0);
    assert_eq!(state.blockquote_depth(), 1);
    state.push_container(NestingKind::BlockQuote, 1);
    assert_eq!(state.blockquote_depth(), 2);
    state.pop_container(&NestingKind::BlockQuote);
    assert_eq!(state.blockquote_depth(), 1);
}

#[test]
fn test_render_state_list_depth() {
    let mut state = RenderState::default();
    assert_eq!(state.list_depth(), 0);
    state.push_container(
        NestingKind::List {
            ordered: false,
            item_count: 0,
        },
        0,
    );
    assert_eq!(state.list_depth(), 1);
    state.push_container(
        NestingKind::List {
            ordered: true,
            item_count: 0,
        },
        1,
    );
    assert_eq!(state.list_depth(), 2);
}

#[test]
fn test_render_state_next_list_number() {
    let mut state = RenderState::default();
    state.push_container(
        NestingKind::List {
            ordered: true,
            item_count: 0,
        },
        0,
    );
    assert_eq!(state.next_list_number(), 1);
    assert_eq!(state.next_list_number(), 2);
    assert_eq!(state.next_list_number(), 3);
}

#[test]
fn test_render_table_markdown_basic() {
    let cells = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    let out = render_table_markdown(&cells);
    assert!(out.contains("| A | B |"), "got: {}", out);
    assert!(out.contains("| --- | --- |"), "got: {}", out);
    assert!(out.contains("| 1 | 2 |"), "got: {}", out);
}

#[test]
fn test_render_table_markdown_empty() {
    let out = render_table_markdown(&[]);
    assert_eq!(out, "");
}

#[test]
fn test_render_table_markdown_escapes_pipe() {
    let cells = vec![vec!["A|B".to_string()], vec!["C|D".to_string()]];
    let out = render_table_markdown(&cells);
    assert!(out.contains("A\\|B"), "pipe should be escaped, got: {}", out);
}

/// xberg-io/xberg#221: the grid is sized from the widest row, not the header.
#[test]
fn should_size_the_grid_from_the_widest_row() {
    let cells = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
    ];
    let out = render_table_markdown(&cells);
    assert_eq!(out, "| A | B |  |\n| --- | --- | --- |\n| 1 | 2 | 3 |\n");
}

/// xberg-io/xberg#163: a line break inside a cell must not end the row.
#[test]
fn should_replace_cell_line_breaks_with_a_break_tag() {
    let cells = vec![
        vec!["H".to_string()],
        vec!["x\ny".to_string()],
        vec!["p\r\nq".to_string()],
    ];
    let out = render_table_markdown(&cells);
    assert_eq!(out, "| H |\n| --- |\n| x<br>y |\n| p<br>q |\n");
    assert_eq!(out.lines().count(), 4, "embedded newlines must not add rows: {out}");
}

#[test]
fn test_render_table_plain_basic() {
    let cells = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    let out = render_table_plain(&cells);
    assert!(out.contains("A B"), "got: {}", out);
    assert!(out.contains("1 2"), "got: {}", out);
}

#[test]
fn test_render_table_plain_empty() {
    let out = render_table_plain(&[]);
    assert_eq!(out, "");
}

#[test]
fn test_footnote_collector_basic() {
    use crate::types::internal_builder::InternalDocumentBuilder;
    let mut b = InternalDocumentBuilder::new("test");
    b.push_footnote_ref("1", "fn1", None);
    let def = b.push_footnote_definition("Note text.", "fn1", None);
    b.set_layer(def, ContentLayer::Footnote);
    let doc = b.build();

    let collector = FootnoteCollector::new(&doc);
    assert_eq!(collector.ref_number(0), Some(1));
    let defs = collector.definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].text, "Note text.");
    assert_eq!(defs[0].number, 1);
}

#[test]
fn test_footnote_collector_multiple() {
    use crate::types::internal_builder::InternalDocumentBuilder;
    let mut b = InternalDocumentBuilder::new("test");
    b.push_footnote_ref("a", "fn1", None);
    b.push_footnote_ref("b", "fn2", None);
    let d1 = b.push_footnote_definition("First.", "fn1", None);
    let d2 = b.push_footnote_definition("Second.", "fn2", None);
    b.set_layer(d1, ContentLayer::Footnote);
    b.set_layer(d2, ContentLayer::Footnote);
    let doc = b.build();

    let collector = FootnoteCollector::new(&doc);
    assert_eq!(collector.ref_number(0), Some(1));
    assert_eq!(collector.ref_number(1), Some(2));
    let defs = collector.definitions();
    assert_eq!(defs.len(), 2);
    assert_eq!(defs[0].number, 1);
    assert_eq!(defs[1].number, 2);
}

/// Regression for #68: a footnote definition that no `FootnoteRef` points at
/// is still authored content and must be emitted, numbered after the
/// referenced ones. Before the fix `definitions` was filled only from inside
/// the reference loop, so "Orphaned." never reached a renderer.
#[test]
fn footnote_collector_emits_unreferenced_definitions() {
    use crate::types::internal_builder::InternalDocumentBuilder;
    let mut b = InternalDocumentBuilder::new("test");
    b.push_footnote_ref("a", "fn1", None);
    let d1 = b.push_footnote_definition("Referenced.", "fn1", None);
    let d2 = b.push_footnote_definition("Orphaned.", "fn2", None);
    b.set_layer(d1, ContentLayer::Footnote);
    b.set_layer(d2, ContentLayer::Footnote);
    let doc = b.build();

    let collector = FootnoteCollector::new(&doc);
    assert_eq!(collector.ref_number(0), Some(1));
    let defs = collector.definitions();
    assert_eq!(defs.len(), 2, "the unreferenced definition must still be emitted");
    assert_eq!(defs[0].text, "Referenced.");
    assert_eq!(defs[0].number, 1);
    assert_eq!(defs[1].text, "Orphaned.");
    assert_eq!(defs[1].number, 2);
}

/// A document with footnote definitions but no references at all still emits
/// every definition, numbered from 1 in document order.
#[test]
fn footnote_collector_emits_definitions_when_no_references_exist() {
    use crate::types::internal_builder::InternalDocumentBuilder;
    let mut b = InternalDocumentBuilder::new("test");
    let d1 = b.push_footnote_definition("First orphan.", "fn1", None);
    let d2 = b.push_footnote_definition("Second orphan.", "fn2", None);
    b.set_layer(d1, ContentLayer::Footnote);
    b.set_layer(d2, ContentLayer::Footnote);
    let doc = b.build();

    let collector = FootnoteCollector::new(&doc);
    let defs = collector.definitions();
    assert_eq!(defs.len(), 2);
    assert_eq!(defs[0].text, "First orphan.");
    assert_eq!(defs[0].number, 1);
    assert_eq!(defs[1].text, "Second orphan.");
    assert_eq!(defs[1].number, 2);
}

/// Two definitions sharing one anchor must not be double-numbered by the
/// orphan tail pass.
#[test]
fn footnote_collector_does_not_duplicate_shared_anchor_definitions() {
    use crate::types::internal_builder::InternalDocumentBuilder;
    let mut b = InternalDocumentBuilder::new("test");
    let d1 = b.push_footnote_definition("Only once.", "fn1", None);
    let d2 = b.push_footnote_definition("Duplicate anchor.", "fn1", None);
    b.set_layer(d1, ContentLayer::Footnote);
    b.set_layer(d2, ContentLayer::Footnote);
    let doc = b.build();

    let collector = FootnoteCollector::new(&doc);
    let defs = collector.definitions();
    assert_eq!(defs.len(), 1, "a repeated anchor must be numbered once");
    assert_eq!(defs[0].text, "Only once.");
    assert_eq!(defs[0].number, 1);
}

#[test]
fn test_footnote_collector_no_footnotes() {
    use crate::types::internal_builder::InternalDocumentBuilder;
    let mut b = InternalDocumentBuilder::new("test");
    b.push_paragraph("No footnotes here", vec![], None, None);
    let doc = b.build();

    let collector = FootnoteCollector::new(&doc);
    assert!(collector.definitions().is_empty());
    assert_eq!(collector.ref_number(0), None);
}

#[test]
fn test_normalize_inline_text_collapses_spaces() {
    assert_eq!(normalize_inline_text("Hello   world"), "Hello world");
}

#[test]
fn test_normalize_inline_text_newlines_to_spaces() {
    assert_eq!(normalize_inline_text("Hello\nworld"), "Hello world");
}

#[test]
fn test_normalize_inline_text_mixed_whitespace() {
    assert_eq!(normalize_inline_text("Hello \n  world"), "Hello world");
}

#[test]
fn test_normalize_inline_text_strips_control_chars() {
    assert_eq!(normalize_inline_text("Hello\x02world"), "Helloworld");
}

#[test]
fn test_normalize_inline_text_preserves_tabs() {
    assert_eq!(normalize_inline_text("Hello\tworld"), "Hello\tworld");
}

#[test]
fn test_normalize_inline_text_empty() {
    assert_eq!(normalize_inline_text(""), "");
}

#[test]
fn test_normalize_inline_text_no_change() {
    let result = normalize_inline_text("Hello world");
    assert!(matches!(result, Cow::Borrowed(_)), "should not allocate when unchanged");
    assert_eq!(result, "Hello world");
}

#[test]
fn test_normalize_inline_text_collapses_spaces_allocates() {
    let result = normalize_inline_text("Hello   world");
    assert!(
        matches!(result, Cow::Owned(_)),
        "should allocate when spaces are collapsed"
    );
    assert_eq!(result, "Hello world");
}

#[test]
fn test_annotation_type_label_highlight() {
    assert_eq!(
        annotation_type_label(crate::types::annotations::PdfAnnotationType::Highlight),
        "Highlight"
    );
}

#[test]
fn test_annotation_type_label_previously_collapsed_variant() {
    assert_eq!(
        annotation_type_label(crate::types::annotations::PdfAnnotationType::Squiggly),
        "Squiggly"
    );
    assert_eq!(
        annotation_type_label(crate::types::annotations::PdfAnnotationType::FileAttachment),
        "FileAttachment"
    );
}

fn make_annotation(content: Option<&str>, marked_text: Option<&str>) -> crate::types::annotations::PdfAnnotation {
    crate::types::annotations::PdfAnnotation {
        annotation_type: crate::types::annotations::PdfAnnotationType::Highlight,
        content: content.map(str::to_string),
        page_number: 1,
        bounding_box: None,
        author: None,
        modified: None,
        color: None,
        subject: None,
        quad_points: None,
        marked_text: marked_text.map(str::to_string),
    }
}

#[test]
fn test_annotation_display_text_prefers_marked_text() {
    let annotation = make_annotation(Some("a comment"), Some("the highlighted words"));
    assert_eq!(annotation_display_text(&annotation), Some("the highlighted words"));
}

#[test]
fn test_annotation_display_text_falls_back_to_content() {
    let annotation = make_annotation(Some("a comment"), None);
    assert_eq!(annotation_display_text(&annotation), Some("a comment"));
}

#[test]
fn test_annotation_display_text_none_when_both_absent() {
    let annotation = make_annotation(None, None);
    assert_eq!(annotation_display_text(&annotation), None);
}

#[test]
fn test_escape_html_text_no_special_chars_returns_borrowed() {
    let result = escape_html_text("plain text");
    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(result, "plain text");
}

#[test]
fn test_escape_html_text_escapes_ampersand_and_angle_brackets() {
    let result = escape_html_text("a < b & c > d");
    assert_eq!(result, "a &lt; b &amp; c &gt; d");
}

#[test]
fn should_validate_issue_2123_math_namespace_and_delimiters() {
    assert_eq!(
        annotated_math(r"$\alpha$", &AnnotationKind::math(false)),
        Some((r"\alpha", false))
    );
    assert_eq!(
        annotated_math("$$a  b$$", &AnnotationKind::math(true)),
        Some(("a  b", true))
    );
    for text in ["$", "$$", "$$x$$", "x$", "$x", "ordinary"] {
        assert_eq!(annotated_math(text, &AnnotationKind::math(false)), None, "{text}");
    }
    for text in ["$$", "$$$$", "$x$", "$$x$"] {
        assert_eq!(annotated_math(text, &AnnotationKind::math(true)), None, "{text}");
    }
    assert_eq!(
        annotated_math(
            "$x$",
            &AnnotationKind::Custom {
                name: "other.math".into(),
                value: Some("inline".into())
            }
        ),
        None
    );
}

#[test]
fn should_bound_issue_2123_same_range_annotation_nesting() {
    let text = "$<x>$";
    for count in [64, 4096] {
        let mut annotations = vec![
            TextAnnotation {
                start: 0,
                end: text.len() as u32,
                kind: AnnotationKind::Bold
            };
            count
        ];
        annotations.push(TextAnnotation {
            start: 0,
            end: text.len() as u32,
            kind: AnnotationKind::math(false),
        });
        let output = render_nested_annotated_text(
            text,
            &annotations,
            &|span, kind| annotated_math(span, kind).map(|_| "<math>x</math>".to_string()),
            &|inner, _| format!("<b>{inner}</b>"),
            &|span| escape_html_text(span).into_owned(),
        );
        assert_eq!(output.matches("<b>").count(), 32);
        assert!(output.contains("$&lt;x&gt;$"));
        assert!(!output.contains("<math>"));
    }
}

#[test]
fn should_reject_issue_2123_unmatched_trailing_dollars() {
    assert_eq!(annotated_math("$x$$", &AnnotationKind::math(false)), None);
    assert_eq!(annotated_math("$$x$$$", &AnnotationKind::math(true)), None);
    assert_eq!(
        annotated_math(r"$x\$$", &AnnotationKind::math(false)),
        Some((r"x\$", false))
    );
    assert_eq!(annotated_math(r"$x\\$$", &AnnotationKind::math(false)), None);
}

#[test]
fn test_holds_html_tag() {
    for tex in [
        "<img src=x>",
        "<b>",
        "x </script >",
        "<!-- c -->",
        "<?php",
        "<!DOCTYPE html>",
        "<![CDATA[x]]>",
        "</script>",
        "<?x?>",
        "<javascript:alert(1)>",
        "<http://e.com>",
        "<a@b.c>",
    ] {
        assert!(holds_html_tag(tex.as_bytes()), "{tex}");
    }
    for tex in ["a<b", "0<F_{t}/F_{r}<0.35", "0<x/y<1", r"x < y \le z", "<x,y>", "p<.05"] {
        assert!(!holds_html_tag(tex.as_bytes()), "{tex}");
    }
}

#[test]
fn test_holds_html_tag_reads_math_shaped_like_an_open_tag_as_one() {
    for tex in ["<x>", "A<B> C", "f<g>"] {
        assert!(holds_html_tag(tex.as_bytes()), "{tex}");
    }
}
