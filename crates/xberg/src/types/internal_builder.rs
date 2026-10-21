//! Ergonomic builder for constructing `InternalDocument` instances.
//!
//! Mirrors the API of `DocumentStructureBuilder`
//! but outputs the flat `InternalDocument` representation
//! instead of a tree-based `DocumentStructure`.
//!
//! # Example
//!
//! ```rust
//! use xberg::types::internal_builder::InternalDocumentBuilder;
//!
//! let mut b = InternalDocumentBuilder::new("markdown");
//! b.push_heading(1, "Introduction", None, None);
//! b.push_paragraph("First paragraph.", vec![], None, None);
//! b.push_heading(2, "Details", None, None);
//! b.push_paragraph("Sub-section content.", vec![], None, None);
//! let doc = b.build();
//! assert_eq!(doc.elements.len(), 4);
//! ```

#![allow(dead_code)]

use ahash::AHashMap;

use super::document_structure::{ContentLayer, TextAnnotation};
use super::extraction::BoundingBox;
use super::internal::{
    ElementKind, InternalDocument, InternalElement, InternalElementId, Relationship, RelationshipKind,
    RelationshipTarget,
};
use super::metadata::Metadata;
use super::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElementLevel, OcrRotation};
use super::tables::Table;
use crate::types::ExtractedImage;
use crate::types::annotations::PdfAnnotation;
use crate::types::extraction::ProcessingWarning;

/// Builder for constructing `InternalDocument` with an ergonomic push-based API.
///
/// Tracks nesting depth automatically for list and quote containers,
/// and generates deterministic element IDs via blake3 hashing.
#[cfg_attr(alef, alef(skip))]
pub struct InternalDocumentBuilder {
    doc: InternalDocument,
    depth: u16,
    node_count: u32,
}

impl InternalDocumentBuilder {
    /// Create a new builder for the given source format.
    pub fn new(source_format: impl Into<String>) -> Self {
        Self {
            doc: InternalDocument::new(source_format),
            depth: 0,
            node_count: 0,
        }
    }

    /// Set the source format identifier (e.g. "docx", "html", "pptx").
    pub fn source_format(&mut self, format: impl Into<String>) {
        self.doc.source_format = format.into();
    }

    /// Set document-level metadata.
    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.doc.metadata = metadata;
    }

    /// Set the MIME type of the source document.
    pub fn set_mime_type(&mut self, mime_type: impl Into<String>) {
        self.doc.mime_type = mime_type.into();
    }

    /// Add a non-fatal processing warning.
    pub fn add_warning(&mut self, warning: ProcessingWarning) {
        self.doc.processing_warnings.push(warning);
    }

    /// Set document-level PDF annotations (links, highlights, notes).
    pub fn set_pdf_annotations(&mut self, annotations: Vec<PdfAnnotation>) {
        self.doc.annotations = Some(annotations);
    }

    /// Push a URI discovered during extraction.
    pub fn push_uri(&mut self, uri: super::uri::ExtractedUri) {
        self.doc.push_uri(uri);
    }

    /// Consume the builder and return the constructed `InternalDocument`.
    pub fn build(self) -> InternalDocument {
        if self.depth != 0 {
            log::warn!("Unclosed containers: depth is {} at build time", self.depth);
        }
        self.doc
    }

    /// Push a heading element.
    ///
    /// Auto-sets depth from the heading level and generates an anchor slug
    /// from the heading text.
    pub fn push_heading(&mut self, level: u8, text: &str, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        let anchor = slugify(text);
        let kind = ElementKind::Heading { level };
        let depth = std::cmp::max(self.depth, level.saturating_sub(1) as u16);
        let elem = self.make_element(kind, text, depth, page, bbox, Some(&anchor));
        self.doc.push_element(elem)
    }

    /// Push a heading whose semantic level is relative to the currently open
    /// structural containers.
    ///
    /// Layout-region groups use builder depth for their own nesting. Adding the
    /// heading level to that depth preserves H1/H2/H3 relationships inside the
    /// region instead of flattening them at the container depth.
    pub(crate) fn push_heading_in_current_container(
        &mut self,
        level: u8,
        text: &str,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let anchor = slugify(text);
        let kind = ElementKind::Heading { level };
        let depth = self.depth.saturating_add(level.saturating_sub(1) as u16);
        let elem = self.make_element(kind, text, depth, page, bbox, Some(&anchor));
        self.doc.push_element(elem)
    }

    /// Push a paragraph element.
    pub fn push_paragraph(
        &mut self,
        text: &str,
        annotations: Vec<TextAnnotation>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        self.push_simple(ElementKind::Paragraph, text, page, bbox, annotations, None, None)
    }

    /// Push a `ListStart` marker and increment depth.
    pub fn push_list(&mut self, ordered: bool) {
        self.push_container_start(ElementKind::ListStart { ordered }, None);
    }

    /// Push a `ListEnd` marker and decrement depth.
    pub fn end_list(&mut self) {
        self.push_container_end(ElementKind::ListEnd);
    }

    /// Push a list item element at the current depth.
    pub fn push_list_item(
        &mut self,
        text: &str,
        ordered: bool,
        annotations: Vec<TextAnnotation>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let kind = ElementKind::ListItem { ordered };
        self.push_simple(kind, text, page, bbox, annotations, None, None)
    }

    /// Push a table element. The table data is stored separately in
    /// `InternalDocument::tables` and referenced by index.
    pub fn push_table(&mut self, table: Table, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        let table_index = self.doc.push_table(table);
        let kind = ElementKind::Table { table_index };
        self.push_simple(kind, "", page, bbox, Vec::new(), None, None)
    }

    /// Push a table element from a 2D cell grid, building a `Table` struct automatically.
    ///
    /// `bbox` lands on both the element *and* the `Table` DTO: the DTO's
    /// `bounding_box` was previously hard-coded to `None`, so every table built
    /// through this path was published to consumers without its geometry even
    /// when the caller knew it (xberg-io/xberg#240).
    pub fn push_table_from_cells(
        &mut self,
        cells: &[Vec<String>],
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        self.push_table_from_cells_with_styles(cells, &[], page, bbox)
    }

    /// Push a table whose cells carry resolved paragraph styles, so a heading inside a cell
    /// stays discoverable (GH#1587).
    ///
    /// `cell_styles` is sparse: only cells that actually declare a style appear. Everything else
    /// behaves exactly as [`Self::push_table_from_cells`], which delegates here with an empty
    /// list, so no existing caller changes shape. ~keep
    pub fn push_table_from_cells_with_styles(
        &mut self,
        cells: &[Vec<String>],
        cell_styles: &[crate::types::TableCellStyle],
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        // A cell keeps its equation in the cell, so the equation cannot also be
        // an element without emptying the cell or repeating it beside the table.
        // The formula list is still expected to hold every formula in the
        // document, so the cell's math is recorded instead.
        //
        // A grid is padded before it reaches here: a cell spanning columns is
        // repeated across them, and a vertically merged cell is copied down from
        // the row above. Both produce a cell identical to its left neighbour or
        // to the cell above it, so those copies are skipped and the equation is
        // recorded once.
        for (row_index, row) in cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                let repeats_left = col_index > 0 && row[col_index - 1] == *cell;
                let repeats_above =
                    row_index > 0 && cells[row_index - 1].get(col_index).is_some_and(|above| above == cell);
                if repeats_left || repeats_above {
                    continue;
                }
                for latex in display_math_spans(cell) {
                    self.record_formula(&latex, page);
                }
            }
        }

        let markdown = crate::rendering::common::render_table_markdown(cells);
        let table = Table {
            cells: cells.to_vec(),
            markdown,
            page_number: page.unwrap_or(0),
            bounding_box: bbox,
            cell_styles: cell_styles.to_vec(),
            ..Default::default()
        };
        self.push_table(table, page, bbox)
    }

    /// Push an image element. The image data is stored separately in
    /// `InternalDocument::images` and referenced by index.
    pub fn push_image(
        &mut self,
        description: Option<&str>,
        image: ExtractedImage,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let image_index = self.doc.push_image(image);
        let kind = ElementKind::Image { image_index };
        let text = description.unwrap_or("");
        self.push_simple(kind, text, page, bbox, Vec::new(), None, None)
    }

    /// Push a code block element. Language is stored in attributes.
    pub fn push_code(
        &mut self,
        text: &str,
        language: Option<&str>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let attrs = language.map(|lang| single_attr("language", lang));
        self.push_simple(ElementKind::Code, text, page, bbox, Vec::new(), attrs, None)
    }

    /// Record a formula that stays inside the text it came from.
    ///
    /// An equation in a table cell or spliced into a sentence cannot become an
    /// element without changing that text, so it goes to the side channel. The
    /// derivation step appends the side channel to the public formula list, so a
    /// caller still sees every formula the document holds.
    pub fn record_formula(&mut self, latex: &str, page: Option<u32>) {
        let latex = latex.trim();
        if latex.is_empty() {
            return;
        }
        self.doc.recorded_formulas.push(crate::types::Formula {
            latex: latex.to_string(),
            bbox: None,
            page,
        });
    }

    /// Push a math formula element.
    pub fn push_formula(&mut self, text: &str, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        self.push_simple(ElementKind::Formula, text, page, bbox, Vec::new(), None, None)
    }

    /// Push a footnote reference marker.
    ///
    /// Creates a `FootnoteRef` element with `anchor = key` and also records
    /// a `Relationship` with `RelationshipTarget::Key(key)` so the derivation
    /// step can resolve it to the definition.
    pub fn push_footnote_ref(&mut self, marker: &str, key: &str, page: Option<u32>) -> u32 {
        let idx = self.push_simple(
            ElementKind::FootnoteRef,
            marker,
            page,
            None,
            Vec::new(),
            None,
            Some(key),
        );
        self.doc.push_relationship(Relationship {
            source: idx,
            target: RelationshipTarget::Key(key.to_string()),
            kind: RelationshipKind::FootnoteReference,
        });
        idx
    }

    /// Push a footnote definition element with `anchor = key`.
    ///
    /// A definition is note furniture, not body flow, so it is placed on
    /// [`ContentLayer::Footnote`], where the plain-text and JSON renderers
    /// collect definitions. On the body layer the plain-text renderer wrote the
    /// reference's `[^n]` marker but never the definition it points at.
    pub fn push_footnote_definition(&mut self, text: &str, key: &str, page: Option<u32>) -> u32 {
        let idx = self.push_simple(
            ElementKind::FootnoteDefinition,
            text,
            page,
            None,
            Vec::new(),
            None,
            Some(key),
        );
        self.set_layer(idx, ContentLayer::Footnote);
        idx
    }

    /// Push a comment reference marker (e.g. a DOCX reviewer comment anchor).
    ///
    /// Creates a `CommentRef` element with `anchor = key` and also records
    /// a `Relationship` with `RelationshipTarget::Key(key)` so the derivation
    /// step can resolve it to the definition. Mirrors [`Self::push_footnote_ref`]
    /// but keeps comments distinguishable from footnotes in `NodeContent`
    /// (xberg-io/xberg#300).
    pub fn push_comment_ref(&mut self, marker: &str, key: &str, page: Option<u32>) -> u32 {
        let idx = self.push_simple(ElementKind::CommentRef, marker, page, None, Vec::new(), None, Some(key));
        self.doc.push_relationship(Relationship {
            source: idx,
            target: RelationshipTarget::Key(key.to_string()),
            kind: RelationshipKind::FootnoteReference,
        });
        idx
    }

    /// Push a comment definition element with `anchor = key`.
    ///
    /// Mirrors [`Self::push_footnote_definition`]; see [`Self::push_comment_ref`].
    pub fn push_comment_definition(&mut self, text: &str, key: &str, page: Option<u32>) -> u32 {
        self.push_simple(
            ElementKind::CommentDefinition,
            text,
            page,
            None,
            Vec::new(),
            None,
            Some(key),
        )
    }

    /// Push a citation / bibliographic reference element.
    pub fn push_citation(&mut self, text: &str, key: &str, page: Option<u32>) -> u32 {
        self.push_simple(ElementKind::Citation, text, page, None, Vec::new(), None, Some(key))
    }

    /// Push a `QuoteStart` marker and increment depth.
    pub fn push_quote_start(&mut self) {
        self.push_container_start(ElementKind::QuoteStart, None);
    }

    /// Push a `QuoteEnd` marker and decrement depth.
    pub fn push_quote_end(&mut self) {
        self.push_container_end(ElementKind::QuoteEnd);
    }

    /// Push a page break marker at depth 0.
    pub fn push_page_break(&mut self) {
        self.push_page_break_with_page(None);
    }

    pub(crate) fn push_page_break_with_page(&mut self, page: Option<u32>) {
        let elem = self.make_element(ElementKind::PageBreak, "", 0, page, None, None);
        self.doc.push_element(elem);
    }

    /// Push a slide element.
    pub fn push_slide(&mut self, number: u32, title: Option<&str>, page: Option<u32>) -> u32 {
        let kind = ElementKind::Slide { number };
        let text = title.unwrap_or("");
        let attrs = title.map(|t| single_attr("title", t));
        self.push_simple(kind, text, page, None, Vec::new(), attrs, None)
    }

    /// Push an admonition / callout element (note, warning, tip, etc.).
    /// Kind and optional title are stored in attributes.
    pub fn push_admonition(&mut self, kind: &str, title: Option<&str>, page: Option<u32>) -> u32 {
        let mut attrs = AHashMap::new();
        attrs.insert("kind".to_string(), kind.to_string());
        if let Some(t) = title {
            attrs.insert("title".to_string(), t.to_string());
        }
        let text = title.unwrap_or(kind);
        self.push_simple(ElementKind::Admonition, text, page, None, Vec::new(), Some(attrs), None)
    }

    /// Push a raw block preserved verbatim. Format is stored in attributes.
    pub fn push_raw_block(&mut self, format: &str, content: &str, page: Option<u32>) -> u32 {
        self.push_simple(
            ElementKind::RawBlock,
            content,
            page,
            None,
            Vec::new(),
            Some(single_attr("format", format)),
            None,
        )
    }

    /// Push a structured metadata block (frontmatter, email headers).
    /// Entries are stored in attributes.
    pub fn push_metadata_block(&mut self, entries: &[(String, String)], page: Option<u32>) -> u32 {
        let attrs: AHashMap<String, String> = entries.iter().cloned().collect();
        let text = entries
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.push_simple(
            ElementKind::MetadataBlock,
            &text,
            page,
            None,
            Vec::new(),
            Some(attrs),
            None,
        )
    }

    /// Push a title element.
    pub fn push_title(&mut self, text: &str, page: Option<u32>, bbox: Option<BoundingBox>) -> u32 {
        self.push_simple(ElementKind::Title, text, page, bbox, Vec::new(), None, None)
    }

    /// Push a definition term element.
    pub fn push_definition_term(&mut self, text: &str, page: Option<u32>) -> u32 {
        self.push_simple(ElementKind::DefinitionTerm, text, page, None, Vec::new(), None, None)
    }

    /// Push a definition description element.
    pub fn push_definition_description(&mut self, text: &str, page: Option<u32>) -> u32 {
        self.push_simple(
            ElementKind::DefinitionDescription,
            text,
            page,
            None,
            Vec::new(),
            None,
            None,
        )
    }

    /// Push an OCR text element with OCR-specific fields populated.
    #[allow(clippy::too_many_arguments)]
    pub fn push_ocr_text(
        &mut self,
        text: &str,
        level: OcrElementLevel,
        geometry: OcrBoundingGeometry,
        confidence: OcrConfidence,
        rotation: Option<OcrRotation>,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
    ) -> u32 {
        let kind = ElementKind::OcrText { level };
        let mut elem = self.make_element(kind, text, self.depth, page, bbox, None);
        elem.ocr_geometry = Some(geometry);
        elem.ocr_confidence = Some(confidence);
        elem.ocr_rotation = rotation;
        self.doc.push_element(elem)
    }

    /// Push a `GroupStart` marker and increment depth.
    pub fn push_group_start(&mut self, label: Option<&str>, page: Option<u32>) {
        let attrs = label.map(|l| single_attr("label", l));
        self.push_container_start_with_attrs(ElementKind::GroupStart, page, attrs);
    }

    /// Push a `GroupEnd` marker and decrement depth.
    pub fn push_group_end(&mut self) {
        self.push_container_end(ElementKind::GroupEnd);
    }

    /// Push a relationship between two elements.
    pub fn push_relationship(&mut self, source: u32, target: RelationshipTarget, kind: RelationshipKind) {
        self.doc.push_relationship(Relationship { source, target, kind });
    }

    /// Set the anchor on an already-pushed element.
    pub fn set_anchor(&mut self, index: u32, anchor: impl Into<String>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.anchor = Some(anchor.into());
        }
    }

    /// Set the content layer on an already-pushed element.
    pub fn set_layer(&mut self, index: u32, layer: ContentLayer) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.layer = layer;
        }
    }

    /// Set attributes on an already-pushed element.
    pub fn set_attributes(&mut self, index: u32, attributes: AHashMap<String, String>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.attributes = Some(attributes);
        }
    }

    /// Insert a single attribute on an already-pushed element, merging into any
    /// attributes already set instead of replacing them wholesale.
    ///
    /// Use this (rather than `set_attributes`) when other code may have already
    /// attached attributes to the same element index.
    pub fn merge_attribute(&mut self, index: u32, key: impl Into<String>, value: impl Into<String>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.attributes
                .get_or_insert_with(AHashMap::new)
                .insert(key.into(), value.into());
        }
    }

    /// Record the literal source list-marker text on an already-pushed list item.
    ///
    /// `normalize_list_text` strips the marker off the item's text so the text
    /// reads as content; without this the original label is lost and renderers
    /// can only synthesize a sequence position. That silently renumbers a
    /// document whose clauses are cross-referenced by their printed label.
    /// An empty label is ignored.
    ///
    pub fn set_list_item_source_label(&mut self, index: u32, label: impl Into<String>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.set_list_item_source_label(label);
        }
    }

    /// Record the measured dominant font size (in points) for an already-pushed
    /// element. An out-of-range index is silently ignored.
    ///
    /// `pdf`-gated to match `InternalElement::set_measured_font_size`, which it wraps:
    /// the sole caller is the PDF structure assembly, and an ungated wrapper would fail
    /// to compile under a bare `ocr-pipeline` build.
    #[cfg(feature = "pdf")]
    pub fn set_measured_font_size(&mut self, index: u32, font_size_pt: f32) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.set_measured_font_size(font_size_pt);
        }
    }

    /// Set annotations on an already-pushed element.
    pub fn set_annotations(&mut self, index: u32, annotations: Vec<TextAnnotation>) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.annotations = annotations;
        }
    }

    /// Set the text content of an already-pushed element.
    pub fn set_text(&mut self, index: u32, text: &str) {
        if let Some(elem) = self.doc.elements.get_mut(index as usize) {
            elem.text = text.into();
        }
    }

    /// Push a pre-constructed `InternalElement` directly.
    ///
    /// Useful when the caller needs to construct an element with fields
    /// that the builder's convenience methods don't cover (e.g. an image
    /// element without `ExtractedImage` data).
    pub fn push_element(&mut self, element: InternalElement) -> u32 {
        self.node_count += 1;
        self.doc.push_element(element)
    }

    pub(crate) fn element_count(&self) -> u32 {
        self.doc.elements.len() as u32
    }

    /// Insert a pre-constructed `InternalElement` immediately before the element
    /// currently at `index`, shifting every later element index by one.
    ///
    /// Used to place out-of-band content (e.g. an image detected mid-page) into
    /// its correct reading-order position after surrounding elements have already
    /// been pushed. Any existing `Relationship::source` or
    /// `RelationshipTarget::Index` that pointed at or past `index` is bumped so
    /// caption and other relationships keep pointing at the same logical element.
    ///
    /// `index` is clamped to the current element count, so passing an
    /// out-of-range index behaves like [`Self::push_element`].
    pub fn insert_element_before(&mut self, index: u32, element: InternalElement) -> u32 {
        let index = (index as usize).min(self.doc.elements.len()) as u32;
        self.doc.elements.insert(index as usize, element);
        self.node_count += 1;

        for relationship in &mut self.doc.relationships {
            if relationship.source >= index {
                relationship.source += 1;
            }
            if let RelationshipTarget::Index(target) = &mut relationship.target
                && *target >= index
            {
                *target += 1;
            }
        }

        index
    }

    /// Append every element from a sub-document into this builder in reading order.
    ///
    /// Tables, images, element relationships, and URIs are carried over, with
    /// `table_index` / `image_index` and relationship element indices remapped into
    /// this document's index space. Used to embed a fragment parsed by another
    /// extractor (e.g. a Jupyter markdown cell parsed by the Markdown extractor)
    /// without re-implementing its element construction. Document-level metadata on
    /// `other` is ignored — only content is merged.
    pub fn append_document(&mut self, other: InternalDocument) {
        let table_offset = self.doc.tables.len() as u32;
        let image_offset = self.doc.images.len() as u32;
        let element_offset = self.doc.elements.len() as u32;

        self.doc.tables.extend(other.tables);
        self.doc.images.extend(other.images);
        self.doc.uris.extend(other.uris);
        self.doc.uris_dropped += other.uris_dropped;

        for mut element in other.elements {
            match &mut element.kind {
                ElementKind::Table { table_index } => *table_index += table_offset,
                ElementKind::Image { image_index } if *image_index != u32::MAX => *image_index += image_offset,
                _ => {}
            }
            self.node_count += 1;
            self.doc.push_element(element);
        }

        for mut relationship in other.relationships {
            relationship.source += element_offset;
            if let RelationshipTarget::Index(index) = &mut relationship.target {
                *index += element_offset;
            }
            self.doc.relationships.push(relationship);
        }
    }

    /// Push a container start marker and increment depth.
    fn push_container_start(&mut self, kind: ElementKind, page: Option<u32>) {
        self.push_simple(kind, "", page, None, Vec::new(), None, None);
        self.depth += 1;
    }

    /// Push a container start marker with attributes and increment depth.
    fn push_container_start_with_attrs(
        &mut self,
        kind: ElementKind,
        page: Option<u32>,
        attrs: Option<AHashMap<String, String>>,
    ) {
        self.push_simple(kind, "", page, None, Vec::new(), attrs, None);
        self.depth += 1;
    }

    /// Push a container end marker and decrement depth.
    fn push_container_end(&mut self, kind: ElementKind) {
        self.depth = self.depth.saturating_sub(1);
        self.push_simple(kind, "", None, None, Vec::new(), None, None);
    }

    /// Get the next index and increment the counter.
    fn next_index(&mut self) -> u32 {
        let idx = self.node_count;
        self.node_count += 1;
        idx
    }

    /// Create an `InternalElement` with common fields. All element construction
    /// is funnelled through this single site to reduce duplication.
    fn make_element(
        &mut self,
        kind: ElementKind,
        text: &str,
        depth: u16,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
        anchor: Option<&str>,
    ) -> InternalElement {
        let idx = self.next_index();
        InternalElement {
            id: InternalElementId::generate(kind.discriminant(), text, page, idx),
            kind,
            text: text.to_string(),
            depth,
            page,
            bbox,
            layer: ContentLayer::Body,
            annotations: Vec::new(),
            attributes: None,
            anchor: anchor.map(|s| s.to_string()),
            ocr_geometry: None,
            ocr_confidence: None,
            ocr_rotation: None,
        }
    }

    /// Push a simple element with the current depth.
    #[allow(clippy::too_many_arguments)]
    fn push_simple(
        &mut self,
        kind: ElementKind,
        text: &str,
        page: Option<u32>,
        bbox: Option<BoundingBox>,
        annotations: Vec<TextAnnotation>,
        attributes: Option<AHashMap<String, String>>,
        anchor: Option<&str>,
    ) -> u32 {
        let mut elem = self.make_element(kind, text, self.depth, page, bbox, anchor);
        elem.annotations = annotations;
        elem.attributes = attributes;
        self.doc.push_element(elem)
    }
}

/// Create a `HashMap` with a single key-value pair.
fn single_attr(key: &str, val: &str) -> AHashMap<String, String> {
    let mut m = AHashMap::with_capacity(1);
    m.insert(key.to_string(), val.to_string());
    m
}

/// Generate a URL-friendly anchor slug from heading text.
///
/// Single-pass, single-allocation: lowercases alphanumeric characters,
/// collapses non-alphanumeric runs into single hyphens, and trims
/// leading/trailing hyphens.
fn slugify(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_dash = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }
    result
}

const _: () = {
    #[allow(dead_code)]
    fn assert_send_sync<T: Send + Sync>() {}
    #[allow(dead_code)]
    fn _check() {
        assert_send_sync::<InternalDocumentBuilder>();
    }
};

/// Return the LaTeX of every `$$...$$` span in a table cell.
///
/// A cell renders its math delimited, so the delimiters mark where each formula
/// starts and ends. Inline `$...$` is left alone: a cell holding a price reads
/// the same as a cell holding one-character math, and inline math stays in its
/// sentence everywhere else.
pub(crate) fn display_math_spans(cell: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cell;
    while let Some(start) = rest.find("$$") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("$$") else { break };
        let latex = after[..end].trim();
        if !latex.is_empty() {
            out.push(latex.to_string());
        }
        rest = &after[end + 2..];
    }
    out
}

#[cfg(test)]
mod tests;
