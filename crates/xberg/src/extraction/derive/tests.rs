use super::test_support::make_doc;
use super::*;
use crate::types::document_structure::NodeContent;
use crate::types::internal::{
    ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
};

/// A table cell records its formula in the side channel with a page but no
/// bounding box. The same equation may appear again in the body as its own
/// element, and both are real formulas: only an OCR entry, which carries a
/// bounding box, is a second representation of an element.
#[test]
fn test_a_table_formula_does_not_suppress_the_same_equation_in_the_body() {
    let mut doc = InternalDocument::new("docx");
    doc.recorded_formulas.push(crate::types::Formula {
        latex: "E = mc^2".to_string(),
        bbox: None,
        page: Some(3),
    });
    doc.push_element(InternalElement::text(ElementKind::Formula, "E = mc^2", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);

    assert_eq!(result.formulas.len(), 2, "the table cell and the body each hold one");
}

#[test]
fn test_markup_formula_elements_reach_public_formulas() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Formula, "E = mc^2", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
    assert_eq!(result.formulas.len(), 1);
    assert_eq!(result.formulas[0].latex, "E = mc^2");
    assert_eq!(result.formulas[0].page, None);
    assert_eq!(result.formulas[0].bbox, None);
}

#[test]
fn should_project_formulas_when_document_structure_is_included() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Formula, "E = mc^2", 0));

    let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Markdown);

    assert_eq!(result.formulas.len(), 1);
    assert_eq!(result.formulas[0].latex, "E = mc^2");
    let structure = result.document.expect("document structure requested");
    assert!(
        structure
            .nodes
            .iter()
            .any(|node| matches!(&node.content, NodeContent::Formula { text } if text == "E = mc^2"))
    );
}

#[test]
fn test_ocr_side_channel_formulas_stay_first() {
    let mut doc = make_doc("pdf");
    doc.push_element(InternalElement::text(ElementKind::Formula, "b", 0));
    doc.formulas.push(crate::types::Formula {
        latex: "a".to_string(),
        bbox: None,
        page: Some(1),
    });

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    let latexes: Vec<&str> = result.formulas.iter().map(|f| f.latex.as_str()).collect();
    assert_eq!(latexes, vec!["a", "b"]);
}

#[test]
fn test_formula_projection_strips_dollar_delimiters() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Formula, "$$x + 1$$", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.formulas[0].latex, "x + 1");
}

#[test]
fn test_empty_formula_elements_are_skipped() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Formula, "   ", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert!(result.formulas.is_empty());
}

#[test]
fn test_projection_carries_element_geometry() {
    let mut doc = make_doc("pdf");
    let mut elem = InternalElement::text(ElementKind::Formula, "a + b", 0);
    elem.page = Some(3);
    elem.bbox = Some(crate::types::BoundingBox {
        x0: 1.0,
        y0: 2.0,
        x1: 3.0,
        y1: 4.0,
    });
    doc.push_element(elem);

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.formulas.len(), 1);
    assert_eq!(result.formulas[0].page, Some(3));
    assert_eq!(result.formulas[0].bbox.unwrap().y1, 4.0);
}

#[test]
fn test_projection_skips_side_channel_duplicates() {
    let mut doc = make_doc("image");
    // The layout image path represents one formula twice: side channel
    // with geometry plus a matching element. Only one may survive.
    doc.formulas.push(crate::types::Formula {
        latex: "E=mc^2".to_string(),
        bbox: None,
        page: Some(1),
    });
    doc.push_element(InternalElement::text(ElementKind::Formula, "E = mc^2", 0));
    doc.push_element(InternalElement::text(ElementKind::Formula, "a + b", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    let latexes: Vec<&str> = result.formulas.iter().map(|f| f.latex.as_str()).collect();
    assert_eq!(latexes, vec!["E=mc^2", "a + b"]);
}

#[test]
fn test_projection_keeps_geometry_bearing_twins() {
    // Mixed native+scanned document: the same equation on two different
    // pages is two formulas, never a duplicate.
    let mut doc = make_doc("pdf");
    doc.formulas.push(crate::types::Formula {
        latex: "E=mc^2".to_string(),
        bbox: None,
        page: Some(7),
    });
    let mut elem = InternalElement::text(ElementKind::Formula, "E = mc^2", 0);
    elem.page = Some(2);
    doc.push_element(elem);

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.formulas.len(), 2, "got: {:?}", result.formulas);
}

#[test]
fn test_projection_dedups_across_delimiter_wrapping() {
    let mut doc = make_doc("image");
    doc.formulas.push(crate::types::Formula {
        latex: "$$x$$".to_string(),
        bbox: None,
        page: Some(1),
    });
    doc.push_element(InternalElement::text(ElementKind::Formula, "x", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.formulas.len(), 1, "got: {:?}", result.formulas);
}

#[test]
fn test_projection_skips_formulas_empty_after_stripping() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Formula, "$ $", 0));

    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert!(result.formulas.is_empty(), "got: {:?}", result.formulas);
}

#[test]
fn test_strip_math_delimiters_ignores_escaped_delimiters() {
    assert_eq!(strip_math_delimiters(r"$a \$ b$"), r"a \$ b");
}

#[test]
fn test_strip_math_delimiters_cases() {
    assert_eq!(strip_math_delimiters("$$a$$"), "a");
    assert_eq!(strip_math_delimiters("\\[ x \\]"), "x");
    assert_eq!(strip_math_delimiters("$x$"), "x");
    // Multi-formula text keeps its delimiters: stripping the outer pair
    // would splice unrelated math together.
    assert_eq!(strip_math_delimiters("$x$ and $y$"), "$x$ and $y$");
    assert_eq!(strip_math_delimiters("$$a$$ text $$b$$"), "$$a$$ text $$b$$");
    assert_eq!(strip_math_delimiters("$"), "$");
    assert_eq!(strip_math_delimiters("$$"), "$$");
    assert_eq!(strip_math_delimiters("plain"), "plain");
}

#[test]
fn test_flat_document_produces_flat_tree() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Title, "My Title", 0));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "First paragraph.", 0));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Second paragraph.", 0));

    resolve_relationships(&mut doc);
    let ds = derive_document_structure_inner(&mut doc);
    assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());
    assert_eq!(ds.len(), 3);

    let roots: Vec<_> = ds.body_roots().collect();
    assert_eq!(roots.len(), 3);

    match &roots[0].1.content {
        NodeContent::Title { text } => assert_eq!(text, "My Title"),
        other => panic!("Expected Title, got {:?}", other),
    }
}

#[test]
fn test_heading_nesting() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter 1", 0));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Intro text.", 1));
    doc.push_element(InternalElement::text(
        ElementKind::Heading { level: 2 },
        "Section 1.1",
        1,
    ));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Section body.", 2));

    resolve_relationships(&mut doc);
    let ds = derive_document_structure_inner(&mut doc);
    assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

    let roots: Vec<_> = ds.body_roots().collect();
    assert_eq!(roots.len(), 1);

    let h1_group = &ds.nodes[roots[0].0.0 as usize];
    match &h1_group.content {
        NodeContent::Group {
            heading_level,
            heading_text,
            ..
        } => {
            assert_eq!(*heading_level, Some(1));
            assert_eq!(heading_text.as_deref(), Some("Chapter 1"));
        }
        other => panic!("Expected Group, got {:?}", other),
    }

    assert_eq!(h1_group.children.len(), 3);

    let heading_node = &ds.nodes[h1_group.children[0].0 as usize];
    assert!(matches!(&heading_node.content, NodeContent::Heading { level: 1, .. }));

    let para_node = &ds.nodes[h1_group.children[1].0 as usize];
    assert!(matches!(&para_node.content, NodeContent::Paragraph { .. }));

    let h2_group = &ds.nodes[h1_group.children[2].0 as usize];
    match &h2_group.content {
        NodeContent::Group {
            heading_level,
            heading_text,
            ..
        } => {
            assert_eq!(*heading_level, Some(2));
            assert_eq!(heading_text.as_deref(), Some("Section 1.1"));
        }
        other => panic!("Expected H2 Group, got {:?}", other),
    }

    assert_eq!(h2_group.children.len(), 2);
}

/// Regression test for xberg-io/xberg#1504: `document.nodes` and `elements` are two
/// views derived from the same `InternalDocument`, and they must agree on heading
/// depth. `document.nodes` already carried the level correctly on
/// `NodeContent::Heading { level, .. }`; the bug was that `elements`' `heading_level`
/// (in `metadata.additional`, via `convert_internal_elements_to_elements`) silently
/// disagreed by reporting nothing at all. This pins the cross-view invariant directly,
/// rather than checking either side in isolation.
#[test]
fn test_elements_and_document_nodes_agree_on_heading_level() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0));
    doc.push_element(InternalElement::text(ElementKind::Heading { level: 2 }, "Section", 1));
    doc.push_element(InternalElement::text(
        ElementKind::Heading { level: 6 },
        "Deep subsection",
        2,
    ));

    // `derive_document_structure_inner` takes `&mut` and moves text/annotations out of
    // elements via `mem::take`, so derive the `elements` view from an independent clone
    // taken before that happens.
    let doc_for_elements = doc.clone();

    resolve_relationships(&mut doc);
    let ds = derive_document_structure_inner(&mut doc);
    assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

    let document_node_levels: Vec<u8> = ds
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Heading { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(document_node_levels, vec![1, 2, 6]);

    let elements = crate::extraction::transform::convert_internal_elements_to_elements(&doc_for_elements, &None);
    let element_levels: Vec<u8> = elements
        .iter()
        .filter_map(|e| e.metadata.additional.get("heading_level"))
        .map(|level| level.parse::<u8>().expect("heading_level must be a decimal string"))
        .collect();

    assert_eq!(
        element_levels, document_node_levels,
        "elements' heading_level must agree with document.nodes' NodeContent::Heading level"
    );
}

#[test]
fn test_group_end_closes_layout_group_beneath_heading() {
    let mut doc = make_doc("pdf");
    doc.push_element(InternalElement::text(ElementKind::GroupStart, "", 0));
    doc.push_element(InternalElement::text(
        ElementKind::Heading { level: 1 },
        "Region heading",
        1,
    ));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Region body", 2));
    doc.push_element(InternalElement::text(ElementKind::GroupEnd, "", 0));
    doc.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 1));
    doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 1));
    doc.push_element(InternalElement::text(ElementKind::GroupStart, "", 1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Next region", 2));
    doc.push_element(InternalElement::text(ElementKind::GroupEnd, "", 1));

    resolve_relationships(&mut doc);
    let ds = derive_document_structure_inner(&mut doc);
    assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

    let roots: Vec<_> = ds.body_roots().collect();
    assert_eq!(roots.len(), 4);
    assert!(matches!(
        &roots[0].1.content,
        NodeContent::Group {
            heading_level: None,
            ..
        }
    ));
    assert!(matches!(&roots[1].1.content, NodeContent::Table { .. }));
    assert!(matches!(&roots[2].1.content, NodeContent::PageBreak));
    assert!(matches!(
        &roots[3].1.content,
        NodeContent::Group {
            heading_level: None,
            ..
        }
    ));

    let first_group = &ds.nodes[roots[0].0.0 as usize];
    assert_eq!(first_group.children.len(), 1);
    let heading_group = &ds.nodes[first_group.children[0].0 as usize];
    assert!(matches!(
        &heading_group.content,
        NodeContent::Group {
            heading_level: Some(1),
            ..
        }
    ));

    let next_group = &ds.nodes[roots[3].0.0 as usize];
    assert_eq!(next_group.children.len(), 1);
    assert!(matches!(
        &ds.nodes[next_group.children[0].0 as usize].content,
        NodeContent::Paragraph { .. }
    ));
}

#[test]
fn test_relationship_resolution() {
    let mut doc = make_doc("markdown");

    doc.push_element(InternalElement::text(ElementKind::Paragraph, "See note [^fn1].", 0));

    doc.push_element(InternalElement::text(ElementKind::FootnoteRef, "fn1", 0).with_anchor("fn1"));

    doc.push_element(
        InternalElement::text(ElementKind::FootnoteDefinition, "This is the footnote.", 0).with_anchor("fn1"),
    );

    doc.push_relationship(Relationship {
        source: 1,
        target: RelationshipTarget::Key("fn1".to_string()),
        kind: RelationshipKind::FootnoteReference,
    });

    resolve_relationships(&mut doc);

    match &doc.relationships[0].target {
        RelationshipTarget::Index(idx) => assert_eq!(*idx, 2),
        RelationshipTarget::Key(k) => panic!("Expected resolved Index, got Key({:?})", k),
    }
}

#[test]
fn test_unresolvable_key_left_as_key() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Ref.", 0));

    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Key("nonexistent".to_string()),
        kind: RelationshipKind::InternalLink,
    });

    resolve_relationships(&mut doc);

    assert!(matches!(
        &doc.relationships[0].target,
        RelationshipTarget::Key(k) if k == "nonexistent"
    ));
}

#[test]
fn test_relationships_in_document_structure() {
    let mut doc = make_doc("markdown");

    doc.push_element(InternalElement::text(ElementKind::Paragraph, "See note.", 0));
    doc.push_element(InternalElement::text(ElementKind::FootnoteDefinition, "The note.", 0).with_anchor("fn1"));

    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Index(1),
        kind: RelationshipKind::FootnoteReference,
    });

    resolve_relationships(&mut doc);
    let ds = derive_document_structure_inner(&mut doc);
    assert!(ds.validate().is_ok());
    assert_eq!(ds.relationships.len(), 1);
    assert_eq!(ds.relationships[0].kind, RelationshipKind::FootnoteReference);
}

/// Regression test for #74: an unresolvable relationship key used to disappear at
/// `log::debug!` only, so a cross-reference or citation whose target was never
/// extracted vanished from `DocumentStructure` with no diagnostic at all — the
/// caller could not distinguish "this document has no cross-references" from
/// "this document's cross-references were silently dropped".
#[test]
fn should_warn_when_a_relationship_key_cannot_be_resolved() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "See [ref].", 0));
    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Key("missing-anchor".to_string()),
        kind: RelationshipKind::CrossReference,
    });

    resolve_relationships(&mut doc);

    assert_eq!(
        doc.processing_warnings.len(),
        1,
        "one warning per document, not per key"
    );
    let warning = &doc.processing_warnings[0];
    assert_eq!(warning.source, "relationships");
    assert_eq!(
        warning.message,
        "1 cross-reference target(s) could not be resolved and were dropped from the \
         document structure: missing-anchor"
    );
}

/// A resolvable key must stay silent — the warning above is only meaningful if the
/// common case does not also emit it.
#[test]
fn should_not_warn_when_every_relationship_key_resolves() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "See note.", 0));
    doc.push_element(InternalElement::text(ElementKind::FootnoteDefinition, "The note.", 0).with_anchor("fn1"));
    doc.push_relationship(Relationship {
        source: 0,
        target: RelationshipTarget::Key("fn1".to_string()),
        kind: RelationshipKind::FootnoteReference,
    });

    resolve_relationships(&mut doc);

    assert!(
        doc.processing_warnings.is_empty(),
        "a resolvable key must not warn; got {:?}",
        doc.processing_warnings
    );
    assert_eq!(doc.relationships[0].target, RelationshipTarget::Index(1));
}

#[test]
fn test_list_container() {
    let mut doc = make_doc("markdown");
    doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
    doc.push_element(InternalElement::text(
        ElementKind::ListItem { ordered: false },
        "Item A",
        1,
    ));
    doc.push_element(InternalElement::text(
        ElementKind::ListItem { ordered: false },
        "Item B",
        1,
    ));
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));

    resolve_relationships(&mut doc);
    let ds = derive_document_structure_inner(&mut doc);
    assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

    let roots: Vec<_> = ds.body_roots().collect();
    assert_eq!(roots.len(), 1);
    assert!(matches!(&roots[0].1.content, NodeContent::List { ordered: false }));

    assert_eq!(ds.nodes[roots[0].0.0 as usize].children.len(), 2);
}

#[test]
fn native_table_geometry_is_discarded_after_cells_change() {
    let mut table = crate::types::Table {
        cells: vec![vec!["Group".into(), "".into()]],
        ..Default::default()
    };
    let mut grid = super::table_to_grid(&table, None);
    grid.cells.truncate(1);
    grid.cells[0].col_span = 2;
    assert_eq!(super::table_to_grid(&table, Some(&grid)).cells[0].col_span, 2);
    table.cells[0][1] = "Separate cell".into();
    assert_eq!(super::table_to_grid(&table, Some(&grid)).cells[0].col_span, 1);
    table.cells[0][1].clear();
    table.cells[0][0] = "Edited".into();
    assert_eq!(super::table_to_grid(&table, Some(&grid)).cells[0].content, "Edited");
    assert_eq!(super::table_to_grid(&table, Some(&grid)).cells[0].col_span, 1);
}

/// The words that OCR read from the picture of the page-building tests below.
const PICTURE_WORDS: &str = "Crate 17 holds forty blue lanterns";

/// A document with no prebuilt pages: page 1 has a paragraph, then a picture whose image
/// element has the text `element_text` and whose OCR text is `ocr_text`.
fn page_with_a_picture(element_text: &str, ocr_text: &str) -> InternalDocument {
    let mut doc = make_doc("ppt");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Stock list", 0).with_page(1));
    doc.images.push(crate::types::ExtractedImage {
        ocr_result: Some(Box::new(crate::types::ExtractedDocument {
            content: ocr_text.to_string(),
            ..Default::default()
        })),
        ..Default::default()
    });
    doc.push_element(InternalElement::text(ElementKind::Image { image_index: 0 }, element_text, 0).with_page(1));
    doc
}

/// The only page of `doc`, built for a document whose `content` is the plain rendering.
fn only_page(doc: &InternalDocument) -> crate::types::page::PageContent {
    let mut pages = build_pages(doc, true).expect("the elements have a page");
    assert_eq!(pages.len(), 1);
    pages.remove(0)
}

/// The content of the only page of `doc` derived for `output_format`, with the picture's words
/// counted in `content` first: `(words in content, page content)`.
fn words_in_content_and_page_content(
    doc: InternalDocument,
    output_format: crate::core::config::OutputFormat,
) -> (usize, String) {
    let result = derive_extraction_result(doc, false, output_format);
    let mut pages = result.pages.expect("the elements have a page");
    assert_eq!(pages.len(), 1);
    (result.content.matches(PICTURE_WORDS).count(), pages.remove(0).content)
}

#[test]
fn build_pages_adds_the_ocr_text_of_a_page_tagged_picture() {
    let page = only_page(&page_with_a_picture("", PICTURE_WORDS));

    assert_eq!(page.content, format!("Stock list\n\n{PICTURE_WORDS}"));
    assert_eq!(page.image_indices, vec![0]);
}

#[test]
fn build_pages_gives_a_page_with_only_a_picture_the_ocr_text() {
    let mut doc = page_with_a_picture("", PICTURE_WORDS);
    doc.elements.remove(0);

    assert_eq!(only_page(&doc).content, PICTURE_WORDS);
}

#[test]
fn build_pages_puts_the_ocr_text_after_the_text_of_the_picture_element() {
    let page = only_page(&page_with_a_picture("A label on a crate", PICTURE_WORDS));

    assert_eq!(
        page.content,
        format!("Stock list\n\nA label on a crate\n\n{PICTURE_WORDS}")
    );
}

#[test]
fn build_pages_adds_the_ocr_text_one_time_when_the_picture_element_holds_it() {
    let page = only_page(&page_with_a_picture(PICTURE_WORDS, PICTURE_WORDS));

    assert_eq!(page.content, format!("Stock list\n\n{PICTURE_WORDS}"));
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
#[test]
fn build_pages_skips_the_ocr_text_of_a_picture_that_page_ocr_replaced() {
    let shown = page_with_a_picture("", PICTURE_WORDS);
    let mut replaced = page_with_a_picture("", PICTURE_WORDS);
    replaced.elements[1].suppress_image_ocr_rendering();

    assert_eq!(only_page(&shown).content.matches(PICTURE_WORDS).count(), 1);
    let page = only_page(&replaced);
    assert_eq!(page.content, "Stock list");
    assert_eq!(page.image_indices, vec![0]);
}

#[test]
fn build_pages_adds_no_separator_for_a_picture_without_text() {
    let page = only_page(&page_with_a_picture("", ""));

    assert_eq!(page.content, "Stock list");
    assert_eq!(page.image_indices, vec![0]);
}

#[test]
fn a_picture_with_empty_ocr_text_adds_no_empty_block_to_page_content_or_to_plain_content() {
    let mut doc = page_with_a_picture("A label on a crate", "");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, "Bay plan", 0).with_page(1));

    assert_eq!(only_page(&doc).content, "Stock list\n\nA label on a crate\n\nBay plan");
    let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
    assert_eq!(result.content, "Stock list\n\nBay plan");
}

#[test]
fn build_pages_adds_no_picture_text_when_the_content_is_not_the_plain_rendering() {
    let mut doc = page_with_a_picture("", PICTURE_WORDS);
    doc.ocr_text_only = true;
    doc.append_ocr_text = true;

    let pages = build_pages(&doc, false).expect("the elements have a page");

    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].content, "Stock list");
    assert_eq!(pages[0].image_indices, vec![0]);
}

#[test]
fn page_content_of_json_output_has_the_text_of_the_elements_only() {
    let mut doc = page_with_a_picture("", PICTURE_WORDS);
    doc.ocr_text_only = true;
    doc.append_ocr_text = true;

    let (_, page_content) = words_in_content_and_page_content(doc, crate::core::config::OutputFormat::Json);

    assert_eq!(page_content, "Stock list");
}

#[test]
fn plain_content_and_plain_page_content_have_the_ocr_text_of_a_picture_once_with_both_settings_off() {
    let doc = page_with_a_picture("", PICTURE_WORDS);
    assert!(!doc.ocr_text_only && !doc.append_ocr_text);

    let (words_in_content, page_content) =
        words_in_content_and_page_content(doc, crate::core::config::OutputFormat::Plain);

    assert_eq!(words_in_content, 1);
    assert_eq!(page_content, format!("Stock list\n\n{PICTURE_WORDS}"));
}

#[test]
fn the_document_for_a_registered_renderer_has_plain_content_and_page_content_without_the_picture_text() {
    let doc = page_with_a_picture("", PICTURE_WORDS);
    assert!(!doc.ocr_text_only && !doc.append_ocr_text);

    let result = derive_renderer_input(doc);

    assert_eq!(result.content.matches(PICTURE_WORDS).count(), 1);
    assert!(result.document.is_some(), "the renderer gets the document structure");
    let mut pages = result.pages.expect("the elements have a page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages.remove(0).content, "Stock list");
}

#[test]
fn a_custom_format_name_with_no_renderer_gives_plain_page_content_with_the_ocr_text_of_a_picture() {
    let doc = page_with_a_picture("", PICTURE_WORDS);
    assert!(!doc.ocr_text_only && !doc.append_ocr_text);
    let no_renderer = crate::core::config::OutputFormat::Custom("no-such-renderer".to_string());

    let (words_in_content, page_content) = words_in_content_and_page_content(doc, no_renderer);

    assert_eq!(words_in_content, 1);
    assert_eq!(page_content, format!("Stock list\n\n{PICTURE_WORDS}"));
}
