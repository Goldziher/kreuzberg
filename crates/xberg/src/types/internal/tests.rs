use super::*;

#[test]
fn test_internal_element_id_deterministic() {
    let id1 = InternalElementId::generate("heading", "Introduction", Some(1), 0);
    let id2 = InternalElementId::generate("heading", "Introduction", Some(1), 0);
    assert_eq!(id1, id2);
}

#[test]
fn test_internal_element_id_differs_by_index() {
    let id1 = InternalElementId::generate("paragraph", "Same text", Some(1), 0);
    let id2 = InternalElementId::generate("paragraph", "Same text", Some(1), 1);
    assert_ne!(id1, id2);
}

#[test]
fn test_internal_element_id_format() {
    let id = InternalElementId::generate("title", "Hello", None, 0);
    assert!(id.as_str().starts_with("ie-"));
    assert_eq!(id.as_str().len(), 3 + 12);
}

#[test]
fn test_element_kind_discriminant() {
    assert_eq!(ElementKind::Title.discriminant(), "title");
    assert_eq!(ElementKind::Heading { level: 2 }.discriminant(), "heading");
    assert_eq!(ElementKind::ListStart { ordered: true }.discriminant(), "list_start");
}

#[test]
fn test_container_markers() {
    assert!(ElementKind::ListStart { ordered: false }.is_container_start());
    assert!(ElementKind::ListEnd.is_container_end());
    assert!(!ElementKind::Paragraph.is_container_start());
    assert_eq!(ElementKind::QuoteStart.matching_end(), Some(ElementKind::QuoteEnd));
}

#[test]
fn test_internal_document_push() {
    let mut doc = InternalDocument::new("markdown");
    let elem = InternalElement::text(ElementKind::Paragraph, "Hello world", 0);
    let idx = doc.push_element(elem);
    assert_eq!(idx, 0);
    assert_eq!(doc.elements.len(), 1);
    assert_eq!(doc.elements[0].text, "Hello world");
}

#[test]
fn public_attributes_preserve_explicit_empty_map() {
    let mut element = InternalElement::text(ElementKind::Paragraph, "text", 0);
    element.attributes = Some(AHashMap::new());

    assert_eq!(element.public_attributes(), Some(std::collections::HashMap::new()));
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[test]
fn public_attributes_hide_internal_image_ocr_suppression() {
    let mut element = InternalElement::text(ElementKind::Image { image_index: 0 }, "", 0);
    element.suppress_image_ocr_rendering();

    assert!(element.public_attributes().is_none());
    assert!(!element.should_render_image_ocr());
}

#[cfg(all(any(feature = "pdf", feature = "office"), feature = "ocr", feature = "tokio-runtime"))]
#[test]
fn an_image_ocr_text_anchor_is_an_empty_paragraph_with_a_hidden_attribute() {
    let anchor = InternalElement::image_ocr_text_anchor(3, Some(2));

    assert_eq!(anchor.kind, ElementKind::Paragraph);
    assert_eq!(anchor.text, "");
    assert_eq!(anchor.page, Some(2));
    assert_eq!(anchor.image_ocr_text_anchor_index(), Some(3));
    assert!(anchor.public_attributes().is_none());
    assert_eq!(
        InternalElement::text(ElementKind::Paragraph, "", 0).image_ocr_text_anchor_index(),
        None
    );
}

/// #### FAILS against unfixed code
/// `set_list_item_source_label`/`list_item_source_label` do not exist yet
/// on unfixed `InternalElement` -- this test does not compile without the
/// fix. Once the fix lands, it proves two things a bare
/// `ElementKind::ListItem { ordered: true }` cannot: the literal marker
/// text round-trips unchanged, and -- unlike the OCR-suppression
/// attribute -- it is NOT filtered out of `public_attributes()`, so it
/// reaches the public `DocumentStructure` tree via `DocumentNode::attributes`.
#[cfg(feature = "pdf")]
#[test]
fn list_item_source_label_round_trips_and_stays_public() {
    let mut element = InternalElement::text(ElementKind::ListItem { ordered: false }, "General Provisions.", 1);
    assert_eq!(element.list_item_source_label(), None);

    element.set_list_item_source_label("B.");

    assert_eq!(element.list_item_source_label(), Some("B."));
    assert_eq!(
        element.public_attributes(),
        Some(std::collections::HashMap::from([(
            "list_marker".to_string(),
            "B.".to_string()
        )]))
    );
}

/// An empty label is a caller bug (e.g. a marker-strip that removed
/// nothing), not a real source marker -- `set_list_item_source_label`
/// must not manufacture a spurious attribute for it.
#[cfg(feature = "pdf")]
#[test]
fn list_item_source_label_ignores_an_empty_label() {
    let mut element = InternalElement::text(ElementKind::ListItem { ordered: true }, "item text", 1);
    element.set_list_item_source_label("");
    assert_eq!(element.list_item_source_label(), None);
    assert_eq!(element.attributes, None);
}

/// Measured font size round-trips and stays out of `public_attributes()` --
/// unlike the list-marker attribute, this is internal plumbing, not document
/// content.
#[cfg(feature = "pdf")]
#[test]
fn measured_font_size_round_trips_and_stays_internal() {
    let mut element = InternalElement::text(ElementKind::Paragraph, "Body text.", 1);
    assert_eq!(element.measured_font_size(), None);

    element.set_measured_font_size(11.5);

    assert_eq!(element.measured_font_size(), Some(11.5));
    // The only attribute set is the measured font size, so it must never reach
    // the public surface: `public_attributes()` reports `None` rather than
    // `Some({..})`.
    assert_eq!(element.public_attributes(), None);
}

/// A non-finite or non-positive value is never stored in the first place.
#[cfg(feature = "pdf")]
#[test]
fn measured_font_size_ignores_non_finite_or_non_positive_values() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -11.5] {
        let mut element = InternalElement::text(ElementKind::Paragraph, "Body text.", 1);
        element.set_measured_font_size(invalid);
        assert_eq!(element.measured_font_size(), None, "{invalid} must not be stored");
        assert_eq!(element.attributes, None);
    }
}

/// The attribute map survives a cache round-trip as plain strings, so an
/// unparseable or corrupted stored value must be treated as untrusted and
/// read back as `None`, not propagated or panicked on.
#[cfg(feature = "pdf")]
#[test]
fn measured_font_size_returns_none_for_unparseable_stored_value() {
    let mut element = InternalElement::text(ElementKind::Paragraph, "Body text.", 1);
    element
        .attributes
        .get_or_insert_with(AHashMap::new)
        .insert("xberg:internal:font-size-pt".to_string(), "not-a-number".to_string());

    assert_eq!(element.measured_font_size(), None);
}

#[cfg(any(feature = "ocr", feature = "pdf", paddle_ocr, feature = "xml", feature = "office"))]
#[test]
fn test_internal_element_builder_pattern() {
    let elem = InternalElement::text(ElementKind::Heading { level: 2 }, "Methods", 1)
        .with_page(3)
        .with_anchor("methods")
        .with_layer(ContentLayer::Body);

    assert_eq!(elem.text, "Methods");
    assert_eq!(elem.page, Some(3));
    assert_eq!(elem.anchor, Some("methods".to_string()));
    assert_eq!(elem.depth, 1);
}

#[test]
fn test_relationship_kind_serde() {
    let kind = RelationshipKind::FootnoteReference;
    let json = serde_json::to_string(&kind).unwrap();
    assert_eq!(json, "\"footnote_reference\"");

    let parsed: RelationshipKind = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, kind);
}

/// Verify that `InternalDocument` round-trips through serde JSON without loss.
///
/// This is the primary correctness gate for foreign-language plugin support:
/// Python/TypeScript/Ruby implementations of `DocumentExtractor` construct
/// an `InternalDocument` as JSON and pass it across the FFI boundary.
#[test]
fn should_round_trip_through_serde_json() {
    let mut doc = InternalDocument::new("pdf");
    doc.mime_type = "application/pdf".to_string();

    let title = InternalElement::text(ElementKind::Title, "Test Document", 0);
    doc.push_element(title);

    let heading = InternalElement::text(ElementKind::Heading { level: 2 }, "Introduction", 1);
    doc.push_element(heading);

    let para = InternalElement::text(ElementKind::Paragraph, "Body text here.", 1);
    doc.push_element(para);

    let list_start = InternalElement::text(ElementKind::ListStart { ordered: true }, "", 1);
    doc.push_element(list_start);
    let item = InternalElement::text(ElementKind::ListItem { ordered: true }, "First item", 2);
    doc.push_element(item);
    let list_end = InternalElement::text(ElementKind::ListEnd, "", 1);
    doc.push_element(list_end);

    let code = InternalElement::text(ElementKind::Code, "fn main() {}", 0);
    doc.push_element(code);

    let pb = InternalElement::text(ElementKind::PageBreak, "", 0);
    doc.push_element(pb);

    let img_elem = InternalElement::text(ElementKind::Image { image_index: 0 }, "", 0);
    doc.push_element(img_elem);

    let ocr = InternalElement::text(
        ElementKind::OcrText {
            level: OcrElementLevel::Word,
        },
        "scanned word",
        0,
    );
    doc.push_element(ocr);

    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Index(2),
        kind: RelationshipKind::FootnoteReference,
    });
    doc.push_relationship(Relationship {
        source: 1,
        target: RelationshipTarget::Key("introduction".to_string()),
        kind: RelationshipKind::CrossReference,
    });

    let json = serde_json::to_string(&doc).expect("serialize InternalDocument");
    let restored: InternalDocument = serde_json::from_str(&json).expect("deserialize InternalDocument");

    assert_eq!(restored.source_format, doc.source_format);
    assert_eq!(restored.mime_type, doc.mime_type);
    assert_eq!(restored.elements.len(), doc.elements.len());
    assert_eq!(restored.relationships.len(), doc.relationships.len());
    assert_eq!(restored.elements[0].kind, ElementKind::Title);
    assert_eq!(restored.elements[1].kind, ElementKind::Heading { level: 2 });
    assert_eq!(restored.elements[4].kind, ElementKind::ListItem { ordered: true });
    assert_eq!(restored.elements[8].kind, ElementKind::Image { image_index: 0 });
    assert_eq!(
        restored.elements[9].kind,
        ElementKind::OcrText {
            level: OcrElementLevel::Word
        }
    );

    assert_eq!(restored.relationships[0].target, RelationshipTarget::Index(2));
    assert_eq!(
        restored.relationships[1].target,
        RelationshipTarget::Key("introduction".to_string())
    );

    assert_eq!(restored.elements[0].id, doc.elements[0].id);

    assert_eq!(restored.elements[0].layer, ContentLayer::Body);
}

#[test]
fn should_preserve_ocr_page_failures_across_public_internal_conversion() {
    let expected = crate::types::OcrPageFailure {
        page: 7,
        error: "backend timed out".to_string(),
        recovered: false,
    };
    let public = crate::types::ExtractedDocument {
        content: "partial text".to_string(),
        mime_type: "application/pdf".into(),
        ocr_page_failures: Some(vec![expected.clone()]),
        ..Default::default()
    };

    let internal = InternalDocument::from(public);
    assert_eq!(internal.ocr_page_failures, vec![expected.clone()]);

    let round_tripped = crate::types::ExtractedDocument::from(internal);
    assert!(!round_tripped.metadata.additional.contains_key("ocr_page_failures"));
    assert_eq!(round_tripped.ocr_page_failures, Some(vec![expected]));
}

#[test]
fn internal_document_omitting_ocr_page_failures_defaults_to_empty() {
    let value = serde_json::to_value(InternalDocument::new("pdf")).unwrap();
    assert_eq!(value.get("ocr_page_failures"), None);

    let restored: InternalDocument = serde_json::from_value(value).unwrap();
    assert_eq!(restored.ocr_page_failures, Vec::<crate::types::OcrPageFailure>::new());
}

#[test]
fn internal_document_round_trips_ocr_page_failures() {
    let mut document = InternalDocument::new("pdf");
    document.ocr_page_failures.push(crate::types::OcrPageFailure {
        page: 3,
        error: "recognition failed".to_string(),
        recovered: true,
    });

    let json = serde_json::to_string(&document).unwrap();
    let restored: InternalDocument = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.ocr_page_failures, document.ocr_page_failures);
}

/// Cover all 27 `ElementKind` variants through a serde JSON round-trip.
///
/// Every variant must be constructed, serialised, and deserialised; the
/// `kind` field is then asserted on each restored element so that a missing
/// or mis-tagged variant surfaces immediately.
#[test]
fn should_cover_all_element_kind_variants() {
    let doc = document_with_all_element_kinds();
    assert_all_element_kinds_round_trip(&doc);
}

fn push_element_and_assert_kind(doc: &mut InternalDocument, kind: ElementKind, text: &str, depth: u16) {
    doc.push_element(InternalElement::text(kind, text, depth));
    assert_eq!(doc.elements.last().unwrap().kind, kind);
}

fn document_with_all_element_kinds() -> InternalDocument {
    let mut doc = InternalDocument::new("test");

    push_element_and_assert_kind(&mut doc, ElementKind::Title, "T", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Heading { level: 1 }, "H1", 1);
    push_element_and_assert_kind(&mut doc, ElementKind::Paragraph, "P", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::ListItem { ordered: false }, "li", 2);
    push_element_and_assert_kind(&mut doc, ElementKind::Code, "x=1", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Formula, "E=mc^2", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::FootnoteDefinition, "note text", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::FootnoteRef, "1", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Citation, "Smith 2020", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Slide { number: 3 }, "slide 3", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::DefinitionTerm, "term", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::DefinitionDescription, "desc", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Admonition, "Note:", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::RawBlock, "<raw/>", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::MetadataBlock, "---", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::ListStart { ordered: true }, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::ListEnd, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::QuoteStart, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::QuoteEnd, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::GroupStart, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::GroupEnd, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Table { table_index: 0 }, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::Image { image_index: 1 }, "", 0);
    push_element_and_assert_kind(&mut doc, ElementKind::PageBreak, "", 0);

    for level in [
        OcrElementLevel::Word,
        OcrElementLevel::Line,
        OcrElementLevel::Block,
        OcrElementLevel::Page,
    ] {
        push_element_and_assert_kind(&mut doc, ElementKind::OcrText { level }, "ocr", 0);
    }

    doc
}

fn assert_all_element_kinds_round_trip(doc: &InternalDocument) {
    let json = serde_json::to_string(&doc).expect("serialize all-variant InternalDocument");
    let restored: InternalDocument = serde_json::from_str(&json).expect("deserialize all-variant InternalDocument");

    assert_eq!(restored.elements.len(), doc.elements.len());

    assert_eq!(restored.elements[0].kind, ElementKind::Title);
    assert_eq!(restored.elements[5].kind, ElementKind::Formula);
    assert_eq!(restored.elements[9].kind, ElementKind::Slide { number: 3 });
    assert_eq!(restored.elements[14].kind, ElementKind::MetadataBlock);
    assert_eq!(restored.elements[16].kind, ElementKind::ListEnd);
    assert_eq!(restored.elements[17].kind, ElementKind::QuoteStart);
    assert_eq!(restored.elements[18].kind, ElementKind::QuoteEnd);
    assert_eq!(restored.elements[19].kind, ElementKind::GroupStart);
    assert_eq!(restored.elements[20].kind, ElementKind::GroupEnd);
    assert_eq!(restored.elements[21].kind, ElementKind::Table { table_index: 0 });
    assert_eq!(restored.elements[23].kind, ElementKind::PageBreak);
    assert_eq!(
        restored.elements[24].kind,
        ElementKind::OcrText {
            level: OcrElementLevel::Word
        }
    );
    assert_eq!(
        restored.elements[27].kind,
        ElementKind::OcrText {
            level: OcrElementLevel::Page
        }
    );
}

/// Verify that both `RelationshipTarget` variants survive a serde JSON
/// round-trip when carried inside a `Relationship`.
#[test]
fn should_round_trip_relationship_targets() {
    let mut doc = InternalDocument::new("test");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "source", 0));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "target", 0));

    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Index(1),
        kind: RelationshipKind::CrossReference,
    });
    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Key("anchor-abc".to_string()),
        kind: RelationshipKind::FootnoteReference,
    });

    let json = serde_json::to_string(&doc).expect("serialize RelationshipTarget variants");
    let restored: InternalDocument = serde_json::from_str(&json).expect("deserialize RelationshipTarget variants");

    assert_eq!(restored.relationships.len(), 2);
    assert_eq!(restored.relationships[0].target, RelationshipTarget::Index(1));
    assert_eq!(
        restored.relationships[1].target,
        RelationshipTarget::Key("anchor-abc".to_string())
    );
    assert_eq!(restored.relationships[0].kind, RelationshipKind::CrossReference);
    assert_eq!(restored.relationships[1].kind, RelationshipKind::FootnoteReference);
}
