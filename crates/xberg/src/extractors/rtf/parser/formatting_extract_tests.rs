use super::{extract_rtf_formatting, spans_to_annotations};
use crate::types::document_structure::AnnotationKind;

type Span = (usize, usize, bool, bool, bool, bool, u16);

fn spans(content: &str) -> Vec<Span> {
    extract_rtf_formatting(content)
        .spans
        .iter()
        .map(|span| {
            (
                span.start,
                span.end,
                span.bold,
                span.italic,
                span.underline,
                span.strikethrough,
                span.color_index,
            )
        })
        .collect()
}

#[test]
fn should_restore_nested_formats_at_unicode_byte_offsets() {
    assert_eq!(
        spans(r"{\rtf1 A{\b B{\i \u233?}}C}"),
        vec![
            (0, 1, false, false, false, false, 0),
            (1, 2, true, false, false, false, 0),
            (2, 4, true, true, false, false, 0),
        ]
    );
}

#[test]
fn should_restore_codepage_and_font_scopes_around_skipped_destinations() {
    assert_eq!(
        spans(concat!(
            r"{\rtf1\ansi\ansicpg1252\deff0",
            r"{\fonttbl{\f0\fcharset0 Latin;}{\f1\fcharset128 Japanese;}}",
            r"{\*\unknown hidden}{\b \'e9{\f1 \'82\'a0}\'e9}Z}"
        )),
        vec![(0, 8, true, false, false, false, 0),]
    );
}

#[test]
fn should_capture_field_result_offsets_and_header_footer_text() {
    let formatting = extract_rtf_formatting(concat!(
        r"{\rtf1{\header H\u233?}{\footer F}A",
        r#"{\field{\*\fldinst HYPERLINK "https://example.test"}{\fldrslt \b Link}}Z}"#
    ));
    assert_eq!(formatting.header_text.as_deref(), Some("Hé"));
    assert_eq!(formatting.footer_text.as_deref(), Some("F"));
    assert_eq!(formatting.hyperlinks, vec![(6, 10, "https://example.test".into())]);
    assert_eq!(
        formatting
            .spans
            .iter()
            .map(|span| (span.start, span.end, span.bold))
            .collect::<Vec<_>>(),
        vec![(0, 6, false), (6, 10, true)]
    );
}

#[test]
fn should_track_plain_reset_and_all_format_attributes() {
    assert_eq!(
        spans(r"{\rtf1\b\i\ul\strike\cf2 X\plain Y\b0\ulnone Z}"),
        vec![(0, 1, true, true, true, true, 2),]
    );
}

#[test]
fn should_preserve_literal_escape_and_control_character_byte_lengths() {
    assert_eq!(
        spans(r"{\rtf1\b \{\}\\\bullet\lquote\rquote\ldblquote\rdblquote\endash\emdash\tab\line\u-10179?\b0 Z}"),
        vec![(0, 26, true, false, false, false, 0)]
    );
}

#[test]
fn should_clip_annotations_and_preserve_attribute_and_link_order() {
    let formatting = extract_rtf_formatting(concat!(
        r"{\rtf1{\colortbl;\red255\green0\blue0;}",
        r#"{\field{\*\fldinst HYPERLINK "https://example.test"}"#,
        r"{\fldrslt \b\i\ul\strike\cf1 ABCD}}}"
    ));
    let annotations = spans_to_annotations(1, 3, &formatting);
    assert_eq!(
        annotations
            .into_iter()
            .map(|annotation| (annotation.start, annotation.end, annotation.kind))
            .collect::<Vec<_>>(),
        vec![
            (0, 2, AnnotationKind::Bold),
            (0, 2, AnnotationKind::Italic),
            (0, 2, AnnotationKind::Underline),
            (0, 2, AnnotationKind::Strikethrough),
            (
                0,
                2,
                AnnotationKind::Color {
                    value: "#ff0000".into()
                }
            ),
            (
                0,
                2,
                AnnotationKind::Link {
                    url: "https://example.test".into(),
                    title: None
                }
            ),
        ]
    );
    assert_eq!(spans_to_annotations(4, 8, &formatting).len(), 0);
}
