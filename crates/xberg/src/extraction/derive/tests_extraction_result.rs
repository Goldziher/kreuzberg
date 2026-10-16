use super::test_support::make_doc;
use super::*;
#[cfg(any(feature = "pdf", feature = "ocr"))]
use crate::types::internal::InternalDocument;
use crate::types::internal::{ElementKind, InternalElement};

#[test]
fn test_derive_extraction_result_basic() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.content, "Hello world.");
    assert_eq!(result.mime_type, "text/markdown");
    assert!(result.document.is_none());
}

#[test]
fn issue_2128_should_locate_all_ten_ocr_pages_in_final_plain_content() {
    assert_final_ocr_page_boundaries("");
}

#[test]
fn issue_2128_should_locate_utf8_ocr_pages_using_byte_offsets() {
    assert_final_ocr_page_boundaries(" — café 東京 🦀");
}

#[test]
fn issue_2128_should_preserve_boundaries_when_page_text_cannot_locate_them() {
    for page_text in [None, Some(""), Some("unmatched page text")] {
        let mut doc = make_doc("pdf");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "actual rendered text", 0));
        if let Some(page_text) = page_text {
            let mut page_doc = make_doc("pdf");
            page_doc.push_element(InternalElement::text(ElementKind::Paragraph, page_text, 0).with_page(1));
            doc.prebuilt_pages = build_pages(&page_doc, true);
        }
        doc.metadata.pages = Some(crate::types::PageStructure {
            total_count: 1,
            unit_type: crate::types::PageUnitType::Page,
            boundaries: Some(vec![crate::types::PageBoundary {
                byte_start: 0,
                byte_end: 20,
                page_number: 1,
            }]),
            pages: None,
        });
        let before = serde_json::to_value(&doc.metadata.pages).unwrap();
        let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        assert_eq!(result.content, "actual rendered text");
        assert_eq!(serde_json::to_value(&result.metadata.pages).unwrap(), before);
    }
}

#[test]
fn issue_2128_should_match_identical_pages_in_sequence() {
    let mut doc = make_doc("pdf");
    for page in 1..=3 {
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Repeated café", 0).with_page(page));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
    }
    doc.metadata.pages = Some(crate::types::PageStructure {
        total_count: 3,
        unit_type: crate::types::PageUnitType::Page,
        boundaries: Some(
            (1..=3)
                .map(|page_number| crate::types::PageBoundary {
                    byte_start: 0,
                    byte_end: 14,
                    page_number,
                })
                .collect(),
        ),
        pages: None,
    });
    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.content, "Repeated café\n\n\nRepeated café\n\n\nRepeated café");
    let boundaries = result.metadata.pages.unwrap().boundaries.unwrap();
    assert_eq!(boundaries.len(), 3);
    for (index, boundary) in boundaries.iter().enumerate() {
        assert_eq!(boundary.page_number, index as u32 + 1);
        assert_eq!(boundary.byte_start, index * 17);
        assert_eq!(boundary.byte_end, index * 17 + 14);
        assert_eq!(&result.content[boundary.byte_start..boundary.byte_end], "Repeated café");
    }
}

#[test]
fn issue_2128_should_preserve_nonplain_and_renderer_input_metadata() {
    let mut doc = make_doc("pdf");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Page one", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Page two", 0).with_page(2));
    doc.metadata.pages = Some(crate::types::PageStructure {
        total_count: 2,
        unit_type: crate::types::PageUnitType::Page,
        boundaries: Some(vec![
            crate::types::PageBoundary {
                byte_start: 0,
                byte_end: 8,
                page_number: 1,
            },
            crate::types::PageBoundary {
                byte_start: 10,
                byte_end: 18,
                page_number: 2,
            },
        ]),
        pages: None,
    });
    let before = serde_json::to_value(&doc.metadata.pages).unwrap();
    let formatted = derive_extraction_result(doc.clone(), false, crate::core::config::OutputFormat::Markdown);
    assert!(formatted.formatted_content.is_some());
    assert_eq!(serde_json::to_value(&formatted.metadata.pages).unwrap(), before);
    let renderer_input = derive_renderer_input(doc.clone());
    assert_eq!(serde_json::to_value(&renderer_input.metadata.pages).unwrap(), before);
    doc.metadata.pages = None;
    let plain = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(
        serde_json::to_value(&plain.metadata.pages).unwrap(),
        serde_json::Value::Null
    );
}

fn assert_final_ocr_page_boundaries(suffix: &str) {
    let mut doc = make_doc("pdf");
    let page_texts: Vec<String> = (1..=10)
        .map(|page| {
            format!(
                "Scanned page {page:02}\nInvoice code 202610{page:02}\nQuantity {} widgets\nTotal {}.50 USD{suffix}",
                page + 20,
                100 + page
            )
        })
        .collect();
    let mut intermediate_offset = 0;
    let boundaries = page_texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let page_number = index as u32 + 1;
            doc.push_element(InternalElement::text(ElementKind::Paragraph, text, 0).with_page(page_number));
            doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
            let boundary = crate::types::PageBoundary {
                byte_start: intermediate_offset,
                byte_end: intermediate_offset + text.len(),
                page_number,
            };
            intermediate_offset += text.len() + 2;
            boundary
        })
        .collect();
    doc.metadata.pages = Some(crate::types::PageStructure {
        total_count: 10,
        unit_type: crate::types::PageUnitType::Page,
        boundaries: Some(boundaries),
        pages: None,
    });
    doc.metadata.title = Some("Synthetic OCR invoice batch".to_string());
    doc.prebuilt_pages = build_pages(&doc, true);
    let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.content, page_texts.join("\n\n\n"));
    assert_eq!(result.metadata.title.as_deref(), Some("Synthetic OCR invoice batch"));
    assert_final_page_slices(&result, &page_texts);
}

fn assert_final_page_slices(result: &ExtractedDocument, expected: &[String]) {
    let structure = result.metadata.pages.as_ref().expect("page metadata");
    assert_eq!(structure.total_count, 10);
    assert_eq!(structure.unit_type, crate::types::PageUnitType::Page);
    let boundaries = structure.boundaries.as_ref().expect("page boundaries");
    let pages = result.pages.as_ref().expect("per-page OCR content");
    assert_eq!(boundaries.len(), expected.len());
    assert_eq!(pages.len(), expected.len());
    let mut final_offset = 0;
    for (index, ((boundary, page), text)) in boundaries.iter().zip(pages).zip(expected).enumerate() {
        assert_eq!(page.page_number, index as u32 + 1);
        assert_eq!(page.content, *text);
        assert_eq!(boundary.page_number, page.page_number);
        assert_eq!(
            result.content.get(boundary.byte_start..boundary.byte_end),
            Some(text.as_str()),
            "page {} boundary must slice its complete final rendered text",
            page.page_number
        );
        assert_eq!(boundary.byte_start, final_offset);
        assert_eq!(boundary.byte_end, final_offset + text.len());
        final_offset += text.len() + 3;
    }
    assert_eq!(boundaries.last().unwrap().byte_end, result.content.len());
}

#[test]
fn should_publish_no_ocr_page_failures_as_none_and_never_serialize_an_empty_list() {
    let result = derive_extraction_result(make_doc("pdf"), false, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.ocr_page_failures, None);
    let serialized = serde_json::to_value(&result).unwrap();
    assert!(serialized.get("content").is_some());
    assert_eq!(serialized.get("ocr_page_failures"), None);
}

#[test]
fn should_publish_ocr_page_failures_in_the_typed_field_only() {
    let mut document = make_doc("pdf");
    let expected = vec![crate::types::OcrPageFailure {
        page: 2,
        error: "backend timed out".to_string(),
        recovered: false,
    }];
    document.ocr_page_failures = expected.clone();

    let result = derive_extraction_result(document, false, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.ocr_page_failures, Some(expected));
    assert!(!result.metadata.additional.contains_key("ocr_page_failures"));
}

/// `OutputFormat::DocTags` must produce the same output as the always-registered
/// built-in "doctags" renderer (`plugins::registry::renderer::DocTagsRenderer`),
/// via its own first-class match arm rather than falling through to the
/// `Custom(_)` renderer-registry lookup path (and its warning-on-miss behavior).
#[test]
fn should_render_doctags_output_format_without_going_through_custom_fallback() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));
    let expected = crate::rendering::render_doctags(&doc);

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::DocTags);

    assert_eq!(result.formatted_content.as_deref(), Some(expected.as_str()));
    assert!(
        result.processing_warnings.is_empty(),
        "DocTags is a first-class, always-registered format and must never warn: {:?}",
        result.processing_warnings
    );
}

/// #208: requesting a custom output format with no matching renderer must
/// leave a `ProcessingWarning` behind — `tracing::warn!` alone is invisible
/// to API and binding consumers, who have no other way to learn that the
/// requested format ("markdwon", a typo) was not actually produced.
#[test]
fn test_derive_extraction_result_unregistered_custom_format_emits_processing_warning() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

    let result = derive_extraction_result(
        doc,
        false,
        crate::core::config::OutputFormat::Custom("markdwon".to_string()),
    );

    assert!(
        result.formatted_content.is_none(),
        "no renderer is registered for 'markdwon'"
    );
    assert_eq!(result.processing_warnings.len(), 1);
    assert_eq!(result.processing_warnings[0].source, "output-format");
    assert!(
        result.processing_warnings[0].message.contains("markdwon"),
        "warning must name the requested format: {}",
        result.processing_warnings[0].message
    );
}

#[test]
fn should_not_mask_epub_custom_renderer_failure_with_pre_rendered_content() {
    let mut document = make_doc("epub");
    document.push_element(InternalElement::text(ElementKind::Paragraph, "element text", 0));
    document.pre_rendered_content = Some("stale custom output".to_string());
    document.metadata.output_format = Some("missing-epub-renderer".to_string());

    let result = derive_extraction_result(
        document,
        false,
        crate::core::config::OutputFormat::Custom("missing-epub-renderer".to_string()),
    );

    assert!(result.formatted_content.is_none());
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("missing-epub-renderer"))
    );
}

#[test]
fn should_keep_elements_authoritative_for_non_epub_plain_documents() {
    let mut document = make_doc("markdown");
    document.push_element(InternalElement::text(ElementKind::Paragraph, "element text", 0));
    document.pre_rendered_content = Some("unrelated pre-render".to_string());
    document.metadata.output_format = Some("plain".to_string());

    let result = derive_extraction_result(document, false, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.content, "element text");
}

#[test]
fn should_render_epub_json_and_html_without_truncating_syntax() {
    let mut document = make_doc("epub");
    document.push_element(InternalElement::text(ElementKind::Paragraph, "bounded text", 0));

    let json = derive_extraction_result(document.clone(), false, crate::core::config::OutputFormat::Json);
    let html = derive_extraction_result(document, false, crate::core::config::OutputFormat::Html);

    serde_json::from_str::<serde_json::Value>(json.formatted_content.as_deref().expect("JSON output"))
        .expect("EPUB JSON output must remain valid");
    let html = html.formatted_content.expect("HTML output");
    assert!(html.contains("bounded text"));
    assert!(html.contains("</p>"));
}

/// A custom output format with a registered renderer must produce no
/// output-format warning at all.
#[test]
fn test_derive_extraction_result_registered_custom_format_emits_no_warning() {
    struct UppercaseRenderer;
    impl crate::plugins::Plugin for UppercaseRenderer {
        fn name(&self) -> &str {
            "shout-259"
        }
    }
    impl crate::plugins::Renderer for UppercaseRenderer {
        fn render_result(&self, result: &crate::types::ExtractedDocument) -> crate::Result<String> {
            Ok(result.content.to_uppercase())
        }
    }
    crate::plugins::register_renderer(std::sync::Arc::new(UppercaseRenderer)).unwrap();

    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

    let result = derive_extraction_result(
        doc,
        false,
        crate::core::config::OutputFormat::Custom("shout-259".to_string()),
    );

    assert_eq!(result.formatted_content.as_deref(), Some("HELLO WORLD."));
    assert!(
        result.processing_warnings.is_empty(),
        "a successful custom render must not warn: {:?}",
        result.processing_warnings
    );

    crate::plugins::unregister_renderer("shout-259").unwrap();
}

/// Regression test mirroring the OCR backend registry's self-heal
/// (`plugins::ocr::ensure_ocr_backends_initialized`): `clear_renderers()` (called here to
/// simulate a sibling test, or any consumer resetting the plugin lifecycle) empties the
/// global renderer registry, including the built-ins. Before
/// `crate::plugins::ensure_renderers_initialized()` was wired into the `Custom` arm of
/// `derive_extraction_result`, a subsequent `Custom("markdown")` render found nothing
/// registered and silently fell back to plain text with a warning, even though
/// "markdown" is a built-in that must always be available. Delete the
/// `ensure_renderers_initialized()` call at `derive.rs`'s `OutputFormat::Custom` arm to
/// verify this test fails without the fix.
#[test]
fn should_reseed_builtin_renderers_after_global_registry_cleared() {
    let _guard = crate::plugins::registry::test_support::RendererRegistryGuard::acquire();
    crate::plugins::clear_renderers().unwrap();
    assert!(
        crate::plugins::list_renderers().unwrap().is_empty(),
        "precondition: the global renderer registry must be empty after clear_renderers()"
    );

    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));
    let expected = crate::rendering::render_markdown(&doc);

    let result = derive_extraction_result(
        doc,
        false,
        crate::core::config::OutputFormat::Custom("markdown".to_string()),
    );

    assert_eq!(
        result.formatted_content.as_deref(),
        Some(expected.as_str()),
        "the built-in 'markdown' renderer must self-heal back into the registry after a clear"
    );
    assert!(
        result.processing_warnings.is_empty(),
        "a healed built-in render must not warn: {:?}",
        result.processing_warnings
    );
}

/// #259: `code_intelligence` must surface the tree-sitter-derived
/// `FormatMetadata::Code` payload instead of being hardcoded to `None`, even
/// for an `InternalDocument` that never went through `CodeExtractor` (so has
/// no `CODE_INTELLIGENCE_SCRATCH_KEY` entry in `metadata.additional`) — the
/// fallback path serializes `CodeMetadata` directly. See
/// `test_derive_extraction_result_prefers_full_process_result_over_code_metadata`
/// for the primary, `CodeExtractor`-shaped path.
#[cfg(feature = "tree-sitter")]
#[test]
fn test_derive_extraction_result_populates_code_intelligence_from_code_metadata() {
    use crate::types::metadata::{CodeChunkInfo, CodeMetadata, FormatMetadata};

    let mut doc = make_doc("code");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "fn main() {}", 0));
    doc.metadata.format = Some(FormatMetadata::Code(CodeMetadata {
        chunks: vec![CodeChunkInfo {
            text: "fn main() {}".to_string(),
            context_path: vec!["main".to_string()],
            node_types: vec!["function_definition".to_string()],
            byte_start: 0,
            byte_end: 12,
        }],
        data: None,
    }));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

    let code_intelligence = result
        .code_intelligence
        .expect("code_intelligence must be populated when FormatMetadata::Code is present");
    assert_eq!(
        code_intelligence["chunks"][0]["context_path"][0],
        serde_json::json!("main")
    );
}

/// #259: when `metadata.additional` carries the full serialized
/// `tree_sitter_language_pack::ProcessResult` under
/// `extractors::code::CODE_INTELLIGENCE_SCRATCH_KEY` (as `CodeExtractor`
/// populates it), derivation must prefer that full payload over the
/// `CodeMetadata`-only fallback, and must remove the scratch key so it does
/// not leak into the final `ExtractedDocument.metadata.additional` map.
#[cfg(feature = "tree-sitter")]
#[test]
fn test_derive_extraction_result_prefers_full_process_result_over_code_metadata() {
    use crate::types::metadata::{CodeMetadata, FormatMetadata};

    let mut doc = make_doc("code");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "def f(): pass", 0));
    doc.metadata.format = Some(FormatMetadata::Code(CodeMetadata::default()));
    doc.metadata.additional.insert(
        std::borrow::Cow::Borrowed(crate::extractors::code::CODE_INTELLIGENCE_SCRATCH_KEY),
        serde_json::json!({"language": "python", "metrics": {"total_lines": 1}}),
    );

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);

    let code_intelligence = result
        .code_intelligence
        .expect("code_intelligence must be populated from the scratch key");
    assert_eq!(code_intelligence["language"], serde_json::json!("python"));
    assert_eq!(code_intelligence["metrics"]["total_lines"], serde_json::json!(1));
    // The stashed key must not leak into the final metadata.
    assert!(
        !result
            .metadata
            .additional
            .contains_key(crate::extractors::code::CODE_INTELLIGENCE_SCRATCH_KEY),
        "scratch key must be removed before assembling the final ExtractedDocument"
    );
}

#[cfg(feature = "tree-sitter")]
#[test]
fn test_derive_extraction_result_code_intelligence_none_without_code_metadata() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert!(result.code_intelligence.is_none());
}

#[cfg(any(feature = "pdf", feature = "ocr"))]
#[test]
fn test_derive_extraction_result_with_structure() {
    let mut doc = make_doc("pdf");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 1).with_page(1));

    let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
    assert!(result.document.is_some());
    let ds = result.document.unwrap();
    assert!(ds.validate().is_ok());
    assert_eq!(ds.source_format.as_deref(), Some("pdf"));
}

#[cfg(any(feature = "pdf", feature = "ocr"))]
#[test]
fn test_source_format_cow_owned_propagates() {
    let owned: std::borrow::Cow<'static, str> = std::borrow::Cow::Owned("epub".to_string());
    let mut doc = InternalDocument::new(owned);
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Ch1", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 1).with_page(1));

    let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
    let ds = result.document.unwrap();
    assert_eq!(ds.source_format.as_deref(), Some("epub"));
}

#[test]
fn test_derive_extraction_result_promotes_extraction_method() {
    let mut doc = make_doc("pdf");
    doc.metadata.additional.insert(
        Cow::Borrowed("extraction_method"),
        serde_json::Value::String("mixed".to_string()),
    );
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed));
}

#[test]
fn test_derive_extraction_result_ignores_unknown_extraction_method() {
    let mut doc = make_doc("pdf");
    doc.metadata.additional.insert(
        Cow::Borrowed("extraction_method"),
        serde_json::Value::String("native_ole".to_string()),
    );
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.extraction_method, None);
}

/// Pages with a heading element must render `# Heading` when output_format=Markdown.
///
/// Before the fix, `PageContent.content` always contained raw element text ("Introduction"),
/// not the formatted representation ("# Introduction"). This tests the full pipeline
/// state as seen by callers (derive + apply_output_format).
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_markdown_heading_is_formatted() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Introduction", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body text here.", 0).with_page(1));

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

    let pages = result
        .pages
        .expect("pages must be populated when elements have page numbers");
    assert_eq!(pages.len(), 1);

    assert!(
        result.content.contains("# Introduction"),
        "full content must have markdown heading, got: {:?}",
        result.content,
    );
    assert!(
        pages[0].content.contains("# Introduction"),
        "page content must use markdown heading format, got: {:?}",
        pages[0].content,
    );
    assert!(
        !pages[0].content.trim_start().starts_with("Introduction"),
        "page content must not start with bare heading text without '#', got: {:?}",
        pages[0].content,
    );
}

/// Plain output must leave page content as raw element text — no regressions.
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_plain_format_unchanged() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Introduction", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body text here.", 0).with_page(1));

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Plain);

    let pages = result.pages.expect("pages must be populated");
    assert_eq!(pages.len(), 1);

    assert!(
        !pages[0].content.contains("# Introduction"),
        "plain-format page content must not contain markdown heading prefix, got: {:?}",
        pages[0].content,
    );
    assert!(
        pages[0].content.contains("Introduction"),
        "plain-format page content must still contain the heading text, got: {:?}",
        pages[0].content,
    );
}

/// Each page's formatted content must only contain that page's elements.
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_per_page_isolation() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter One", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Content of page one.", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter Two", 0).with_page(2));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Content of page two.", 0).with_page(2));

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

    let pages = result.pages.expect("pages must be populated");
    assert_eq!(pages.len(), 2);

    let p1 = pages.iter().find(|p| p.page_number == 1).expect("page 1");
    let p2 = pages.iter().find(|p| p.page_number == 2).expect("page 2");

    assert!(
        !p1.content.contains("Chapter Two"),
        "page 1 must not bleed page 2's content, got: {:?}",
        p1.content,
    );
    assert!(
        !p2.content.contains("Chapter One"),
        "page 2 must not include page 1's content, got: {:?}",
        p2.content,
    );
    assert!(
        p1.content.contains("# Chapter One"),
        "page 1 heading must be markdown-formatted, got: {:?}",
        p1.content,
    );
    assert!(
        p2.content.contains("# Chapter Two"),
        "page 2 heading must be markdown-formatted, got: {:?}",
        p2.content,
    );
}

/// List items must render as `- item` in markdown output, not bare text.
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_markdown_list_items_formatted() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::ListItem { ordered: false }, "First item", 1).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::ListItem { ordered: false }, "Second item", 1).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0).with_page(1));

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

    let pages = result.pages.expect("pages must be populated");
    assert_eq!(pages.len(), 1);

    assert!(
        pages[0].content.contains("- First item") || pages[0].content.contains("* First item"),
        "page content must use markdown list syntax, got: {:?}",
        pages[0].content,
    );
    assert!(
        pages[0].content.contains("- Second item") || pages[0].content.contains("* Second item"),
        "page content must use markdown list syntax for all items, got: {:?}",
        pages[0].content,
    );
}

/// HTML output must render headings as `<h1>` tags, not bare text.
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_html_format_renders_headings() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 0).with_page(1));

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Html);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Html);

    let pages = result.pages.expect("pages must be populated");
    assert_eq!(pages.len(), 1);

    assert!(
        pages[0].content.contains("<h1"),
        "html-format page content must contain heading markup, got: {:?}",
        pages[0].content,
    );
    assert!(
        !pages[0].content.trim_start().starts_with("Title\n"),
        "html-format page content must not be bare plain text, got: {:?}",
        pages[0].content,
    );
}

/// Prebuilt pages whose page_number has no matching page-tagged elements must
/// be returned unchanged. This is the normal path for native PDF extraction,
/// OCR on images, and Excel/PPTX where the extractor sets prebuilt_pages but
/// does not attach page numbers to individual InternalElements.
#[test]
fn page_content_prebuilt_pages_no_page_elements_unchanged() {
    let mut doc = make_doc("pdf");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 0));
    doc.prebuilt_pages = Some(vec![crate::types::page::PageContent {
        page_number: 1,
        content: "Native PDF page content.".to_string(),
        tables: vec![],
        image_indices: vec![],
        image_preprocessing: None,
        hierarchy: None,
        is_blank: None,
        layout_regions: None,
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence: None,
        native_content: None,
    }]);

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

    let pages = result.pages.expect("pages must be populated");
    assert_eq!(pages.len(), 1);
    assert_eq!(
        pages[0].content, "Native PDF page content.",
        "prebuilt content must not be overwritten when no elements are page-tagged, got: {:?}",
        pages[0].content,
    );
}

/// A page whose page_number appears in prebuilt_pages but has no matching
/// page-tagged elements must keep its original content unchanged. This covers the
/// per-page early-return branch inside apply_page_content_format.
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_page_without_matching_elements_unchanged() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter", 0).with_page(1));
    doc.prebuilt_pages = Some(vec![
        crate::types::page::PageContent {
            page_number: 1,
            content: "Page 1 plain".to_string(),
            tables: vec![],
            image_indices: vec![],
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        },
        crate::types::page::PageContent {
            page_number: 2,
            content: "Page 2 native content.".to_string(),
            tables: vec![],
            image_indices: vec![],
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        },
    ]);

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

    let pages = result.pages.expect("pages must be populated");
    assert_eq!(pages.len(), 2);

    let p2 = pages.iter().find(|p| p.page_number == 2).expect("page 2");
    assert_eq!(
        p2.content, "Page 2 native content.",
        "page with no matching elements must keep its original content, got: {:?}",
        p2.content,
    );
    let p1 = pages.iter().find(|p| p.page_number == 1).expect("page 1");
    assert!(
        p1.content.contains("# Chapter"),
        "page 1 must still be markdown-formatted, got: {:?}",
        p1.content,
    );
}

/// OutputFormat::Json: result.content is rendered JSON but pages keep raw extracted
/// text. The asymmetry is intentional — splitting JSON into per-page sub-objects
/// would produce malformed fragments. The comment in apply_page_content_format
/// explains the rationale; this test locks the observable contract.
#[cfg(any(
    feature = "ocr",
    feature = "office",
    feature = "pdf",
    paddle_ocr,
    feature = "xml",
    feature = "hwpx",
    feature = "quality",
    feature = "chunking"
))]
#[test]
fn page_content_json_format_pages_stay_raw() {
    let mut doc = make_doc("docx");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 0).with_page(1));

    let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Json);
    let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Json);

    assert!(
        result.content.contains('"'),
        "json format must produce JSON-structured result.content, got: {:?}",
        result.content,
    );
    let pages = result
        .pages
        .expect("pages must be populated when elements have page numbers");
    assert_eq!(pages.len(), 1);
    assert!(
        !pages[0].content.starts_with('{'),
        "page content must not be JSON-structured, got: {:?}",
        pages[0].content,
    );
    assert!(
        pages[0].content.contains("Title") || pages[0].content.contains("Body"),
        "page content must contain raw extracted text, got: {:?}",
        pages[0].content,
    );
}
