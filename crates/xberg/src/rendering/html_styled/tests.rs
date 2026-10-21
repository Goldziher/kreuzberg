use super::*;
use crate::core::config::html_output::HtmlOutputConfig;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

fn make_doc(kind: ElementKind, text: &str) -> InternalDocument {
    let mut doc = InternalDocument::new("text/plain");
    doc.push_element(InternalElement::text(kind, text, 0));
    doc
}

fn render(config: HtmlOutputConfig, doc: &InternalDocument) -> String {
    StyledHtmlRenderer::new(config).unwrap().render(doc).unwrap()
}

/// A definition on the footnote layer (where `push_footnote_definition` puts it, and
/// where DOCX and ODT always moved theirs) is rendered once, next to its reference.
#[test]
fn footnote_layer_definition_is_rendered_once_with_its_reference() {
    let mut b = crate::types::internal_builder::InternalDocumentBuilder::new("test");
    b.push_paragraph("Body", vec![], None, None);
    b.push_footnote_ref("1", "fn1", None);
    b.push_footnote_definition("FOOT", "fn1", None);
    let doc = b.build();
    let out = render(HtmlOutputConfig::default(), &doc);
    assert!(out.contains(r##"href="#fn-fn1""##), "reference missing: {out}");
    assert_eq!(
        out.matches(r#"id="fn-fn1""#).count(),
        1,
        "definition not rendered once: {out}"
    );
    assert!(out.contains(">FOOT</aside>"), "definition text missing: {out}");
}

#[test]
fn paragraph_emits_kb_p() {
    let doc = make_doc(ElementKind::Paragraph, "Hello");
    let out = render(HtmlOutputConfig::default(), &doc);
    assert!(out.contains(r#"class="kb-p""#), "missing kb-p: {out}");
    assert!(out.contains("Hello"), "text missing: {out}");
}

#[test]
fn heading_emits_level_classes() {
    let doc = make_doc(ElementKind::Heading { level: 2 }, "Section");
    let out = render(HtmlOutputConfig::default(), &doc);
    assert!(out.contains(r#"class="kb-h kb-h2""#), "missing heading classes: {out}");
    assert!(out.contains("<h2"), "wrong tag: {out}");
}

#[test]
fn wrapper_always_present() {
    let doc = InternalDocument::new("text/plain");
    let out = render(HtmlOutputConfig::default(), &doc);
    assert!(out.contains(r#"class="kb-doc""#));
    assert!(out.contains(r#"class="kb-content""#));
}

#[test]
fn embed_css_true_includes_style_block() {
    let doc = InternalDocument::new("text/plain");
    let out = render(
        HtmlOutputConfig {
            embed_css: true,
            theme: HtmlTheme::Default,
            ..Default::default()
        },
        &doc,
    );
    assert!(out.contains("<style>"), "style block missing: {out}");
}

#[test]
fn embed_css_false_omits_style_block() {
    let doc = InternalDocument::new("text/plain");
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(!out.contains("<style>"), "unexpected style block: {out}");
}

#[test]
fn user_css_appears_in_style_block() {
    let doc = InternalDocument::new("text/plain");
    let cfg = HtmlOutputConfig {
        css: Some(".kb-p { color: red; }".to_string()),
        embed_css: true,
        ..Default::default()
    };
    let out = render(cfg, &doc);
    assert!(out.contains(".kb-p { color: red; }"), "user css missing: {out}");
}

#[test]
fn custom_prefix_replaces_kb() {
    let doc = make_doc(ElementKind::Paragraph, "test");
    let cfg = HtmlOutputConfig {
        class_prefix: "my-".to_string(),
        embed_css: false,
        ..Default::default()
    };
    let out = render(cfg, &doc);
    assert!(out.contains(r#"class="my-doc""#), "custom prefix missing: {out}");
    assert!(out.contains(r#"class="my-p""#), "custom prefix on p missing: {out}");
    assert!(!out.contains("kb-"), "old prefix still present: {out}");
}

#[test]
fn formula_emits_dollar_math_not_code_block() {
    let doc = make_doc(ElementKind::Formula, "x^2 + y^2 = z^2");
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(
        out.contains(r#"<div class="kb-formula kb-math" data-math-style="display">$$x^2 + y^2 = z^2$$</div>"#),
        "unexpected formula markup: {out}"
    );
    assert!(
        !out.contains("<pre"),
        "formula should not render as a code block: {out}"
    );
    assert!(
        !out.contains("<code"),
        "formula should not render as a code block: {out}"
    );
}

#[test]
fn formula_escapes_html_special_characters() {
    let doc = make_doc(ElementKind::Formula, "a < b & b > c");
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(
        out.contains("$$a &lt; b &amp; b &gt; c$$"),
        "escaped formula missing: {out}"
    );
    assert!(!out.contains("a < b & b > c"), "raw unescaped formula leaked: {out}");
}

#[test]
fn text_content_is_escaped() {
    let doc = make_doc(ElementKind::Paragraph, "<script>alert(1)</script>");
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(!out.contains("<script>"), "unescaped script tag: {out}");
    assert!(out.contains("&lt;script&gt;"), "escaped form missing: {out}");
}

#[test]
fn list_items_wrapped_in_ul() {
    let mut doc = InternalDocument::new("text/plain");
    doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
    doc.push_element(InternalElement::text(
        ElementKind::ListItem { ordered: false },
        "Item",
        1,
    ));
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(out.contains(r#"class="kb-list kb-ul""#), "ul missing: {out}");
    assert!(out.contains(r#"class="kb-li""#), "li missing: {out}");
    assert!(out.contains("</ul>"), "closing ul missing: {out}");
}

#[test]
fn table_emits_thead_tbody() {
    use crate::types::tables::Table;
    let mut doc = InternalDocument::new("text/plain");
    let table = Table {
        cells: vec![
            vec!["H1".to_string(), "H2".to_string()],
            vec!["A".to_string(), "B".to_string()],
        ],
        markdown: String::new(),
        page_number: 1,
        bounding_box: None,
        ..Default::default()
    };
    let idx = doc.push_table(table);
    doc.push_element(InternalElement::text(ElementKind::Table { table_index: idx }, "", 0));
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(out.contains(r#"class="kb-table""#), "table class missing: {out}");
    assert!(out.contains(r#"class="kb-thead""#), "thead missing: {out}");
    assert!(out.contains(r#"class="kb-tbody""#), "tbody missing: {out}");
    assert!(out.contains("<th"), "th missing: {out}");
    assert!(out.contains("<td"), "td missing: {out}");
}

fn make_image_doc(ocr_text_only: bool, append_ocr_text: bool) -> InternalDocument {
    use crate::types::ExtractedImage;
    use std::borrow::Cow;
    let mut doc = InternalDocument::new("test");
    doc.ocr_text_only = ocr_text_only;
    doc.append_ocr_text = append_ocr_text;
    let idx = doc.push_image(ExtractedImage {
        data: bytes::Bytes::from_static(b"\x89PNG"),
        format: Cow::Borrowed("png"),
        image_index: 0,
        page_number: None,
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: None,
        ocr_result: Some(Box::new(crate::types::ExtractedDocument {
            content: "OCR caption".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        })),
        bounding_box: None,
        source_path: None,
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    });
    doc.push_element(crate::types::internal::InternalElement::text(
        crate::types::internal::ElementKind::Image { image_index: idx },
        "alt text",
        0,
    ));
    doc
}

#[test]
fn image_no_flags_emits_figure_without_figcaption() {
    let doc = make_image_doc(false, false);
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(out.contains("<figure"), "expected <figure>: {out}");
    assert!(
        !out.contains("<figcaption"),
        "unexpected figcaption when both flags false: {out}"
    );
    assert!(
        !out.contains("OCR caption"),
        "unexpected OCR text when both flags false: {out}"
    );
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[test]
fn image_internal_suppression_preserves_figure_without_nested_ocr() {
    let mut doc = make_image_doc(false, true);
    doc.elements[0].suppress_image_ocr_rendering();

    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );

    assert!(out.contains("<figure"));
    assert!(!out.contains("OCR caption"));
    assert!(doc.images[0].ocr_result.is_some());
}

#[test]
fn image_append_ocr_text_emits_figcaption() {
    let doc = make_image_doc(false, true);
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(out.contains("<figure"), "expected <figure>: {out}");
    assert!(
        out.contains("<figcaption"),
        "expected figcaption with append_ocr_text=true: {out}"
    );
    assert!(out.contains("OCR caption"), "expected OCR text in figcaption: {out}");
}

#[test]
fn image_ocr_text_only_emits_paragraph_not_figure() {
    let doc = make_image_doc(true, false);
    let out = render(
        HtmlOutputConfig {
            embed_css: false,
            ..Default::default()
        },
        &doc,
    );
    assert!(
        !out.contains("<figure"),
        "unexpected <figure> when ocr_text_only=true: {out}"
    );
    assert!(out.contains("OCR caption"), "expected OCR text as paragraph: {out}");
    assert!(out.contains("kb-ocr-text"), "expected ocr-text class: {out}");
}
