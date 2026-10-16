use super::*;

#[test]
fn should_render_issue_2123_nested_math_with_escaped_html() {
    let text = r"$\alpha  < b$";
    let mut builder = crate::types::internal_builder::InternalDocumentBuilder::new("test");
    builder.push_paragraph(
        text,
        vec![
            crate::types::TextAnnotation {
                start: 0,
                end: text.len() as u32,
                kind: AnnotationKind::math(false),
            },
            crate::types::TextAnnotation {
                start: 0,
                end: text.len() as u32,
                kind: AnnotationKind::Bold,
            },
            crate::types::TextAnnotation {
                start: 0,
                end: text.len() as u32,
                kind: AnnotationKind::Link {
                    url: "https://example.com".into(),
                    title: None,
                },
            },
        ],
        None,
        None,
    );
    let doc = builder.build();
    assert_eq!(
        render_inline(&doc, &doc.elements[0], "kb-"),
        r#"<a class="kb-link" href="https:&#x2f;&#x2f;example.com"><strong><span class="kb-math" data-math-style="inline">$\alpha  &lt; b$</span></strong></a>"#
    );
}
