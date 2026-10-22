//! Native DOC extractor for Word 97-2003 binary format.
//!
//! Extracts text directly from OLE/CFB compound documents without LibreOffice.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::mime::LEGACY_WORD_MIME_TYPE;
use crate::extraction::doc::{
    DocParagraph, DocPicture, DocSubdocument, DocSubdocumentKind, DocTableMark, DocWithImages, extract_doc_text,
    extract_doc_with_images,
};
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::document_structure::ContentLayer;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{Metadata, Table};
mod images;
use images::{parse_doc, push_doc_image, push_paragraph_runs};

use ahash::AHashMap;
use async_trait::async_trait;
use std::borrow::Cow;
#[cfg_attr(alef, alef(skip))]
/// Native DOC extractor using OLE/CFB parsing.
///
/// This extractor handles Word 97-2003 binary (.doc) files without
/// requiring LibreOffice, providing ~50x faster extraction.
pub struct DocExtractor;

impl DocExtractor {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for DocExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DocExtractor {
    fn name(&self) -> &str {
        "doc-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Native DOC text extraction via OLE/CFB parsing"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for DocExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let image_limits = config
            .needs_image_data()
            .then(|| config.security_limits.clone().unwrap_or_default());
        let extracted = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() || image_limits.is_some() {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::XbergError::Cancelled);
                }
                let content_owned = content.to_vec();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
                    let _guard = span.entered();
                    parse_doc(&content_owned, image_limits.as_ref())
                })
                .await
                .map_err(|e| crate::error::XbergError::parsing(format!("DOC extraction task failed: {e}")))?
            } else {
                parse_doc(content, image_limits.as_ref())
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                if config.cancel_token.as_ref().map(|t| t.is_cancelled()).unwrap_or(false) {
                    return Err(crate::error::XbergError::Cancelled);
                }
                parse_doc(content, image_limits.as_ref())
            }
        }?;

        let result = extracted.text;
        let pictures = extracted.pictures;
        let mut doc = InternalDocument::new("doc");
        doc.images = pictures.iter().map(|picture| picture.image.clone()).collect();
        doc.mime_type = mime_type.to_string();
        doc.processing_warnings.extend(result.processing_warnings);

        doc.metadata = build_metadata(result.metadata);

        // Elements follow Word's own paragraph structure, matching what the
        // DOCX path does with `w:p`. The blank-line fallback is only for
        // documents that carry no paragraph properties at all (Word 6/95, or
        // the contiguous fallback), where there is nothing finer to use; those
        // paths have no subdocuments either.
        if result.paragraphs.is_empty() && result.subdocuments.is_empty() && pictures.is_empty() {
            push_blank_line_chunks(&mut doc, &result.content);
        } else {
            let mut budget = SecurityBudget::from_config(config);
            push_paragraph_elements_with_images(
                &mut doc,
                &result.paragraphs,
                &pictures,
                config.wants_own_bytes_in_result(),
                &mut budget,
            )?;
            push_subdocuments(&mut doc, &result.subdocuments, config.content_filter.as_ref());
        }

        // As in the DOCX extractor, the content filter decides here whether
        // header and footer text is kept.
        if let Some(filter) = &config.content_filter {
            doc.elements.retain(|element| match element.layer {
                ContentLayer::Header => filter.include_headers,
                ContentLayer::Footer => filter.include_footers,
                ContentLayer::Footnote if element.kind == ElementKind::FootnoteDefinition => filter.include_footnotes,
                _ => true,
            });
        }

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[LEGACY_WORD_MIME_TYPE]
    }

    fn priority(&self) -> i32 {
        60
    }
}

/// Map the `.doc` module's metadata onto the public [`Metadata`] shape.
fn build_metadata(source: crate::extraction::doc::DocMetadata) -> Metadata {
    let mut additional = AHashMap::new();
    if let Some(revision) = source.revision_number {
        additional.insert(Cow::Borrowed("revision"), serde_json::Value::String(revision));
    }
    additional.insert(
        Cow::Borrowed("extraction_method"),
        serde_json::Value::String("native_ole".to_string()),
    );

    let (authors, created_by) = match source.author {
        Some(author) => (Some(vec![author.clone()]), Some(author)),
        None => (None, None),
    };

    Metadata {
        title: source.title,
        subject: source.subject,
        authors,
        created_by,
        modified_by: source.last_author,
        additional,
        ..Default::default()
    }
}

/// Whether a chunk looks like a heading, by shape rather than by style.
///
/// Used only for documents whose style sheet declares no heading style at all.
/// #1553 measured the alternative: deriving headings purely from `istd` would
/// have deleted all 13 headings from one reporter document and all 12 from
/// another, because both style their headings as bold `Normal`. Half this
/// corpus does. Shape is the only signal those documents carry. ~keep
fn looks_like_heading(text: &str, next: Option<&str>) -> bool {
    let is_single_line = !text.contains('\n');
    let is_short = text.len() <= 80;
    let no_trailing_punct = !text.ends_with('.') && !text.ends_with(':') && !text.ends_with(';');
    let next_is_longer = next.is_some_and(|next| !next.is_empty() && next.len() > text.len());
    is_single_line && is_short && no_trailing_punct && next_is_longer
}

/// Emit one element per blank-line-separated chunk.
///
/// Only reachable for documents that carry no paragraph properties -- Word
/// 6/95, and the contiguous fallback -- where Word's own paragraph boundaries
/// are not available. It merges any two paragraphs not separated by a blank
/// line, which is why it is no longer the main path.
fn push_blank_line_chunks(doc: &mut InternalDocument, content: &str) {
    let chunks: Vec<&str> = content.split("\n\n").collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }
        let next = chunks.get(i + 1).map(|next| next.trim());
        if looks_like_heading(trimmed, next.filter(|next| !next.is_empty())) {
            doc.push_element(InternalElement::text(ElementKind::Heading { level: 2 }, trimmed, 0));
        } else {
            doc.push_element(InternalElement::text(ElementKind::Paragraph, trimmed, 0));
        }
    }
}

/// Emit one element per Word paragraph, so a list-bound paragraph becomes a
/// `ListItem` inside a list container -- the shape the DOCX path already
/// produces for `w:numPr`.
///
/// Consecutive bound paragraphs share a container. A change of nesting depth
/// opens or closes nested containers, and a change of *kind* at the same depth
/// closes and reopens: a document can move from a numbered run straight into a
/// bulleted one at the same level, and merging those would label half the
/// items wrongly.
///
/// Consecutive table paragraphs become one `Table`, cell by cell and row by
/// row, the way the DOCX path reads `w:tbl`. ~keep
#[cfg(test)]
fn push_paragraph_elements(
    doc: &mut InternalDocument,
    paragraphs: &[DocParagraph],
    budget: &mut SecurityBudget,
) -> Result<()> {
    push_paragraph_elements_with_images(doc, paragraphs, &[], true, budget)
}

fn push_paragraph_elements_with_images(
    doc: &mut InternalDocument,
    paragraphs: &[DocParagraph],
    pictures: &[DocPicture],
    visible_images: bool,
    budget: &mut SecurityBudget,
) -> Result<()> {
    // The switch is "does this document EMIT styled headings", which is
    // narrower than either alternative that looks right.
    //
    // Not "does the style sheet define headings": nearly every Word style
    // sheet defines heading 1..9 whether or not the author applied one, so
    // that answers yes almost always.
    //
    // And not merely "does any paragraph carry a heading style": a list-bound
    // paragraph is emitted as a ListItem regardless of its style, matching the
    // DOCX path's handling of `w:numPr`. `simple.doc` is the case -- its one
    // Heading 1 paragraph is also list-bound, so counting it flipped the
    // document into styled mode and suppressed every heading while emitting
    // none, leaving the document with no heading structure at all. ~keep
    let styled_headings = paragraphs
        .iter()
        .any(|paragraph| paragraph.heading_level.is_some() && paragraph.list.is_none() && paragraph.table.is_none());
    // One entry per open container, holding whether it is ordered.
    let mut open: Vec<bool> = Vec::new();
    let mut table = PendingTable::default();
    let mut picture_cursor = 0;

    for (i, paragraph) in paragraphs.iter().enumerate() {
        let start = picture_cursor;
        while pictures
            .get(picture_cursor)
            .is_some_and(|picture| picture.paragraph_index == i)
        {
            picture_cursor += 1;
        }
        let paragraph_pictures = &pictures[start..picture_cursor];
        for picture in paragraph_pictures.iter().filter(|picture| picture.standalone) {
            table.flush(doc, budget)?;
            close_lists(doc, &mut open, 0);
            push_doc_image(doc, picture, visible_images);
        }
        let text = paragraph.content.trim();
        if let Some(mark) = paragraph.table {
            close_lists(doc, &mut open, 0);
            table.add(text, mark, budget)?;
            continue;
        }
        table.flush(doc, budget)?;
        if text.is_empty() {
            continue;
        }

        let Some(list) = paragraph.list else {
            close_lists(doc, &mut open, 0);
            let kind = match heading_kind(paragraphs, i, text, styled_headings) {
                Some(level) => ElementKind::Heading { level },
                None => ElementKind::Paragraph,
            };
            push_paragraph_runs(doc, kind, text, paragraph_pictures, visible_images);
            continue;
        };

        push_list_paragraph(doc, &mut open, list, text);
    }

    table.flush(doc, budget)?;
    close_lists(doc, &mut open, 0);
    for picture in &pictures[picture_cursor..] {
        push_doc_image(doc, picture, visible_images);
    }
    Ok(())
}

fn push_list_paragraph(
    doc: &mut InternalDocument,
    open: &mut Vec<bool>,
    list: crate::extraction::doc::DocListMembership,
    text: &str,
) {
    let depth = usize::from(list.level) + 1;
    close_lists(doc, open, depth);
    if open.len() == depth && open.last() != Some(&list.ordered) {
        close_lists(doc, open, depth - 1);
    }
    while open.len() < depth {
        let element_depth = u16::try_from(open.len()).unwrap_or(u16::MAX);
        doc.push_element(InternalElement::text(
            ElementKind::ListStart { ordered: list.ordered },
            "",
            element_depth,
        ));
        open.push(list.ordered);
    }

    let element_depth = u16::try_from(open.len()).unwrap_or(u16::MAX);
    doc.push_element(InternalElement::text(
        ElementKind::ListItem { ordered: list.ordered },
        text,
        element_depth,
    ));
}

/// Emit header, footer, footnote, comment and text-box text after the body, the
/// way the DOCX path does: headers and footers one paragraph per line on their
/// own layers, each footnote and comment as one definition on the Footnote layer
/// with its paragraphs joined by a space, and text boxes as body paragraphs.
fn push_subdocuments(
    doc: &mut InternalDocument,
    subdocuments: &[DocSubdocument],
    content_filter: Option<&crate::core::config::ContentFilterConfig>,
) {
    let mut footnotes = 0;
    let mut comments = 0;
    for subdocument in subdocuments {
        if subdocument.kind == DocSubdocumentKind::HeaderFooter
            && content_filter.is_some_and(|filter| !filter.include_headers || !filter.include_footers)
        {
            continue;
        }
        let (kind, layer, anchor) = match subdocument.kind {
            DocSubdocumentKind::Header | DocSubdocumentKind::HeaderFooter => {
                (ElementKind::Paragraph, ContentLayer::Header, None)
            }
            DocSubdocumentKind::Footer => (ElementKind::Paragraph, ContentLayer::Footer, None),
            DocSubdocumentKind::TextBox => (ElementKind::Paragraph, ContentLayer::Body, None),
            DocSubdocumentKind::Footnote => {
                footnotes += 1;
                let anchor = format!("fn{footnotes}");
                (ElementKind::FootnoteDefinition, ContentLayer::Footnote, Some(anchor))
            }
            DocSubdocumentKind::Comment => {
                comments += 1;
                let anchor = format!("cmt{comments}");
                (ElementKind::CommentDefinition, ContentLayer::Footnote, Some(anchor))
            }
        };
        let lines: Vec<&str> = subdocument
            .text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let texts = match anchor {
            Some(_) => vec![lines.join(" ")],
            None => lines.iter().map(|line| line.to_string()).collect(),
        };
        for text in texts {
            let mut element = InternalElement::text(kind, text, 0);
            element.layer = layer;
            element.anchor = anchor.clone();
            doc.push_element(element);
        }
    }
}

/// The rows of the table being read, until a paragraph outside it ends it.
#[derive(Default)]
struct PendingTable {
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    cell: String,
    columns: usize,
    reserved_cells: usize,
}

impl PendingTable {
    /// Add one table paragraph. A cell's paragraphs are joined by line breaks.
    fn add(&mut self, text: &str, mark: DocTableMark, budget: &mut SecurityBudget) -> Result<()> {
        if !text.is_empty() {
            if !self.cell.is_empty() {
                self.cell.push('\n');
            }
            self.cell.push_str(text);
        }
        if mark.cell_end {
            self.end_cell(budget)?;
        }
        if mark.row_end {
            self.end_row(budget)?;
        }
        Ok(())
    }

    fn end_cell(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        let grid = self
            .row
            .len()
            .checked_add(1)
            .map(|columns| columns.max(self.columns))
            .and_then(|columns| {
                self.rows
                    .len()
                    .checked_add(1)?
                    .checked_mul(columns)
                    .map(|cells| (columns, cells))
            });
        let (columns, cells) = grid.ok_or_else(|| crate::XbergError::security("DOC table cell count overflow"))?;
        // Markdown pads every row to the widest one, including rows already read. ~keep
        // Reserve that whole rectangle before allocating another cell. ~keep
        budget.add_cells(cells - self.reserved_cells)?;
        self.columns = columns;
        self.reserved_cells = cells;
        self.row.push(std::mem::take(&mut self.cell));
        Ok(())
    }

    fn end_row(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        if !self.cell.is_empty() {
            self.end_cell(budget)?;
        }
        if !self.row.is_empty() {
            self.rows.push(std::mem::take(&mut self.row));
        }
        Ok(())
    }

    /// Emit the table read so far, unless it holds no text, and start over.
    fn flush(&mut self, doc: &mut InternalDocument, budget: &mut SecurityBudget) -> Result<()> {
        self.end_row(budget)?;
        let cells = std::mem::take(&mut self.rows);
        self.columns = 0;
        self.reserved_cells = 0;
        if cells.iter().flatten().all(|cell| cell.trim().is_empty()) {
            return Ok(());
        }
        let markdown = crate::rendering::common::render_table_markdown(&cells);
        let table_index = doc.push_table(Table {
            cells,
            markdown,
            ..Default::default()
        });
        doc.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
        Ok(())
    }
}

/// Decide whether a paragraph is a heading, and at what level.
///
/// #1553: the two signals are not interchangeable and neither is usable alone.
/// A document that declares heading styles is taken at its word -- `istd` is
/// what Word itself renders from, and the shape heuristic invents headings
/// there (1 detected against 7 declared in one corpus document). A document
/// that declares none has nothing to be taken at its word about, and falls
/// back to shape.
///
/// The switch is per *document*, not per paragraph. Per-paragraph fallback
/// would re-add the invented headings alongside the declared ones, which is
/// the worst of both. ~keep
fn heading_kind(paragraphs: &[DocParagraph], index: usize, text: &str, styled_headings: bool) -> Option<u8> {
    if styled_headings {
        return paragraphs.get(index).and_then(|paragraph| paragraph.heading_level);
    }
    let next = paragraphs
        .get(index + 1)
        .map(|next| next.content.trim())
        .filter(|next| !next.is_empty());
    // The shape heuristic has no notion of depth; it only ever claimed h2.
    looks_like_heading(text, next).then_some(2)
}

/// Close open list containers until only `target` remain.
fn close_lists(doc: &mut InternalDocument, open: &mut Vec<bool>, target: usize) {
    while open.len() > target {
        open.pop();
        let element_depth = u16::try_from(open.len()).unwrap_or(u16::MAX);
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", element_depth));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::doc::tests::{
        PlcKind, SEPARATOR_STORIES, SyntheticDoc, build_synthetic_doc, header_doc_stories,
    };

    fn corpus(relative: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
        assert!(
            path.exists(),
            "corpus fixture missing at {}; fetch test_documents rather than skipping",
            path.display()
        );
        std::fs::read(&path).expect("read fixture")
    }

    async fn internal_document(relative: &str) -> InternalDocument {
        DocExtractor::new()
            .extract_content(&corpus(relative), LEGACY_WORD_MIME_TYPE, &ExtractionConfig::default())
            .await
            .expect("DOC extraction should succeed")
    }

    /// #1550: `ordered` distinguishes a numbered list from a bulleted one, and
    /// it is resolved from the list tables' `nfc` rather than assumed.
    ///
    /// A reader that never resolved `nfc` and fell back to a constant would
    /// emit one kind for everything and still look entirely plausible --
    /// twelve ordered lists and zero bulleted reads as a clean result, not as
    /// a lookup that never ran. That is precisely what happened while the
    /// `LVL` blocks were being sliced off at `lcbPlfLst`, because they live
    /// *past* that length.
    ///
    /// FIXTURE REQUIREMENT: this assertion can only fail if the document
    /// contains **both** kinds. `unit_test_lists.doc` carries `nfc` 0 (arabic)
    /// and `nfc` 23 (bullet). Swapping in a bullets-only or numbers-only
    /// document -- or "simplifying" this one -- disarms the guard silently: it
    /// keeps passing while `nfc` resolution is dead. The property that makes
    /// this test able to fail belongs to the corpus, not to the code, so it is
    /// recorded here next to the assertion that depends on it. ~keep
    #[tokio::test]
    async fn list_containers_carry_the_kind_resolved_from_nfc() {
        let doc = internal_document("../../test_documents/doc/unit_test_lists.doc").await;

        let mut ordered = 0;
        let mut bulleted = 0;
        for element in &doc.elements {
            if let ElementKind::ListStart { ordered: is_ordered } = element.kind {
                if is_ordered { ordered += 1 } else { bulleted += 1 }
            }
        }

        assert!(
            ordered > 0 && bulleted > 0,
            "the document mixes nfc 0 and nfc 23, so both container kinds must appear; \
             got {ordered} ordered and {bulleted} bulleted -- a single kind means nfc was \
             never read and a default was used"
        );
    }

    /// Every opened list container is closed, at the right nesting depth.
    #[tokio::test]
    async fn list_containers_are_balanced() {
        let doc = internal_document("../../test_documents/doc/unit_test_lists.doc").await;

        let mut depth: i32 = 0;
        for element in &doc.elements {
            match element.kind {
                ElementKind::ListStart { .. } => depth += 1,
                ElementKind::ListEnd => {
                    depth -= 1;
                    assert!(depth >= 0, "a list was closed that had not been opened");
                }
                _ => {}
            }
        }
        assert_eq!(depth, 0, "every opened list container must be closed");
    }

    /// #1553: a document that applies heading styles is taken at its word,
    /// and the level comes from the style rather than being a fixed h2.
    ///
    /// FIXTURE REQUIREMENT: `unit_test_lists.doc` applies `heading 1` and
    /// `heading 3` to seven paragraphs that are NOT list-bound. A fixture
    /// whose styled headings were all list-bound would emit them as ListItems
    /// and exercise the fallback instead, passing this file's other test while
    /// leaving this one asserting nothing. ~keep
    #[tokio::test]
    async fn a_document_that_applies_heading_styles_uses_them_and_their_levels() {
        let doc = internal_document("../../test_documents/doc/unit_test_lists.doc").await;

        let levels: Vec<u8> = doc
            .elements
            .iter()
            .filter_map(|element| match element.kind {
                ElementKind::Heading { level } => Some(level),
                _ => None,
            })
            .collect();

        assert_eq!(
            levels.len(),
            7,
            "expected the 7 style-declared headings; got {levels:?}"
        );
        assert!(
            levels.contains(&1) && levels.contains(&3),
            "levels must come from the styles actually applied (heading 1 and heading 3); got {levels:?}"
        );
        assert!(
            !levels.contains(&2),
            "h2 is what the shape heuristic emits for everything; seeing it here means the \
             heuristic ran instead of the styles: {levels:?}"
        );
    }

    /// #1553: a document whose only heading-styled paragraph is list-bound
    /// emits no styled heading at all, so it must fall back to shape rather
    /// than end up with no heading structure.
    ///
    /// `simple.doc` is exactly that: its `Heading 1` paragraph also carries a
    /// list binding, and a list binding wins (matching the DOCX path's
    /// handling of `w:numPr`). Counting it as "this document uses heading
    /// styles" suppressed the heuristic while emitting nothing in its place.
    #[tokio::test]
    async fn a_document_whose_only_styled_heading_is_list_bound_falls_back_to_shape() {
        let doc = internal_document("../../test_documents/vendored/unstructured/doc/simple.doc").await;

        let headings = doc
            .elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::Heading { .. }))
            .count();

        assert_eq!(
            headings, 3,
            "expected the shape heuristic's 3 headings; 0 means the document was treated as \
             style-declaring on the strength of a paragraph emitted as a ListItem"
        );
    }

    /// A document with no list bindings must keep the blank-line chunking it
    /// had before #1550 -- Word paragraphs are finer-grained than those chunks.
    #[tokio::test]
    async fn a_list_free_document_emits_no_list_elements() {
        let doc = internal_document("../../test_documents/vendored/unstructured/doc/fake.doc").await;

        assert!(
            !doc.elements.iter().any(|element| matches!(
                element.kind,
                ElementKind::ListStart { .. } | ElementKind::ListEnd | ElementKind::ListItem { .. }
            )),
            "a document with no list bindings must not gain list structure"
        );
    }

    #[tokio::test]
    async fn test_doc_extractor_plugin_interface() {
        let extractor = DocExtractor::new();
        assert_eq!(extractor.name(), "doc-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types(), &["application/msword"]);
    }

    #[tokio::test]
    async fn test_doc_extractor_initialize_shutdown() {
        let extractor = DocExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_doc_extractor_real_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/doc/simple.doc");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let extractor = DocExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_content(&content, "application/msword", &config)
            .await
            .expect("DOC extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(!result.content.is_empty(), "Should extract text from DOC");
        assert_eq!(&*result.mime_type, "application/msword");
    }

    #[tokio::test]
    async fn test_doc_document_structure_with_heuristic_headings() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/doc/simple.doc");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let extractor = DocExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_content(&content, "application/msword", &config)
            .await
            .expect("DOC extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(result.document.is_some(), "Should produce document structure for DOC");
        let doc = result.document.unwrap();
        assert!(!doc.nodes.is_empty(), "Document structure should have nodes");
    }

    /// GH#1460: the field result text must survive extraction while the field
    /// instruction is dropped, when run through the real `.doc` binary parser end
    /// to end.
    ///
    /// The nine unit tests for `normalize_doc_text` in
    /// `crate::extraction::doc::mod` only ever feed it a synthetic `&str` built
    /// from escape literals (`"\x13 HYPERLINK ... \x14 ... \x15"`). They prove the
    /// normalization function is correct in isolation, but they cannot catch a
    /// regression in the OLE/CFB parsing or piece-table walk that sits upstream of
    /// it — e.g. if the parser stopped delivering `0x13`/`0x14`/`0x15` bytes at
    /// all, or if extraction routed around `normalize_doc_text` entirely, every
    /// one of those tests would still pass. This test exercises a real vendored
    /// `.doc` file whose stream is known (via `grep -a`) to contain a literal
    /// `HYPERLINK "http://github.com/"` field instruction followed by the result
    /// text `A Link example`.
    ///
    /// The DOC extractor has no URI-extraction feature (no metadata field or
    /// extracted-links list surfaces hyperlink targets separately), so the
    /// instruction's URL is not expected to legitimately appear anywhere else in
    /// the output; asserting its absence from the text is safe.
    #[tokio::test]
    async fn should_drop_hyperlink_instruction_but_keep_result_text_when_extracting_real_doc() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/doc/fake-doc-emphasized-text.doc");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let extractor = DocExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_content(&content, "application/msword", &config)
            .await
            .expect("DOC extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

        assert!(
            result.content.contains("A Link example"),
            "field result text must be kept: {:?}",
            result.content
        );
        assert!(
            !result.content.contains("HYPERLINK"),
            "field instruction keyword must be dropped: {:?}",
            result.content
        );
        assert!(
            !result.content.contains("http://github.com/"),
            "field instruction URL must be dropped: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_doc_paragraph_mapping() {
        let test_file =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/doc/unit_test_lists.doc");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let extractor = DocExtractor::new();
        let config = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        let result = extractor
            .extract_content(&content, "application/msword", &config)
            .await
            .expect("DOC extraction failed");
        let result =
            crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
        assert!(result.document.is_some(), "Should produce document structure");
        let doc = result.document.unwrap();
        let has_paragraph = doc.nodes.iter().any(|n| {
            matches!(
                n.content,
                crate::types::document_structure::NodeContent::Paragraph { .. }
            )
        });
        assert!(has_paragraph, "DOC should produce Paragraph nodes");
    }

    // --- Subdocument stories a real file cannot show (#2054) ---
    //
    // `tests/doc_subdocuments.rs` covers a well-formed LibreOffice document through
    // the public API. These build the cases it cannot: story tables that are absent
    // or malformed, and separator stories that carry text. ~keep

    const FOOTNOTES: [&str; 2] = ["\u{2}First footnote\r", "\u{2}Second footnote\r"];
    const COMMENTS: [&str; 2] = ["\u{5}First comment\r", "\u{5}Second comment\r"];

    async fn extract_synthetic(plcf_hdd: PlcKind, note_plcs: PlcKind) -> InternalDocument {
        extract_synthetic_with_config(plcf_hdd, note_plcs, ExtractionConfig::default()).await
    }

    async fn extract_synthetic_with_config(
        plcf_hdd: PlcKind,
        note_plcs: PlcKind,
        config: ExtractionConfig,
    ) -> InternalDocument {
        let bytes = build_synthetic_doc(&SyntheticDoc {
            body: "Body paragraph\r",
            footnotes: &FOOTNOTES,
            header_stories: &header_doc_stories(),
            comments: &COMMENTS,
            plcf_hdd,
            note_plcs,
        });
        DocExtractor::new()
            .extract_content(&bytes, LEGACY_WORD_MIME_TYPE, &config)
            .await
            .expect("a synthetic DOC should extract without error")
    }

    fn on_layer(doc: &InternalDocument, layer: ContentLayer, needle: &str) -> bool {
        doc.elements.iter().any(|e| e.layer == layer && e.text.contains(needle))
    }

    /// `(anchor, text)` of every element of `kind`.
    fn definitions(doc: &InternalDocument, kind: ElementKind) -> Vec<(Option<&str>, &str)> {
        doc.elements
            .iter()
            .filter(|e| e.kind == kind)
            .map(|e| (e.anchor.as_deref(), e.text.as_str()))
            .collect()
    }

    fn warned(doc: &InternalDocument) -> bool {
        doc.processing_warnings.iter().any(|w| w.source == "doc")
    }

    #[tokio::test]
    async fn separator_stories_are_never_emitted() {
        let doc = extract_synthetic(PlcKind::Valid, PlcKind::Valid).await;

        // The markers are in the document, so their absence is the reader's doing.
        assert!(on_layer(&doc, ContentLayer::Header, "Odd page header"));
        for separator in SEPARATOR_STORIES {
            let marker = separator.trim_end();
            assert!(
                !doc.elements.iter().any(|e| e.text.contains(marker)),
                "separator story {marker:?} must not be emitted"
            );
        }
    }

    /// PlcfHdd missing or malformed: the header story cannot be split, so all of
    /// it is emitted as Header, with a warning and no error.
    async fn assert_header_story_falls_back(plcf_hdd: PlcKind) {
        let doc = extract_synthetic(plcf_hdd, PlcKind::Valid).await;

        for needle in ["Odd page header", "Odd page footer"] {
            assert!(on_layer(&doc, ContentLayer::Header, needle), "{needle:?} not on Header");
        }
        assert!(!doc.elements.iter().any(|e| e.layer == ContentLayer::Footer));
        assert!(
            warned(&doc),
            "the fallback must be reported: {:?}",
            doc.processing_warnings
        );
    }

    #[tokio::test]
    async fn a_missing_plcf_hdd_falls_back_to_the_whole_header_story() {
        assert_header_story_falls_back(PlcKind::Absent).await;
    }

    #[tokio::test]
    async fn a_plcf_hdd_past_the_table_stream_falls_back_to_the_whole_header_story() {
        assert_header_story_falls_back(PlcKind::OutOfRange).await;
    }

    /// A readable table with the wrong story count must not be split, or the
    /// stories after the separators are filed wrongly or dropped as separators.
    #[tokio::test]
    async fn a_plcf_hdd_with_a_partial_section_falls_back_to_the_whole_header_story() {
        assert_header_story_falls_back(PlcKind::PartialSection).await;
    }

    #[tokio::test]
    async fn ambiguous_header_footer_text_requires_both_filter_permissions() {
        for plcf_hdd in [PlcKind::Absent, PlcKind::OutOfRange, PlcKind::PartialSection] {
            for (include_headers, include_footers, expected) in [
                (false, false, false),
                (true, false, false),
                (false, true, false),
                (true, true, true),
            ] {
                let config = ExtractionConfig {
                    content_filter: Some(crate::core::config::ContentFilterConfig {
                        include_headers,
                        include_footers,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let doc = extract_synthetic_with_config(plcf_hdd, PlcKind::Valid, config).await;
                let ambiguous_text_present = doc.elements.iter().any(|element| {
                    element.text.contains("Odd page header") || element.text.contains("Odd page footer")
                });

                assert_eq!(
                    ambiguous_text_present, expected,
                    "ambiguous header/footer text with include_headers={include_headers} and include_footers={include_footers}"
                );
            }
        }
    }

    #[tokio::test]
    async fn include_footnotes_false_drops_footnotes_but_preserves_comments() {
        let config = ExtractionConfig {
            content_filter: Some(crate::core::config::ContentFilterConfig::default()),
            ..Default::default()
        };
        let doc = extract_synthetic_with_config(PlcKind::Valid, PlcKind::Valid, config).await;

        assert_eq!(definitions(&doc, ElementKind::FootnoteDefinition), Vec::new());
        assert_eq!(
            definitions(&doc, ElementKind::CommentDefinition),
            vec![(Some("cmt1"), "First comment"), (Some("cmt2"), "Second comment")]
        );
    }

    /// A table that stops before `ccp - 1` would silently lose the notes after
    /// its last entry if it were used.
    #[tokio::test]
    async fn a_note_table_that_omits_the_last_note_falls_back_to_one_definition_with_a_warning() {
        let doc = extract_synthetic(PlcKind::Valid, PlcKind::MissingLastStory).await;

        let footnotes = definitions(&doc, ElementKind::FootnoteDefinition);
        assert_eq!(footnotes.len(), 1, "{footnotes:?}");
        assert!(
            footnotes[0].1.contains("Second footnote"),
            "the uncovered note must not be dropped: {footnotes:?}"
        );
        assert!(
            warned(&doc),
            "the fallback must be reported: {:?}",
            doc.processing_warnings
        );
    }

    #[tokio::test]
    async fn missing_note_tables_make_each_note_story_one_definition_with_a_warning() {
        let doc = extract_synthetic(PlcKind::Valid, PlcKind::Absent).await;

        let footnotes = definitions(&doc, ElementKind::FootnoteDefinition);
        assert_eq!(footnotes.len(), 1, "{footnotes:?}");
        assert_eq!(footnotes[0].0, Some("fn1"));
        assert!(footnotes[0].1.contains("First footnote") && footnotes[0].1.contains("Second footnote"));
        let comments = definitions(&doc, ElementKind::CommentDefinition);
        assert_eq!(comments.len(), 1, "{comments:?}");
        assert_eq!(comments[0].0, Some("cmt1"));
        assert!(comments[0].1.contains("First comment") && comments[0].1.contains("Second comment"));
        assert!(
            warned(&doc),
            "the fallback must be reported: {:?}",
            doc.processing_warnings
        );
    }

    mod tables;
}
